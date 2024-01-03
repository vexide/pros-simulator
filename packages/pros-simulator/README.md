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

## Feature Reference

### Overview

- [x] **Concurrent multitasking**: Spawn tasks and manage them.
- [x] **LLEMU**: Print messages to V5 LCD display.
- [x] **Serial connection**: Print messages to debug terminal.
- [x] **Mutexes**: Synchronize tasks.
- [x] **Local storage**: Manage global variables that are specific to each task.
- [x] **Timings**: Sleep program and get elapsed time.
- [x] **Abort messages**: Get stack trace & error message on any panic or abort (including segfaults).
- [x] **Controllers**: Control simulated robot using any SDL-compatible wired or bluetooth controller.
- [ ] **Motors**: Simulate VEX Smart Motors
- [ ] **Sensors**: Simulate V5-compatible sensors
- [ ] **Physics**: Physics simulation and graphical representation of simulated robot

### API

See official PROS website for documentation and signatures. API is 1:1 except when mentioned otherwise. WebAssembly import module is `env`.

#### LCD

* `lcd_initialize`
* `lcd_set_text`
* `lcd_clear_line`
* `lcd_clear`
* `lcd_register_btnN_cb`

#### Mutex

* `mutex_create`
* `mutex_delete`
* `mutex_give`
* `mutex_take`

#### Thread locals

See FreeRTOS documentation

* `pvTaskGetThreadLocalStoragePointer`
* `vTaskSetThreadLocalStoragePointer`

#### Tasks

* `task_get_current`
* `task_create`

#### Timing

* `delay`
* `millis`

#### Errors/Debugging

* `__errno()`
* `sim_abort(msg: *const char) -> !` (Custom, simulator-only abort function with panic message)
* `puts(msg: *const char)`

#### Controller

* `controller_get_analog`
* `controller_get_digital`
* `controller_get_digital_new_press`
* `controller_is_connected`
* `controller_get_battery_capacity`
* `controller_get_battery_level` (Return value always equal to capacity)
