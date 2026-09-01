use core::result::Result;

pub enum TransportError {
    WriteError,
    ReadError,
    FlushError,
}

pub trait Transport {
    type Error;

    fn write(&mut self, data: &[u8]) -> Result<(), Self::Error>;
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;
    fn flush(&mut self) -> Result<(), Self::Error>;
}

/*
pub struct DummyTransport {
}

impl Transport for DummyTransport {
    type Error = TransportError;

    fn write(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        let mut ibuffer = String::new();
        io::stdin().read_line(&mut ibuffer).expect("Failed to read line");
        let len = usize::min(ibuffer.len(), buffer.len());
        buffer[..len].copy_from_slice(&ibuffer[..len].as_bytes());
        Ok(len)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub struct NamedPipeTransport {
    path: String,
    file: Option<File>
}

impl NamedPipeTransport {
    pub fn new(path: String) -> Self {
        NamedPipeTransport {
            path,
            file: None
        }
    }

    pub fn open(&mut self) -> io::Result<()> {
        let file = File::open(&self.path)?;
        self.file = Some(file);
        Ok(())
    }

    pub fn close(&mut self) {
        self.file = None;
    }
}

impl Transport for NamedPipeTransport {
    type Error = io::Error;

    fn write(&mut self, data: &[u8]) -> std::result::Result<(), Self::Error> {
        if let Some(file) = &mut self.file {
            file.write_all(data)?;
            file.flush()?;
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::NotConnected, "Pipe not open"))
        }
    }

    fn read(&mut self, buffer: &mut [u8]) -> std::result::Result<usize, Self::Error> {
        if let Some(file) = &mut self.file {
            let bytes_read = file.read(buffer)?;
            Ok(bytes_read)
        } else {
            Err(io::Error::new(io::ErrorKind::NotConnected, "Pipe not open"))
        }
    }

    fn flush(&mut self) -> std::result::Result<(), Self::Error> {
        if let Some(file) = &mut self.file {
            file.flush()?;
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::NotConnected, "Pipe not open"))
        }
    }
}
*/
