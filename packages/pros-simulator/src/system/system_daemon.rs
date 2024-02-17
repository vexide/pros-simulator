use std::{
    sync::{mpsc::Receiver, Arc},
    time::Duration,
};

use pros_simulator_interface::{CompetitionPhase, SimulatorEvent, SimulatorMessage};
use pros_sys::{COMPETITION_AUTONOMOUS, COMPETITION_CONNECTED, COMPETITION_DISABLED};
use tokio::time::{interval, sleep};
use wasmtime::Caller;

use crate::{
    host::{
        lcd::Lcd,
        task::{Task, TaskOptions, TaskState},
        Host, HostCtx,
    },
    mutex::Mutex,
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
    mut ready_to_init: Option<&mut bool>,
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
                let mut phase = caller.competition_phase_lock().await;
                *phase = new_phase;
            }
            SimulatorMessage::PortsUpdate(ports) => {
                let mut smart_ports = caller.smart_ports_lock().await;
                smart_ports.update_specs(&ports);
            }
            SimulatorMessage::BeginSimulation => {
                if let Some(ready_to_init) = ready_to_init.as_deref_mut() {
                    *ready_to_init = true;
                }
            }
        }
    }

    Ok(())
}

async fn system_daemon_task(
    mut caller: Caller<'_, Host>,
    mut messages: Receiver<SimulatorMessage>,
) -> anyhow::Result<()> {
    let mut status = None::<CompetitionPhase>;
    // let mut state = None;

    let host = caller.data().clone();

    let mut ready_to_init = false;

    // wait for initialize to finish
    while !ready_to_init {
        do_background_operations(&mut caller, &mut messages, Some(&mut ready_to_init)).await?;
        sleep(Duration::from_millis(2)).await;
    }

    host.interface().send(SimulatorEvent::RobotCodeRunning);

    let mut competition_task = {
        let mut pool = caller.tasks_lock().await;
        let init_options = TaskOptions::new_global(&mut pool, &host, "initialize")?
            .name("User Initialization (PROS)");
        pool.spawn(init_options, &host.module(), &host.interface())
            .await?
    };

    let mut delay = interval(Duration::from_millis(2));

    // wait for initialize to finish
    while competition_task.lock().await.state() != TaskState::Finished {
        do_background_operations(&mut caller, &mut messages, None).await?;
        sleep(Duration::from_millis(2)).await;
    }

    loop {
        do_background_operations(&mut caller, &mut messages, None).await?;

        let new_status = *caller.competition_phase_lock().await;

        if status.is_none() || status != Some(new_status) {
            let old_status = status.unwrap_or_default();
            status = Some(new_status);

            if !new_status.enabled && !old_status.enabled {
                // Don't restart the disabled task even if other bits have changed (e.g. auton bit)
                continue;
            }

            // competition initialize only runs when disabled and competition connection
            // status has changed to true
            let state =
                if !old_status.is_competition && new_status.is_competition && !new_status.enabled {
                    UserTask::CompInit
                } else if !new_status.enabled {
                    UserTask::Disabled
                } else if new_status.autonomous {
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

pub trait CompetitionPhaseExt {
    fn as_bits(&self) -> u8;
}

impl CompetitionPhaseExt for CompetitionPhase {
    fn as_bits(&self) -> u8 {
        let mut bits = 0;

        if self.autonomous {
            bits |= COMPETITION_AUTONOMOUS;
        }

        if !self.enabled {
            bits |= COMPETITION_DISABLED;
        }

        if self.is_competition {
            bits |= COMPETITION_CONNECTED;
        }

        bits
    }
}
