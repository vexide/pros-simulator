#![no_main]
#![no_std]

use core::time::Duration;

use pros::prelude::*;

pub struct SimRobot;

impl SimRobot {
    fn new() -> Self {
        pros::logger::ProsLogger::init().unwrap();
        pros::lcd::buttons::register(
            || {
                println!("Left button pressed!");
            },
            Button::Left,
        );
        Self
    }
}

impl Robot for SimRobot {}
robot!(SimRobot, SimRobot::new());
