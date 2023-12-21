use std::{
    collections::HashMap,
    future::Future,
    pin::{pin, Pin},
    process::exit,
    sync::Arc,
    task::Poll,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Context};
use pros_simulator_interface::SimulatorEvent;
use pros_sys::TIMEOUT_MAX;
use tokio::{
    sync::{Mutex, MutexGuard},
    time::sleep,
};
use wasmtime::{
    AsContextMut, Caller, Engine, Func, Instance, Linker, Module, SharedMemory, Store, Table,
    TypedFunc, WasmBacktrace, WasmParams,
};

use super::{
    memory::SharedMemoryExt, thread_local::TaskStorage, Host, HostCtx, ResultExt, WasmAllocator,
};
use crate::{api::configure_api, interface::SimulatorInterface};

pub enum TaskState {
    Running,
    Idle,
    Finished,
}

pub const TASK_PRIORITIES: u32 = 16;

pub struct TaskOptions {
    priority: u32,
    store: Store<Host>,
    entrypoint: TypedFunc<(), ()>,
    name: Option<String>,
}

impl TaskOptions {
    /// Create options for a task who's entrypoint is a function from robot code.
    ///
    /// # Arguments
    ///
    /// * `pool` - The task pool to create the task in.
    /// * `host` - The host to use for the task.
    /// * `task_start` - The index of the task entrypoint in the task table.
    ///   Function pointers are transformed into indices in the `__indirect_function_table`
    ///   by the linker.
    /// * `args` - The arguments to pass to the task entrypoint.
    pub fn new_extern<P: WasmParams + 'static>(
        pool: &mut TaskPool,
        host: &Host,
        task_start: u32,
        args: P,
    ) -> anyhow::Result<Self> {
        let args = Arc::new(Mutex::new(Some(args)));
        Self::new_closure(pool, host, move |mut caller| {
            let args = args.clone();
            Box::new(async move {
                let entrypoint = {
                    let tasks = caller.tasks();
                    let tasks = tasks.lock().await;
                    let current_task = tasks.current_lock().await;
                    current_task
                        .indirect_call_table
                        .get(&mut caller, task_start)
                        .context("Task entrypoint is out of bounds")?
                };

                let entrypoint = entrypoint
                    .funcref()
                    .context("Task entrypoint is not a function")?
                    .context("Task entrypoint is NULL")?
                    .typed::<P, ()>(&mut caller)
                    .context("Task entrypoint has invalid signature")?;

                entrypoint
                    .call_async(&mut caller, args.lock().await.take().unwrap())
                    .await?;
                Ok(())
            })
        })
    }

    /// Create options for a task who's entrypoint is a custom closure created by the host.
    /// These are treated the same as "real" tasks that have entrypoints in robot code.
    pub fn new_closure(
        pool: &mut TaskPool,
        host: &Host,
        task_closure: impl for<'a> FnOnce(
                Caller<'a, Host>,
            ) -> Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>
            + Send
            + 'static,
    ) -> anyhow::Result<Self> {
        let mut store = pool.create_store(host)?;
        let task_closure = Arc::new(Mutex::new(Some(task_closure)));
        let entrypoint = Func::wrap0_async(&mut store, move |caller: Caller<'_, Host>| {
            let task_closure = task_closure.clone();
            Box::new(async move {
                let task_closure = task_closure
                    .lock()
                    .await
                    .take()
                    .expect("Expected task to only be started once");
                Pin::from(task_closure(caller)).await
            })
        })
        .typed::<(), ()>(&mut store)?;

        Ok(Self {
            priority: 7,
            entrypoint,
            store,
            name: None,
        })
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn priority(mut self, priority: u32) -> Self {
        assert!(priority < TASK_PRIORITIES);
        self.priority = priority;
        self
    }
}

pub struct Task {
    id: u32,
    name: String,
    local_storage: Option<TaskStorage>,
    task_impl: TypedFunc<(), ()>,
    priority: u32,
    errno: Option<Errno>,
    pub instance: Instance,
    allocator: WasmAllocator,
    pub indirect_call_table: Table,
    store: Arc<Mutex<Store<Host>>>,
    is_finished: bool,
}

impl Task {
    fn new(
        id: u32,
        name: String,
        mut store: Store<Host>,
        instance: Instance,
        task_impl: TypedFunc<(), ()>,
    ) -> Self {
        Self {
            id,
            name,
            local_storage: None,
            task_impl,
            priority: 0,
            errno: None,
            allocator: WasmAllocator::new(&mut store, &instance),
            indirect_call_table: instance
                .get_table(&mut store, "__indirect_function_table")
                .unwrap(),
            instance,
            store: Arc::new(Mutex::new(store)),
            is_finished: false,
        }
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

    pub fn start(&mut self) -> impl Future<Output = anyhow::Result<()>> {
        let store = self.store.clone();
        let task_impl = self.task_impl;
        async move {
            let mut store = store.lock().await;
            task_impl.call_async(&mut *store, ()).await
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
    shared_memory: SharedMemory,
}

impl TaskPool {
    pub fn new(engine: Engine, shared_memory: SharedMemory) -> anyhow::Result<Self> {
        Ok(Self {
            pool: HashMap::new(),
            newest_task_id: 0,
            current_task: None,
            engine,
            shared_memory,
        })
    }

    pub fn create_store(&mut self, host: &Host) -> anyhow::Result<Store<Host>> {
        let mut store = Store::new(&self.engine, host.clone());
        Ok(store)
    }

    pub async fn instantiate(
        &mut self,
        store: &mut Store<Host>,
        module: &Module,
        interface: &SimulatorInterface,
    ) -> anyhow::Result<Instance> {
        let mut linker = Linker::<Host>::new(&self.engine);

        configure_api(&mut linker, store, self.shared_memory.clone())?;

        for import in module.imports() {
            if linker
                .get(&mut *store, import.module(), import.name())
                .is_none()
            {
                interface.send(SimulatorEvent::Warning(format!(
                    "Unimplemented API `{}` (Robot code will crash if this is used)",
                    import.name()
                )));
            }
        }

        linker.define_unknown_imports_as_traps(module)?;
        let instance = linker.instantiate_async(store, module).await?;

        Ok(instance)
    }

    pub async fn spawn(
        &mut self,
        opts: TaskOptions,
        module: &Module,
        interface: &SimulatorInterface,
    ) -> anyhow::Result<TaskHandle> {
        let TaskOptions {
            priority,
            entrypoint,
            mut store,
            name,
            ..
        } = opts;

        let instance = self.instantiate(&mut store, module, interface).await?;

        self.newest_task_id += 1;
        let id = self.newest_task_id;

        let mut task = Task::new(
            id,
            name.unwrap_or_else(|| format!("Task {id}")),
            store,
            instance,
            entrypoint,
        );
        task.priority = priority;
        let task = Arc::new(Mutex::new(task));
        self.pool.insert(id, task.clone());
        Ok(task)
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

    pub async fn current_lock(&self) -> MutexGuard<'_, Task> {
        self.current_task
            .as_ref()
            .expect("using the current task may only happen while a task is being executed")
            .lock()
            .await
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

    pub async fn run_to_completion(host: &Host) -> anyhow::Result<()> {
        let mut futures =
            HashMap::<u32, Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>>::new();
        loop {
            let mut tasks = host.tasks_lock().await;
            let running = tasks.cycle_tasks().await;
            if !running {
                break Ok(());
            }

            let task = tasks.current().clone();
            let mut task = task.lock().await;
            let id = task.id();
            let future = futures.entry(id).or_insert_with(|| Box::pin(task.start()));
            drop((tasks, task));

            let result = futures::poll!(future);
            if let Poll::Ready(result) = result {
                futures.remove(&id);
                host.tasks_lock().await.pool.remove(&id);
                result?;
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
