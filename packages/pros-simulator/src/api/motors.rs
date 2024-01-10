//! Motors API.
//!
//! ## Reference
//!
//! ### Movement
//!
//! * `motor_move`
//! * `motor_move_absolute`
//! * `motor_move_relative`
//! * `motor_move_velocity`
//! * `motor_move_voltage`
//! * `motor_brake`
//! * `motor_modify_profiled_velocity`
//! * `motor_get_target_position`
//! * `motor_get_target_velocity`
//!
//! ### Telemetry
//!
//! * `motor_get_actual_velocity`
//! * `motor_get_current_draw`
//! * `motor_get_direction`
//! * `motor_get_efficiency`
//! * `motor_get_faults`
//! * `motor_get_flags`
//! * `motor_get_position`
//! * `motor_get_power`
//! * `motor_get_raw_position`
//! * `motor_get_temperature`
//! * `motor_get_torque`
//! * `motor_get_voltage`
//! * `motor_get_zero_position_flag`
//! * `motor_is_stopped`
//! * `motor_is_over_current`
//! * `motor_is_over_temp`
//!
//! ### Configuration
//!
//! * `motor_get_brake_mode`
//! * `motor_get_current_limit`
//! * `motor_get_encoder_units`
//! * `motor_get_gearing`
//! * `motor_get_voltage_limit`
//! * `motor_is_reversed`
//! * `motor_set_brake_mode`
//! * `motor_set_current_limit`
//! * `motor_set_encoder_units`
//! * `motor_set_gearing`
//! * `motor_set_reversed`
//! * `motor_set_voltage_limit`
//! * `motor_set_zero_position`
//! * `motor_tare_position`

use std::process::exit;

use anyhow::Context;
use pros_simulator_interface::SimulatorEvent;
use wasmtime::{Caller, Linker, WasmBacktrace};

use crate::host::{memory::SharedMemoryExt, task::TaskPool, Host, HostCtx};

pub fn configure_motors_api(linker: &mut Linker<Host>) -> anyhow::Result<()> {
    linker.func_wrap2_async(
        "env",
        "motor_move",
        |mut caller: Caller<'_, Host>, port: u32, voltage: i32| {
            Box::new(async move {
                let mut ports = caller.smart_ports_lock().await;
                let motor = ports.get_mut(0)?.as_motor()?;
                Ok(())
            })
        },
    )?;

    Ok(())
}
