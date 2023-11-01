use std::sync::{Arc, Mutex, MutexGuard};

use pros_sys::error as errno;

use crate::interface::HostInterface;

pub type LcdLines = [String; HEIGHT as usize];

#[derive(Debug)]
pub struct AlreadyInitializedError;

const HEIGHT: u32 = 8;
const WIDTH: u32 = 50;

pub struct Lcd {
    lines: LcdLines,
    interface: Arc<Mutex<HostInterface>>,
}

impl Lcd {
    pub fn new(interface: Arc<Mutex<HostInterface>>) -> Self {
        Self {
            lines: Default::default(),
            interface,
        }
    }

    #[inline]
    fn interface(&self) -> MutexGuard<'_, HostInterface> {
        self.interface.lock().unwrap()
    }

    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.interface().lcd_interface.is_some()
    }

    fn assert_initialized(&self) -> Result<(), i32> {
        if !self.is_initialized() {
            tracing::error!("Not initialized");
            return Err(errno::ENXIO);
        }
        Ok(())
    }

    fn assert_line_in_bounds(&self, line: i32) -> Result<(), i32> {
        if line < 0 || line >= HEIGHT as i32 {
            tracing::error!("Line {line} not in bounds");
            return Err(errno::EINVAL);
        }
        Ok(())
    }

    fn assert_text_length_in_bounds(&self, text: &str) -> Result<(), i32> {
        if text.len() > WIDTH as usize {
            tracing::error!("Text too long for LCD");
            return Err(errno::EINVAL);
        }
        Ok(())
    }

    pub fn initialize(&mut self) -> Result<(), AlreadyInitializedError> {
        if self.is_initialized() {
            return Err(AlreadyInitializedError);
        }
        {
            let mut interface = self.interface();
            let init_lcd = interface
                .init_lcd
                .as_mut()
                .expect("Simulator interface does not implement the LCD");
            interface.lcd_interface = Some(init_lcd());
        }
        self.draw();
        Ok(())
    }

    pub fn set_line(&mut self, line: i32, text: &str) -> Result<(), i32> {
        self.assert_initialized()?;
        self.assert_line_in_bounds(line)?;
        self.assert_text_length_in_bounds(text)?;

        self.lines[line as usize] = text.to_string();
        self.draw();
        Ok(())
    }

    pub fn draw(&self) {
        let mut interface = self.interface();
        let draw = &mut interface.lcd_interface.as_mut().unwrap().draw;
        draw(&self.lines);
    }

    pub fn clear(&mut self) -> Result<(), i32> {
        self.assert_initialized()?;
        for line in 0..HEIGHT {
            self.lines[line as usize] = String::new();
        }
        self.draw();
        Ok(())
    }

    pub fn clear_line(&mut self, line: i32) -> Result<(), i32> {
        self.assert_initialized()?;
        self.assert_line_in_bounds(line)?;

        self.lines[line as usize] = String::new();
        self.draw();
        Ok(())
    }
}
