# PROS Simulator Server

> Stream newline-delimited JSON events from `pros-simulator` with a standalone binary

[![CI Status](https://github.com/pros-rs/pros-simulator/actions/workflows/rust.yml/badge.svg)](https://github.com/pros-rs/pros-simulator/actions/workflows/rust.yml)
![MIT License](https://img.shields.io/crates/l/pros-simulator-server)
![Crates.io](https://img.shields.io/crates/v/pros-simulator-server)

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
