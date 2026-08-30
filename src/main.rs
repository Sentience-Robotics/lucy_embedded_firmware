pub mod servo_driver;
pub mod metadata_temp;
pub mod driver_generic;
pub mod servo_hub_driver;
pub mod pressure_sensor_driver;
use crate::{servo_driver::SG90ModBusAdapter};

fn main() {
    println!("Starting!");
    let servo: SG90ModBusAdapter = SG90ModBusAdapter::new(0, 0 ,0 ,0, 0, 0, 0);


}
