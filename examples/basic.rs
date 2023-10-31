use std::path::Path;

use pros_simulator::simulate;
use tracing_subscriber::{filter::LevelFilter, EnvFilter};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
    let args = std::env::args_os().collect::<Vec<_>>();
    let binary_name = args
        .get(1)
        .unwrap_or_else(|| panic!("missing argument: need path to wasm"));
    let robot_code = Path::new(binary_name.as_os_str());
    simulate(robot_code).await.unwrap();
}
