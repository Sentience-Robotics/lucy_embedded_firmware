use lucy_embedded_firmware_core::pwm::{PwmChannel};

use embedded_hal::{
    delay::DelayNs,
    digital::OutputPin,
    pwm::SetDutyCycle,
    i2c::I2c,
};

use rp2040_hal::{
    fugit::RateExtU32,
    fugit::MicrosDuration,
    clocks::init_clocks_and_plls,
    gpio::{Pins, FunctionPio0, FunctionPwm, FunctionI2C, PullUp},
    pac,
    i2c::I2C,
    pwm::{Slices, Pwm0, Slice, FreeRunning},
    pio::PIOExt,
    sio::Sio,
    timer::Timer,
    watchdog::Watchdog,
    Clock
};

pub struct Rp2040PwmChannel {
    pub pwm: Slice<Pwm0, FreeRunning>,
}

impl PwmChannel for Rp2040PwmChannel {
    fn set_pwm(&mut self, pulse: u16) {
        self.pwm.channel_a.set_duty_cycle(pulse).unwrap();
    }
}
