use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const LCD_HEIGHT: u32 = 8;
pub const LCD_WIDTH: u32 = 40;
pub type LcdLines = [String; LCD_HEIGHT as usize];

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum SmartDeviceSpec {
    Motor,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DigitalControllerState {
    pub l1: bool,
    pub l2: bool,
    pub r1: bool,
    pub r2: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub x: bool,
    pub b: bool,
    pub y: bool,
    pub a: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AnalogControllerState {
    pub left_x: i8,
    pub left_y: i8,
    pub right_x: i8,
    pub right_y: i8,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ControllerState {
    pub digital: DigitalControllerState,
    pub analog: AnalogControllerState,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompetitionPhase {
    pub autonomous: bool,
    pub enabled: bool,
    pub is_competition: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(u32)]
pub enum MotorEncoderUnits {
    #[default]
    Degrees = 0,
    Rotations,
    Counts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(u32)]
pub enum MotorBrakeMode {
    #[default]
    Coast = 0,
    Brake,
    Hold,
}

/// An event that happens inside the simulator that the API consumer might want to know about.
/// Use this to monitor robot code progress, simulated LCD updates, log messages, and more.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum SimulatorEvent {
    /// A warning message has been emitted by the simulator backend. The robot code is likely using the PROS API incorrectly.
    Warning(String),
    /// The robot code has written the following text to the simulated serial port. A trailing newline should not be assumed.
    ConsoleMessage(String),

    /// The robot code is being loaded and validated.
    Loading,
    /// The interface should configure the simulator world and must send a [`SimulatorMessage::Initialize`] to begin robot code execution.
    ResourcesRequired,
    /// The task scheduler has begun running robot code.
    RobotCodeRunning,
    /// All tasks have exited and the robot code has finished.
    AllTasksFinished,
    /// The robot code has panicked or otherwise faulted.
    RobotCodeError { message: String, backtrace: String },

    /// The LCD has been initialized and may be updated in the future.
    LcdInitialized,
    /// The LCD has been updated and should be redrawn.
    LcdUpdated(LcdLines),
    /// The robot code has requested that the LCD color change to the provided foreground/background (RGBA).
    LcdColorsUpdated { foreground: u32, background: u32 },
    /// The LCD has shut down and should be blanked.
    LcdShutdown,

    MotorUpdated {
        port: u32,
        volts: i8,
        encoder_units: MotorEncoderUnits,
        brake_mode: MotorBrakeMode,
    },
}

/// A message sent to the simulator to control the robot code environment.
/// The `pros-simulator` API accepts these over an async stream, and API consumers can use
/// them to simulate changes in robot hardware (like controller input and LCD touch events).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum SimulatorMessage {
    /// Master and Partner controllers have updated (in that order). None = disconnected.
    ControllerUpdate(Option<ControllerState>, Option<ControllerState>),

    /// An LCD button has been pressed/released. The 3 booleans represent
    /// whether each button is being pressed, from left to right. This API technically supports
    /// pressing multiple buttons at once, but that won't ever happen on a real robot.
    LcdButtonsUpdate([bool; 3]), // {"LcdButtonsUpdate": [true, false, false]}
    /// The robot has switched competition modes (opcontrol or autonomous or disabled).
    PhaseChange(CompetitionPhase),

    /// The robot's smart ports have been updated. Map of port numbers to device specs.
    PortsUpdate(HashMap<u32, SmartDeviceSpec>),

    /// The simulator has been configured, its smart ports have been set, and it is allowed to start robot code execution.
    BeginSimulation,
}
