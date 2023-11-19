#![no_main]
#![no_std]

use core::time::Duration;

use pros::{prelude::*, task::delay};

#[derive(Default)]
pub struct SimRobot;

impl SyncRobot for SimRobot {
    fn opcontrol(&mut self) -> pros::Result {
        delay(Duration::from_secs(2));
        println!("Hello from simulator!");
        delay(Duration::from_secs(3));
        println!("Goodbye from simulator!");
        delay(Duration::from_secs(1));
        Ok(())
    }
}
sync_robot!(SimRobot);
