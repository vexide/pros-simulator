//! Generic I/O API - undocumented/internal PROS functions that are required to support
//! miscellaneous IO like errno, the debug terminal, and panicking.
//!
//! ## Reference
//!
//! * `__errno`
//! * `sim_abort`
//!   This is a simulator-specific function that will print the given message to stderr and exit.
//! * `sim_log_backtrace`
//!   This is a simulator-specific function that will print a backtrace to the debug terminal.
//! * `exit`
//! * `puts`

use std::process::exit;

use pros_simulator_interface::SimulatorEvent;
use wasmtime::{Caller, Linker, WasmBacktrace};

use crate::host::{memory::SharedMemoryExt, task::TaskPool, ContextExt, Host, HostCtx};

pub fn configure_generic_io_api(linker: &mut Linker<Host>) -> anyhow::Result<()> {
    linker.func_wrap0_async("env", "__errno", |mut caller: Caller<'_, Host>| {
        Box::new(async move { Ok(caller.errno_address().await) })
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
            let mut console_message = caller.memory().read_c_str(buffer).unwrap();
            console_message.push('\n');
            caller
                .interface()
                .send(SimulatorEvent::ConsoleMessage(console_message));
            u32::from(true)
        })
    })?;

    linker.func_wrap3_async(
        "env",
        "write",
        |mut caller: Caller<'_, Host>, fd: i32, buffer: u32, count: u32| {
            Box::new(async move {
                if fd < 0 || count > i32::MAX as u32 {
                    caller.set_errno(pros_sys::EINVAL).await;
                    return Ok(-1);
                }
                if fd != 1 && fd != 2 {
                    caller.set_errno(pros_sys::EBADF).await;
                    return Ok(-1);
                }

                let buffer = caller
                    .memory()
                    .read_relaxed(buffer as usize, count as usize)?;
                let buffer_string = String::from_utf8(buffer).unwrap();
                caller
                    .interface()
                    .send(SimulatorEvent::ConsoleMessage(buffer_string));
                Ok(count as i32)
            })
        },
    )?;

    linker.func_wrap1_async("env", "exit", |caller: Caller<'_, Host>, code: i32| {
        Box::new(async move {
            if code != 0 {
                caller
                    .interface()
                    .send(SimulatorEvent::ConsoleMessage(format!("Error {code}\n")));
            }
            {
                let mut tasks = caller.tasks_lock().await;
                tasks.start_shutdown();
            }
            TaskPool::yield_now().await;
            unreachable!("exit")
        })
    })?;

    linker.func_wrap0_async("env", "sim_log_backtrace", |caller: Caller<'_, Host>| {
        Box::new(async move {
            let backtrace = WasmBacktrace::force_capture(&caller);
            caller
                .interface()
                .send(SimulatorEvent::ConsoleMessage(format!("{backtrace}\n",)));
        })
    })?;

    Ok(())
}
