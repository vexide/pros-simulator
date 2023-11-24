#![no_main]
#![no_std]

use core::time::Duration;

use pros::prelude::*;

#[derive(Default)]
pub struct SimRobot;

impl Robot for SimRobot {
    fn opcontrol(&mut self) -> pros::Result {
        sleep(Duration::from_secs(2));
        pros::task::spawn(|| {
            sleep(Duration::from_secs(1));
            println!("Hello from task!");
        });
        println!("Hello from simulator!");
        sleep(Duration::from_secs(3));
        println!("Goodbye from simulator!");
        sleep(Duration::from_secs(1));
        Ok(())
    }
}
robot!(SimRobot);
