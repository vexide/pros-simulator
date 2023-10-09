use async_trait::async_trait;
use std::{alloc::Layout, collections::HashSet};
use tokio::sync::Mutex;
use wasmtime::{AsContextMut, Caller, Instance, Memory, TypedFunc};

pub mod lcd;
pub mod thread_local;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Task;

use lcd::Lcd;

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

pub type Host = Mutex<InnerHost>;

#[derive(Default)]
pub struct InnerHost {
    pub autonomous: Option<TypedFunc<(), ()>>,
    pub initialize: Option<TypedFunc<(), ()>>,
    pub disabled: Option<TypedFunc<(), ()>>,
    pub competition_initialize: Option<TypedFunc<(), ()>>,
    pub op_control: Option<TypedFunc<(), ()>>,
    pub memory: Option<Memory>,
    pub lcd: Lcd,
    /// Pointers to mutexes created with mutex_create
    pub mutexes: HashSet<u32>,
    pub wasm_allocator: Option<WasmAllocator>,
    pub errno_address: Option<u32>,
}

pub const ERRNO_LAYOUT: Layout = Layout::new::<i32>();

#[async_trait]
pub trait ErrnoExt {
    async fn errno_address(&mut self) -> u32;
    async fn set_errno(&mut self, new_errno: i32);
}

#[async_trait]
impl<'a> ErrnoExt for Caller<'a, Host> {
    async fn errno_address(&mut self) -> u32 {
        let data = self.data().lock().await;
        if let Some(errno_address) = data.errno_address {
            return errno_address;
        }
        let allocator = data.wasm_allocator.clone();
        drop(data);
        let errno_address = allocator.unwrap().memalign(&mut *self, ERRNO_LAYOUT).await;
        self.data_mut().lock().await.errno_address = Some(errno_address);
        errno_address
    }
    async fn set_errno(&mut self, new_errno: i32) {
        let address = self.errno_address().await;

        let memory = self.data().lock().await.memory.unwrap();
        memory
            .write(&mut *self, address as usize, &new_errno.to_le_bytes())
            .unwrap();
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
        if let Err(errno) = self {
            caller.set_errno(errno).await;
        }
        self.is_ok()
    }
}
