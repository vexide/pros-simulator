//! Legacy LCD Emulator API
//!
//! ## Reference
//!
//! * `lcd_clear`
//! * `lcd_clear_line`
//! * `lcd_initialize`
//! * `lcd_is_initialized` (not implemented)
//! * `lcd_print` (not implemented)
//! * `lcd_read_buttons` (not implemented)
//! * `lcd_register_btn0_cb`
//! * `lcd_register_btn1_cb`
//! * `lcd_register_btn2_cb`
//! * `lcd_set_text`
//! * `lcd_shutdown` (not implemented)
//! * `lcd_set_background_color` (not implemented)
//! * `lcd_set_text_color` (not implemented)

use wasmtime::{Caller, Linker};

use crate::host::{memory::SharedMemoryExt, Host, HostCtx, ResultExt};

pub fn configure_llemu_api(linker: &mut Linker<Host>) -> anyhow::Result<()> {
    linker.func_wrap0_async("env", "lcd_initialize", |caller: Caller<'_, Host>| {
        Box::new(async move {
            let res = caller.lcd_lock().await.initialize();
            Ok(u32::from(res.is_ok()))
        })
    })?;

    linker.func_wrap2_async(
        "env",
        "lcd_set_text",
        |mut caller: Caller<'_, Host>, line: i32, text_ptr: u32| {
            Box::new(async move {
                let text = caller.memory().read_c_str(text_ptr)?;
                let res = caller.lcd_lock().await.set_line(line, &text);
                Ok(u32::from(res.unwrap_or_errno(&mut caller).await))
            })
        },
    )?;

    linker.func_wrap1_async(
        "env",
        "lcd_clear_line",
        |mut caller: Caller<'_, Host>, line: i32| {
            Box::new(async move {
                let res = caller.lcd_lock().await.clear_line(line);
                Ok(u32::from(res.unwrap_or_errno(&mut caller).await))
            })
        },
    )?;

    linker.func_wrap0_async("env", "lcd_clear", |mut caller: Caller<'_, Host>| {
        Box::new(async move {
            let res = caller.lcd_lock().await.clear();
            Ok(u32::from(res.unwrap_or_errno(&mut caller).await))
        })
    })?;

    for lcd_button in 0..3 {
        linker.func_wrap1_async(
            "env",
            &format!("lcd_register_btn{lcd_button}_cb"),
            move |mut caller: Caller<'_, Host>, cb: u32| {
                Box::new(async move {
                    let res = {
                        caller
                            .lcd_lock()
                            .await
                            .set_btn_press_callback(lcd_button, cb)
                    };
                    Ok(u32::from(res.unwrap_or_errno(&mut caller).await))
                })
            },
        )?;
    }

    Ok(())
}
