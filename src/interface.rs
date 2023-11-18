use std::sync::{Arc, Mutex};

use wasmtime::WasmBacktrace;

use crate::host::lcd::LcdLines;

#[derive(Debug)]
pub enum SimulatorEvent {
    Warning(String),

    RobotCodeLoading,
    RobotCodeStarted,
    RobotCodeFinished,
    RobotCodeError(String, WasmBacktrace),

    LcdInitialized,
    LcdUpdated(LcdLines),
    LcdColorsUpdated(u32, u32),
    LcdShutdown,
}

#[derive(Clone)]
pub struct SimulatorInterface {
    callback: Arc<Mutex<dyn FnMut(SimulatorEvent) + Send>>,
}

impl<T> From<T> for SimulatorInterface
where
    T: FnMut(SimulatorEvent) + Send + 'static,
{
    fn from(callback: T) -> Self {
        Self {
            callback: Arc::new(Mutex::new(callback)),
        }
    }
}

impl SimulatorInterface {
    pub(crate) fn send(&self, event: SimulatorEvent) {
        let mut callback = self.callback.lock().unwrap();
        callback(event);
    }
}
