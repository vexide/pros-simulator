#![no_main]
#![no_std]

use core::time::Duration;

use pros::{devices::Controller, prelude::*};

pub struct SimRobot {
    controller: Controller,
}

impl SimRobot {
    fn new() -> Self {
        println!("Hello world");
        Self {
            controller: Controller::Master,
        }
    }
}

impl SyncRobot for SimRobot {}
sync_robot!(SimRobot, SimRobot::new());
