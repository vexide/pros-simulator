use std::{thread::sleep, time::Duration};

use anyhow::bail;
use anyhow::{anyhow, Result};
use host::ErrnoExt;
use host::Host;
use host::ResultExt;
use wasmtime::*;

pub mod host;

#[tokio::main]
async fn main() -> Result<()> {
    let engine = Engine::new(Config::new().async_support(true)).unwrap();
    let mut linker = Linker::<Host>::new(&engine);
    let mut store = Store::new(&engine, Host::default());

    linker.func_wrap0_async("env", "lcd_initialize", |mut caller: Caller<'_, Host>| {
        Box::new(async move {
            let mut host = caller.data_mut().lock().await;
            let res = host.lcd.initialize();
            drop(host);

            Ok(u32::from(res.is_ok()))
        })
    })?;

    linker.func_wrap2_async(
        "env",
        "lcd_set_text",
        |mut caller: Caller<'_, Host>, line: i32, text_ptr: u32| {
            Box::new(async move {
                let memory = caller.data_mut().lock().await.memory.unwrap();
                let (data, host) = memory.data_and_store_mut(&mut caller);
                let text = data
                    .get(text_ptr as usize..)
                    .and_then(|arr| arr.iter().position(|&x| x == 0))
                    .and_then(|len| std::str::from_utf8(&data[text_ptr as usize..][..len]).ok())
                    .ok_or_else(|| anyhow!("invalid UTF-8 string"))?;

                let res = host.lock().await.lcd.set_line(line, text);
                Ok(u32::from(res.use_errno(&mut caller).await))
            })
        },
    )?;

    linker.func_wrap1_async(
        "env",
        "lcd_clear_line",
        |mut caller: Caller<'_, Host>, line: i32| {
            Box::new(async move {
                let mut host = caller.data_mut().lock().await;
                let res = host.lcd.clear_line(line);
                drop(host);
                Ok(u32::from(res.use_errno(&mut caller).await))
            })
        },
    )?;

    linker.func_wrap0_async("env", "lcd_clear", |mut caller: Caller<'_, Host>| {
        Box::new(async move {
            let mut host = caller.data_mut().lock().await;
            let res = host.lcd.clear();
            drop(host);
            Ok(u32::from(res.use_errno(&mut caller).await))
        })
    })?;

    // mutexes are currently a no-op because threads aren't implemented yet

    linker.func_wrap("env", "mutex_create", || 0u32)?;

    linker.func_wrap("env", "mutex_delete", |_mutex: u32| {})?;

    linker.func_wrap("env", "mutex_give", |_mutex: u32| -> u32 { true.into() })?;

    linker.func_wrap("env", "mutex_take", |_mutex: u32, _timeout: u32| -> u32 {
        true.into()
    })?;

    linker.func_wrap2_async(
        "env",
        "pvTaskGetThreadLocalStoragePointer",
        |mut caller: Caller<'_, Host>, task_handle: u32, storage_index: i32| {
            Box::new(async move {
                let mut host = caller.data_mut().lock().await;
                let allocator = host.wasm_allocator.clone().unwrap();
                let Some(task) = host.tasks.by_id(task_handle) else {
                    bail!("invalid task handle: {task_handle}");
                };
                drop(host);

                let storage = task
                    .lock()
                    .await
                    .local_storage(&mut caller, &allocator)
                    .await;
                Ok(storage.get_address(storage_index))
            })
        },
    )?;

    linker.func_wrap1_async("env", "delay", |_caller: Caller<'_, Host>, millis: u32| {
        Box::new(async move {
            sleep(Duration::from_millis(millis.into()));
            Ok(())
        })
    })?;

    linker.func_wrap0_async("env", "__errno", |mut caller: Caller<'_, Host>| {
        Box::new(async move { caller.errno_address().await })
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

    let instance = linker.instantiate_async(&mut store, &module).await?;

    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("Robot code should export memory");
    store.data_mut().lock().await.memory = Some(memory);

    // Like before, we can get the run function and execute it.
    let run = instance.get_typed_func::<(), ()>(&mut store, "initialize")?;
    run.call_async(&mut store, ()).await?;

    Ok(())
}
