#![no_main]
use pros::prelude::{println, *};
use pros::task::sleep;
use std::time::Duration;

pub struct SimRobot;
impl Default for SimRobot {
    fn default() -> Self {
        println!("Hello, world!");
        sleep(Duration::from_secs(1));
        println!("Goodbye, world!");
        SimRobot
    }
}
impl Robot for SimRobot {}
robot!(SimRobot);
