use std::io::{Read, Write};
use std::fs::File;
use std::fmt;
use std::time::Duration;
use modbus_core::{Request, Response, FunctionCode};
use serialport::{SerialPortType, UsbPortInfo};
use memmap2::MmapMut;

use lucy_embedded_firmware_core::{
    modbus::{RegisterTable}
};

struct RegisterHeader {
    header: [u8; 32],
    iterator: u16,
}

impl RegisterHeader {
    fn get_register_status(&mut self, register: u16) -> bool {
        let index: u16 = register / 8;
        let index2: u16 = register % 8;
        ((self.header[index as usize] >> (7 - index2)) & 0b1) != 0
    }

    fn switch_register_status(&mut self, register: u16) {
        let index: u16 = register / 8;
        let index2: u16 = register % 8;
        self.header[index as usize] = self.header[index as usize] ^ ((1 & 0xFF) << (7 - index2))
    }

    fn set_dirty(&mut self, register: u16) {
        if self.get_register_status(register) {
            return
        }
        self.switch_register_status(register);
    }

    fn set_clean(&mut self, register: u16) {
        if !self.get_register_status(register) {
            return
        }
        self.switch_register_status(register);
    }
}

impl fmt::Display for RegisterHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[\n");
        for tmp in self.header.iter() {
            write!(f, "{:08b}\n", tmp);
        }
        write!(f, "]")
    }
}

impl Iterator for RegisterHeader {
    type Item = (u16, bool);

    fn next(&mut self) -> Option<Self::Item> {
        if self.iterator < 32 * 8 {
            let value = (self.iterator, self.get_register_status(self.iterator));
            self.iterator += 1;
            Some(value)
        } else {
            None
        }
    }
}

fn append_crc(frame: &mut Vec<u8>) {
    let crc = modbus_core::rtu::crc16(&frame);
    frame.extend_from_slice(&crc.to_be_bytes());
}

fn write_register(
    slave_addr: u8,
    reg_addr: u16,
    reg_value: u16
) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.push(slave_addr);

    frame.push(FunctionCode::WriteSingleRegister.value());
    frame.extend_from_slice(&reg_addr.to_be_bytes());
    frame.extend_from_slice(&reg_value.to_be_bytes());

    append_crc(&mut frame);
    frame
}

fn main() {
    /*let shm_path = "/dev/shm/firmware.bin";
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&shm_path).unwrap();
    let mut mmap = unsafe { MmapMut::map_mut(&file).unwrap() };*/

    let rt = RegisterTable::default();
    let mut header = RegisterHeader {
        header: [0u8; 32],
        iterator: 0,
    }; 

    let target_vid = 0x16c0;
    let target_pid = 0x27dd;

    let ports = serialport::available_ports().unwrap();
    let matching_port = ports.into_iter().find(|p| {
        if let SerialPortType::UsbPort(UsbPortInfo { vid, pid, .. }) = p.port_type {
            vid == target_vid && pid == target_pid
        } else {
            false
        }
    });

    let port_info = matching_port.ok_or("Périphérique RP2040 introuvable. Est-il branché ?").unwrap();
    println!("Périphérique trouvé sur : {}", port_info.port_name);

    let mut port = serialport::new(&port_info.port_name, 115_200)
        .timeout(Duration::from_millis(1000))
        .open().unwrap();

    loop {
        print!("> ");
        std::io::stdout().flush();
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        let input = input.trim();

        let vec: Vec<_> = input.split(',').collect();
        let reg = vec[0].parse::<u16>().unwrap();
        let value = vec[1].parse::<u16>().unwrap();
        rt.registers[reg as usize].set(value);
        header.set_dirty(reg);

        let mut changes:Vec<u16> = Vec::new();
        header.iterator = 0;
        for (iterator, status) in &mut header {
            if status {
                changes.push(iterator);
            }
        }
        for register in changes {
            let packet = write_register(0x01, register, rt.registers[register as usize].get());
            let _ = port.write_all(&packet);
            header.set_clean(register);
            println!("Changes send");
        }
    }
}
