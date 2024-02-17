use std::{
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
    sync::Arc,
    task::Poll,
};

use anyhow::{bail, Context};
use pros_simulator_interface::SimulatorEvent;
use wasmtime::{
    AsContextMut, Caller, Engine, Func, Instance, Linker, Module, SharedMemory, Store, Table,
    TypedFunc, WasmParams,
};

use super::{memory::SharedMemoryExt, thread_local::TaskStorage, Host, HostCtx, WasmAllocator};
use crate::{
    api::configure_api,
    interface::SimulatorInterface,
    mutex::{Mutex, MutexGuard},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Active and currently executing. This is the current task.
    Running,
    /// Idle and ready to resume
    Ready,
    /// Finished executing and will be removed from the task pool
    Finished,
    Blocked,
    // Suspended,
    Deleted,
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
                    let task_handle = caller.current_task().await;
                    let current_task = task_handle.lock().await;
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

    /// Create options for a task who's entrypoint is a global function from robot code.
    pub fn new_global(
        pool: &mut TaskPool,
        host: &Host,
        func_name: &'static str,
    ) -> anyhow::Result<Self> {
        Self::new_closure(pool, host, move |mut caller| {
            Box::new(async move {
                let instance = {
                    let task_handle = caller.current_task().await;
                    let this_task = task_handle.lock().await;
                    this_task.instance
                };

                let func = instance.get_func(&mut caller, func_name).with_context(|| {
                    format!("entrypoint missing: expected {func_name} to be defined")
                })?;
                let func = func
                    .typed(&mut caller)
                    .with_context(|| format!("invalid {func_name} signature: expected () -> ()"))?;

                func.call_async(&mut caller, ()).await
            })
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
    state: TaskState,
    marked_for_delete: bool,
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
            state: TaskState::Ready,
            marked_for_delete: false,
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
            task_impl.call_async(&mut **store, ()).await
        }
    }

    pub fn state(&self) -> TaskState {
        self.state
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn allocator(&self) -> WasmAllocator {
        self.allocator.clone()
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
    deleted_tasks: HashSet<u32>,
    newest_task_id: u32,
    current_task: Option<TaskHandle>,
    engine: Engine,
    shared_memory: SharedMemory,
    scheduler_suspended: u32,
    yield_pending: bool,
    shutdown_pending: bool,
    interface: SimulatorInterface,
    allow_yield: bool,
}

impl TaskPool {
    pub fn new(
        engine: Engine,
        shared_memory: SharedMemory,
        interface: SimulatorInterface,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            pool: HashMap::new(),
            deleted_tasks: HashSet::new(),
            newest_task_id: 0,
            current_task: None,
            engine,
            shared_memory,
            scheduler_suspended: 0,
            yield_pending: false,
            shutdown_pending: false,
            interface,
            allow_yield: false,
        })
    }

    pub fn create_store(&mut self, host: &Host) -> anyhow::Result<Store<Host>> {
        let store = Store::new(&self.engine, host.clone());
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
            name.unwrap_or_else(|| format!("task {id}")),
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

    #[inline]
    pub async fn yield_now() {
        futures_util::pending!();
    }

    /// Prevent context switches from happening until `resume_all` is called.
    pub fn suspend_all(&mut self) {
        self.scheduler_suspended += 1;
    }

    /// Resumes the scheduler, causing a yield if one is pending
    ///
    /// Returns whether resuming the scheduler caused a yield.
    pub async fn resume_all(&mut self) -> anyhow::Result<bool> {
        if self.scheduler_suspended == 0 {
            bail!("rtos_resume_all called without a matching rtos_suspend_all");
        }

        self.scheduler_suspended -= 1;

        if self.yield_pending && self.scheduler_suspended == 0 {
            Self::yield_now().await;
            Ok(true)
        } else {
            Ok(false)
        }
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
        if self.scheduler_suspended != 0 {
            if self.current_task.is_some() {
                self.yield_pending = true;
                return true;
            } else {
                panic!("Scheduler is suspended and current task is missing");
            }
        }
        self.yield_pending = false;

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

            let mut task = tasks.current_lock().await;
            let id = task.id();
            let future = futures.entry(id).or_insert_with(|| Box::pin(task.start()));
            drop(task);
            drop(tasks);

            let result = futures::poll!(future);

            let tasks = host.tasks();
            let mut tasks = tasks
                .try_lock()
                .expect("attempt to yield while task mutex is locked");
            let task = tasks.current();
            let mut task = task
                .try_lock()
                .expect("attempt to yield while current task is locked");

            if tasks.shutdown_pending {
                break Ok(());
            }

            if let Poll::Ready(result) = result {
                task.marked_for_delete = true;
                task.state = TaskState::Finished;
                result?;
            } else if task.marked_for_delete {
                task.state = TaskState::Deleted;
            }

            if task.marked_for_delete {
                if tasks.scheduler_suspended != 0 {
                    // task called rtos_suspend_all and ended before calling rtos_resume_all
                    tasks.interface.send(SimulatorEvent::Warning(format!(
                        "Task `{}` (#{}) exited with scheduler in suspended state",
                        &task.name, task.id,
                    )));
                }
                drop(task);

                tasks.scheduler_suspended = 0;
                futures.remove(&id);
                tasks.pool.remove(&id);
            }
        }
    }

    pub async fn task_state(&self, task_id: u32) -> Option<TaskState> {
        if self.deleted_tasks.contains(&task_id) {
            return Some(TaskState::Deleted);
        }
        if let Some(task) = self.pool.get(&task_id) {
            let task = task.lock().await;
            Some(task.state)
        } else {
            None
        }
    }

    pub async fn delete_task(&mut self, task_id: u32) {
        let task = self.pool.get(&task_id);
        if let Some(task) = task {
            let mut task = task.lock().await;
            if task.state == TaskState::Running {
                task.marked_for_delete = true;
                Self::yield_now().await;
                unreachable!("Deleted task may not continue execution");
            }

            task.state = TaskState::Deleted;
            drop(task);
            self.pool.remove(&task_id).unwrap();
            self.deleted_tasks.insert(task_id);
        }
    }

    pub fn start_shutdown(&mut self) {
        self.shutdown_pending = true;
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
