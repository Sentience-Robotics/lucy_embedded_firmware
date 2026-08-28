mod transport;
mod modbus;

use crate::transport::Transport;
use crate::modbus::{parse_modbus_frame, route_modbus_request, RegisterTable, Slave};

fn main() {
    let slave: Slave = Slave { address: 0x01 };
    let mut register_table: RegisterTable = RegisterTable { registers: [0; 0xFF] };

    let mut buffer: [u8; 256] = [0; 256];
    let mut transport = transport::NamedPipeTransport::new(String::from(r"./modbus_pipe"));
    if let Err(e) = transport.open() {
        eprintln!("Failed to open named pipe: {}", e);
        return;
    }

    loop {
        let size = match transport.read(&mut buffer) {
            Ok(size) => size,
            Err(e) => {
                eprintln!("Error reading from named pipe: {}", e);
                break;
            }
        };
        if size == 0 {
            continue;
        }
        let result = parse_modbus_frame(&buffer[..size]);
        match result {
            Ok(request) => {
                println!("Parsed request");
            },
            Err(e) => {
                print!("Error parsing request: ");
                for byte in &buffer[..size] {
                    print!("{:02X} ", byte);
                }
                println!();
            }
        };
        println!("Received request");
    }
    println!("Closing")
}
