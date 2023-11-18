use std::{
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use anyhow::Result;
use futures::{executor::block_on, FutureExt, Stream};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver},
    task::JoinHandle,
};

use crate::{interface::SimulatorEvent, simulate};

/// Start a simulator in a new tokio task and stream the events from it.
pub fn start_simulator(robot_code: PathBuf) -> impl Stream<Item = Result<SimulatorEvent>> {
    let (tx, rx) = mpsc::unbounded_channel();

    SimulatorStream {
        finished: false,
        rx,
        future: tokio::task::spawn_blocking(move || {
            let tx = Arc::new(Mutex::new(tx));
            let res = block_on(simulate(&robot_code, {
                let tx = tx.clone();
                move |event| {
                    tx.lock().unwrap().send(Ok(event)).unwrap();
                }
            }));
            if let Err(e) = res {
                tx.lock().unwrap().send(Err(e)).unwrap();
            }
        }),
    }
}

struct SimulatorStream {
    rx: UnboundedReceiver<Result<SimulatorEvent>>,
    finished: bool,
    future: JoinHandle<()>,
}

impl Stream for SimulatorStream {
    type Item = Result<SimulatorEvent>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let sim = self.get_mut();

        if !sim.finished {
            if let Poll::Ready(res) = sim.future.poll_unpin(cx) {
                if let Err(err) = res {
                    return Poll::Ready(Some(Err(err.into())));
                }
                sim.finished = true;
            }
        }

        sim.rx.poll_recv(cx)
    }
}
