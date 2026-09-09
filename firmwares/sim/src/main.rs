use std::io::{Read, Write};
use std::fs::{File, OpenOptions};
use std::fmt;
use std::time::Duration;
use core::cell::Cell;
use modbus_core::{Request, Response, FunctionCode};
use serialport::{SerialPortType, UsbPortInfo};

use memmap2::MmapMut;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, TryFromBytes};

use libc;
use std::ffi::CString;


#[repr(C)]
struct RegisterTable {
    pub registers: [Cell<u16>; 0xFF],
}


impl Clone for RegisterTable {
    fn clone(&self) -> Self {
        let ntable = RegisterTable::default();
        for i in 0..0xFF {
            ntable.registers[i].set(self.registers[i].get());
        }
        ntable
    }
}

pub trait Copy: Clone {}

impl RegisterTable {
    const fn new() -> Self {
        Self {
            registers: [const { Cell::new(0) }; 0xFF],
        } 
    }
}

impl Default for RegisterTable {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PosixNamedSem {
    sem: *mut libc::sem_t,
    name: CString,
}

impl PosixNamedSem {
    pub fn create(name: &str, val: u32) -> Self {
        let c_name = CString::new(name).unwrap();
        let sem = unsafe {
            libc::sem_open(
                c_name.as_ptr(),
                libc::O_CREAT,
                0o644,
                val as libc::c_uint,
            )
        };
        PosixNamedSem {
            sem: sem,
            name: c_name,
        }
    }

    pub fn wait(&self) {
        unsafe { libc::sem_wait(self.sem) };
    }

    pub fn post(&self) {
        unsafe { libc::sem_post(self.sem) };
    }
}

#[repr(C)]
struct RegisterHeader {
    header: [u8; 32],
    iterator: u16,
}

impl RegisterHeader {
    fn iter(&self) -> RegisterHeaderIter<'_> {
        RegisterHeaderIter {
            header: self,
            cursor: 0,
        }
    }

    fn get_register_status(&self, register: u16) -> bool {
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

pub struct RegisterHeaderIter<'a> {
    header: &'a RegisterHeader,
    cursor: u16,
}

impl<'a> Iterator for RegisterHeaderIter<'a> {
    type Item = (u16, bool);

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor < 32 * 8 {
            let value = (self.cursor, self.header.get_register_status(self.cursor));
            self.cursor += 1;
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
    let angle: u16 = ((1000 as f32 / 1000f32) * (180.0 / 3.14)) as u16;
    println!("{}", angle);

    let mut reg_table_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/shm/lucy_hardware_interface_left_arm.lucy_reg_table").unwrap();
    let mut mmap = unsafe { MmapMut::map_mut(&reg_table_file).unwrap() };
    let rt: &RegisterTable = unsafe {
        &*(mmap.as_ptr() as *const RegisterTable)
    };

    let mut reg_header_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/shm/lucy_hardware_interface_left_arm.lucy_reg_header").unwrap();
    let mut mmap = unsafe { MmapMut::map_mut(&reg_header_file).unwrap() };
    let rh: &mut RegisterHeader = unsafe {
        &mut *(mmap.as_ptr() as *mut RegisterHeader)
    };

    let sem = PosixNamedSem::create("/lucy_hardware_interface_left_arm", 1);
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
        /*let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        let input = input.trim();

        let vec: Vec<_> = input.split(',').collect();
        let reg = vec[0].parse::<u16>().unwrap();
        let value = vec[1].parse::<u16>().unwrap();
        rt.registers[reg as usize].set(value);
        rh.set_dirty(reg);
        println!("Done");*/

        let mut changes:Vec<u16> = Vec::new();
        //sem.wait();
        for (iterator, status) in rh.iter() {
            if status {
                changes.push(iterator);
            }
        }
        for register in changes {
            println!("Register done on {} - {}", register, rt.registers[register as usize].get());
            let packet = write_register(0x01, register, rt.registers[register as usize].get());
            let _ = port.write_all(&packet);
            rh.set_clean(register);
            std::thread::sleep(Duration::from_millis(10));
        }
        {
            let mut buf = [0; 0xff];
            port.read(&mut buf);
            /*print!("Port:");
            for ch in buf {
                print!("{:?}", ch as char)
            }
            println!("");*/

        }
        //sem.post();
    }
}
