pub mod lcd;
pub mod memory;
pub mod multitasking;
pub mod task;
pub mod thread_local;

use std::{
    alloc::Layout,
    sync::{mpsc::Receiver, Arc},
    time::Instant,
};

use async_trait::async_trait;
use futures::Stream;
use lcd::Lcd;
use pros_simulator_interface::SimulatorMessage;
use tokio::sync::Mutex;
use wasmtime::{
    AsContextMut, Caller, Config, Engine, Instance, MemoryType, Module, SharedMemory, TypedFunc,
    WasmBacktraceDetails,
};

use self::{multitasking::MutexPool, task::TaskPool};
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

pub type Host = Arc<Mutex<InnerHost>>;

pub struct InnerHost {
    pub memory: SharedMemory,
    pub lcd: Lcd,
    /// Pointers to mutexes created with mutex_create
    pub mutexes: MutexPool,
    pub tasks: TaskPool,
    pub start_time: Instant,
    /// Interface for simulator output (e.g. log messages)
    pub interface: SimulatorInterface,
    pub module: Module,
}

impl InnerHost {
    pub fn new(
        engine: Engine,
        memory: SharedMemory,
        interface: SimulatorInterface,
        module: Module,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            tasks: TaskPool::new(engine, memory.clone())?,
            memory,
            lcd: Lcd::new(interface.clone()),
            mutexes: MutexPool::default(),
            start_time: Instant::now(),
            interface,
            module,
        })
    }
}
#[async_trait]
pub trait ResultExt {
    /// If this result is an error, sets the simulator's [`errno`](Host::errno_address) to the Err value.
    /// Returns `true` if the result was Ok and `false` if it was Err.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let res = host.lcd.set_line(line, "");
    /// Ok(res.use_errno(&mut caller).await.into())
    /// ```
    async fn use_errno(self, caller: &mut Caller<'_, Host>) -> bool;
}

#[async_trait]
impl<T: Send> ResultExt for Result<T, i32> {
    async fn use_errno(self, caller: &mut Caller<'_, Host>) -> bool {
        if let Err(code) = self {
            let data = caller.data_mut().lock().await;
            let current_task = data.tasks.current();
            let memory = data.memory.clone();
            drop(data);
            let errno = current_task.lock().await.errno(caller).await;
            errno.set(&memory, code);
        }
        self.is_ok()
    }
}
