use std::{cell::RefCell, rc::Rc, thread::sleep, time::Duration};

use anyhow::{anyhow, Result};
use host::{Host, SimulatorState};
use wasmtime::*;
use wasmtime_wasi::sync::WasiCtxBuilder;

pub mod host;

fn main() -> Result<()> {
    // Define the WASI functions globally on the `Config`.
    let engine = Engine::default();
    let mut linker = Linker::<SimulatorState>::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |s| &mut s.wasi)?;

    let host = Rc::new(RefCell::new(Host::default()));

    // Create a WASI context and put it in a Store; all instances in the store
    // share this context. `WasiCtxBuilder` provides a number of ways to
    // configure what the target program will have access to.
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_args()?
        .build();
    let state = SimulatorState { host, wasi };
    let mut store = Store::new(&engine, state);

    // lcd_initialize
    linker.func_wrap(
        "env",
        "lcd_initialize",
        |caller: Caller<'_, SimulatorState>| -> anyhow::Result<u32> {
            let mut host = caller.data().host.borrow_mut();
            let res = host.lcd.initialize();

            Ok(res.into())
        },
    )?;

    linker.func_wrap(
        "env",
        "lcd_set_text",
        |caller: Caller<'_, SimulatorState>, line: u32, ptr: u32| -> anyhow::Result<u32> {
            let mut host = caller.data().host.borrow_mut();
            let memory = host.memory.as_ref().unwrap();
            let text = memory
                .data(&caller)
                .get(ptr as usize..)
                .and_then(|arr| arr.iter().position(|&x| x == 0))
                .and_then(|len| {
                    std::str::from_utf8(&memory.data(&caller)[ptr as usize..][..len]).ok()
                })
                .ok_or_else(|| anyhow!("invalid UTF-8 string"))?;
            let res = host.lcd.set_line(line, text);

            Ok(res.into())
        },
    )?;

    linker.func_wrap(
        "env",
        "lcd_clear_line",
        |caller: Caller<'_, SimulatorState>, line: u32| -> anyhow::Result<u32> {
            let mut host = caller.data().host.borrow_mut();
            let res = host.lcd.set_line(line, "");

            Ok(res.into())
        },
    )?;

    linker.func_wrap(
        "env",
        "lcd_clear",
        |caller: Caller<'_, SimulatorState>| -> anyhow::Result<u32> {
            let mut host = caller.data().host.borrow_mut();
            let res = host.lcd.clear();

            Ok(res.into())
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

    linker.func_wrap(
        "env",
        "__main_argc_argv",
        |_caller: Caller<'_, SimulatorState>, _argc: u32, _argv: u32| {
            Err::<u32, _>(anyhow!("main() is not implemented in the PROS simulator"))
        },
    )?;

    // Instantiate our module with the imports we've created, and run it.
    let module = Module::from_file(&engine, "./example.wasm")?;

    let instance = linker.instantiate(&mut store, &module)?;

    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("Robot code should export memory");
    let host = store.data().host.clone();
    host.borrow_mut().memory = Some(memory);

    // Like before, we can get the run function and execute it.
    let run = instance.get_typed_func::<(), ()>(&mut store, "initialize")?;
    run.call(&mut store, ())?;

    Ok(())
}
