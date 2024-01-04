use std::{path::Path, sync::mpsc::Receiver};

use anyhow::Result;
use host::{task::TaskPool, Host};
use interface::SimulatorInterface;
use pros_simulator_interface::{SimulatorEvent, SimulatorMessage};
use wasmtime::*;

use crate::system::system_daemon::system_daemon_initialize;

mod api;
pub mod host;
pub mod interface;
pub mod stream;
mod system;

/// Simulate the WebAssembly robot program at the given path.
///
/// # Arguments
///
/// - `robot_code`: The path to the robot program to simulate.
/// - `interface`: A callback function that will be invoked with any events that occur during
///   simulation.
/// - `messages`: Input message stream to send to the robot program. This can be used to simulate
///  controller input, LCD touch events, and more.
pub async fn simulate(
    robot_code: &Path,
    interface: impl Into<SimulatorInterface>,
    messages: Receiver<SimulatorMessage>,
) -> Result<()> {
    let interface: SimulatorInterface = interface.into();
    tracing::info!("Initializing WASM runtime");
    let engine = Engine::new(
        Config::new()
            .async_support(true)
            .wasm_threads(true)
            .debug_info(true)
            .wasm_backtrace_details(WasmBacktraceDetails::Enable),
    )
    .unwrap();

    tracing::info!("JIT compiling your robot code... 🚀");
    interface.send(SimulatorEvent::RobotCodeLoading);

    let module = Module::from_file(&engine, robot_code)?;

    let shared_memory = SharedMemory::new(&engine, MemoryType::shared(18, 16384))?;
    let host = Host::new(
        engine.clone(),
        shared_memory.clone(),
        interface.clone(),
        module.clone(),
    )?;

    system_daemon_initialize(&host, messages).await?;

    TaskPool::run_to_completion(&host).await?;
    eprintln!("All tasks are finished. ✅");
    interface.send(SimulatorEvent::RobotCodeFinished);

    Ok(())
}
