//! Motors API.
//!
//! ## Reference
//!
//! ### Movement
//!
//! * `motor_move`
//! * `motor_move_absolute` (not implemented)
//! * `motor_move_relative` (not implemented)
//! * `motor_move_velocity` (not implemented)
//! * `motor_move_voltage` (not implemented)
//! * `motor_brake` (not implemented)
//! * `motor_modify_profiled_velocity` (not implemented)
//! * `motor_get_target_position` (not implemented)
//! * `motor_get_target_velocity` (not implemented)
//!
//! ### Telemetry
//!
//! * `motor_get_actual_velocity` (not implemented)
//! * `motor_get_current_draw` (not implemented)
//! * `motor_get_direction` (not implemented)
//! * `motor_get_efficiency` (not implemented)
//! * `motor_get_faults` (not implemented)
//! * `motor_get_flags` (not implemented)
//! * `motor_get_position` (not implemented)
//! * `motor_get_power` (not implemented)
//! * `motor_get_raw_position` (not implemented)
//! * `motor_get_temperature` (not implemented)
//! * `motor_get_torque` (not implemented)
//! * `motor_get_voltage` (not implemented)
//! * `motor_get_zero_position_flag` (not implemented)
//! * `motor_is_stopped` (not implemented)
//! * `motor_is_over_current` (not implemented)
//! * `motor_is_over_temp` (not implemented)
//!
//! ### Configuration
//!
//! * `motor_get_brake_mode` (not implemented)
//! * `motor_get_current_limit` (not implemented)
//! * `motor_get_encoder_units` (not implemented)
//! * `motor_get_gearing` (not implemented)
//! * `motor_get_voltage_limit` (not implemented)
//! * `motor_is_reversed` (not implemented)
//! * `motor_set_brake_mode`
//! * `motor_set_current_limit` (not implemented)
//! * `motor_set_encoder_units`
//! * `motor_set_gearing` (not implemented)
//! * `motor_set_reversed` (not implemented)
//! * `motor_set_voltage_limit` (not implemented)
//! * `motor_set_zero_position` (not implemented)
//! * `motor_tare_position` (not implemented)

use wasmtime::{Caller, Linker};

use crate::host::{Host, HostCtx};

pub fn configure_motors_api(linker: &mut Linker<Host>) -> anyhow::Result<()> {
    linker.func_wrap2_async(
        "env",
        "motor_move",
        |caller: Caller<'_, Host>, port: u32, voltage: i32| {
            Box::new(async move {
                let mut ports = caller.smart_ports_lock().await;
                let motor = ports.get_mut(port)?.as_motor_mut()?;
                motor.set_output_volts(voltage.try_into().unwrap());
                Ok(1)
            })
        },
    )?;

    linker.func_wrap2_async(
        "env",
        "motor_set_brake_mode",
        |caller: Caller<'_, Host>, port: u32, mode: u32| {
            Box::new(async move {
                let mut ports = caller.smart_ports_lock().await;
                let motor = ports.get_mut(port)?.as_motor_mut()?;
                motor.set_brake_mode(mode)?; // todo: use errno
                Ok(1)
            })
        },
    )?;

    linker.func_wrap2_async(
        "env",
        "motor_set_encoder_units",
        |caller: Caller<'_, Host>, port: u32, units: u32| {
            Box::new(async move {
                let mut ports = caller.smart_ports_lock().await;
                let motor = ports.get_mut(port)?.as_motor_mut()?;
                motor.set_encoder_units(units)?;
                Ok(1)
            })
        },
    )?;

    Ok(())
}
