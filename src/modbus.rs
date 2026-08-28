use modbus_core::Request;
use modbus_core::Response;
use crc::{Crc, CRC_16_MODBUS};

const MODBUS_CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);

pub struct RegisterTable {
    pub registers: [u16; 0xFF]
}

pub struct Slave {
    pub address: u16,
}

enum ModbusError {
    InvalidAddress,
    InvalidFrame,
    CrcError,
    UnknownOpcode
}

fn parse_modbus_frame(frame: &[u8]) -> Result<Request<'_>, ModbusError> {
    let len = frame.len();

    if len < 4 {
        return Err(ModbusError::InvalidFrame);
    }
    if frame[0] != 0x01 {
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

fn route_modbus_request(register_table: &mut RegisterTable, request: Request<'_>) -> Result<(), ModbusError> {
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
