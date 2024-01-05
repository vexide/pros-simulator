use std::{
    io::{stdin, stdout, BufReader},
    path::PathBuf,
    process::exit,
    sync::mpsc,
};

use clap::Parser;
use jsonl::{read, write, ReadError};
use pros_simulator_interface::SimulatorMessage;

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
        let (tx, rx) = mpsc::channel::<SimulatorMessage>();
        tokio::task::spawn_blocking(move || {
            let mut reader = BufReader::new(stdin().lock());
            loop {
                let event = read(&mut reader);
                match event {
                    Ok(message) => _ = tx.send(message),
                    Err(ReadError::Eof) => break,
                    Err(err) => {
                        eprintln!("Error reading from stdio: {}", err);
                        exit(1);
                    }
                }
            }
        });
        pros_simulator::simulate(
            &args.robot_code,
            move |event| {
                write(stdout().lock(), &event).unwrap();
            },
            rx,
        )
        .await
        .unwrap();
    } else {
        panic!("No connection method: append the --stdio flag to use stdin/stdout.")
    }
    exit(0);
}
