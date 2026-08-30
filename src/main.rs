pub mod drivers;
use crate::{drivers::servo_driver::SG90ModBusAdapter};

fn main() {
    println!("Starting!");
    let servo: SG90ModBusAdapter = SG90ModBusAdapter::new(0, 0 ,0 ,0, 0, 0, 0);


}
