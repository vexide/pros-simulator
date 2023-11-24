use std::{
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use anyhow::Result;
use futures::{executor::block_on, FutureExt, Stream};
use tokio::{
    sync::{
        mpsc::{self, UnboundedReceiver},
        oneshot,
    },
    task::JoinHandle,
};

use crate::{interface::SimulatorEvent, simulate};

pub struct StreamedSimulatorEvent {
    pub inner: SimulatorEvent,
    pub unpause: Option<oneshot::Sender<()>>,
}

/// Start a simulator in a new tokio task and stream the events from it.
pub fn start_simulator(
    robot_code: PathBuf,
    require_unpause: bool,
) -> impl Stream<Item = Result<StreamedSimulatorEvent>> {
    let (tx, rx) = mpsc::unbounded_channel();

    SimulatorStream {
        finished: false,
        rx,
        future: tokio::task::spawn_blocking(move || {
            let tx = Arc::new(Mutex::new(tx));
            let res = block_on(simulate(&robot_code, {
                let tx = tx.clone();
                move |inner| {
                    if require_unpause {
                        let (tx_unpause, rx_unpause) = oneshot::channel();
                        let event = StreamedSimulatorEvent {
                            inner,
                            unpause: Some(tx_unpause),
                        };
                        tx.lock().unwrap().send(Ok(event)).unwrap();
                        _ = rx_unpause.blocking_recv();
                    } else {
                        let event = StreamedSimulatorEvent {
                            inner,
                            unpause: None,
                        };
                        tx.lock().unwrap().send(Ok(event)).unwrap();
                    }
                }
            }));
            if let Err(e) = res {
                tx.lock().unwrap().send(Err(e)).unwrap();
            }
        }),
    }
}

struct SimulatorStream {
    rx: UnboundedReceiver<Result<StreamedSimulatorEvent>>,
    finished: bool,
    future: JoinHandle<()>,
}

impl Stream for SimulatorStream {
    type Item = Result<StreamedSimulatorEvent>;
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
