# PROS Simulator

> Run PROS robot code without the need for real VEX V5 hardware.

[![CI Status](https://github.com/pros-rs/pros-simulator/actions/workflows/rust.yml/badge.svg)](https://github.com/pros-rs/pros-simulator/actions/workflows/rust.yml)
![MIT License](https://img.shields.io/crates/l/pros-simulator)
![Crates.io](https://img.shields.io/crates/v/pros-simulator)

## Installation

```sh
cargo add pros-simulator
```

Or, as an executable JSON-based server:

```sh
cargo install pros-simulator-server
```

## Overview

This Rust crate is a WebAssembly-based runtime for simulating [VEX V5](https://www.vexrobotics.com/v5) robot code, without the need for any special hardware. It's the best way to program from home, debug misbehaving programs, and quickly iterate code design.

This runtime implements a portion of the [PROS](https://pros.cs.purdue.edu/) C interface, allowing pre-existing PROS-based programs to function in the simulator without the need for invasive modification. Support for `pros-simulator` is built directly into the [`pros`](https://crates.io/crates/pros) crate, so programs using it will be compatible with this simulator without extra work.

## Usage

PROS Simulator is available in library form, and also as a JSON-based server that's inspired by the LSP protocol and ideal for integrating into other programs (see releases page for ready made binaries). This project contains the core of the simulator, which handles loading and running user-generated robot code, and requires a custom interface (like a GUI or TUI) to be useful. There are a few example interfaces provided, like the TUI-based one below.

### TUI Interface

![TUI interface](./assets/tui.gif)

To build the example simulator program, you'll need a nightly Rust toolchain and the was32-unknown-unknown target installed. In the `example` directory, run the following command to build:

```terminal
cargo pros build -s
```

Then, in the project root, run the following command to start the TUI:

```terminal
cargo run --example tui ./example/target/wasm32-unknown-unknown/debug/example.wasm
```

The simulator (and its TUI interface) support the use of breakpoints in robot code! Try opening this project in VS Code and pressing F5 to start debugging the example program.

## Feature Overview


- [x] **Concurrent multitasking**: Spawn tasks and manage them.
- [x] **LLEMU**: Print messages to V5 LCD display.
- [x] **Serial connection**: Print messages to debug terminal.
- [x] **Mutexes**: Synchronize tasks.
- [x] **Task-local storage**: Manage global variables that are specific to each task.
- [x] **Timings**: Sleep program and get elapsed time.
- [x] **Abort messages**: Get stack trace & error message on any panic or abort (including segfaults).
- [x] **Controllers**: Control simulated robot using any SDL-compatible wired or bluetooth controller.
- [x] **Competition Status**: Control autonomous/opcontrol/disabled status of simulated robot.
- [ ] **Motors**: Simulate VEX Smart Motors
- [ ] **Sensors**: Simulate V5-compatible sensors
- [ ] **Physics**: Physics simulation and graphical representation of simulated robot

## Robot Code API Reference

See PROS docs for signatures and documentation. API is 1:1 except where mentioned otherwise.

- [ ] **LLEMU (Legacy LCD Emulator)** C API
  - [x] `lcd_clear`
  - [x] `lcd_clear_line`
  - [x] `lcd_initialize`
  - [ ] `lcd_is_initialized`
  - [ ] `lcd_print`
  - [ ] `lcd_read_buttons`
  - [x] `lcd_register_btn0_cb`
  - [x] `lcd_register_btn1_cb`
  - [x] `lcd_register_btn2_cb`
  - [x] `lcd_set_text`
  - [ ] `lcd_shutdown`
  - [ ] `lcd_set_background_color`
  - [ ] `lcd_set_text_color`
- [ ] **Miscellaneous** C API
  - [ ] `battery_get_capacity`
  - [ ] `battery_get_current`
  - [ ] `battery_get_temperature`
  - [ ] `battery_get_voltage`
  - [x] `competition_get_status`
  - [x] `competition_is_autonomous`
  - [x] `competition_is_connected`
  - [x] `competition_is_disabled`
  - [ ] `controller_clear`
  - [x] `controller_clear_line`
  - [x] `controller_get_analog`
  - [x] `controller_get_battery_capacity`
  - [ ] `controller_get_battery_level` (Return value always equal to capacity)
  - [x] `controller_get_digital`
  - [x] `controller_get_digital_new_press`
  - [x] `controller_is_connected`
  - [ ] `controller_print`
  - [ ] `controller_rumble`
  - [ ] `controller_set_text`
  - [ ] `usd_is_installed`
- [ ] **RTOS Facilities** C API
  - [x] `delay`
  - [x] `millis`
  - [ ] `micros`
  - [x] `mutex_create`
  - [x] `mutex_delete`
  - [x] `mutex_give`
  - [x] `mutex_take`
  - [x] `task_create`
  - [ ] `task_delay`
  - [ ] `task_delay_until`
  - [ ] `task_delete`
  - [ ] `task_get_by_name`
  - [ ] `task_get_count`
  - [ ] `task_get_current`
  - [ ] `task_get_name`
  - [ ] `task_get_priority`
  - [ ] `task_get_state`
  - [ ] `task_notify`
  - [ ] `task_notify_clear`
  - [ ] `task_notify_ext`
  - [ ] `task_notify_take`
  - [ ] `task_join`
  - [ ] `task_resume`
  - [ ] `task_set_priority`
  - [ ] `task_suspend`
- [x] Generic I/O API

    Undocumented/internal PROS functions that are required to support
    miscellaneous IO like `errno`, the debug terminal, and panicking.

  - [x] `_errno`: Returns a mutable pointer to the errno value of the current task.
  - [x] `sim_abort(*const char) -> !`: Simulator-only API for aborting with an error message.
  - [x] `puts`: Write to the debug terminal (`pros terminal` command from official PROS CLI)
