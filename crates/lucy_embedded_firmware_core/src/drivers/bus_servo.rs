use crate::{serial::SerialChannel};
use crate::actuators::{*};
use crate::{utils::map_range};
use core::f32::consts::TAU;

fn compute_checksum(payload: &[u8]) -> u8 {
    let sum: u8 = payload.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    !sum
}

pub struct BusServoConfig {
    pub id: u8,
    pub min_pulse: u16,
    pub max_pulse: u16,
    pub min_angle: f64,
    pub max_angle: f64,
    pub default_angle: f64,
}

const INST_WRITE: u8 = 0x03;

pub struct BusServoDriver<'cfg, 'bus, U> {
    pub config: &'cfg BusServoConfig,
    pub channel: &'bus mut U,
}

pub enum BusServoError {
    OutOfLimits,
    CommunicationError,
}

impl<'cfg, 'bus, S: SerialChannel> JointTrajectoryInterface for BusServoDriver<'cfg, 'bus, S> {
    type Error = BusServoError;

    fn set_joint_trajectory(&mut self, joint_trajectory_point: JointTrajectoryPoint) -> Result<(), Self::Error> {
        let position = joint_trajectory_point.position.clamp(self.config.min_angle as f64, self.config.max_angle as f64);
        let time: u16 = 0;
        let velocity: u16 = joint_trajectory_point.velocity as u16;

        let pulse = (map_range(
            position as f64,
            0 as f64,
            TAU as f64,
            self.config.min_pulse as f64,
            self.config.max_pulse as f64,
        ) + 0.5) as u16;

        const REG_TARGET_POSITION: u8 = 0x2A;

        let [pos_l, pos_h] = pulse.to_le_bytes();
        let [time_l, time_h] = time.to_le_bytes();
        let [spd_l, spd_h] = velocity.to_le_bytes();
        let length = 9u8;

        let mut frame = [
            0xFF,
            0xFF,
            self.config.id,
            length,
            INST_WRITE,
            REG_TARGET_POSITION,
            pos_l,
            pos_h,
            time_l,
            time_h,
            spd_l,
            spd_h,
            0x00
        ];
        let payload_to_sum = &frame[2..frame.len() - 1];
        let checksum = compute_checksum(payload_to_sum);
        frame[frame.len() - 1] = checksum;

        self.channel
            .write(&frame)
            .map_err(|_| BusServoError::CommunicationError)?;
        Ok(())
    }
}

impl<'cfg, 'bus, S: SerialChannel> TorqueEnableInterface for BusServoDriver<'cfg, 'bus, S> {
    type Error = BusServoError;

    fn set_torque_enable(&mut self, state: TorqueStatus) -> Result<(), Self::Error>{
        const REG_TORQUE_ENABLE: u8 = 0x28;
        let length = 4u8;
        let value = match state {
            TorqueStatus::Enabled => 1u8,
            TorqueStatus::Disabled => 0u8,
        };

        let mut frame = [
            0xFF,
            0xFF,
            self.config.id,
            length,
            INST_WRITE,
            REG_TORQUE_ENABLE,
            value,
            0x00,
        ];

        let payload_to_sum = &frame[2..frame.len() - 1];
        let checksum = compute_checksum(payload_to_sum);
        frame[frame.len() - 1] = checksum;

        self.channel
            .write(&frame)
            .map_err(|_| BusServoError::CommunicationError)?;
        Ok(())
    }
}
