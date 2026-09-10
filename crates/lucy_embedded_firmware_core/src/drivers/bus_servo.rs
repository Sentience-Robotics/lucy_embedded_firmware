use crate::{uart::UartChannel};
use crate::{modbus::RegisterView, modbus::ModbusAdapter, utils::map_range};

fn compute_checksum(payload: &[u8]) -> u8 {
    let sum: u8 = payload.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    !sum
}

pub struct BusServoConfig {
    pub min_pulse: u16,
    pub max_pulse: u16,
    pub min_angle: u16,
    pub max_angle: u16,
    pub default_angle: u16
}

pub struct BusServoDriver<U> {
    pub config: BusServoConfig,
    pub channel: U,
}

impl<U: UartChannel> BusServoDriver<U> {
    pub fn enable_led(&mut self, id: u8, enable: bool) {
        const REG: u8 = 0x2F;
        const INST_WRITE: u8 = 0x03;
        let length = 4u8;
        let value = if enable { 1u8 } else { 0u8 };

        let mut frame = [
            0xFF,
            0xFF,
            id,
            length,
            INST_WRITE,
            REG,
            value,
            0x00,
        ];

        let payload_to_sum = &frame[2..frame.len() - 1];
        let checksum = compute_checksum(payload_to_sum);
        frame[frame.len() - 1] = checksum;

        self.channel.write(&frame);
    }

    pub fn set_servo_mode(&mut self, id: u8) {
        const REG_TORQUE_ENABLE: u8 = 0x21;
        const INST_WRITE: u8 = 0x03;
        let length = 4u8;
        let value = 0x00;

        let mut frame = [
            0xFF,
            0xFF,
            id,
            length,
            INST_WRITE,
            REG_TORQUE_ENABLE,
            value,
            0x00,
        ];

        let payload_to_sum = &frame[2..frame.len() - 1];
        let checksum = compute_checksum(payload_to_sum);
        frame[frame.len() - 1] = checksum;

        self.channel.write(&frame);
    }

    pub fn enable_torque(&mut self, id: u8, enable: bool) {
        const REG_TORQUE_ENABLE: u8 = 0x28;
        const INST_WRITE: u8 = 0x03;
        let length = 4u8;
        let value = if enable { 1u8 } else { 0u8 };

        let mut frame = [
            0xFF,
            0xFF,
            id,
            length,
            INST_WRITE,
            REG_TORQUE_ENABLE,
            value,
            0x00,
        ];

        let payload_to_sum = &frame[2..frame.len() - 1];
        let checksum = compute_checksum(payload_to_sum);
        frame[frame.len() - 1] = checksum;

        self.channel.write(&frame);
    }

    pub fn move_angle(&mut self, id: u8, angle_rad: u16) {
        let angle_deg = (angle_rad as f32 / 1000.0).to_degrees();
        let clamped_deg = angle_deg.clamp(0 as f32, 360 as f32);
        let pulse = (map_range(
            clamped_deg,
            self.config.min_angle as f32,
            self.config.max_angle as f32,
            self.config.min_pulse as f32,
            self.config.max_pulse as f32,
        ) + 0.5) as u16;



        const REG_TARGET_POSITION: u8 = 0x2A;
        const INST_WRITE: u8 = 0x03;
        let length = 9u8;

        let time: u16 = 0;
        let speed: u16 = 1000;

        let [pos_l, pos_h] = pulse.to_le_bytes();
        let [time_l, time_h] = time.to_le_bytes();
        let [spd_l, spd_h] = speed.to_le_bytes();

        let mut frame = [
            0xFF,
            0xFF,
            id,
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

        self.channel.write(&frame);
    }
}


pub struct BusServoModbusAdapter<'a, U> {
    pub base_register: u16,
    pub cmd_reg_off: u16,
    pub id_reg_off: u16,
    pub angle_reg_off: u16,
    pub driver: &'a mut BusServoDriver<U>,
}

impl<'a, U: UartChannel> ModbusAdapter for BusServoModbusAdapter<'a, U> {
    fn tick(&mut self, rv: &mut RegisterView) {
        let cmd = rv.read_register(self.cmd_reg_off);
        let id = rv.read_register(self.id_reg_off) as u8;
        rv.write_register(self.cmd_reg_off, 0);
        match cmd {
            1 => {
                let angle = rv.read_register(self.angle_reg_off);
                self.driver.move_angle(id, angle);
            },
            2 => {
                self.driver.set_servo_mode(id);
            },
            3 => {
                self.driver.enable_torque(id, true);
            },
            4 => {
                self.driver.enable_led(id, false);
            },
            _ => {

            }
        }
    }

    fn get_nb_register(&self) -> u16 {
        2
    }

    fn get_base_register(&self) -> u16 {
        self.base_register
    }
}
