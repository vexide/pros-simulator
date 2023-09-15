#![no_main]
use pros_sys::*;
use std::ffi::CString;

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
