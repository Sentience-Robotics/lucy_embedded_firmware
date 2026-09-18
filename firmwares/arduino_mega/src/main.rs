#![no_std]
#![no_main]

use panic_halt as _;

mod channel;

struct ServoTimer1 {
    tc1: arduino_hal::pac::TC1,
}

impl ServoTimer1 {
    fn new(tc1: arduino_hal::pac::TC1) -> Self {
        unsafe {
            tc1.tccr1a()
                .write(|w| w.wgm1().bits(0b10).com1a().match_clear());

            tc1.tccr1b()
                .write(|w| w.wgm1().bits(0b11).cs1().prescale_8());

            tc1.icr1().write(|w| w.bits(39999));
        }

        Self { tc1 }
    }

    fn set_angle(&mut self, angle: u16) {
        let angle = angle.min(180);
        let ticks = 2000 + (angle as u32 * 2000 / 180) as u16;

        unsafe {
            self.tc1.ocr1a().write(|w| w.bits(ticks));
        }
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let _servo_pin = pins.d11.into_output();

    let mut servo = ServoTimer1::new(dp.TC1);

    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

loop {
    // Prompt
    for byte in b"Angle (0-180): " {
        serial.write_byte(*byte);
    }

    // Read a line
    let mut angle: u16 = 0;

    loop {
        let byte = serial.read_byte();

        match byte {
            b'0'..=b'9' => {
                angle = angle
                    .saturating_mul(10)
                    .saturating_add((byte - b'0') as u16);

                // Echo character
                serial.write_byte(byte);
            }

            b'\r' | b'\n' => {
                // Enter pressed
                break;
            }

            8 | 127 => {
                // Backspace
                // For a simple implementation, ignore it.
            }

            _ => {
                // Ignore anything else
            }
        }
    }

    // Clamp to valid servo range
    angle = angle.min(180);

    servo.set_angle(angle);

    // Report what we did
    for byte in b"\r\nServo angle set!\r\n\r\n" {
        serial.write_byte(*byte);
    }
}
}