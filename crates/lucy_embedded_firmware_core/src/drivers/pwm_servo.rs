use crate::pwm::PwmChannel;
use crate::{
    modbus::{ModbusAdapter, RegisterView},
    utils::{deg_to_millirad, millirad_to_pulse},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct PwmServoConfig {
    pub min_pulse: u16,
    pub max_pulse: u16,
    pub min_angle: u16,
    pub max_angle: u16,
    /// Default angle in **degrees** (converted to milliradians on reset).
    pub default_angle: u16,
}

pub struct PwmServoDriver<C> {
    pub config: PwmServoConfig,
    pub channel: C,
}

impl<C: PwmChannel> PwmServoDriver<C> {
    /// `angle_millirad` is radians × 1000 (matches LucySystemHardware SHM encoding).
    pub fn move_angle(&mut self, angle_millirad: u16) {
        let pulse = millirad_to_pulse(
            angle_millirad,
            self.config.min_angle,
            self.config.max_angle,
            self.config.min_pulse,
            self.config.max_pulse,
        );
        let _ = self.channel.set_pwm(pulse);
    }

    pub fn reset_angle(&mut self) {
        self.move_angle(deg_to_millirad(self.config.default_angle as f32));
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
            }
            2 => {
                self.driver.reset_angle();
            }
            _ => {}
        }
    }

    fn get_nb_register(&self) -> u16 {
        2
    }

    fn get_base_register(&self) -> u16 {
        self.base_register
    }
}
