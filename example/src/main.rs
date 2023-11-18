#![no_main]
#![no_std]

use core::time::Duration;

use pros::{
    prelude::{println, *},
    task::delay,
};

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
