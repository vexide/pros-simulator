#![no_main]
#![no_std]

extern crate alloc;

use pros::prelude::*;

pub struct SimRobot;
impl Default for SimRobot {
    fn default() -> Self {
        println!("Hello world!");
        SimRobot
    }
}
impl Robot for SimRobot {}
robot!(SimRobot);
