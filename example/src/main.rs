#![no_main]
use std::{ffi::{c_char, CString}, thread::sleep, time::Duration};

#[link(wasm_import_module = "pros")]
extern "C" {
    pub fn lcd_initialize() -> bool;
    pub fn lcd_set_text(line: u16, text: *const c_char) -> bool;
    pub fn lcd_clear_line(line: u16) -> bool;
    pub fn lcd_clear() -> bool;
    pub fn delay(millis: u32);
}

#[no_mangle]
pub extern "C" fn initialize() {
    unsafe {
        lcd_initialize();
        delay(1000);
        let text = CString::new("Hello, world!").unwrap();
        lcd_set_text(0, text.as_ptr());
        delay(1000);
        let text = CString::new("This is from inside a simulator!").unwrap();
        lcd_set_text(1, text.as_ptr());
        delay(1000);
        let text = CString::new("Wow!").unwrap();
        lcd_set_text(2, text.as_ptr());
        delay(1000);
    }
}
