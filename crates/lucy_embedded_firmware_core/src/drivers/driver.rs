pub enum DriverError {
    CommunicationError,
}

pub trait Driver {
    fn tick(&mut self) -> Result<(), DriverError>;
}
