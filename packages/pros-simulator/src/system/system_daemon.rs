use std::{
    sync::{mpsc::Receiver, Arc},
    time::Duration,
};

use pros_simulator_interface::{CompetitionPhase, SimulatorMessage};
use tokio::{
    sync::Mutex,
    time::{interval, sleep},
};
use wasmtime::Caller;

use crate::host::{
    lcd::Lcd,
    task::{Task, TaskOptions, TaskState},
    Host, HostCtx,
};

enum UserTask {
    Opcontrol,
    Auton,
    Disabled,
    CompInit,
}

async fn spawn_user_code(
    caller: &mut Caller<'_, Host>,
    host: &Host,
    task: UserTask,
) -> anyhow::Result<Arc<Mutex<Task>>> {
    let entrypoint = match task {
        UserTask::Opcontrol => "opcontrol",
        UserTask::Auton => "autonomous",
        UserTask::Disabled => "disabled",
        UserTask::CompInit => "competition_initialize",
    };

    let name = match task {
        UserTask::Opcontrol => "User Operator Control (PROS)",
        UserTask::Auton => "User Autonomous (PROS)",
        UserTask::Disabled => "User Disabled (PROS)",
        UserTask::CompInit => "User Comp. Init. (PROS)",
    };

    let mut pool = caller.tasks_lock().await;
    let init_options = TaskOptions::new_global(&mut pool, host, entrypoint)?.name(name);
    pool.spawn(init_options, &host.module(), &host.interface())
        .await
}

async fn do_background_operations(
    caller: &mut Caller<'_, Host>,
    messages: &mut Receiver<SimulatorMessage>,
    phase: &mut Option<CompetitionPhase>,
) -> anyhow::Result<()> {
    while let Ok(message) = messages.try_recv() {
        match message {
            SimulatorMessage::ControllerUpdate(master, partner) => {
                let mut controllers = caller.controllers_lock().await;
                controllers.update(master, partner);
            }
            SimulatorMessage::LcdButtonsUpdate(btns) => {
                let cb_table = {
                    let task_handle = caller.current_task().await;
                    let current_task = task_handle.lock().await;
                    current_task.indirect_call_table
                };

                Lcd::press(&caller.lcd(), &mut *caller, cb_table, btns).await?;
            }
            SimulatorMessage::PhaseChange(new_phase) => {
                *phase = Some(new_phase);
            }
        }
    }

    Ok(())
}

async fn system_daemon_task(
    mut caller: Caller<'_, Host>,
    mut messages: Receiver<SimulatorMessage>,
) -> anyhow::Result<()> {
    let mut last_phase = None::<CompetitionPhase>;
    // let mut state = None;

    let host = caller.data().clone();

    let mut competition_task = {
        let mut pool = caller.tasks_lock().await;
        let init_options = TaskOptions::new_global(&mut pool, &host, "initialize")?
            .name("User Initialization (PROS)");
        pool.spawn(init_options, &host.module(), &host.interface())
            .await?
    };

    let mut delay = interval(Duration::from_millis(2));
    let mut phase = None;

    // wait for initialize to finish
    while competition_task.lock().await.state() != TaskState::Finished {
        do_background_operations(&mut caller, &mut messages, &mut phase).await?;
        sleep(Duration::from_millis(2)).await;
    }

    loop {
        do_background_operations(&mut caller, &mut messages, &mut phase).await?;

        if let Some(phase) = phase {
            if let Some(last_phase) = last_phase {
                if last_phase == phase {
                    continue;
                }
                if !last_phase.enabled && !phase.enabled {
                    // Don't restart the disabled task even if other bits have changed (e.g. auton bit)
                    continue;
                }
            }

            // competition initialize only runs when disabled and competition connection
            // status has changed to true
            let state = if last_phase.map(|p| !p.is_competition).unwrap_or(true)
                && phase.is_competition
                && !phase.enabled
            {
                UserTask::CompInit
            } else if !phase.enabled {
                UserTask::Disabled
            } else if phase.autonomous {
                UserTask::Auton
            } else {
                UserTask::Opcontrol
            };

            let task = competition_task.lock().await;
            if task.state() == TaskState::Ready {
                let id = task.id();
                let mut tasks = caller.tasks_lock().await;
                tasks.delete_task(id).await;
            }
            drop(task);

            last_phase = Some(phase);

            competition_task = spawn_user_code(&mut caller, &host, state).await?;
        }

        delay.tick().await;
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
