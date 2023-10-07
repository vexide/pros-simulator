use std::{alloc::Layout, collections::HashSet};
use wasmtime::{AsContextMut, Caller, Instance, Memory, TypedFunc};

pub mod lcd;

use lcd::Lcd;

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

    pub fn memalign(&self, mut store: impl AsContextMut, layout: Layout) -> u32 {
        let size = layout.size().try_into().unwrap();
        let alignment = layout.align().try_into().unwrap();
        let ptr = self
            .wasm_memalign
            .call(&mut store, (alignment, size))
            .unwrap();
        if ptr == 0 {
            panic!("wasm_memalign failed");
        }
        ptr
    }

    pub fn free(&self, mut store: impl AsContextMut, ptr: u32) {
        self.wasm_free.call(&mut store, ptr).unwrap()
    }
}

#[derive(Default)]
pub struct Host {
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

pub trait ErrnoExt {
    fn errno_address(&mut self) -> u32;
    fn set_errno(&mut self, new_errno: i32);
}

impl<'a> ErrnoExt for Caller<'a, Host> {
    fn errno_address(&mut self) -> u32 {
        self.as_context_mut()
            .data()
            .errno_address
            .unwrap_or_else(|| {
                let allocator = self.data().wasm_allocator.clone();
                let errno_address = allocator.unwrap().memalign(&mut *self, ERRNO_LAYOUT);
                self.data_mut().errno_address = Some(errno_address);
                errno_address
            })
    }
    fn set_errno(&mut self, new_errno: i32) {
        let address = self.errno_address();

        let memory = self.data().memory.unwrap().data_mut(&mut *self);
        let data = &mut memory
            .get_mut(address as usize..)
            .expect("expected valid pointer")[0..][..ERRNO_LAYOUT.size()];
        data.clone_from_slice(&new_errno.to_le_bytes());
    }
}

pub trait ResultExt {
    /// If this result is an error, sets the simulator's [`errno`](Host::errno_address) to the Err value.
    fn set_errno(&self, caller: &mut Caller<'_, Host>);
    /// If this result is an error, sets the simulator's [`errno`](Host::errno_address) to the Err value.
    /// Returns `true` if the result was Ok and `false` if it was Err.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let res = host.lcd.set_line(line, "");
    /// Ok(res.use_errno(&mut caller).into())
    /// ```
    fn use_errno(&self, caller: &mut Caller<'_, Host>) -> bool;
}

impl<T> ResultExt for Result<T, i32> {
    fn set_errno(&self, caller: &mut Caller<'_, Host>) {
        if let Err(errno) = self {
            caller.set_errno(*errno);
        }
    }
    fn use_errno(&self, caller: &mut Caller<'_, Host>) -> bool {
        self.set_errno(caller);
        self.is_ok()
    }
}
