//! Miscellaneous API functions.
//!
//! ## Reference
//!
//! * `battery_get_capacity` (not implemented)
//! * `battery_get_current` (not implemented)
//! * `battery_get_temperature` (not implemented)
//! * `battery_get_voltage` (not implemented)
//! * `competition_get_status`
//! * `competition_is_autonomous`
//! * `competition_is_connected`
//! * `competition_is_disabled`
//! * `controller_clear` (not implemented)
//! * `controller_clear_line` (not implemented)
//! * `controller_get_analog`
//! * `controller_get_battery_capacity`
//! * `controller_get_battery_level` (Return value always equal to capacity)
//! * `controller_get_digital`
//! * `controller_get_digital_new_press`
//! * `controller_is_connected`
//! * `controller_print` (not implemented)
//! * `controller_rumble` (not implemented)
//! * `controller_set_text` (not implemented)
//! * `usd_is_installed` (not implemented)

use wasmtime::{Caller, Linker};

use crate::{
    host::{Host, HostCtx, ResultExt},
    system::system_daemon::CompetitionPhaseExt,
};

pub fn configure_misc_api(linker: &mut Linker<Host>) -> anyhow::Result<()> {
    linker.func_wrap2_async(
        "env",
        "controller_get_analog",
        |mut caller: Caller<'_, Host>, id: u32, channel: u32| {
            Box::new(async move {
                let controllers = caller.controllers_lock().await;
                let res = controllers.get_analog(id, channel);
                drop(controllers);
                Ok(res.unwrap_or_errno_as(&mut caller, 0).await)
            })
        },
    )?;

    linker.func_wrap2_async(
        "env",
        "controller_get_digital",
        |mut caller: Caller<'_, Host>, id: u32, button: u32| {
            Box::new(async move {
                let controllers = caller.controllers_lock().await;
                let res = controllers.get_digital(id, button);
                drop(controllers);
                Ok(i32::from(res.unwrap_or_errno_as(&mut caller, false).await))
            })
        },
    )?;

    linker.func_wrap2_async(
        "env",
        "controller_get_digital_new_press",
        |mut caller: Caller<'_, Host>, id: u32, button: u32| {
            Box::new(async move {
                let mut controllers = caller.controllers_lock().await;
                let res = controllers.get_digital_new_press(id, button);
                drop(controllers);
                Ok(i32::from(res.unwrap_or_errno_as(&mut caller, false).await))
            })
        },
    )?;

    linker.func_wrap1_async(
        "env",
        "controller_is_connected",
        |mut caller: Caller<'_, Host>, id: u32| {
            Box::new(async move {
                let controllers = caller.controllers_lock().await;
                let res = controllers.is_connected(id);
                drop(controllers);
                Ok(i32::from(res.unwrap_or_errno(&mut caller).await))
            })
        },
    )?;

    linker.func_wrap1_async(
        "env",
        "controller_get_battery_capacity",
        |_caller: Caller<'_, Host>, _id: u32| Box::new(async move { Ok(100i32) }),
    )?;

    linker.func_wrap1_async(
        "env",
        "controller_get_battery_level",
        |_caller: Caller<'_, Host>, _id: u32| Box::new(async move { Ok(100i32) }),
    )?;

    linker.func_wrap0_async(
        "env",
        "competition_get_status",
        |caller: Caller<'_, Host>| {
            Box::new(async move {
                let phase = caller.competition_phase_lock().await;
                Ok(phase.as_bits() as i32)
            })
        },
    )?;

    linker.func_wrap0_async(
        "env",
        "competition_is_autonomous",
        |caller: Caller<'_, Host>| {
            Box::new(async move {
                let phase = caller.competition_phase_lock().await;
                Ok(i32::from(phase.autonomous))
            })
        },
    )?;

    linker.func_wrap0_async(
        "env",
        "competition_is_connected",
        |caller: Caller<'_, Host>| {
            Box::new(async move {
                let phase = caller.competition_phase_lock().await;
                Ok(i32::from(phase.is_competition))
            })
        },
    )?;

    linker.func_wrap0_async(
        "env",
        "competition_is_disabled",
        |caller: Caller<'_, Host>| {
            Box::new(async move {
                let phase = caller.competition_phase_lock().await;
                Ok(i32::from(!phase.enabled))
            })
        },
    )?;

    Ok(())
}
