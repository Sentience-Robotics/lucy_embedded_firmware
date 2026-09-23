use std::io::{Read, Write};
use std::io;
use std::fs::File;
use std::fmt;
use std::time::Duration;
use core::cell::Cell;
use serialport::{SerialPortType, UsbPortInfo};

use lucy_embedded_firmware_core::data::{ActuatorSharedState};
use lucy_embedded_firmware_core::actuators::{JointTrajectoryPoint, JointTrajectoryInterface, TorqueStatus, TorqueEnableInterface, JointStateInterface, TemperatureInterface, TorqueInterface};
use lucy_embedded_firmware_core::link::{Link, AnyLink, Controller};
use lucy_embedded_firmware_core::serial::SerialChannel;
use lucy_embedded_firmware_core::drivers::bus_servo::{BusServoDriver, BusServoConfig};

use memmap2::MmapMut;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, TryFromBytes};

use libc;
use std::ffi::CString;
use std::os::fd::FromRawFd;

pub struct UsbPort {
    target_pid: u16,
    target_vid: u16,
    baud_rate: u32,
    port: Option<Box<dyn serialport::SerialPort>>,
}

impl UsbPort {
    pub const fn new(target_pid: u16, target_vid: u16, baud_rate: u32) -> Self {
        Self {
            target_pid,
            target_vid,
            baud_rate,
            port: None,
        }
    }
}

impl SerialChannel for UsbPort {
    type Error = std::io::Error;

    fn open(&mut self) -> Result<(), Self::Error> {
        let ports = serialport::available_ports().unwrap();

        let matching_port = ports.into_iter().find(|p| {
            if let SerialPortType::UsbPort(UsbPortInfo { vid, pid, .. }) = p.port_type {
                vid == self.target_vid && pid == self.target_pid
            } else {
                false
            }
        });

        match matching_port {
            Some(port_info) => {
                let port = serialport::new(&port_info.port_name, self.baud_rate)
                    .timeout(Duration::from_millis(50))
                    .open()?;
                self.port = Some(port);
                Ok(())
            }
            None => {
                Err(io::Error::new(io::ErrorKind::NotFound, "No matching USB"))
            }
        }
    }

    fn close(&mut self) -> Result<(), Self::Error> {
        self.port = None;
        Ok(())
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        if let Some(port) = &mut self.port {
            port.write_all(bytes)?;
        }
        Ok(())
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        if let Some(port) = &mut self.port {
            let n = port.read_exact(buffer)?;
            Ok(buffer.len())
        } else {
            println!("Error");
            Ok(0)
        }
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        if let Some(port) = &mut self.port {
            port.clear(serialport::ClearBuffer::All)?;
        }
        Ok(())
    }
}

fn node_name() -> String {
    std::env::args()
        .nth(1)
        .or_else(|| std::env::var("LUCY_NODE_NAME").ok())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "lucy".to_string())
}

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
    let file = unsafe { File::from_raw_fd(fd) };
    let len = file
        .metadata()
        .unwrap_or_else(|e| panic!("stat on shm object \"{name}\" failed: {e}"))
        .len() as usize;
    if len < min_len {
        panic!("shm object \"{name}\" is {len} bytes, expected at least {min_len}");
    }
    file
}

fn main() {
    let node = node_name();
    println!("Attaching to Lucy shared memory for node_name={node}");

    let shm_file = open_shm(
        &format!("/{node}"),
        size_of::<ActuatorSharedState>(),
    );

    let table_map = unsafe { MmapMut::map_mut(&shm_file).unwrap() };
    let ass: &mut ActuatorSharedState = unsafe {
        &mut *(table_map.as_ptr() as *mut ActuatorSharedState)
    };

    let target_vid = 0x1a86;
    let target_pid = 0x55d3;

    let robot = [
        BusServoConfig { id: 1, ..Default::default() },
        BusServoConfig { id: 2, ..Default::default() },
        BusServoConfig { id: 3, ..Default::default() },
        BusServoConfig { id: 4, ..Default::default() },
        BusServoConfig { id: 5, ..Default::default() },
        BusServoConfig { id: 6, ..Default::default() },
    ];

    let last_command_seq = ass.commands.command_seq.load(core::sync::atomic::Ordering::SeqCst);
    let last_state_seq = ass.states.state_seq.load(core::sync::atomic::Ordering::SeqCst);
    let mut channel = UsbPort {
        target_pid,
        target_vid,
        baud_rate: 1_000_000,
        port: None,
    };
    let link = AnyLink::Disconnected(Link::new(channel));
    let mut controller = Controller { link: Some(link) };

    let period = Duration::from_millis(20);
    let mut next_tick = std::time::Instant::now();

    loop {
        next_tick += period;

        controller.tick();

        let command_seq = ass.commands.command_seq.load(core::sync::atomic::Ordering::SeqCst);
        let state_seq = ass.states.state_seq.load(core::sync::atomic::Ordering::SeqCst);

        let link = controller.link.as_mut().unwrap().as_connected_mut().unwrap();


        if command_seq != last_command_seq {
            for (index, config) in robot.iter().enumerate() {
                let mut driver = BusServoDriver {
                    config: &config,
                    link: link
                };
                let joint_trajectory_point = JointTrajectoryPoint {
                    position: ass.commands.hw_commands[index],
                    velocity: 0f64,
                    acceleration: ass.commands.hw_accelerations[index],
                };
                let torque_status = TorqueStatus::from(ass.commands.hw_torque_enabled[index]);
                if (torque_status == TorqueStatus::Enabled) {
                    driver.set_joint_trajectory(joint_trajectory_point);
                }
                driver.set_torque_enable(torque_status);
            }
        }

        for (index, config) in robot.iter().enumerate() {
            let mut driver = BusServoDriver {
                config: &config,
                link: link
            };
            if let Ok(position) = driver.get_position() {
                ass.states.state_seq.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
                ass.states.hw_positions[index] = position;
                ass.states.state_seq.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
            }
        }
        
        let now = std::time::Instant::now();
        if now < next_tick {
            std::thread::sleep(next_tick - now);
        } else {
            eprintln!("Warning: tick took longer than expected {:?}", now - next_tick);
            next_tick = now;
        }
    }
}
