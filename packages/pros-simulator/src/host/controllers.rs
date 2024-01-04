use std::mem;

use pros_simulator_interface::{ControllerState, DigitalControllerState};
use pros_sys::{
    misc::E_CONTROLLER_DIGITAL_R1, EINVAL, E_CONTROLLER_ANALOG_LEFT_X, E_CONTROLLER_ANALOG_LEFT_Y,
    E_CONTROLLER_ANALOG_RIGHT_X, E_CONTROLLER_ANALOG_RIGHT_Y, E_CONTROLLER_DIGITAL_A,
    E_CONTROLLER_DIGITAL_B, E_CONTROLLER_DIGITAL_DOWN, E_CONTROLLER_DIGITAL_L1,
    E_CONTROLLER_DIGITAL_L2, E_CONTROLLER_DIGITAL_LEFT, E_CONTROLLER_DIGITAL_R2,
    E_CONTROLLER_DIGITAL_RIGHT, E_CONTROLLER_DIGITAL_UP, E_CONTROLLER_DIGITAL_X,
    E_CONTROLLER_DIGITAL_Y, E_CONTROLLER_MASTER, E_CONTROLLER_PARTNER,
};
struct Controller {
    state: ControllerState,
    new_presses: DigitalControllerState,
}

impl From<ControllerState> for Controller {
    fn from(state: ControllerState) -> Self {
        Self {
            new_presses: state.digital.clone(),
            state,
        }
    }
}

impl Controller {
    /// Update controller state and set new press values to true if either it is currently true or it has changed to true.
    pub fn update(&mut self, state: ControllerState) {
        self.new_presses = DigitalControllerState {
            l1: (state.digital.l1 && !self.state.digital.l1) || self.new_presses.l1,
            l2: (state.digital.l2 && !self.state.digital.l2) || self.new_presses.l2,
            r1: (state.digital.r1 && !self.state.digital.r1) || self.new_presses.r1,
            r2: (state.digital.r2 && !self.state.digital.r2) || self.new_presses.r2,
            up: (state.digital.up && !self.state.digital.up) || self.new_presses.up,
            down: (state.digital.down && !self.state.digital.down) || self.new_presses.down,
            left: (state.digital.left && !self.state.digital.left) || self.new_presses.left,
            right: (state.digital.right && !self.state.digital.right) || self.new_presses.right,
            x: (state.digital.x && !self.state.digital.x) || self.new_presses.x,
            b: (state.digital.b && !self.state.digital.b) || self.new_presses.b,
            y: (state.digital.y && !self.state.digital.y) || self.new_presses.y,
            a: (state.digital.a && !self.state.digital.a) || self.new_presses.a,
        };
        self.state = state;
    }
}

/// Stores state of VEX V5 master and partner controllers.
pub struct Controllers {
    master: Option<Controller>,
    partner: Option<Controller>,
}

impl Controllers {
    pub fn new(master: Option<ControllerState>, partner: Option<ControllerState>) -> Self {
        Self {
            master: master.map(|v| v.into()),
            partner: partner.map(|v| v.into()),
        }
    }

    /// Update state of both controllers and set new press values.
    pub fn update(
        &mut self,
        new_master: Option<ControllerState>,
        new_partner: Option<ControllerState>,
    ) {
        if let Some(new_master) = new_master {
            if let Some(master) = &mut self.master {
                master.update(new_master);
            } else {
                self.master = Some(new_master.into());
            }
        }
        if let Some(new_partner) = new_partner {
            if let Some(partner) = &mut self.partner {
                partner.update(new_partner);
            } else {
                self.partner = Some(new_partner.into());
            }
        }
    }

    pub fn is_connected(&self, controller_id: u32) -> Result<bool, i32> {
        match controller_id {
            E_CONTROLLER_MASTER => Ok(self.master.is_some()),
            E_CONTROLLER_PARTNER => Ok(self.partner.is_some()),
            _ => Err(EINVAL),
        }
    }

    /// Get the state of a controller by ID. Fails with EINVAL if the controller ID is invalid.
    fn get_controller_state(&self, controller_id: u32) -> Result<Option<&Controller>, i32> {
        match controller_id {
            E_CONTROLLER_MASTER => Ok(self.master.as_ref()),
            E_CONTROLLER_PARTNER => Ok(self.partner.as_ref()),
            _ => Err(EINVAL),
        }
    }

    /// Get the state of a controller by ID. Fails with EINVAL if the controller ID is invalid.
    fn get_controller_state_mut(
        &mut self,
        controller_id: u32,
    ) -> Result<Option<&mut Controller>, i32> {
        match controller_id {
            E_CONTROLLER_MASTER => Ok(self.master.as_mut()),
            E_CONTROLLER_PARTNER => Ok(self.partner.as_mut()),
            _ => Err(EINVAL),
        }
    }

    /// Returns the current state of a specific analog channel (joystick) on a specific controller.
    ///
    /// This function retrieves the current state of a given analog channel (joystick) on a specific controller.
    /// The state is represented as a integer value in the range [-127, 127] where -127 is full down or left,
    /// and 127 is full up or right.
    ///
    /// # Arguments
    ///
    /// * `controller_id` - A u32 that holds the ID of the controller.
    /// * `channel` - A u32 that represents the axis whose state is to be retrieved.
    ///
    /// # Returns
    ///
    /// * `Ok(i32)` - If the controller and button exist, returns the state of the axis.
    /// * `Err(i32)` - If the controller or button does not exist, returns an error with the code `EINVAL`.
    ///
    /// # Example
    ///
    /// ```
    /// if controllers.get_analog(pros_sys::E_CONTROLLER_MASTER, pros_sys::E_CONTROLLER_ANALOG_LEFT_X)? > 0 {
    ///     println!("Left joystick is pushed right")
    /// }
    /// ```
    pub fn get_analog(&self, controller_id: u32, channel: u32) -> Result<i32, i32> {
        let controller = self.get_controller_state(controller_id)?;
        if let Some(Controller { state, .. }) = controller {
            match channel {
                E_CONTROLLER_ANALOG_LEFT_X => Ok(state.analog.left_x),
                E_CONTROLLER_ANALOG_LEFT_Y => Ok(state.analog.left_y),
                E_CONTROLLER_ANALOG_RIGHT_X => Ok(state.analog.right_x),
                E_CONTROLLER_ANALOG_RIGHT_Y => Ok(state.analog.right_y),
                _ => Err(EINVAL),
            }
        } else {
            Ok(0)
        }
        .map(|v| v as i32)
    }

    /// Returns the current state of a specific button on a specific controller.
    ///
    /// This function retrieves the current state of a given button on a specific controller.
    /// The state is represented as a boolean value, where `true` indicates that the button is currently pressed,
    /// and `false` indicates that it is not.
    ///
    /// # Arguments
    ///
    /// * `controller_id` - A u32 that holds the ID of the controller.
    /// * `button` - A u32 that represents the button whose state is to be retrieved.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - If the controller and button exist, returns a boolean indicating whether the button is currently pressed.
    /// * `Err(i32)` - If the controller or button does not exist, returns an error with the code `EINVAL`.
    ///
    /// # Example
    ///
    /// ```
    /// if controllers.get_digital(pros_sys::E_CONTROLLER_MASTER, pros_sys::E_CONTROLLER_DIGITAL_X)? {
    ///     println!("Button X pressed")
    /// }
    /// ```
    pub fn get_digital(&self, controller_id: u32, button: u32) -> Result<bool, i32> {
        let controller = self.get_controller_state(controller_id)?;
        if let Some(Controller { state, .. }) = controller {
            match button {
                E_CONTROLLER_DIGITAL_L1 => Ok(state.digital.l1),
                E_CONTROLLER_DIGITAL_L2 => Ok(state.digital.l2),
                E_CONTROLLER_DIGITAL_R1 => Ok(state.digital.r1),
                E_CONTROLLER_DIGITAL_R2 => Ok(state.digital.r2),
                E_CONTROLLER_DIGITAL_UP => Ok(state.digital.up),
                E_CONTROLLER_DIGITAL_DOWN => Ok(state.digital.down),
                E_CONTROLLER_DIGITAL_LEFT => Ok(state.digital.left),
                E_CONTROLLER_DIGITAL_RIGHT => Ok(state.digital.right),
                E_CONTROLLER_DIGITAL_X => Ok(state.digital.x),
                E_CONTROLLER_DIGITAL_B => Ok(state.digital.b),
                E_CONTROLLER_DIGITAL_Y => Ok(state.digital.y),
                E_CONTROLLER_DIGITAL_A => Ok(state.digital.a),
                _ => Err(EINVAL),
            }
        } else {
            Ok(false)
        }
    }

    /// Returns whether a new press event occurred for a specific button on a specific controller.
    ///
    /// This function checks if a new press event has occurred for a given button on a specific controller.
    /// If a new press event has occurred, it returns `true` and resets the state to `false`.
    /// If no new press event has occurred, it returns `false`.
    /// If the controller or button does not exist, it returns an error.
    ///
    /// State is shared between tasks so to get notified on every change it's best to always call this from
    /// the same task.
    ///
    /// # Arguments
    ///
    /// * `controller_id` - A u32 that holds the ID of the controller.
    /// * `button` - A u32 that represents the button to check for a new press event.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - If the controller and button exist, returns whether a new press event has occurred.
    /// * `Err(i32)` - If the controller or button does not exist, returns an error.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if controllers.get_digital_new_press(pros_sys::E_CONTROLLER_MASTER, pros_sys::E_CONTROLLER_DIGITAL_X)? {
    ///     println!("Button X has been pressed since last call");
    /// }
    /// ```
    pub fn get_digital_new_press(&mut self, controller_id: u32, button: u32) -> Result<bool, i32> {
        let mut controller = self.get_controller_state_mut(controller_id)?;
        if let Some(Controller { new_presses, .. }) = &mut controller {
            let field = match button {
                E_CONTROLLER_DIGITAL_L1 => Ok(&mut new_presses.l1),
                E_CONTROLLER_DIGITAL_L2 => Ok(&mut new_presses.l2),
                E_CONTROLLER_DIGITAL_R1 => Ok(&mut new_presses.r1),
                E_CONTROLLER_DIGITAL_R2 => Ok(&mut new_presses.r2),
                E_CONTROLLER_DIGITAL_UP => Ok(&mut new_presses.up),
                E_CONTROLLER_DIGITAL_DOWN => Ok(&mut new_presses.down),
                E_CONTROLLER_DIGITAL_LEFT => Ok(&mut new_presses.left),
                E_CONTROLLER_DIGITAL_RIGHT => Ok(&mut new_presses.right),
                E_CONTROLLER_DIGITAL_X => Ok(&mut new_presses.x),
                E_CONTROLLER_DIGITAL_B => Ok(&mut new_presses.b),
                E_CONTROLLER_DIGITAL_Y => Ok(&mut new_presses.y),
                E_CONTROLLER_DIGITAL_A => Ok(&mut new_presses.a),
                _ => Err(EINVAL),
            }?;

            Ok(mem::replace(field, false))
        } else {
            Ok(false)
        }
    }
}
