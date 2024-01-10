use wasmtime::{Linker, SharedMemory, Store};

use crate::host::Host;

mod generic_io;
mod llemu;
mod misc;
mod motors;
mod rtos_facilities;

pub fn configure_api(
    linker: &mut Linker<Host>,
    store: &mut Store<Host>,
    shared_memory: SharedMemory,
) -> anyhow::Result<()> {
    linker.define(&mut *store, "env", "memory", shared_memory.clone())?;

    llemu::configure_llemu_api(&mut *linker)?;
    misc::configure_misc_api(&mut *linker)?;
    motors::configure_motors_api(&mut *linker)?;
    rtos_facilities::configure_rtos_facilities_api(&mut *linker)?;

    generic_io::configure_generic_io_api(&mut *linker)?;

    Ok(())
}
