use std::{sync::mpsc::Receiver, time::Duration};

use futures::Future;
use pros_simulator_interface::SimulatorMessage;
use tokio::time::sleep;
use wasmtime::{AsContext, AsContextMut, Caller};

use crate::host::{lcd::Lcd, task::TaskOptions, Host, HostCtx};

async fn system_daemon_task(
    mut caller: Caller<'_, Host>,
    messages: Option<Receiver<SimulatorMessage>>,
) -> anyhow::Result<()> {
    if let Some(messages) = messages {
        loop {
            while let Ok(message) = messages.try_recv() {
                match message {
                    SimulatorMessage::ControllerUpdate(master, partner) => {
                        let mut controllers = caller.controllers_lock().await;
                        controllers.update(master, partner);
                    }
                    SimulatorMessage::LcdButtonsUpdate(btns) => {
                        let table = {
                            let tasks = caller.tasks_lock().await;
                            let current_task = tasks.current_lock().await;
                            current_task.indirect_call_table
                        };
                        let lcd = caller.lcd();
                        Lcd::press(&lcd, &mut caller, table, btns).await?;
                    }
                }
            }

            sleep(Duration::from_millis(20)).await;
        }
    }

    Ok(())
}

pub async fn system_daemon_initialize(
    host: &Host,
    messages: Option<Receiver<SimulatorMessage>>,
) -> anyhow::Result<()> {
    let mut tasks = host.tasks_lock().await;

    let daemon = TaskOptions::new_closure(&mut tasks, host, |caller: Caller<'_, Host>| {
        Box::new(system_daemon_task(caller, messages))
    })?
    .name("PROS System Daemon");

    tasks
        .spawn(daemon, &host.module(), &host.interface())
        .await?;

    Ok(())
}
