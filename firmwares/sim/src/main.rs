use std::io::{Read, Write};
use std::fs::File;
use std::fmt;
use std::time::Duration;
use core::cell::Cell;
use modbus_core::{Request, Response, FunctionCode};
use serialport::{SerialPortType, UsbPortInfo};

use memmap2::MmapMut;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, TryFromBytes};

use libc;
use std::ffi::CString;
use std::os::fd::FromRawFd;


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
        // SEM_FAILED stored unchecked would surface as UB on the first wait().
        // macOS caps these names at 30 bytes, so a long node name lands here.
        if sem == libc::SEM_FAILED {
            panic!(
                "sem_open(\"{name}\") failed: {}",
                std::io::Error::last_os_error(),
            );
        }
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

/// Node name the ROS 2 hardware interface used to name its shared-memory objects.
///
/// Mirrors LucySystemHardware's `node_name` hardware parameter (default `lucy`),
/// so the two sides agree without either hard-coding a robot. Override with
/// argv[1] or LUCY_NODE_NAME to attach to a differently-named instance.
fn node_name() -> String {
    std::env::args()
        .nth(1)
        .or_else(|| std::env::var("LUCY_NODE_NAME").ok())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "lucy".to_string())
}

/// Open an existing POSIX shared-memory object by name.
///
/// shm_open rather than a path under /dev/shm: that directory is Linux-only and
/// macOS gives these objects no filesystem entry at all. The hardware interface
/// owns the segment, so O_CREAT is deliberately not passed - failing here is the
/// right outcome when the ROS 2 side is not up yet.
fn open_shm(name: &str, min_len: usize) -> File {
    let c_name = CString::new(name).expect("shm name contains a NUL byte");
    let fd = unsafe { libc::shm_open(c_name.as_ptr(), libc::O_RDWR, 0o666 as libc::c_uint) };
    if fd == -1 {
        panic!(
            "shm_open(\"{name}\") failed: {}. Is the ROS 2 stack running with node_name={}?",
            std::io::Error::last_os_error(),
            name.trim_start_matches('/').split('.').next().unwrap_or(name),
        );
    }
    // Takes ownership of the descriptor, so it closes with the File.
    let file = unsafe { File::from_raw_fd(fd) };
    let len = file
        .metadata()
        .unwrap_or_else(|e| panic!("stat on shm object \"{name}\" failed: {e}"))
        .len() as usize;
    // mmap covers the whole object; reading a struct out of a shorter one is UB.
    if len < min_len {
        panic!("shm object \"{name}\" is {len} bytes, expected at least {min_len}");
    }
    file
}

fn main() {
    let node = node_name();
    println!("Attaching to Lucy shared memory for node_name={node}");

    let reg_table_file = open_shm(
        &format!("/{node}.lucy_reg_table"),
        size_of::<RegisterTable>(),
    );
    // Kept in scope: dropping the mapping would invalidate `rt`.
    let table_map = unsafe { MmapMut::map_mut(&reg_table_file).unwrap() };
    let rt: &RegisterTable = unsafe {
        &*(table_map.as_ptr() as *const RegisterTable)
    };

    let reg_header_file = open_shm(
        &format!("/{node}.lucy_reg_header"),
        size_of::<RegisterHeader>(),
    );
    let mut header_map = unsafe { MmapMut::map_mut(&reg_header_file).unwrap() };
    let rh: &mut RegisterHeader = unsafe {
        &mut *(header_map.as_mut_ptr() as *mut RegisterHeader)
    };

    let sem = PosixNamedSem::create(&format!("/{node}"), 1);
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
