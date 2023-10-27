use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use tokio::sync::Mutex;
use wasmtime::{AsContextMut, Engine, Instance, SharedMemory, Store, TypedFunc};

use super::{memory::SharedMemoryExt, thread_local::TaskStorage, Host, WasmAllocator};

pub enum TaskState {
    Running,
    Idle,
    Finished,
}

pub struct Task {
    id: u32,
    local_storage: Option<TaskStorage>,
    task_impl: TypedFunc<(), ()>,
    priority: u32,
    errno: Option<Errno>,
    instance: Instance,
    allocator: WasmAllocator,
    store: Arc<Mutex<Store<Host>>>,
    is_finished: bool,
}

impl Task {
    fn new(
        id: u32,
        mut store: Store<Host>,
        instance: Instance,
        task_impl: TypedFunc<(), ()>,
    ) -> TaskHandle {
        let allocator = WasmAllocator::new(&mut store, &instance);
        Arc::new(Mutex::new(Self {
            id,
            local_storage: None,
            task_impl,
            priority: 0,
            errno: None,
            instance,
            allocator,
            store: Arc::new(Mutex::new(store)),
            is_finished: false,
        }))
    }

    pub async fn local_storage(
        &mut self,
        store: impl AsContextMut<Data = impl Send>,
    ) -> TaskStorage {
        if let Some(storage) = self.local_storage {
            return storage;
        }
        let storage = TaskStorage::new(store, &self.allocator).await;
        self.local_storage = Some(storage);
        storage
    }

    pub async fn errno(&mut self, store: impl AsContextMut<Data = impl Send>) -> Errno {
        if let Some(errno) = self.errno {
            return errno;
        }
        let errno = Errno::new(store, &self.allocator).await;
        self.errno = Some(errno);
        errno
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn start(&mut self) -> impl Future<Output = ()> {
        let store = self.store.clone();
        let task_impl = self.task_impl.clone();
        async move {
            let mut store = store.lock().await;
            task_impl.call_async(&mut *store, ()).await.unwrap();
        }
    }

    pub fn is_finished(&self) -> bool {
        self.is_finished
    }
}
impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Task {}

pub type TaskHandle = Arc<Mutex<Task>>;

pub struct TaskPool {
    pool: HashMap<u32, TaskHandle>,
    newest_task_id: u32,
    current_task: Option<TaskHandle>,
    engine: Engine,
}

impl TaskPool {
    pub fn new(engine: Engine) -> Self {
        Self {
            pool: HashMap::new(),
            newest_task_id: 0,
            current_task: None,
            engine,
        }
    }

    pub fn spawn(
        &mut self,
        instance: Instance,
        store: Store<Host>,
        task_impl: TypedFunc<(), ()>,
    ) -> TaskHandle {
        self.newest_task_id += 1;
        let id = self.newest_task_id;

        let task = Task::new(id, store, instance, task_impl);
        self.pool.insert(id, task.clone());
        task
    }

    pub fn by_id(&self, task_id: u32) -> Option<TaskHandle> {
        if task_id == 0 {
            return Some(self.current());
        }
        self.pool.get(&task_id).cloned()
    }

    pub fn current(&self) -> TaskHandle {
        self.current_task
            .clone()
            .expect("using the current task may only happen while a task is being executed")
    }

    async fn highest_priority_task_ids(&self) -> Vec<u32> {
        let mut highest_priority = 0;
        let mut highest_priority_tasks = vec![];
        for task in self.pool.values() {
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
    pub async fn cycle_tasks(&mut self) -> bool {
        let task_candidates = self.highest_priority_task_ids().await;
        let current_task_id = if let Some(task) = &self.current_task {
            task.lock().await.id
        } else {
            0
        };
        let next_task = task_candidates
            .iter()
            .find(|id| **id > current_task_id)
            .or_else(|| task_candidates.first())
            .and_then(|id| self.by_id(*id));
        self.current_task = next_task;
        self.current_task.is_some()
    }

    pub async fn run_to_completion(host: &Host) {
        let mut futures = HashMap::<u32, Pin<Box<dyn Future<Output = ()>>>>::new();
        loop {
            let mut host_inner = host.lock().await;
            let running = host_inner.tasks.cycle_tasks().await;
            if !running {
                break;
            }

            let task = host_inner.tasks.current().clone();
            let mut task = task.lock().await;
            let id = task.id();
            let future = futures.entry(id).or_insert_with(|| Box::pin(task.start()));
            drop(host_inner);
            drop(task);

            let result = futures::poll!(future);
            if result.is_ready() {
                futures.remove(&id);
                let mut host = host.lock().await;
                host.tasks.pool.remove(&id);
            }
        }
    }

    pub async fn task_state(&self, task: Arc<Mutex<Task>>) -> Option<TaskState> {
        if let Some(current_task) = &self.current_task {
            if Arc::ptr_eq(current_task, &task) {
                return Some(TaskState::Running);
            }
        }

        let task = task.lock().await;
        if task.is_finished() {
            Some(TaskState::Finished)
        } else {
            Some(TaskState::Idle)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Errno {
    address: u32,
}

impl Errno {
    pub async fn new(
        store: impl AsContextMut<Data = impl Send>,
        allocator: &WasmAllocator,
    ) -> Self {
        let address = allocator
            .memalign(store, std::alloc::Layout::new::<i32>())
            .await;
        Self { address }
    }
    pub fn address(&self) -> u32 {
        self.address
    }
    pub fn set(&self, memory: &SharedMemory, new_errno: i32) {
        let buffer = new_errno.to_le_bytes();
        memory
            .write_relaxed(self.address as usize, &buffer)
            .unwrap();
    }
}
