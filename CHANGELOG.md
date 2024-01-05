# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2024-01-04

### Added

- Implemented Controller apis
- Implemented `competition_*` apis
- Implemented `task_delay`, `task_delay_until`, `task_get_name`, `task_delete`
- Implemented `rtos_[suspend/remove]_all`
- Implemented `exit`
- New sim-specific API: `sim_log_backtrace`
- Added new Competition Phase simulator message (`SimulatorMessage::PhaseChange`)

### Changed

- Added feature reference documentation
- Phase entrypoints (`opcontrol`, `autonomous`, etc.) will not run until the first competition phase change message has been sent (**breaking change**)

## [0.4.0] - 2023-12-20

### Added

- Unknown and unimplemented PROS APIs will now only crash the simulator if they are used. If robot code imports an unknown API a warning will be printed before the simulator starts.
- The simulator can now be sent Messages via a `Receiver` passed to the `simulate` function. Messages are events that happen in the that the robot code should know about; for now this means LCD button presses and controller joystick movements.
- Implemented the `lcd_register_btnN_cb` API (robot code can now listen for LCD button presses).
- Implemented the `task_create` API (robot code can now spawn FreeRTOS-style tasks which will be executed concurrently but not in parallel).

### Changed

- Updated `wasmtime` to v16

### Known Issues

- If a task is spawned with a higher priority than others, the other tasks will be ignored by the scheduler until the high priority one finishes.

## [0.3.0] - 2023-12-8

### Added

- PROS debug terminal support

### Fixed

- LCD width is now smaller (50 -> 40)

### Changed

- Updated `wasmtime` to v15

## [Server 0.1.1] - 2023-11-27

### Added

- The Simulator Server to allow for easier integration into other apps.

[unreleased]: https://github.com/pros-rs/pros-simulator/compare/server-v0.5.0...HEAD
[0.5.0]: https://github.com/pros-rs/pros-simulator/releases/tag/v0.5.0
[0.4.0]: https://github.com/pros-rs/pros-simulator/releases/tag/v0.4.0
[0.3.0]: https://github.com/pros-rs/pros-simulator/releases/tag/v0.3.0
[server 0.1.1]: https://github.com/pros-rs/pros-simulator/releases/tag/server-v0.1.1
