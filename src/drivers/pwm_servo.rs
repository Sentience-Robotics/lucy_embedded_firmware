use crate::{pwm::PwmChannel};
use crate::{modbus::RegisterView, modbus::ModbusAdapter, utils::map_range};

pub struct PwmServoDriver<C> {
    pub channel: C,
    pub min_pulse: u16,
    pub max_pulse: u16,
    pub min_angle: u16,
    pub max_angle: u16,
    pub default_angle: u16
}

impl<C: PwmChannel> PwmServoDriver<C> {
    pub fn move_angle(&mut self, angle: u16) {
        let angle = angle.clamp(self.min_angle, self.max_angle);
        let pulse = map_range(angle as f32, self.min_angle as f32, self.max_angle as f32, self.min_pulse as f32, self.max_pulse as f32) as u16;
        self.channel.set_pwm(pulse);
    }

    pub fn reset_angle(&mut self) {
        let angle = self.default_angle.clamp(self.min_angle, self.max_angle);
        let pulse = map_range(angle as f32, self.min_angle as f32, self.max_angle as f32, self.min_pulse as f32, self.max_pulse as f32) as u16;
        self.channel.set_pwm(pulse);
    }
}

pub struct PwmServoModbusAdapter<'a, C> {
    pub base_register: u16,
    pub cmd_reg_off: u16,
    pub angle_reg_off: u16,
    pub driver: &'a mut PwmServoDriver<C>,
}

impl<'a, C: PwmChannel> ModbusAdapter for PwmServoModbusAdapter<'a, C> {
    fn tick(&mut self, rv: &mut RegisterView) {
        let cmd = rv.read_register(self.cmd_reg_off);
        rv.write_register(self.cmd_reg_off, 0);
        match cmd {
            1 => {
                let angle = rv.read_register(self.angle_reg_off);
                self.driver.move_angle(angle);
            },
            2 => {
                self.driver.reset_angle();
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
