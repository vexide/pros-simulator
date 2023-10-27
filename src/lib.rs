use std::path::Path;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use host::memory::SharedMemoryExt;
use host::task::TaskPool;
use host::thread_local::CallerExt;
use host::Host;
use host::InnerHost;
use host::ResultExt;
use tokio::sync::Mutex;
use tokio::time::sleep;
use wasmtime::*;

pub mod host;

pub async fn simulate(robot_code: &Path) -> Result<()> {
    let engine = Engine::new(
        Config::new()
            .async_support(true)
            .debug_info(true)
            .wasm_backtrace_details(WasmBacktraceDetails::Enable),
    )
    .unwrap();
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
                let data = caller.data_mut().lock().await;
                let memory = data.memory.clone();
                Ok(storage.get(memory, storage_index))
            })
        },
    )?;

    linker.func_wrap3_async(
        "env",
        "vTaskSetThreadLocalStoragePointer",
        |mut caller: Caller<'_, Host>, task_handle: u32, storage_index: i32, value: u32| {
            Box::new(async move {
                let mut storage = caller.task_storage(task_handle).await;
                let data = caller.data_mut().lock().await;
                let memory = data.memory.clone();
                drop(data);
                storage.set(memory, storage_index, value)
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
            sleep(Duration::from_millis(millis.into())).await;
            Ok(())
        })
    })?;

    linker.func_wrap0_async("env", "__errno", |mut caller: Caller<'_, Host>| {
        Box::new(async move {
            let data = caller.data_mut().lock().await;
            let current_task = data.tasks.current();
            drop(data);
            let errno = current_task.lock().await.errno(&mut caller).await;
            Ok(errno.address())
        })
    })?;

    linker.func_wrap0_async("env", "millis", |mut caller: Caller<'_, Host>| {
        Box::new(async move {
            let data = caller.data_mut().lock().await;
            let start_time = data.start_time;
            drop(data);
            Ok(start_time.elapsed().as_millis() as u32)
        })
    })?;

    linker.func_wrap(
        "env",
        "__main_argc_argv",
        |_caller: Caller<'_, Host>, _argc: u32, _argv: u32| {
            Err::<u32, _>(anyhow!("main() is not implemented in the PROS simulator"))
        },
    )?;

    linker.func_wrap1_async("env", "sim_abort", |caller: Caller<'_, Host>, msg: u32| {
        Box::new(async move {
            let backtrace = WasmBacktrace::force_capture(&caller);
            let data = caller.data().lock().await;
            let abort_msg = data.memory.read_c_str(msg).unwrap();
            println!("{abort_msg}");
            println!("{backtrace}");
            exit(1);
        })
    })?;

    // Instantiate our module with the imports we've created, and run it.
    let module = Module::from_file(&engine, robot_code)?;

    let instance = linker.instantiate_async(&mut store, &module).await?;

    let initialize = instance.get_typed_func::<(), ()>(&mut store, "initialize")?;
    let opcontrol = instance.get_typed_func::<(), ()>(&mut store, "opcontrol")?;
    let robot_code_runner = Func::wrap0_async(&mut store, move |mut caller: Caller<'_, Host>| {
        Box::new(async move {
            initialize.call_async(&mut caller, ()).await?;
            opcontrol.call_async(&mut caller, ()).await?;
            Ok(())
        })
    })
    .typed::<(), ()>(&mut store)
    .unwrap();

    {
        let mut host = host.lock().await;
        host.tasks.spawn(instance, store, robot_code_runner);
    }
    TaskPool::run_to_completion(&host).await;

    Ok(())
}
