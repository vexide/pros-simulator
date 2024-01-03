use std::{path::Path, sync::mpsc::Receiver, time::Duration};

use anyhow::Result;
use host::{task::TaskPool, Host};
use interface::SimulatorInterface;
use pros_simulator_interface::{SimulatorEvent, SimulatorMessage};
use tokio::time::sleep;
use wasmtime::*;

use crate::{
    host::{lcd::Lcd, task::TaskOptions, HostCtx},
    system::system_daemon::system_daemon_initialize,
};

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

    {
        system_daemon_initialize(&host, messages).await?;

        interface.send(SimulatorEvent::RobotCodeStarting);
        tracing::info!("Starting the init/opcontrol task... 🏁");

        let task_opts = TaskOptions::new_closure(
            &mut *host.tasks_lock().await,
            &host,
            move |mut caller: Caller<'_, Host>| {
                Box::new(async move {
                    let current_task = caller.tasks_lock().await.current();
                    let instance = current_task.lock().await.instance;
                    let initialize =
                        instance.get_typed_func::<(), ()>(&mut caller, "initialize")?;
                    let opcontrol = instance.get_typed_func::<(), ()>(&mut caller, "opcontrol")?;
                    drop(current_task);

                    initialize.call_async(&mut caller, ()).await?;
                    opcontrol.call_async(&mut caller, ()).await?;

                    Ok(())
                })
            },
        )?
        .name("User Operator Control (PROS)");

        host.tasks_lock()
            .await
            .spawn(task_opts, &module, &interface)
            .await?;
    }

    TaskPool::run_to_completion(&host).await?;
    eprintln!("All tasks are finished. ✅");
    interface.send(SimulatorEvent::RobotCodeFinished);

    Ok(())
}
