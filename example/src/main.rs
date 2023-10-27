#![no_main]
#![no_std]

use pros::prelude::{println, *};
use pros::task::delay;
use core::time::Duration;

pub struct SimRobot;
impl Default for SimRobot {
    fn default() -> Self {
        println!("Hello from simulator!");
        delay(Duration::from_secs(1));
        println!("Goodbye from simulator!");
        SimRobot
    }
}
impl SyncRobot for SimRobot {}
sync_robot!(SimRobot);
