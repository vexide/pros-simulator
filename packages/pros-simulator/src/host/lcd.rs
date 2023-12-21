use std::mem::replace;

use pros_simulator_interface::{LcdLines, SimulatorEvent, LCD_HEIGHT, LCD_WIDTH};
use pros_sys::error as errno;
use wasmtime::{AsContextMut, Module, Store, Table, TypedFunc};

use super::{
    task::{TaskOptions, TaskPool},
    Host, InnerHost,
};
use crate::interface::SimulatorInterface;

#[derive(Debug)]
pub struct AlreadyInitializedError;

pub struct LcdColors {
    pub background: u32,
    pub foreground: u32,
}

pub struct Lcd {
    lines: LcdLines,
    interface: SimulatorInterface,
    initialized: bool,
    button_presses: [bool; 3],
    button_callbacks: [Option<u32>; 3],
}

impl Lcd {
    pub fn new(interface: SimulatorInterface) -> Self {
        Self {
            lines: Default::default(),
            interface,
            initialized: false,
            button_presses: [false; 3],
            button_callbacks: [None; 3],
        }
    }

    fn assert_initialized(&self) -> Result<(), i32> {
        if !self.initialized {
            tracing::error!("Not initialized");
            return Err(errno::ENXIO);
        }
        Ok(())
    }

    fn assert_line_in_bounds(&self, line: i32) -> Result<(), i32> {
        if line < 0 || line >= LCD_HEIGHT as i32 {
            tracing::error!("Line {line} not in bounds");
            return Err(errno::EINVAL);
        }
        Ok(())
    }

    fn assert_text_length_in_bounds(&self, text: &str) -> Result<(), i32> {
        if text.len() > LCD_WIDTH as usize {
            tracing::error!("Text too long for LCD");
            return Err(errno::EINVAL);
        }
        Ok(())
    }

    pub fn initialize(&mut self) -> Result<(), AlreadyInitializedError> {
        if self.initialized {
            return Err(AlreadyInitializedError);
        }
        self.initialized = true;
        self.button_presses = Default::default();
        self.button_callbacks = Default::default();
        self.interface.send(SimulatorEvent::LcdInitialized);
        Ok(())
    }

    pub fn set_line(&mut self, line: i32, text: &str) -> Result<(), i32> {
        self.assert_initialized()?;
        self.assert_line_in_bounds(line)?;
        self.assert_text_length_in_bounds(text)?;

        self.lines[line as usize] = text.to_string();
        self.interface
            .send(SimulatorEvent::LcdUpdated(self.lines.clone()));
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), i32> {
        self.assert_initialized()?;
        for line in &mut self.lines {
            line.clear();
        }
        self.interface
            .send(SimulatorEvent::LcdUpdated(self.lines.clone()));
        Ok(())
    }

    pub fn clear_line(&mut self, line: i32) -> Result<(), i32> {
        self.assert_initialized()?;
        self.assert_line_in_bounds(line)?;

        self.lines[line as usize] = String::new();
        self.interface
            .send(SimulatorEvent::LcdUpdated(self.lines.clone()));
        Ok(())
    }

    pub fn set_btn_press_callback(&mut self, button: usize, callback: u32) -> Result<(), i32> {
        self.assert_initialized()?;

        self.button_callbacks[button] = Some(callback);
        Ok(())
    }

    /// Marks certain LCD buttons as being pressed. If a button was not pressed before
    /// but is now, the callback for that button will be called.
    pub async fn press(
        &mut self,
        mut store: impl AsContextMut<Data = impl Send>,
        callback_table: Table,
        buttons: [bool; 3],
    ) -> anyhow::Result<()> {
        let previous_presses = replace(&mut self.button_presses, buttons);

        for (index, button_pressed) in self.button_presses.iter().enumerate() {
            if *button_pressed && !previous_presses[index] {
                eprintln!("Calling callback for button {}", index);
                if let Some(cb_index) = &self.button_callbacks[index] {
                    eprintln!("CB exists");
                    let callback = callback_table.get(&mut store, *cb_index).unwrap();
                    let callback = callback.funcref().unwrap().unwrap();
                    let callback = callback.typed::<(), ()>(&mut store).unwrap();
                    callback.call_async(&mut store, ()).await?;
                }
            }
        }

        Ok(())
    }
}
