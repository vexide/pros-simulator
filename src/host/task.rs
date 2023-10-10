use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;
use wasmtime::{AsContextMut, TypedFunc};

use super::{thread_local::TaskStorage, WasmAllocator};

pub struct Task {
    id: u32,
    local_storage: Option<TaskStorage>,
    task_impl: TypedFunc<(), ()>,
}
impl Task {
    fn new(id: u32, task_impl: TypedFunc<(), ()>) -> TaskHandle {
        Arc::new(Mutex::new(Self {
            id,
            local_storage: None,
            task_impl,
        }))
    }
    pub async fn local_storage(
        &mut self,
        store: impl AsContextMut<Data = impl Send>,
        allocator: &WasmAllocator,
    ) -> TaskStorage {
        if let Some(storage) = self.local_storage {
            return storage;
        }
        let storage = TaskStorage::new(store, allocator).await;
        self.local_storage = Some(storage);
        storage
    }
}
impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Task {}

pub type TaskHandle = Arc<Mutex<Task>>;

#[derive(Default)]
pub struct TaskPool {
    tasks: HashMap<u32, TaskHandle>,
    next_task_id: u32,
    current_task: Option<TaskHandle>,
}

impl TaskPool {
    pub fn spawn(&mut self, task_impl: TypedFunc<(), ()>) -> TaskHandle {
        let id = self.next_task_id;
        self.next_task_id += 1;

        let task = Task::new(id, task_impl);
        self.tasks.insert(id, task.clone());
        task
    }

    pub fn by_id(&mut self, task_id: u32) -> Option<TaskHandle> {
        if task_id == 0 {
            return Some(
                self.current_task
                    .clone()
                    .expect("getting the current task should only happen while a task is being executed"),
            );
        }
        self.tasks.get(&task_id).cloned()
    }
}
