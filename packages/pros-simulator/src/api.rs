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

mod generic_io;
mod llemu;
mod misc;
mod rtos_facilities;

pub fn configure_api(
    linker: &mut Linker<Host>,
    store: &mut Store<Host>,
    shared_memory: SharedMemory,
) -> anyhow::Result<()> {
    linker.define(&mut *store, "env", "memory", shared_memory.clone())?;

    llemu::configure_llemu_api(&mut *linker)?;
    misc::configure_misc_api(&mut *linker)?;
    rtos_facilities::configure_rtos_facilities_api(&mut *linker)?;

    generic_io::configure_generic_io_api(&mut *linker)?;

    Ok(())
}
