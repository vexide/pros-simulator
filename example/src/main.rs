#![no_main]
use pros::multitasking::sleep;
use pros::prelude::{println, *};
use std::time::Duration;

#[no_mangle]
pub extern "C" fn initialize() {
    let sleep_duration = Duration::from_secs(1);
    sleep(sleep_duration);
    println!("Hello world");
    sleep(sleep_duration);
    println!("This is from inside a simulator!");
    sleep(sleep_duration);
    println!("Wow!");
    sleep(sleep_duration);
}
