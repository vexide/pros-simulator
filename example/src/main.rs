#![no_main]
#![no_std]

extern crate alloc;

use alloc::rc::Rc;
use core::future::Future;
use core::task::Poll;
use core::time::Duration;
use pros::async_runtime::spawn;
use pros::prelude::executor::Executor;
use pros::prelude::{println, *};
use pros::task::{delay, sleep};

pub struct SleepFuture<'a> {
    target_millis: u32,
    executor: &'a Executor,
}
impl<'a> Future for SleepFuture<'a> {
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        if self.target_millis < unsafe { pros_sys::millis() } {
            Poll::Ready(())
        } else {
            self.executor
                .reactor
                .sleepers
                .borrow_mut()
                .push(cx.waker().clone(), self.target_millis);
            Poll::Pending
        }
    }
}

pub struct SimRobot;
impl Default for SimRobot {
    fn default() -> Self {
        let executor = Rc::new(Executor::new());
        let e = executor.clone();
        executor.block_on(async move {
            println!("Hello from async!");
            // sleep(Duration::from_secs(1)).await;
            SleepFuture {
                target_millis: unsafe { pros_sys::millis() } + 1000,
                executor: &e,
            }
            .await;
            println!("Goodbye from async!");
        });
        executor.complete();
        SimRobot
    }
}
impl Robot for SimRobot {}
robot!(SimRobot);
