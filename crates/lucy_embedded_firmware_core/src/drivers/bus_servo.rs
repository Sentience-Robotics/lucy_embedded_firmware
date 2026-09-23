use crate::{serial::SerialChannel};
use crate::link::{Link, Connected, Disconnected};
use crate::actuators::{*};
use crate::{utils::map_range};
use core::f32::consts::{TAU, PI};

fn compute_checksum(payload: &[u8]) -> u8 {
    let sum: u8 = payload.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    !sum
}

pub struct BusServoConfig {
    pub id: u8,
    pub amplitude: f64,
    pub min_angle: f64,
    pub max_angle: f64,
    pub default_angle: f64,
    pub min_pulse: u16,
    pub max_pulse: u16,
}

impl Default for BusServoConfig {
    fn default() -> Self {
        BusServoConfig {
            id: 1,
            amplitude: TAU as f64,
            min_angle: 0.0,
            max_angle: TAU as f64,
            default_angle: PI as f64,
            min_pulse: 0,
            max_pulse: 4095,
        }
    }
}

const INST_READ: u8 = 0x02;
const INST_WRITE: u8 = 0x03;

pub struct BusServoDriver<'cfg, 'bus, C: SerialChannel> {
    pub config: &'cfg BusServoConfig,
    pub link: &'bus mut Link<Connected, C>,
}

pub enum BusServoError {
    OutOfLimits,
    CommunicationError,
}

impl<'cfg, 'bus, C: SerialChannel> JointTrajectoryInterface for BusServoDriver<'cfg, 'bus, C> {
    type Error = BusServoError;

    fn set_joint_trajectory(&mut self, joint_trajectory_point: JointTrajectoryPoint) -> Result<(), Self::Error> {
        let position = joint_trajectory_point.position.clamp(self.config.min_angle as f64, self.config.max_angle as f64);
        let time: u16 = 0;
        let velocity: u16 = joint_trajectory_point.velocity as u16;

        let pulse = (map_range(
            position as f64,
            0 as f64,
            self.config.amplitude,
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

        self.link.channel
            .clear()
            .map_err(|_| BusServoError::CommunicationError)?;

        self.link.channel
            .write(&frame)
            .map_err(|_| BusServoError::CommunicationError)?;

        let mut echo_frame = [0u8; 8];
        self.link.channel
            .read(&mut echo_frame)
            .map_err(|_| BusServoError::CommunicationError)?;
        Ok(())
    }
}

impl<'cfg, 'bus, C: SerialChannel> TorqueEnableInterface for BusServoDriver<'cfg, 'bus, C> {
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

        self.link.channel
            .clear()
            .map_err(|_| BusServoError::CommunicationError)?;

        self.link.channel
            .write(&frame)
            .map_err(|_| BusServoError::CommunicationError)?;

        let mut echo_frame = [0u8; 6];
        self.link.channel
            .read(&mut echo_frame)
            .map_err(|_| BusServoError::CommunicationError)?;
        Ok(())
    }
}

impl<'cfg, 'bus, C: SerialChannel> JointStateInterface for BusServoDriver<'cfg, 'bus, C> {
    type Error = BusServoError;

    fn get_position(&mut self) -> Result<f64, Self::Error> {
        const REG_STATE_POSITION: u8 = 0x38;
        let length = 4u8;
        let bytes_to_read = 2u8;

        let mut tx_frame = [
            0xFF,
            0xFF,
            self.config.id,
            length,
            INST_READ,
            REG_STATE_POSITION,
            bytes_to_read,
            0x00,
        ];
        let payload = &tx_frame[2..tx_frame.len() - 1];
        tx_frame[tx_frame.len() - 1] = compute_checksum(payload);

        self.link.channel
            .clear()
            .map_err(|_| BusServoError::CommunicationError)?;

        self.link.channel
            .write(&tx_frame)
            .map_err(|_| BusServoError::CommunicationError)?;

        let mut rx_frame = [0u8; 8];
        let n = self.link.channel
            .read(&mut rx_frame)
            .map_err(|_| BusServoError::CommunicationError)?;

        let rx_payload = &rx_frame[2..rx_frame.len() - 1];
        let expected_checksum = compute_checksum(rx_payload);
        if rx_frame[rx_frame.len() - 1] != expected_checksum {
            return Err(BusServoError::CommunicationError.into());
        }

        let error_status = rx_frame[4];
        let pulse = u16::from_le_bytes([rx_frame[5], rx_frame[6]]);
        let position = map_range(
            pulse as f64,
            0f64,
            4096f64,
            0f64,
            TAU as f64
        );

        Ok(position)
    }
}

impl<'cfg, 'bus, C: SerialChannel> TemperatureInterface for BusServoDriver<'cfg, 'bus, C> {
    type Error = BusServoError;

    fn get_temperature(&mut self) -> Result<f64, Self::Error> {
        const REG_TEMP: u8 = 0x3F;
        let length = 4u8;
        let bytes_to_read = 1u8;

        let mut tx_frame = [
            0xFF,
            0xFF,
            self.config.id,
            length,
            INST_READ,
            REG_TEMP,
            bytes_to_read,
            0x00,
        ];
        let payload = &tx_frame[2..tx_frame.len() - 1];
        tx_frame[tx_frame.len() - 1] = compute_checksum(payload);

        self.link.channel
            .clear()
            .map_err(|_| BusServoError::CommunicationError)?;

        self.link.channel
            .write(&tx_frame)
            .map_err(|_| BusServoError::CommunicationError)?;

        let mut rx_frame = [0u8; 7];
        let n = self.link.channel
            .read(&mut rx_frame)
            .map_err(|_| BusServoError::CommunicationError)?;

        let rx_payload = &rx_frame[2..rx_frame.len() - 1];
        let expected_checksum = compute_checksum(rx_payload);
        if rx_frame[rx_frame.len() - 1] != expected_checksum {
            return Err(BusServoError::CommunicationError.into());
        }

        let error_status = rx_frame[4];
        let temperature = rx_frame[5];

        Ok(temperature as f64)
    }
}

impl<'cfg, 'bus, C: SerialChannel> TorqueInterface for BusServoDriver<'cfg, 'bus, C> {
    type Error = BusServoError;

    fn get_torque(&mut self) -> Result<f64, Self::Error> {
        const REG_LOAD: u8 = 0x3c;
        let length = 4u8;
        let bytes_to_read = 2u8;

        let mut tx_frame = [
            0xFF,
            0xFF,
            self.config.id,
            length,
            INST_READ,
            REG_LOAD,
            bytes_to_read,
            0x00,
        ];
        let payload = &tx_frame[2..tx_frame.len() - 1];
        tx_frame[tx_frame.len() - 1] = compute_checksum(payload);

        self.link.channel
            .clear()
            .map_err(|_| BusServoError::CommunicationError)?;

        self.link.channel
            .write(&tx_frame)
            .map_err(|_| BusServoError::CommunicationError)?;

        let mut rx_frame = [0u8; 8];
        let n = self.link.channel
            .read(&mut rx_frame)
            .map_err(|_| BusServoError::CommunicationError)?;

        let rx_payload = &rx_frame[2..rx_frame.len() - 1];
        let expected_checksum = compute_checksum(rx_payload);
        if rx_frame[rx_frame.len() - 1] != expected_checksum {
            return Err(BusServoError::CommunicationError.into());
        }

        let error_status = rx_frame[4];

        let raw = u16::from_le_bytes([rx_frame[5], rx_frame[6]]);
        let magnitude = (raw & 0x3FF) as f64 / 1000.0;
        let sign = if raw & 0x400 != 0 { -1.0 } else { 1.0 };
        let torque = sign * magnitude;

        Ok(torque)
    }
}
