# PROS Simulator Interface

> Connect your app to `pros-simulator`

[![CI Status](https://github.com/pros-rs/pros-simulator/actions/workflows/rust.yml/badge.svg)](https://github.com/pros-rs/pros-simulator/actions/workflows/rust.yml)
![MIT License](https://img.shields.io/crates/l/pros-simulator-interface)
![Crates.io](https://img.shields.io/crates/v/pros-simulator-interface)

## Installation

```sh
cargo add pros-simulator-interface
```

## Overview

The `SimulatorEvent` type contained in this crate is used by the `pros-simulator` crate to communicate with applications. It implements `serde::Serialize` and `serde::Deserialize`, making it easy to send and receive data over IPC or WebSocket.
