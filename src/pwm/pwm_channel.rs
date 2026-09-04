pub trait PwmChannel {
    fn set_pwm(&mut self, pulse: u16);
}
