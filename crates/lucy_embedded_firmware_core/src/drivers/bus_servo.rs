use crate::{uart::UartChannel};
use crate::{modbus::RegisterView, modbus::ModbusAdapter, utils::map_range};

fn compute_checksum(payload: &[u8]) -> u8 {
    let sum: u8 = payload.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    !sum
}

pub struct BusServoConfig {
    pub id: u8,
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
    pub fn enable_led(&mut self, enable: bool) {
        const REG: u8 = 0x2F;
        const INST_WRITE: u8 = 0x03;
        let length = 4u8;
        let value = if enable { 1u8 } else { 0u8 };

        let mut frame = [
            0xFF,
            0xFF,
            self.config.id,
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

    pub fn set_servo_mode(&mut self) {
        const REG_TORQUE_ENABLE: u8 = 0x21;
        const INST_WRITE: u8 = 0x03;
        let length = 4u8;
        let value = 0x00;

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

        self.channel.write(&frame);
    }

    pub fn enable_torque(&mut self, enable: bool) {
        const REG_TORQUE_ENABLE: u8 = 0x28;
        const INST_WRITE: u8 = 0x03;
        let length = 4u8;
        let value = if enable { 1u8 } else { 0u8 };

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

        self.channel.write(&frame);
    }

    pub fn move_angle(&mut self, angle: u16) {
        const REG_TARGET_POSITION: u8 = 0x2A;
        const INST_WRITE: u8 = 0x03;
        let length = 9u8;

        let time: u16 = 0;
        let speed: u16 = 1000;

        let [pos_l, pos_h] = angle.to_le_bytes();
        let [time_l, time_h] = time.to_le_bytes();
        let [spd_l, spd_h] = speed.to_le_bytes();

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

        self.channel.write(&frame);
    }
}


pub struct BusServoModbusAdapter<'a, U> {
    pub base_register: u16,
    pub cmd_reg_off: u16,
    pub angle_reg_off: u16,
    pub driver: &'a mut BusServoDriver<U>,
}

impl<'a, U: UartChannel> ModbusAdapter for BusServoModbusAdapter<'a, U> {
    fn tick(&mut self, rv: &mut RegisterView) {
        let cmd = rv.read_register(self.cmd_reg_off);
        rv.write_register(self.cmd_reg_off, 0);
        match cmd {
            1 => {
                let angle = rv.read_register(self.angle_reg_off);
                self.driver.move_angle(angle);
            },
            2 => {
                self.driver.set_servo_mode();
            },
            3 => {
                self.driver.enable_torque(true);
            },
            4 => {
                self.driver.enable_led(false);
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
