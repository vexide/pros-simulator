use std::collections::HashMap;

use pros_simulator_interface::SmartDeviceSpec;

#[derive(Debug, Default)]
pub struct SmartPorts {
    devices: HashMap<u32, SmartDevice>,
}

impl SmartPorts {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }

    pub fn update_specs(&mut self, specs: &HashMap<u32, SmartDeviceSpec>) {
        for (port, spec) in specs.iter() {
            let current = self.devices.get(port);
            if current.map(SmartDeviceSpec::from) != Some(*spec) {
                self.devices.insert(*port, SmartDevice::from(*spec));
            }
        }
    }

    pub fn get(&self, port: u32) -> Option<&SmartDevice> {
        self.devices.get(&port)
    }

    pub fn get_mut(&mut self, port: u32) -> Option<&mut SmartDevice> {
        self.devices.get_mut(&port)
    }
}

#[derive(Debug)]
pub enum SmartDevice {
    Motor(Motor),
}

impl SmartDevice {
    pub fn as_motor(&self) -> Option<&Motor> {
        match self {
            SmartDevice::Motor(m) => Some(m),
        }
    }
}

impl From<SmartDeviceSpec> for SmartDevice {
    fn from(value: SmartDeviceSpec) -> Self {
        match value {
            SmartDeviceSpec::Motor => SmartDevice::Motor(Motor {}),
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
pub struct Motor {}
