use std::{
    process::exit,
    time::{Duration, Instant},
};

use pros_simulator_interface::SimulatorEvent;
use pros_sys::TIMEOUT_MAX;
use tokio::time::sleep;
use wasmtime::{Caller, Linker, SharedMemory, Store, WasmBacktrace};

use crate::host::{
    memory::SharedMemoryExt, task::TaskOptions, thread_local::GetTaskStorage, Host, HostCtx,
    ResultExt,
};

pub fn configure_api(
    linker: &mut Linker<Host>,
    store: &mut Store<Host>,
    shared_memory: SharedMemory,
) -> anyhow::Result<()> {
    linker.define(&mut *store, "env", "memory", shared_memory.clone())?;

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
                Ok(u32::from(res.use_errno(&mut caller).await))
            })
        },
    )?;

    linker.func_wrap1_async(
        "env",
        "lcd_clear_line",
        |mut caller: Caller<'_, Host>, line: i32| {
            Box::new(async move {
                let res = caller.lcd_lock().await.clear_line(line);
                Ok(u32::from(res.use_errno(&mut caller).await))
            })
        },
    )?;

    linker.func_wrap0_async("env", "lcd_clear", |mut caller: Caller<'_, Host>| {
        Box::new(async move {
            let res = caller.lcd_lock().await.clear();
            Ok(u32::from(res.use_errno(&mut caller).await))
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
                    Ok(u32::from(res.use_errno(&mut caller).await))
                })
            },
        )?;
    }

    linker.func_wrap0_async("env", "mutex_create", |caller: Caller<'_, Host>| {
        Box::new(async move {
            let mutex_id = caller.mutexes_lock().await.create_mutex();
            Ok(mutex_id as u32)
        })
    })?;

    linker.func_wrap1_async(
        "env",
        "mutex_delete",
        |caller: Caller<'_, Host>, mutex_id: u32| {
            Box::new(async move {
                caller.mutexes_lock().await.delete_mutex(mutex_id as usize);
                Ok(())
            })
        },
    )?;

    linker.func_wrap1_async(
        "env",
        "mutex_give",
        |caller: Caller<'_, Host>, mutex_id: u32| {
            Box::new(async move {
                caller.mutexes_lock().await.unlock(mutex_id as usize);

                Ok(u32::from(true))
            })
        },
    )?;

    linker.func_wrap2_async(
        "env",
        "mutex_take",
        |caller: Caller<'_, Host>, mutex_id: u32, timeout: u32| {
            Box::new(async move {
                let timeout = (timeout != TIMEOUT_MAX)
                    .then(|| Instant::now() + Duration::from_millis(timeout.into()));
                let success = caller
                    .mutexes_lock()
                    .await
                    .lock(mutex_id as usize, timeout)
                    .await;
                Ok(u32::from(success))
            })
        },
    )?;

    linker.func_wrap2_async(
        "env",
        "pvTaskGetThreadLocalStoragePointer",
        |mut caller: Caller<'_, Host>, task_handle: u32, storage_index: i32| {
            Box::new(async move {
                let storage = caller.task_storage(task_handle).await;
                Ok(storage.get(caller.memory(), storage_index))
            })
        },
    )?;

    linker.func_wrap3_async(
        "env",
        "vTaskSetThreadLocalStoragePointer",
        |mut caller: Caller<'_, Host>, task_handle: u32, storage_index: i32, value: u32| {
            Box::new(async move {
                let mut storage = caller.task_storage(task_handle).await;
                storage.set(caller.memory(), storage_index, value)
            })
        },
    )?;

    linker.func_wrap0_async("env", "task_get_current", |caller: Caller<'_, Host>| {
        #[allow(clippy::let_and_return)]
        Box::new(async move {
            let current = caller.current_task().await;

            let id = current.lock().await.id();
            // fixing warning causes compile error
            id
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
            let current_task = caller.current_task().await;
            let errno = current_task.lock().await.errno(&mut caller).await;
            Ok(errno.address())
        })
    })?;

    linker.func_wrap0_async("env", "millis", |caller: Caller<'_, Host>| {
        Box::new(async move { Ok(caller.start_time().elapsed().as_millis() as u32) })
    })?;

    linker.func_wrap1_async("env", "sim_abort", |caller: Caller<'_, Host>, msg: u32| {
        Box::new(async move {
            let backtrace = WasmBacktrace::force_capture(&caller);
            let abort_msg = caller.memory().read_c_str(msg).unwrap();
            eprintln!("{abort_msg}");
            eprintln!("{backtrace}");
            exit(1);
        })
    })?;

    linker.func_wrap1_async("env", "puts", |caller: Caller<'_, Host>, buffer: u32| {
        Box::new(async move {
            let console_message = caller.memory().read_c_str(buffer).unwrap();
            caller
                .interface()
                .send(SimulatorEvent::ConsoleMessage(console_message));
            u32::from(true)
        })
    })?;

    // task_t task_create ( task_fn_t function,
    //     void* parameters,
    //      uint8_t prio,
    //      uint16_t stack_depth,
    //      const char* name )
    linker.func_wrap5_async(
        "env",
        "task_create",
        |caller: Caller<'_, Host>,
         function: u32,
         parameters: u32,
         priority: u32,
         _stack_depth: u32,
         _name: u32| {
            Box::new(async move {
                let mut tasks = caller.tasks_lock().await;
                let opts =
                    TaskOptions::new_extern(&mut tasks, caller.data(), function, parameters)?
                        .priority(priority - 1);
                let task = tasks
                    .spawn(opts, &caller.module(), &caller.interface())
                    .await?;

                let task = task.lock().await;
                Ok(task.id())
            })
        },
    )?;

    Ok(())
}
