//! Generic I/O API - undocumented/internal PROS functions that are required to support
//! miscellaneous IO like errno, the debug terminal, and panicking.
//!
//! ## Reference
//!
//! * `__errno`
//! * `sim_abort`
//!   This is a simulator-specific function that will print the given message to stderr and exit.
//! * `puts`

use std::process::exit;

use pros_simulator_interface::SimulatorEvent;
use wasmtime::{Caller, Linker, SharedMemory, Store, WasmBacktrace};

use crate::{
    host::{memory::SharedMemoryExt, Host, HostCtx, ResultExt},
    system::system_daemon::CompetitionPhaseExt,
};

pub fn configure_generic_io_api(linker: &mut Linker<Host>) -> anyhow::Result<()> {
    linker.func_wrap0_async("env", "__errno", |mut caller: Caller<'_, Host>| {
        Box::new(async move {
            let current_task = caller.current_task().await;
            let errno = current_task.lock().await.errno(&mut caller).await;
            Ok(errno.address())
        })
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

    Ok(())
}
