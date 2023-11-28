use std::path::PathBuf;

use clap::Parser;
use jsonl::Connection;

/// Simulate a VEX V5 robot using the PROS API interface.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Stream line delimited JSON events over stdio.
    #[clap(long)]
    stdio: bool,

    /// The robot code to simulate (WASM file).
    robot_code: PathBuf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Args::parse();

    if args.stdio {
        let mut connection = Connection::new_from_stdio();
        pros_simulator::simulate(&args.robot_code, move |event| {
            connection.write(&event).unwrap();
        })
        .await
        .unwrap();
    } else {
        panic!("No connection method: append the --stdio flag to use stdin/stdout.")
    }
}
