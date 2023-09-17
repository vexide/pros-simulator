#![no_main]
use pros::multitasking::sleep;
use pros::prelude::{println, *};
use std::time::Duration;

struct SimRobot;
impl Robot for SimRobot {
    fn init() {
        let sleep_duration = Duration::from_secs(1);
        println!("Hello world");
        sleep(sleep_duration);
        println!("This is from inside a simulator!");
        sleep(sleep_duration);
        println!("Wow!");
        sleep(sleep_duration);
    }
}
robot!(SimRobot);
