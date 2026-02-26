# PROS Simulator Interface

> Connect your app to `pros-simulator`

![Maintenance](https://img.shields.io/maintenance/no/2024)
![MIT License](https://img.shields.io/crates/l/pros-simulator-interface)
![Crates.io](https://img.shields.io/crates/v/pros-simulator-interface)

This crate is unmaintained and deprecated and only ever supported PROS programs written in Rust. You should consider using [vexide](https://vexide.dev) (a Rust framework with built-in simulation capabilities) or [vex-v5-qemu](https://github.com/vexide/vex-v5-qemu) (which supports simulating PROS programs written in C++).

## Installation

```sh
cargo add pros-simulator-interface
```

## Overview

The `SimulatorEvent` type contained in this crate is used by the `pros-simulator` crate to communicate with applications. It implements `serde::Serialize` and `serde::Deserialize`, making it easy to send and receive data over IPC or WebSocket.
