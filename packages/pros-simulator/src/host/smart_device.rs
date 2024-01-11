use std::collections::HashMap;

use anyhow::bail;
use pros_simulator_interface::{
    MotorBrakeMode, MotorEncoderUnits, SimulatorEvent, SmartDeviceSpec,
};
use snafu::{OptionExt, Snafu};

use crate::interface::SimulatorInterface;

#[derive(Debug, Snafu)]
#[snafu(display("Smart port {port} is not configured"))]
pub struct PortNotConfiguredError {
    pub port: u32,
}

#[derive(Debug, Snafu)]
#[snafu(display("Smart port {port} is configured as a {actual:?}, not a {expected:?}"))]
pub struct IncorrectDeviceTypeError {
    pub port: u32,
    pub expected: SmartDeviceSpec,
    pub actual: SmartDeviceSpec,
}

#[derive(Debug)]
pub struct SmartPorts {
    devices: HashMap<u32, SmartDevice>,
    interface: SimulatorInterface,
}

impl SmartPorts {
    pub fn new(interface: SimulatorInterface) -> Self {
        Self {
            devices: HashMap::new(),
            interface,
        }
    }

    pub fn update_specs(&mut self, specs: &HashMap<u32, SmartDeviceSpec>) {
        for (port, spec) in specs.iter() {
            let current = self.devices.get(port);
            if current.map(SmartDeviceSpec::from) != Some(*spec) {
                self.devices.insert(
                    *port,
                    SmartDevice::new(*spec, *port, self.interface.clone()),
                );
            }
        }
    }

    pub fn get(&self, port: u32) -> Result<&SmartDevice, PortNotConfiguredError> {
        self.devices
            .get(&port)
            .context(PortNotConfiguredSnafu { port })
    }

    pub fn get_mut(&mut self, port: u32) -> Result<&mut SmartDevice, PortNotConfiguredError> {
        self.devices
            .get_mut(&port)
            .context(PortNotConfiguredSnafu { port })
    }
}

#[derive(Debug)]
pub enum SmartDevice {
    Motor(Motor),
}

impl SmartDevice {
    pub fn new(spec: SmartDeviceSpec, port: u32, interface: SimulatorInterface) -> Self {
        match spec {
            SmartDeviceSpec::Motor => Self::Motor(Motor::new(port, interface)),
        }
    }
    pub fn as_motor(&self) -> Result<&Motor, IncorrectDeviceTypeError> {
        match self {
            SmartDevice::Motor(m) => Ok(m),
        }
    }
    pub fn as_motor_mut(&mut self) -> Result<&mut Motor, IncorrectDeviceTypeError> {
        match self {
            SmartDevice::Motor(m) => Ok(m),
        }
    }
}

impl From<&SmartDevice> for SmartDeviceSpec {
    fn from(value: &SmartDevice) -> Self {
        match value {
            SmartDevice::Motor(_) => SmartDeviceSpec::Motor,
        }
    }
}

#[derive(Debug)]
pub struct Motor {
    port: u32,
    brake_mode: MotorBrakeMode,
    encoder_units: MotorEncoderUnits,
    output_volts: i8,
    interface: SimulatorInterface,
}

impl Motor {
    pub fn new(port: u32, interface: SimulatorInterface) -> Self {
        Self {
            port,
            brake_mode: MotorBrakeMode::default(),
            encoder_units: MotorEncoderUnits::default(),
            output_volts: 0,
            interface,
        }
    }

    fn publish(&self) {
        self.interface.send(SimulatorEvent::MotorUpdated {
            port: self.port,
            brake_mode: self.brake_mode,
            encoder_units: self.encoder_units,
            volts: self.output_volts,
        });
    }

    pub fn set_output_volts(&mut self, volts: i8) {
        if volts < -127 {
            self.interface.send(SimulatorEvent::Warning(format!(
                "Motor voltage out of range: {volts} < -127"
            )))
        }
        self.output_volts = volts.clamp(-127, 127);
        self.publish();
    }

    pub fn set_encoder_units(&mut self, units: u32) -> anyhow::Result<()> {
        let units = match units {
            0 => MotorEncoderUnits::Degrees,
            1 => MotorEncoderUnits::Rotations,
            2 => MotorEncoderUnits::Counts,
            _ => bail!("Invalid encoder unit `{}`", units),
        };
        self.encoder_units = units;
        self.publish();
        Ok(())
    }

    pub fn set_brake_mode(&mut self, mode: u32) -> anyhow::Result<()> {
        let mode = match mode {
            0 => MotorBrakeMode::Coast,
            1 => MotorBrakeMode::Brake,
            2 => MotorBrakeMode::Hold,
            _ => bail!("Invalid brake mode `{}`", mode),
        };
        self.brake_mode = mode;
        self.publish();
        Ok(())
    }
}
