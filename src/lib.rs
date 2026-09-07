#![no_std]

mod driver_generic;
mod modbus;
mod pressure_sensor_driver;
mod servo_driver;
mod servo_hub_driver;

pub use driver_generic::*;
pub use pressure_sensor_driver::*;
pub use servo_driver::*;
pub use servo_hub_driver::*;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
