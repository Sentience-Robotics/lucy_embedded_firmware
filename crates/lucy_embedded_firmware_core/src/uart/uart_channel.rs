use core::result::Result;

pub trait UartChannel {
    type Error;
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;
}
