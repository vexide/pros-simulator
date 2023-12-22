#![no_main]
#![no_std]

use core::time::Duration;

use pros::prelude::*;

pub struct SimRobot {
    controller: Controller,
}

impl SimRobot {
    fn new() -> Self {
        pros::logger::ProsLogger::init().unwrap();
        pros::lcd::buttons::register(
            || {
                println!("Left button pressed!");
            },
            Button::Left,
        );
        Self {
            controller: Controller::Master,
        }
    }
}

impl Robot for SimRobot {
    fn opcontrol(&mut self) -> pros::Result {
        let mut x_was_pressed = false;
        loop {
            let controller_state = self.controller.state();

            if controller_state.buttons.x {
                if !x_was_pressed {
                    x_was_pressed = true;
                    println!("X button pressed!");
                }
            } else {
                x_was_pressed = false;
            }

            println!("Speed: {}", controller_state.joysticks.left.y);

            sleep(Duration::from_millis(20));
        }
    }
}
robot!(SimRobot, SimRobot::new());
