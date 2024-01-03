use std::{sync::mpsc::Receiver, time::Duration};

use anyhow::Context;
use futures::Future;
use pros_simulator_interface::SimulatorMessage;
use tokio::time::{interval, sleep};
use wasmtime::{AsContext, AsContextMut, Caller, TypedFunc};

use crate::host::{lcd::Lcd, task::TaskOptions, Host, HostCtx};

enum TaskState {
    Opcontrol,
    Auton,
    Disabled,
    CompInit,
}

async fn system_daemon_task(
    mut caller: Caller<'_, Host>,
    messages: Receiver<SimulatorMessage>,
) -> anyhow::Result<()> {
    let mut last_phase = None;
    let mut state = None;

    let host = caller.data().clone();
    let delay_interval = interval(Duration::from_millis(20));

    let mut competition_task = {
        let mut pool = caller.tasks_lock().await;
        let init_options = TaskOptions::new_global(&mut pool, &host, "initialize")?
            .name("User Initialization (PROS)");
        pool.spawn(init_options, &host.module(), &host.interface())
            .await?
    };

    while !competition_task.lock().await.is_finished() {
        sleep(Duration::from_millis(2)).await;
    }

    let cb_table = {
        let task_handle = caller.current_task().await;
        let current_task = task_handle.lock().await;
        current_task.indirect_call_table
    };

    loop {
        while let Ok(message) = messages.try_recv() {
            match message {
                SimulatorMessage::ControllerUpdate(master, partner) => {
                    let mut controllers = caller.controllers_lock().await;
                    controllers.update(master, partner);
                }
                SimulatorMessage::LcdButtonsUpdate(btns) => {
                    Lcd::press(&caller.lcd(), &mut caller, cb_table, btns).await?;
                }
                SimulatorMessage::PhaseChange(phase) => {
                    if last_phase == phase {
                        continue;
                    }
                }
            }
        }

        sleep(Duration::from_millis(2)).await;
    }
}

pub async fn system_daemon_initialize(
    host: &Host,
    messages: Receiver<SimulatorMessage>,
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
