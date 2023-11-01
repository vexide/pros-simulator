use crate::host::lcd::LcdLines;

#[allow(clippy::type_complexity)] // it's not that bad... right?
#[derive(Default)]
pub struct HostInterface {
    pub init_lcd: Option<Box<dyn Send + FnMut() -> LcdInterface>>,
    pub lcd_interface: Option<LcdInterface>,
}

impl HostInterface {
    pub fn lcd(mut self, init_lcd: impl 'static + Send + Fn() -> LcdInterface) -> Self {
        self.init_lcd = Some(Box::new(init_lcd));
        self
    }
}

#[allow(clippy::type_complexity)]
pub struct LcdInterface {
    pub draw: Box<dyn Send + FnMut(&LcdLines)>,
}

impl LcdInterface {
    pub fn new(draw: impl 'static + Send + Fn(&LcdLines)) -> Self {
        Self {
            draw: Box::new(draw),
        }
    }
}
