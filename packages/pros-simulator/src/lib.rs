use std::{
    path::Path,
    process::exit,
    sync::{mpsc::Receiver, Arc},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use futures::Stream;
use host::{
    memory::SharedMemoryExt, task::TaskPool, thread_local::CallerExt, Host, InnerHost, ResultExt,
};
use interface::SimulatorInterface;
use pros_simulator_interface::{SimulatorEvent, SimulatorMessage};
use pros_sys::TIMEOUT_MAX;
use tokio::{sync::Mutex, time::sleep};
use wasmtime::*;

use crate::host::task::TaskOptions;

mod api;
pub mod host;
pub mod interface;
pub mod stream;

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
    messages: Option<Receiver<SimulatorMessage>>,
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
    let host = Arc::new(Mutex::new(InnerHost::new(
        engine.clone(),
        shared_memory.clone(),
        interface.clone(),
        module.clone(),
    )?));

    {
        let mut host_access = host.lock().await;

        let sensor_task = TaskOptions::new_closure(
            &mut host_access.tasks,
            &host,
            move |mut caller: Caller<'_, Host>| {
                Box::new(async move {
                    if let Some(messages) = messages {
                        loop {
                            while let Ok(message) = messages.try_recv() {
                                tracing::debug!("Received message: {:?}", message);
                                let lcd = &mut caller.data().lock().await.lcd;
                                match message {
                                    SimulatorMessage::ControllerUpdate(master, partner) => {
                                        eprintln!("Controller update: {master:?} {partner:?}");
                                    }
                                    SimulatorMessage::LcdButtonsUpdate(btns) => {
                                        eprintln!("LCD buttons update: {btns:?}");
                                        lcd.set_line(0, &format!("LCD buttons: {btns:?}"));
                                    }
                                }
                            }
                            sleep(Duration::from_millis(20)).await;
                        }
                    }

                    Ok(())
                })
            },
        )?
        .name("PROS System Daemon");

        host_access
            .tasks
            .spawn(sensor_task, &module, &interface)
            .await?;

        interface.send(SimulatorEvent::RobotCodeStarting);
        tracing::info!("Starting the init/opcontrol task... 🏁");

        let task_opts = TaskOptions::new_closure(
            &mut host_access.tasks,
            &host,
            move |mut caller: Caller<'_, Host>| {
                Box::new(async move {
                    let current_task = caller.data().lock().await.tasks.current();
                    let instance = current_task.lock().await.instance.clone();
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

        host_access
            .tasks
            .spawn(task_opts, &module, &interface)
            .await?;
    }

    TaskPool::run_to_completion(&host).await;
    tracing::info!("All tasks are finished. ✅");
    interface.send(SimulatorEvent::RobotCodeFinished);

    Ok(())
}
