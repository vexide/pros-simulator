use std::{collections::HashMap, future::Future, sync::Arc};

use tokio::sync::Mutex;
use wasmtime::{AsContextMut, TypedFunc};

use super::{thread_local::TaskStorage, WasmAllocator};

pub struct Task {
    id: u32,
    local_storage: Option<TaskStorage>,
    task_impl: TypedFunc<(), ()>,
    priority: u32,
}

impl Task {
    fn new(id: u32, task_impl: TypedFunc<(), ()>) -> TaskHandle {
        Arc::new(Mutex::new(Self {
            id,
            local_storage: None,
            task_impl,
            priority: 0,
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
    pub fn id(&self) -> u32 {
        self.id
    }
    pub async fn start(&mut self, store: impl AsContextMut<Data = impl Send>) {
        self.task_impl.call_async(store, ()).await.unwrap();
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
    newest_task_id: u32,
    current_task: Option<TaskHandle>,
}

impl TaskPool {
    pub fn spawn(&mut self, task_impl: TypedFunc<(), ()>) -> TaskHandle {
        self.newest_task_id += 1;
        let id = self.newest_task_id;

        let task = Task::new(id, task_impl);
        self.tasks.insert(id, task.clone());
        task
    }

    pub fn by_id(&self, task_id: u32) -> Option<TaskHandle> {
        if task_id == 0 {
            return Some(self.current());
        }
        self.tasks.get(&task_id).cloned()
    }

    pub fn current(&self) -> TaskHandle {
        self.current_task
            .clone()
            .expect("using the current task may only happen while a task is being executed")
    }

    async fn highest_priority_task_ids(&self) -> Vec<u32> {
        let mut highest_priority = 0;
        let mut highest_priority_tasks = vec![];
        for task in self.tasks.values() {
            let task = task.lock().await;
            if task.priority > highest_priority {
                highest_priority = task.priority;
                highest_priority_tasks.clear();
            }
            if task.priority == highest_priority {
                highest_priority_tasks.push(task.id);
            }
        }
        highest_priority_tasks.sort();
        highest_priority_tasks
    }

    /// Switches to the next task in the task pool, if any. Returns whether there are running
    /// tasks remaining.
    ///
    /// This function will loop through the tasks in a round-robin fashion, giving each task a
    /// chance to run before looping back around to the beginning. Only tasks with the highest
    /// priority will be considered.
    pub async fn next_task(&mut self) -> bool {
        let task_candidates = self.highest_priority_task_ids().await;
        let current_task_id = self.current().lock().await.id;
        let next_task = task_candidates
            .iter()
            .find(|id| **id > current_task_id)
            .or_else(|| task_candidates.first())
            .and_then(|id| self.by_id(*id));
        self.current_task = next_task;
        self.current_task.is_some()
    }
}
