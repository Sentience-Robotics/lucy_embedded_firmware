use crate::{pwm::PwmChannel};
use crate::{modbus::RegisterView, modbus::ModbusAdapter, utils::map_range};
use crate::actuators::{*};
use core::f32::consts::PI;

pub enum PwmServoError {
    CommunicationError
}

pub struct PwmServoConfig {
    pub amplitude: f64,
    pub min_angle: f64,
    pub max_angle: f64,
    pub default_angle: f64,
    pub min_pulse: u16,
    pub max_pulse: u16,
}

pub struct PwmServoDriver<'cfg, 'bus, C: PwmChannel> {
    pub config: &'cfg PwmServoConfig,
    pub channel: &'bus mut C,
}

impl<'cfg, 'bus, C: PwmChannel> JointTrajectoryInterface for PwmServoDriver<'cfg, 'bus, C> {
    type Error = PwmServoError;

    fn set_joint_trajectory(&mut self, jtp: JointTrajectoryPoint) -> Result<(), Self::Error> {
        let position = jtp.position.clamp(self.config.min_angle, self.config.max_angle);

        let pulse = (map_range(
            position,
            0f64,
            self.config.amplitude,
            self.config.min_pulse as f64,
            self.config.max_pulse as f64,
        ) + 0.5) as u16;

        self.channel
            .set_pwm(pulse)
            .map_err(|_| PwmServoError::CommunicationError)?;
        Ok(())
    }
}

impl<'cfg, 'bus, C: PwmChannel> TorqueEnableInterface for PwmServoDriver<'cfg, 'bus, C> {
    type Error = PwmServoError;

    fn set_torque_enable(&mut self, status: TorqueStatus) -> Result<(), Self::Error> {
        if status == TorqueStatus::Disabled {
            self.channel
                .set_pwm(0)
                .map_err(|_| PwmServoError::CommunicationError)?;
        }

        Ok(())
    }
}

