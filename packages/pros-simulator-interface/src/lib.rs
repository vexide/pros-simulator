use serde::{Deserialize, Serialize};

pub const LCD_HEIGHT: u32 = 8;
pub const LCD_WIDTH: u32 = 40;
pub type LcdLines = [String; LCD_HEIGHT as usize];

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

/// The current phase of robot code execution.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum RobotPhase {
    Opcontrol,
    Autonomous,
}

/// An event that happens inside the simulator that the API consumer might want to know about.
/// Use this to monitor robot code progress, simulated LCD updates, log messages, and more.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum SimulatorEvent {
    /// A warning message has been emitted by the simulator backend. The robot code is likely using the PROS API incorrectly.
    Warning(String),
    /// The robot code has written the following text to the simulated serial port. A trailing newline should not be assumed.
    ConsoleMessage(String),

    /// The robot code is being loaded into the simulator and compiled.
    RobotCodeLoading,
    /// The robot code has begun executing and the initialize/opcontrol task is about to be spawned.
    RobotCodeStarting,
    /// All tasks have finished executing.
    RobotCodeFinished,
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
    /// The robot has switched modes (opcontrol or autonomous). None = disabled.
    PhaseChange(Option<RobotPhase>),
}
