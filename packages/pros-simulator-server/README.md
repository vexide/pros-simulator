# PROS Simulator Server

> Stream newline-delimited JSON events from `pros-simulator` with a standalone binary

![Maintenance](https://img.shields.io/maintenance/no/2024)
![MIT License](https://img.shields.io/crates/l/pros-simulator-server)
![Crates.io](https://img.shields.io/crates/v/pros-simulator-server)

This crate is unmaintained and deprecated and only ever supported PROS programs written in Rust. You should consider using [vexide](https://vexide.dev) (a Rust framework with built-in simulation capabilities) or [vex-v5-qemu](https://github.com/vexide/vex-v5-qemu) (which supports simulating PROS programs written in C++).

## Installation

```sh
cargo add pros-simulator-server
```

## Overview

This is a standalone server for the VEX V5 robot simulator crate [`pros-simulator`](https://crates.io/crates/pros-simulator). It outputs newline-delimited JSON events about what is happening in the simulator.

```console
$ pros-simulator-server my_program_using_pros_api.wasm --stdio
"RobotCodeLoading"
"RobotCodeStarting"
"LcdInitialized"
{"LcdUpdated":["","","","","","","","Hello from simulator!"]}
{"LcdUpdated":["","","","","","","Hello from simulator!","Hello from simulator!"]}
{"LcdUpdated":["","","","","","","Hello from simulator!","Goodbye from simulator!"]}
"RobotCodeFinished"
```
