use std::io::{Read, Write};
use std::fs::File;
use std::fmt;
use std::time::Duration;
use core::cell::Cell;
use serialport::{SerialPortType, UsbPortInfo};

use lucy_embedded_firmware_core::data::{ActuatorSharedState};
use lucy_embedded_firmware_core::actuators::{JointTrajectoryPoint, JointTrajectoryInterface, TorqueStatus, TorqueEnableInterface};
use lucy_embedded_firmware_core::serial::SerialChannel;
use lucy_embedded_firmware_core::drivers::bus_servo::{BusServoDriver, BusServoConfig};

use memmap2::MmapMut;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, TryFromBytes};

use libc;
use std::ffi::CString;
use std::os::fd::FromRawFd;

pub struct UsbPort {
    port: Box<dyn serialport::SerialPort>,
}

impl SerialChannel for UsbPort {
    type Error = std::io::Error;

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.port.write_all(bytes)?;
        Ok(())
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        let n = self.port.read(buffer)?;
        Ok(n)
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
    let ass: &ActuatorSharedState = unsafe {
        &*(table_map.as_ptr() as *const ActuatorSharedState)
    };

    let target_vid = 0x1a86;
    let target_pid = 0x55d3;

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
        .timeout(Duration::from_millis(50))
        .open().unwrap();

    let config = BusServoConfig {
        id: 1,
        min_pulse: 0,
        max_pulse: 4096,
        min_angle: 0.0,
        max_angle: 360.0,
        default_angle: 5.0,
    };

    let last_command_seq = ass.commands.command_seq.load(core::sync::atomic::Ordering::SeqCst);
    let last_state_seq = ass.states.state_seq.load(core::sync::atomic::Ordering::SeqCst);
    let mut channel = UsbPort {
        port,
    };

    loop {
        let command_seq = ass.commands.command_seq.load(core::sync::atomic::Ordering::SeqCst);
        let state_seq = ass.states.state_seq.load(core::sync::atomic::Ordering::SeqCst);
        if command_seq != last_command_seq {

            let mut driver = BusServoDriver {
                config: &config,
                channel: &mut channel,
            };

            let joint_trajectory_point = JointTrajectoryPoint {
                position: ass.commands.hw_commands[0],
                velocity: ass.commands.hw_velocities[0],
                acceleration: ass.commands.hw_accelerations[0],
            };
            driver.set_joint_trajectory(joint_trajectory_point);

            let torque_status: TorqueStatus = TorqueStatus::from(ass.commands.hw_torque_enabled[0]);
            driver.set_torque_enable(torque_status);
        }

        /*
        let mut changes: Vec<(u16, u16)> = Vec::new();
        sem.wait();
        for (iterator, status) in rh.iter() {
            if status {
                changes.push((iterator, rt.registers[iterator as usize].get()));
            }
        }
        for (register, _) in &changes {
            rh.set_clean(*register);
        }
        sem.post();

        for (register, value) in changes {
            println!("Register done on {} - {}", register, value);
            let packet = write_register(0x01, register, value);
            let _ = port.write_all(&packet);
            std::thread::sleep(Duration::from_millis(10));
        }
        {
            let mut buf = [0u8; 0xff];
            if let Ok(n) = port.read(&mut buf) {
                if n > 0 {
                    print!("  <- firmware: {}", String::from_utf8_lossy(&buf[..n]));
                }
            }
        }*/
    }
}
