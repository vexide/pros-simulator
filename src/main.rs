use std::{thread::sleep, time::Duration};

use anyhow::{anyhow, Result};
use host::ErrnoExt;
use host::Host;
use host::ResultExt;
use wasmtime::*;

pub mod host;

fn main() -> Result<()> {
    // Define the WASI functions globally on the `Config`.
    let engine = Engine::default();
    let mut linker = Linker::<Host>::new(&engine);
    let mut store = Store::new(&engine, Host::default());

    // lcd_initialize
    linker.func_wrap(
        "env",
        "lcd_initialize",
        |mut caller: Caller<'_, Host>| -> anyhow::Result<u32> {
            let host = caller.data_mut();
            let res = host.lcd.initialize();

            Ok(res.is_ok().into())
        },
    )?;

    linker.func_wrap(
        "env",
        "lcd_set_text",
        |mut caller: Caller<'_, Host>, line: i32, text_ptr: u32| -> anyhow::Result<u32> {
            let memory = caller.data_mut().memory.unwrap();
            let (data, host) = memory.data_and_store_mut(&mut caller);
            let text = data
                .get(text_ptr as usize..)
                .and_then(|arr| arr.iter().position(|&x| x == 0))
                .and_then(|len| std::str::from_utf8(&data[text_ptr as usize..][..len]).ok())
                .ok_or_else(|| anyhow!("invalid UTF-8 string"))?;

            let res = host.lcd.set_line(line, text);
            Ok(res.use_errno(&mut caller).into())
        },
    )?;

    linker.func_wrap(
        "env",
        "lcd_clear_line",
        |mut caller: Caller<'_, Host>, line: i32| -> anyhow::Result<u32> {
            let host = caller.data_mut();
            let res = host.lcd.clear_line(line);
            Ok(res.use_errno(&mut caller).into())
        },
    )?;

    linker.func_wrap(
        "env",
        "lcd_clear",
        |mut caller: Caller<'_, Host>| -> anyhow::Result<u32> {
            let host = caller.data_mut();
            let res = host.lcd.clear();
            Ok(res.use_errno(&mut caller).into())
        },
    )?;

    // mutexes are currently a no-op because threads aren't implemented yet

    linker.func_wrap("env", "mutex_create", || 0u32)?;

    linker.func_wrap("env", "mutex_delete", |_mutex: u32| {})?;

    linker.func_wrap("env", "mutex_give", |_mutex: u32| -> u32 { true.into() })?;

    linker.func_wrap("env", "mutex_take", |_mutex: u32, _timeout: u32| -> u32 {
        true.into()
    })?;

    linker.func_wrap("env", "delay", |millis: u32| {
        sleep(Duration::from_millis(millis.into()));
    })?;

    linker.func_wrap("env", "__errno", |mut caller: Caller<'_, Host>| -> u32 {
        caller.errno_address()
    })?;

    linker.func_wrap(
        "env",
        "__main_argc_argv",
        |_caller: Caller<'_, Host>, _argc: u32, _argv: u32| {
            Err::<u32, _>(anyhow!("main() is not implemented in the PROS simulator"))
        },
    )?;

    // Instantiate our module with the imports we've created, and run it.
    let module = Module::from_file(&engine, "./example.wasm")?;

    let instance = linker.instantiate(&mut store, &module)?;

    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("Robot code should export memory");
    store.data_mut().memory = Some(memory);

    // Like before, we can get the run function and execute it.
    let run = instance.get_typed_func::<(), ()>(&mut store, "initialize")?;
    run.call(&mut store, ())?;

    Ok(())
}
