# PROS Simulator

> Run PROS robot code without the need for real VEX V5 hardware.

[![CI Status](https://github.com/pros-rs/pros-simulator/actions/workflows/rust.yml/badge.svg)](https://github.com/pros-rs/pros-simulator/actions/workflows/rust.yml)

## Installation

```sh
cargo add pros-simulator
```

## Overview

This Rust crate is a WebAssembly-based runtime for simulating [VEX V5](https://www.vexrobotics.com/v5) robot code, without the need for any special hardware. It acts as a plug and play solution for programming from home, debugging misbehaving programs, and quickly iterating code design.

This runtime implements a portion of the [PROS](https://pros.cs.purdue.edu/) C interface, allowing pre-existing PROS-based programs to function in the simulator without the need for invasive modification. Support for `pros-simulator` is built directly into the [`pros`](https://crates.io/crates/pros) crate, so programs using it will be compatible with this simulator for free.

## Usage

TODO
