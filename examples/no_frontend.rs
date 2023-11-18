use std::{ffi::OsString, path::Path};

use pros_simulator::{
    interface::{SimulatorEvent, SimulatorInterface},
    simulate,
};

#[tokio::main]
async fn main() {
    let args = std::env::args_os().collect::<Vec<_>>();
    let binary_name = args.get(1).cloned().unwrap_or_else(|| {
        OsString::from("./example/target/wasm32-unknown-unknown/debug/example.wasm")
    });
    let robot_code = Path::new(binary_name.as_os_str());

    simulate(robot_code, |event| match event {
        SimulatorEvent::LcdUpdated(lines) => {
            println!("LCD updated: {lines:?}");
        }
        SimulatorEvent::LcdInitialized => {
            println!("LCD initialized");
        }
        other => {
            println!("{other:?}");
        }
    })
    .await
    .unwrap();
}
