#![no_main]
use pros::prelude::{println, *};
use pros::task::sleep;
use std::time::Duration;

struct SimRobot;
#[robot]
impl Robot for SimRobot {
    fn init() -> Result<SimRobot, Box<(dyn std::error::Error + 'static)>> {
        let sleep_duration = Duration::from_secs(1);
        println!("Hello world");
        sleep(sleep_duration);
        println!("This is from inside a simulator!");
        sleep(sleep_duration);
        println!("Wow!");
        sleep(sleep_duration);
        Ok(SimRobot)
    }
}
