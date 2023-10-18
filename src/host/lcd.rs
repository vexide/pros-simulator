use pros_sys::error as errno;

#[derive(Debug)]
pub struct AlreadyInitializedError;

#[derive(Debug, Default)]
pub struct Lcd {
    initialized: bool,
    lines: [String; Lcd::HEIGHT as usize],
}

impl Lcd {
    const HEIGHT: u32 = 8;
    const WIDTH: u32 = 50;

    fn assert_initialized(&self) -> Result<(), i32> {
        if !self.initialized {
            eprintln!("Already initialized");
            return Err(errno::ENXIO);
        }
        Ok(())
    }

    fn assert_line_in_bounds(&self, line: i32) -> Result<(), i32> {
        if line < 0 || line >= Lcd::HEIGHT as i32 {
            eprintln!("Line {line} not in bounds");
            return Err(errno::EINVAL);
        }
        Ok(())
    }

    fn assert_text_length_in_bounds(&self, text: &str) -> Result<(), i32> {
        if text.len() > Lcd::WIDTH as usize {
            eprintln!("Text too long for LCD");
            return Err(errno::EINVAL);
        }
        Ok(())
    }

    pub fn initialize(&mut self) -> Result<(), AlreadyInitializedError> {
        if self.initialized {
            return Err(AlreadyInitializedError);
        }
        self.initialized = true;
        self.draw(false);
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn set_line(&mut self, line: i32, text: &str) -> Result<(), i32> {
        self.assert_initialized()?;
        self.assert_line_in_bounds(line)?;
        self.assert_text_length_in_bounds(text)?;

        self.lines[line as usize] = text.to_string();
        self.draw(true);
        Ok(())
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

    pub fn clear(&mut self) -> Result<(), i32> {
        self.assert_initialized()?;
        for line in 0..Lcd::HEIGHT {
            self.lines[line as usize] = String::new();
        }
        self.draw(true);
        Ok(())
    }

    pub fn clear_line(&mut self, line: i32) -> Result<(), i32> {
        self.assert_initialized()?;
        self.assert_line_in_bounds(line)?;

        self.lines[line as usize] = String::new();
        self.draw(true);
        Ok(())
    }
}
