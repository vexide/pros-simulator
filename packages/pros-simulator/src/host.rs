pub mod controllers;
pub mod lcd;
pub mod memory;
pub mod multitasking;
pub mod smart_device;
pub mod task;
pub mod thread_local;

use std::{alloc::Layout, sync::Arc, time::Instant};

use async_trait::async_trait;
use lcd::Lcd;
use pros_simulator_interface::CompetitionPhase;
use tokio::sync::{Mutex, MutexGuard};
use wasmtime::{
    AsContext, AsContextMut, Caller, Engine, Instance, Module, SharedMemory, TypedFunc,
};

use self::{
    controllers::Controllers,
    multitasking::MutexPool,
    smart_device::SmartPorts,
    task::{TaskHandle, TaskPool},
};
use crate::interface::SimulatorInterface;

/// This struct contains the functions necessary to send buffers to the sandbox.
/// By letting the sandboxed allocator know that we want to write a buffer
/// it can tell us where to put it without overriding anything important
/// in the sandbox's heap.
///
/// `wasm_memalign` is used to request a place to write a buffer, and `wasm_free` is
/// used to tell the sandbox that we're done with the buffer.
#[derive(Clone)]
pub struct WasmAllocator {
    wasm_memalign: TypedFunc<(u32, u32), u32>,
    wasm_free: TypedFunc<u32, ()>,
}

impl WasmAllocator {
    pub fn new(mut store: impl AsContextMut, instance: &Instance) -> Self {
        Self {
            wasm_memalign: instance
                .get_typed_func::<(u32, u32), u32>(&mut store, "wasm_memalign")
                .unwrap(),
            wasm_free: instance
                .get_typed_func::<u32, ()>(&mut store, "wasm_free")
                .unwrap(),
        }
    }

    pub async fn memalign(
        &self,
        mut store: impl AsContextMut<Data = impl Send>,
        layout: Layout,
    ) -> u32 {
        let size = layout.size().try_into().unwrap();
        let alignment = layout.align().try_into().unwrap();
        let ptr = self
            .wasm_memalign
            .call_async(&mut store, (alignment, size))
            .await
            .unwrap();
        if ptr == 0 {
            panic!("wasm_memalign failed");
        }
        ptr
    }

    pub async fn free(&self, mut store: impl AsContextMut<Data = impl Send>, ptr: u32) {
        self.wasm_free.call_async(&mut store, ptr).await.unwrap()
    }
}

#[derive(Clone)]
pub struct Host {
    memory: SharedMemory,
    module: Module,
    /// Interface for simulator output (e.g. log messages)
    interface: SimulatorInterface,
    lcd: Arc<Mutex<Lcd>>,
    /// Pointers to mutexes created with mutex_create
    mutexes: Arc<Mutex<MutexPool>>,
    tasks: Arc<Mutex<TaskPool>>,
    controllers: Arc<Mutex<Controllers>>,
    competition_phase: Arc<Mutex<CompetitionPhase>>,
    smart_ports: Arc<Mutex<SmartPorts>>,
    start_time: Instant,
}

impl Host {
    pub fn new(
        engine: Engine,
        memory: SharedMemory,
        interface: SimulatorInterface,
        module: Module,
    ) -> anyhow::Result<Self> {
        let lcd = Lcd::new(interface.clone());
        let mutexes = MutexPool::default();
        let tasks = TaskPool::new(engine, memory.clone(), interface.clone())?;
        let controllers = Controllers::new(None, None);

        Ok(Self {
            memory,
            module,
            interface,
            lcd: Arc::new(Mutex::new(lcd)),
            mutexes: Arc::new(Mutex::new(mutexes)),
            tasks: Arc::new(Mutex::new(tasks)),
            controllers: Arc::new(Mutex::new(controllers)),
            competition_phase: Default::default(),
            smart_ports: Default::default(),
            start_time: Instant::now(),
        })
    }
}

#[async_trait]
pub trait HostCtx {
    fn memory(&self) -> SharedMemory;
    fn module(&self) -> Module;
    fn interface(&self) -> SimulatorInterface;
    fn lcd(&self) -> Arc<Mutex<Lcd>>;
    async fn lcd_lock(&self) -> MutexGuard<'_, Lcd>;
    fn mutexes(&self) -> Arc<Mutex<MutexPool>>;
    async fn mutexes_lock(&self) -> MutexGuard<'_, MutexPool>;
    fn tasks(&self) -> Arc<Mutex<TaskPool>>;
    async fn tasks_lock(&self) -> MutexGuard<'_, TaskPool>;
    fn start_time(&self) -> Instant;
    async fn current_task(&self) -> TaskHandle;
    fn controllers(&self) -> Arc<Mutex<Controllers>>;
    async fn controllers_lock(&self) -> MutexGuard<'_, Controllers>;
    fn competition_phase(&self) -> Arc<Mutex<CompetitionPhase>>;
    async fn competition_phase_lock(&self) -> MutexGuard<'_, CompetitionPhase>;
    fn smart_ports(&self) -> Arc<Mutex<SmartPorts>>;
    async fn smart_ports_lock(&self) -> MutexGuard<'_, SmartPorts>;
}

#[async_trait]
impl HostCtx for Host {
    fn memory(&self) -> SharedMemory {
        self.memory.clone()
    }

    fn module(&self) -> Module {
        self.module.clone()
    }

    fn interface(&self) -> SimulatorInterface {
        self.interface.clone()
    }

    fn lcd(&self) -> Arc<Mutex<Lcd>> {
        self.lcd.clone()
    }

    async fn lcd_lock(&self) -> MutexGuard<'_, Lcd> {
        self.lcd.lock().await
    }

    fn mutexes(&self) -> Arc<Mutex<MutexPool>> {
        self.mutexes.clone()
    }

    async fn mutexes_lock(&self) -> MutexGuard<'_, MutexPool> {
        self.mutexes.lock().await
    }

    fn tasks(&self) -> Arc<Mutex<TaskPool>> {
        self.tasks.clone()
    }

    async fn tasks_lock(&self) -> MutexGuard<'_, TaskPool> {
        self.tasks.lock().await
    }

    fn start_time(&self) -> Instant {
        self.start_time
    }

    async fn current_task(&self) -> TaskHandle {
        self.tasks.lock().await.current()
    }

    fn controllers(&self) -> Arc<Mutex<Controllers>> {
        self.controllers.clone()
    }

    async fn controllers_lock(&self) -> MutexGuard<'_, Controllers> {
        self.controllers.lock().await
    }

    fn competition_phase(&self) -> Arc<Mutex<CompetitionPhase>> {
        self.competition_phase.clone()
    }

    async fn competition_phase_lock(&self) -> MutexGuard<'_, CompetitionPhase> {
        self.competition_phase.lock().await
    }

    fn smart_ports(&self) -> Arc<Mutex<SmartPorts>> {
        self.smart_ports.clone()
    }

    async fn smart_ports_lock(&self) -> MutexGuard<'_, SmartPorts> {
        self.smart_ports.lock().await
    }
}

#[async_trait]
impl<T> HostCtx for T
where
    T: AsContext<Data = Host> + Sync,
{
    fn memory(&self) -> SharedMemory {
        self.as_context().data().memory()
    }

    fn module(&self) -> Module {
        self.as_context().data().module()
    }

    fn interface(&self) -> SimulatorInterface {
        self.as_context().data().interface()
    }

    fn lcd(&self) -> Arc<Mutex<Lcd>> {
        self.as_context().data().lcd()
    }

    async fn lcd_lock(&self) -> MutexGuard<'_, Lcd> {
        self.as_context().data().lcd_lock().await
    }

    fn mutexes(&self) -> Arc<Mutex<MutexPool>> {
        self.as_context().data().mutexes()
    }

    async fn mutexes_lock(&self) -> MutexGuard<'_, MutexPool> {
        self.as_context().data().mutexes_lock().await
    }

    fn tasks(&self) -> Arc<Mutex<TaskPool>> {
        self.as_context().data().tasks()
    }

    async fn tasks_lock(&self) -> MutexGuard<'_, TaskPool> {
        self.as_context().data().tasks_lock().await
    }

    fn start_time(&self) -> Instant {
        self.as_context().data().start_time()
    }

    async fn current_task(&self) -> TaskHandle {
        self.as_context().data().tasks_lock().await.current()
    }

    fn controllers(&self) -> Arc<Mutex<Controllers>> {
        self.as_context().data().controllers()
    }

    async fn controllers_lock(&self) -> MutexGuard<'_, Controllers> {
        self.as_context().data().controllers_lock().await
    }

    fn competition_phase(&self) -> Arc<Mutex<CompetitionPhase>> {
        self.as_context().data().competition_phase()
    }

    async fn competition_phase_lock(&self) -> MutexGuard<'_, CompetitionPhase> {
        self.as_context().data().competition_phase_lock().await
    }

    fn smart_ports(&self) -> Arc<Mutex<SmartPorts>> {
        self.as_context().data().smart_ports()
    }

    async fn smart_ports_lock(&self) -> MutexGuard<'_, SmartPorts> {
        self.as_context().data().smart_ports_lock().await
    }
}

#[async_trait]
pub trait ResultExt<T> {
    /// If this result is an error, sets the simulator's [`errno`](Host::errno_address) to the Err value.
    /// Returns `true` if the result was Ok and `false` if it was Err.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let res = lcd.set_line(line, "");
    /// Ok(res.unwrap_or_errno(&mut caller).await.into())
    /// ```
    async fn unwrap_or_errno(self, caller: &mut Caller<'_, Host>) -> bool;

    /// If this result is an error, sets the simulator's [`errno`](Host::errno_address) to the Err value.
    /// Returns the `T` value if the result was Ok and the `error_value` parameter if it was Err.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let res = controllers.get_analog(pros_sys::E_CONTROLLER_MASTER);
    /// Ok(res.unwrap_or_errno_as(&mut caller, 0).await)
    /// ```
    async fn unwrap_or_errno_as(self, caller: &mut Caller<'_, Host>, error_value: T) -> T;
}

#[async_trait]
impl<T: Send> ResultExt<T> for Result<T, i32> {
    async fn unwrap_or_errno(self, caller: &mut Caller<'_, Host>) -> bool {
        if let Err(code) = self {
            let current_task = caller.data().tasks_lock().await.current();
            let memory = caller.data().memory();
            let errno = current_task.lock().await.errno(&mut *caller).await;
            errno.set(&memory, code);
        }
        self.is_ok()
    }

    async fn unwrap_or_errno_as(self, caller: &mut Caller<'_, Host>, error_value: T) -> T {
        match self {
            Err(code) => {
                let current_task = caller.data().tasks_lock().await.current();
                let memory = caller.data().memory();
                let errno = current_task.lock().await.errno(&mut *caller).await;
                errno.set(&memory, code);
                error_value
            }
            Ok(value) => value,
        }
    }
}
