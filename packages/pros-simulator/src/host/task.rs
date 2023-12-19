use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    process::exit,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail};
use pros_simulator_interface::SimulatorEvent;
use pros_sys::TIMEOUT_MAX;
use tokio::{sync::Mutex, time::sleep};
use wasmtime::{
    AsContextMut, Caller, Engine, Func, Instance, Linker, Module, SharedMemory, Store, TypedFunc,
    WasmBacktrace,
};

use super::{
    memory::SharedMemoryExt,
    thread_local::{CallerExt, TaskStorage},
    Host, ResultExt, WasmAllocator,
};
use crate::interface::SimulatorInterface;

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
    // instance: Instance,
    allocator: WasmAllocator,
    store: Arc<Mutex<Store<Host>>>,
    is_finished: bool,
}

impl Task {
    fn new(
        id: u32,
        mut store: Store<Host>,
        instance: &Instance,
        task_impl: TypedFunc<(), ()>,
    ) -> TaskHandle {
        Arc::new(Mutex::new(Self {
            id,
            local_storage: None,
            task_impl,
            priority: 0,
            errno: None,
            allocator: WasmAllocator::new(&mut store, instance),
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
        let task_impl = self.task_impl;
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

        linker.define(&mut *store, "env", "memory", self.shared_memory.clone())?;

        linker.func_wrap0_async("env", "lcd_initialize", |mut caller: Caller<'_, Host>| {
            Box::new(async move {
                let mut host = caller.data().lock().await;
                let res = host.lcd.initialize();
                drop(host);

                Ok(u32::from(res.is_ok()))
            })
        })?;

        linker.func_wrap2_async(
            "env",
            "lcd_set_text",
            |mut caller: Caller<'_, Host>, line: i32, text_ptr: u32| {
                Box::new(async move {
                    let mut data = caller.data().lock().await;
                    let text = data.memory.read_c_str(text_ptr)?;
                    let res = data.lcd.set_line(line, &text);
                    drop(data);
                    Ok(u32::from(res.use_errno(&mut caller).await))
                })
            },
        )?;

        linker.func_wrap1_async(
            "env",
            "lcd_clear_line",
            |mut caller: Caller<'_, Host>, line: i32| {
                Box::new(async move {
                    let mut host = caller.data().lock().await;
                    let res = host.lcd.clear_line(line);
                    drop(host);
                    Ok(u32::from(res.use_errno(&mut caller).await))
                })
            },
        )?;

        linker.func_wrap0_async("env", "lcd_clear", |mut caller: Caller<'_, Host>| {
            Box::new(async move {
                let mut host = caller.data().lock().await;
                let res = host.lcd.clear();
                drop(host);
                Ok(u32::from(res.use_errno(&mut caller).await))
            })
        })?;

        // for lcd_button in 0..3 {
        //     linker.func_wrap1_async(
        //         "env",
        //         &format!("lcd_register_btn{lcd_button}_cb"),
        //         move |mut caller: Caller<'_, Host>, cb: Option<Func>| {
        //             Box::new(async move {
        //                 if let Some(cb) = cb {
        //                     let cb = cb.typed(&mut caller)?;
        //                     let res = {
        //                         let mut host = caller.data().lock().await;
        //                         host.lcd.set_btn_press_callback(lcd_button, cb)
        //                     };
        //                     Ok(u32::from(res.use_errno(&mut caller).await))
        //                 } else {
        //                     bail!("Expected non-null callback")
        //                 }
        //             })
        //         },
        //     )?;
        // }

        linker.func_wrap0_async("env", "mutex_create", |mut caller: Caller<'_, Host>| {
            Box::new(async move {
                let mut host = caller.data().lock().await;
                let mutex_id = host.mutexes.create_mutex();
                Ok(mutex_id as u32)
            })
        })?;

        linker.func_wrap1_async(
            "env",
            "mutex_delete",
            |mut caller: Caller<'_, Host>, mutex_id: u32| {
                Box::new(async move {
                    let mut host = caller.data().lock().await;
                    host.mutexes.delete_mutex(mutex_id as usize);
                    Ok(())
                })
            },
        )?;

        linker.func_wrap1_async(
            "env",
            "mutex_give",
            |mut caller: Caller<'_, Host>, mutex_id: u32| {
                Box::new(async move {
                    let mut host = caller.data().lock().await;
                    host.mutexes.unlock(mutex_id as usize);

                    Ok(u32::from(true))
                })
            },
        )?;

        linker.func_wrap2_async(
            "env",
            "mutex_take",
            |mut caller: Caller<'_, Host>, mutex_id: u32, timeout: u32| {
                Box::new(async move {
                    let mut host = caller.data().lock().await;
                    let timeout = (timeout != TIMEOUT_MAX)
                        .then(|| Instant::now() + Duration::from_millis(timeout.into()));
                    let success = host.mutexes.lock(mutex_id as usize, timeout).await;
                    Ok(u32::from(success))
                })
            },
        )?;

        linker.func_wrap2_async(
            "env",
            "pvTaskGetThreadLocalStoragePointer",
            |mut caller: Caller<'_, Host>, task_handle: u32, storage_index: i32| {
                Box::new(async move {
                    let storage = caller.task_storage(task_handle).await;
                    let data = caller.data().lock().await;
                    let memory = data.memory.clone();
                    Ok(storage.get(memory, storage_index))
                })
            },
        )?;

        linker.func_wrap3_async(
            "env",
            "vTaskSetThreadLocalStoragePointer",
            |mut caller: Caller<'_, Host>, task_handle: u32, storage_index: i32, value: u32| {
                Box::new(async move {
                    let mut storage = caller.task_storage(task_handle).await;
                    let data = caller.data().lock().await;
                    let memory = data.memory.clone();
                    drop(data);
                    storage.set(memory, storage_index, value)
                })
            },
        )?;

        linker.func_wrap0_async("env", "task_get_current", |caller: Caller<'_, Host>| {
            Box::new(async move {
                let data = caller.data().lock().await;
                data.tasks.current().lock().await.id()
            })
        })?;

        linker.func_wrap1_async("env", "delay", |_caller: Caller<'_, Host>, millis: u32| {
            Box::new(async move {
                sleep(Duration::from_millis(millis.into())).await;
                Ok(())
            })
        })?;

        linker.func_wrap0_async("env", "__errno", |mut caller: Caller<'_, Host>| {
            Box::new(async move {
                let data = caller.data().lock().await;
                let current_task = data.tasks.current();
                drop(data);
                let errno = current_task.lock().await.errno(&mut caller).await;
                Ok(errno.address())
            })
        })?;

        linker.func_wrap0_async("env", "millis", |mut caller: Caller<'_, Host>| {
            Box::new(async move {
                let data = caller.data().lock().await;
                let start_time = data.start_time;
                drop(data);
                Ok(start_time.elapsed().as_millis() as u32)
            })
        })?;

        linker.func_wrap(
            "env",
            "__main_argc_argv",
            |_caller: Caller<'_, Host>, _argc: u32, _argv: u32| {
                Err::<u32, _>(anyhow!("main() is not implemented in the PROS simulator"))
            },
        )?;

        linker.func_wrap1_async("env", "sim_abort", |caller: Caller<'_, Host>, msg: u32| {
            Box::new(async move {
                let backtrace = WasmBacktrace::force_capture(&caller);
                let data = caller.data().lock().await;
                let abort_msg = data.memory.read_c_str(msg).unwrap();
                eprintln!("{abort_msg}");
                eprintln!("{backtrace}");
                exit(1);
            })
        })?;

        linker.func_wrap1_async("env", "puts", |caller: Caller<'_, Host>, buffer: u32| {
            Box::new(async move {
                let data = caller.data().lock().await;
                let console_message = data.memory.read_c_str(buffer).unwrap();
                data.interface
                    .send(SimulatorEvent::ConsoleMessage(console_message));
                u32::from(true)
            })
        })?;

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

    pub fn spawn(
        &mut self,
        instance: &Instance,
        store: Store<Host>,
        task_impl: TypedFunc<(), ()>,
    ) -> anyhow::Result<TaskHandle> {
        self.newest_task_id += 1;
        let id = self.newest_task_id;

        let task = Task::new(id, store, instance, task_impl);
        self.pool.insert(id, task.clone());
        Ok(task)
    }

    pub fn spawn_closure<T, R>(
        &mut self,
        instance: &Instance,
        host: &Host,
        task_closure: T,
    ) -> anyhow::Result<TaskHandle>
    where
        T: 'static + Send + FnOnce(Caller<'_, Host>) -> R,
        R: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let mut store = self.create_store(host)?;
        let task_closure = Arc::new(Mutex::new(Some(task_closure)));
        let task_impl = Func::wrap0_async(&mut store, move |caller: Caller<'_, Host>| {
            let task_closure = task_closure.clone();
            Box::new(async move {
                let task_closure = task_closure
                    .lock()
                    .await
                    .take()
                    .expect("Expected task to only be started once");
                task_closure(caller).await?;
                Ok(())
            })
        })
        .typed::<(), ()>(&mut store)
        .unwrap();
        self.spawn(instance, store, task_impl)
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
        let mut futures = HashMap::<u32, Pin<Box<dyn Future<Output = ()> + Send>>>::new();
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
