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
