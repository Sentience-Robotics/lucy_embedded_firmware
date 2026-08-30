use crate::metadata_temp::{RegisterView};

#[derive(Debug)]
pub enum ModBusDriverError {
    Error
}



pub trait ModBusDriver {
    fn tick(&mut self, view: RegisterView) -> Result<(), ModBusDriverError>;

    fn getNbRegisters() -> u16;

    fn getBaseRegister(&mut self) ->  u16;
}
