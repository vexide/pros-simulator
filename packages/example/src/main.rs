#![no_main]
#![no_std]

use core::time::Duration;

use pros::prelude::*;

extern "C" {
    fn rtos_suspend_all();
}

pub struct SimRobot {
    controller: Controller,
}

impl SimRobot {
    fn new() -> Self {
        // pros::logger::ProsLogger::init().unwrap();
        pros::task::spawn(|| unsafe {
            rtos_suspend_all();
        });
        Self {
            controller: Controller::Master,
        }
    }
}

impl SyncRobot for SimRobot {}
sync_robot!(SimRobot, SimRobot::new());
