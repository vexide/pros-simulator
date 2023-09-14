use std::{cell::RefCell, rc::Rc};

use wasmtime::{TypedFunc, Memory};
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

#[derive(Default)]
pub struct Host {
    pub autonomous: Option<TypedFunc<(), ()>>,
    pub initialize: Option<TypedFunc<(), ()>>,
    pub disabled: Option<TypedFunc<(), ()>>,
    pub competition_initialize: Option<TypedFunc<(), ()>>,
    pub op_control: Option<TypedFunc<(), ()>>,
    pub memory: Option<Memory>,
    pub lcd: Lcd,
}

pub struct SimulatorState {
    pub wasi: WasiCtx,
    pub host: Rc<RefCell<Host>>,
}
