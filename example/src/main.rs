#![no_main]
#![no_std]

use pros::async_runtime::spawn;
use pros::prelude::{println, *};
use pros::task::sleep;
use core::time::Duration;

pub struct SimRobot;
impl Default for SimRobot {
    fn default() -> Self {
        block_on(async {
            println!("Hello from async!");
            sleep(Duration::from_secs(1)).await;
            println!("Goodbye from async!");
        });
        SimRobot
    }
}
impl Robot for SimRobot {}
robot!(SimRobot);
