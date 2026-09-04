use std::io::{Read, Write};
use std::time::Duration;
use modbus_core::{Request, Response, FunctionCode};
use serialport::{SerialPortType, UsbPortInfo};

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

    let mut angle: u16 = 180;

    let packet = write_register(0x01, 0x01, angle);
    let _ = port.write_all(&packet);
    println!("Packet send: >{:x?}<", packet);
}
