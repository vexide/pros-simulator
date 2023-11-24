use pros_sys::error as errno;

use crate::interface::{SimulatorEvent, SimulatorInterface};

pub type LcdLines = [String; LCD_HEIGHT as usize];

#[derive(Debug)]
pub struct AlreadyInitializedError;

pub const LCD_HEIGHT: u32 = 8;
pub const LCD_WIDTH: u32 = 50;

pub struct LcdColors {
    pub background: u32,
    pub foreground: u32,
}

pub struct Lcd {
    lines: LcdLines,
    interface: SimulatorInterface,
    initialized: bool,
}

impl Lcd {
    pub fn new(interface: SimulatorInterface) -> Self {
        Self {
            lines: Default::default(),
            interface,
            initialized: false,
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
}
