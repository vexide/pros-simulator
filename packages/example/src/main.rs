#![no_main]
#![no_std]

use core::time::Duration;

use pros::prelude::*;

pub struct SimRobot;

impl SimRobot {
    fn new() -> Self {
        pros::logger::ProsLogger::init().unwrap();
        println!("registering");
        // pros::lcd::buttons::register(
        //     || {
        //         println!("Button pressed!");
        //     },
        //     Button::Left,
        // );
        println!("done");
        Self
    }
}

impl Robot for SimRobot {
    fn opcontrol(&mut self) -> pros::Result {
        pros::task::spawn(|| {
            println!("Hello from task!");
            loop {
                sleep(Duration::from_secs(1));
            }
        });
        sleep(Duration::from_secs(2));
        println!("Hello world!");
        sleep(Duration::from_secs(3));
        loop {
            // info!("Hello from simulator!");
            sleep(Duration::from_secs(1));
        }
        Ok(())
    }
}
robot!(SimRobot, SimRobot::new());
