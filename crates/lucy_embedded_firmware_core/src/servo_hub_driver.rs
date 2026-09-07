use crate::driver_generic::{ModBusDriver, ModBusDriverError};
use crate::modbus::{*};
use core::result::Result;
use core::error::Error;

#[derive(Debug)]
pub enum ServoHubDriverError {
    Error
}

pub struct PCA9685Driver {
    base_reg: u16,
    channel: PCA9685PWMChannel,
}

impl ModBusDriver for PCA9685Driver {
    fn tick(&mut self, view: RegisterView) -> Result<(), ModBusDriverError> {
        Ok(())
    }

    fn getNbRegisters() -> u16 {
        1
    }

    fn getBaseRegister(&mut self) ->  u16 {
        self.base_reg
    }
}

impl PCA9685Driver {
    fn new (base_reg: u16) -> Self {
        PCA9685Driver {
            base_reg,
            channel: PCA9685PWMChannel::new(),
        }
    }

    fn getChannel(self, channel: u16) -> impl IPWMChannel {
        self.channel
    }
}

pub trait IPWMChannel {
    fn setPWM (&mut self, pulse: u16);
}

pub struct PCA9685PWMChannel {
    pulse: u16,
}

impl PCA9685PWMChannel {
    fn new () -> Self {
        PCA9685PWMChannel { pulse: 0 }
    }
}

impl IPWMChannel for PCA9685PWMChannel {
    fn setPWM (&mut self, pulse: u16) {
        self.pulse = pulse
    }
}