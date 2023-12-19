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
    let shared_memory = SharedMemory::new(&engine, MemoryType::shared(18, 16384))?;
    let host = Arc::new(Mutex::new(InnerHost::new(
        engine.clone(),
        shared_memory.clone(),
        interface.clone(),
    )?));

    tracing::info!("JIT compiling your Rust... 🚀");
    interface.send(SimulatorEvent::RobotCodeLoading);

    let tasks = &mut host.lock().await.tasks;
    let module = Module::from_file(&engine, robot_code)?;
    let mut store = tasks.create_store(&host)?;
    let instance = tasks.instantiate(&mut store, &module, &interface).await?;

    // tasks.spawn_closure(
    //     &instance,
    //     &host,
    //     |mut caller: Caller<'_, Host>| async move {
    //         if let Some(messages) = messages {
    //             loop {
    //                 while let Ok(message) = messages.try_recv() {
    //                     tracing::debug!("Received message: {:?}", message);
    //                     match message {
    //                         SimulatorMessage::ControllerUpdate(master, partner) => {
    //                             eprintln!("Controller update: {master:?} {partner:?}");
    //                         }
    //                         SimulatorMessage::LcdButtonsUpdate(a, b, c) => {
    //                             eprintln!("LCD buttons update: {a:?} {b:?} {c:?}");
    //                         }
    //                     }
    //                 }
    //                 sleep(Duration::from_millis(20)).await;
    //             }
    //         }

    //         Ok(())
    //     },
    // )?;

    interface.send(SimulatorEvent::RobotCodeStarting);
    tracing::info!("Starting the init/opcontrol task... 🏁");

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
        host.tasks.spawn(&instance, store, robot_code_runner)?;
    }
    TaskPool::run_to_completion(&host).await;
    tracing::info!("All tasks are finished. ✅");
    interface.send(SimulatorEvent::RobotCodeFinished);

    Ok(())
}
