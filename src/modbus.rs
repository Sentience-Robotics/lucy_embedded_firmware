use modbus_core::{Request, Response};
use crc::{Crc, CRC_16_MODBUS};

use core::{
    error::Error,
    result::Result::{self, Err, Ok},
    option::Option::{self, None, Some},
};

const MODBUS_CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);



pub struct RegisterTable {
    pub registers: [u16; 0xFF]
}

pub struct RegisterView<'a> {
    pub table: &'a mut RegisterTable
}

impl<'a> RegisterView<'a> {
    pub fn new(table: &'a mut RegisterTable) -> Self {
        RegisterView { table }
    }

    pub fn read_register(&self, index: u16) -> u16 {
        if index < self.table.registers.len() as u16 {
            self.table.registers[index as usize]
        } else {
            0
        }
    }

    pub fn write_register(&mut self, index: u16, value: u16) {
        if index < self.table.registers.len() as u16 {
            self.table.registers[index as usize] = value;
        }
    }
}



pub struct Slave {
    pub address: u8,
}

pub enum ModbusError {
    InvalidAddress,
    InvalidFrame,
    CrcError,
    UnknownOpcode
}

pub fn parse_modbus_frame<'a>(slave: &'a Slave, frame: &'a[u8]) -> Result<Request<'a>, ModbusError> {
    let len = frame.len();

    if len < 4 {
        return Err(ModbusError::InvalidFrame);
    }
    if frame[0] != slave.address {
        return Err(ModbusError::InvalidAddress);
    }

    let payload_to_check = &frame[..len - 2];
    let crc = MODBUS_CRC.checksum(payload_to_check);
    let crc_received = u16::from_le_bytes([frame[len - 2], frame[len - 1]]);
    if crc != crc_received {
        return Err(ModbusError::CrcError);
    }

    let pdu = &frame[1..len - 2];

    match Request::try_from(pdu) {
        Ok(request) => Ok(request),
        Err(_) => Err(ModbusError::UnknownOpcode)
    }
}

pub fn route_modbus_request(register_table: &mut RegisterTable, request: Request<'_>) -> Result<(), ModbusError> {
    match request {
        Request::ReadHoldingRegisters(addr, quantity) => {
            let registers = &register_table.registers[addr as usize..(addr + quantity) as usize];
            Ok(())
        },
        Request::WriteSingleRegister(addr, value) => {
            register_table.registers[addr as usize] = value;
            Ok(())
        },
        Request::WriteMultipleRegisters(addr, data) => {
            let end_addr = addr as usize + data.len();
            if end_addr > register_table.registers.len() {
                return Err(ModbusError::InvalidFrame);
            }
            for i in 0..data.len() {
                if let Some(value) = data.get(i) {
                    register_table.registers[addr as usize + i] = value;
                }
            }
            Ok(())
        },
        _ => {
            Err(ModbusError::UnknownOpcode)
        }
    }
}
