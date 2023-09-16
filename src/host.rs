use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use wasmtime::{AsContextMut, Instance, Memory, TypedFunc};
use wasmtime_wasi::WasiCtx;

#[derive(Debug, Default)]
pub struct Lcd {
    initialized: bool,
    lines: [String; Lcd::HEIGHT as usize],
}

impl Lcd {
    const HEIGHT: u32 = 8;
    const WIDTH: u32 = 50;

    pub fn initialize(&mut self) -> bool {
        if self.initialized {
            return false;
        }
        self.initialized = true;
        self.draw(false);
        true
    }

    pub fn set_line(&mut self, line: u32, text: &str) -> bool {
        if !self.initialized {
            return false;
        }
        if text.len() > Lcd::WIDTH as usize {
            return false;
        }
        if line >= Lcd::HEIGHT {
            return false;
        }
        self.lines[line as usize] = text.to_string();
        self.draw(true);
        true
    }

    pub fn draw(&self, redraw: bool) {
        if redraw {
            // move up Lcd::Height
            print!("\x1b[{}A", Lcd::HEIGHT);
        } else {
            println!("LCD Display:");
        }
        for line in &self.lines {
            println!(">{line:width$}<", width = Lcd::WIDTH as usize);
        }
    }

    pub fn clear(&mut self) -> bool {
        if !self.initialized {
            return false;
        }
        for line in 0..Lcd::HEIGHT {
            self.lines[line as usize] = String::new();
        }
        self.draw(true);
        true
    }
}

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

    pub fn memalign(&self, mut store: impl AsContextMut, alignment: u32, size: u32) -> u32 {
        self.wasm_memalign
            .call(&mut store, (alignment, size))
            .unwrap()
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
}

pub struct SimulatorState {
    pub wasi: WasiCtx,
    pub host: Rc<RefCell<Host>>,
}
