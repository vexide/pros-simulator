use std::{ffi::OsString, path::PathBuf};

use futures::TryStreamExt;
use pros_simulator::stream::start_simulator;
use pros_simulator_interface::SimulatorEvent;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = std::env::args_os().collect::<Vec<_>>();
    let binary_name = args.get(1).cloned().unwrap_or_else(|| {
        OsString::from("./example/target/wasm32-unknown-unknown/debug/example.wasm")
    });
    let robot_code = PathBuf::from(binary_name);

    let mut sim = start_simulator(robot_code, false, None);

    while let Some(event) = sim.try_next().await? {
        match event.inner {
            SimulatorEvent::LcdUpdated(lines) => {
                println!("LCD updated: {lines:?}");
            }
            SimulatorEvent::LcdInitialized => {
                println!("LCD initialized");
            }
            other => {
                println!("{other:?}");
            }
        }
    }

    Ok(())
}
