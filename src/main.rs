use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::{thread::sleep, time::Duration};

use anyhow::bail;
use anyhow::{anyhow, Result};
use host::memory::SharedMemoryExt;
use host::thread_local::CallerExt;
use host::Host;
use host::InnerHost;
use host::ResultExt;
use tokio::sync::Mutex;
use wasmtime::*;

pub mod host;
// pub mod runtime;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let engine = Engine::new(Config::new().async_support(true).wasm_threads(true)).unwrap();
    let shared_memory = SharedMemory::new(&engine, MemoryType::shared(18, 16384))?;
    let host = Arc::new(Mutex::new(InnerHost::new(
        engine.clone(),
        shared_memory.clone(),
    )));

    let mut linker = Linker::<Host>::new(&engine);
    let mut store = Store::new(&engine, host.clone());

    linker.define(&mut store, "env", "memory", shared_memory)?;
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
                let mut data = caller.data_mut().lock().await;
                let text = data.memory.read_c_str(text_ptr)?;
                let res = data.lcd.set_line(line, &text);
                drop(data);
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
                let storage = caller.task_storage(task_handle).await;
                Ok(storage.get_address(storage_index))
            })
        },
    )?;

    linker.func_wrap3_async(
        "env",
        "vTaskSetThreadLocalStoragePointer",
        |mut caller: Caller<'_, Host>, task_handle: u32, storage_index: i32, address: u32| {
            Box::new(async move {
                let mut storage = caller.task_storage(task_handle).await;
                let data = caller.data_mut().lock().await;
                let memory = data.memory.clone();
                drop(data);
                storage.set_address(memory, storage_index, address)
            })
        },
    )?;

    linker.func_wrap0_async("env", "task_get_current", |caller: Caller<'_, Host>| {
        Box::new(async move {
            let data = caller.data().lock().await;
            data.tasks.current().lock().await.id()
        })
    })?;

    linker.func_wrap1_async("env", "delay", |_caller: Caller<'_, Host>, millis: u32| {
        Box::new(async move {
            sleep(Duration::from_millis(millis.into()));
            Ok(())
        })
    })?;

    linker.func_wrap0_async("env", "__errno", |mut caller: Caller<'_, Host>| {
        Box::new(async move {
            let data = caller.data_mut().lock().await;
            let current_task = data.tasks.current();
            let errno = current_task.lock().await.errno().await;
            Ok(errno.address())
        })
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

    // Like before, we can get the run function and execute it.
    let initialize = instance.get_typed_func::<(), ()>(&mut store, "initialize")?;

    initialize.call_async(&mut store, ()).await?;

    Ok(())
}

// async fn initialize_robot(instance: Instance, mut store: Store<Host>) {
//     let initialize = instance
//         .get_typed_func::<(), ()>(&mut store, "initialize")
//         .unwrap();

//     {
//         let store_data = store.data().clone();
//         let mut host = store_data.lock().await;
//         let task = host.tasks.spawn(initialize);
//     }

//     let mut futures = HashMap::<u32, Box<dyn Future<Output = anyhow::Result<()>>>>::new();
//     loop {
//         let mut host = store.data_mut().lock().await;
//         let running = host.tasks.next_task().await;
//         if !running {
//             break;
//         }

//         let task = host.tasks.current();
//         let mut task = task.lock().await;
//         let future = task.start(&mut store);
//     }
// }
