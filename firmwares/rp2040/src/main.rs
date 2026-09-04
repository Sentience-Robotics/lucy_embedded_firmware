#![no_std]
#![no_main]

mod channel;
use channel::Rp2040PwmChannel;

use lucy_embedded_firmware_core::pwm::{PwmChannel};
use lucy_embedded_firmware_core::drivers::pwm_servo::{PwmServoDriver, PwmServoModbusAdapter};
use lucy_embedded_firmware_core::modbus::{
    ModbusError,
    ModbusAdapter,
    RegisterView, RegisterTable,
    Slave,
    parse_modbus_frame, route_modbus_request
};

use embedded_hal::{
    delay::DelayNs,
    digital::OutputPin,
    pwm::SetDutyCycle,
    i2c::I2c,
};

use rp2040_hal::{
    fugit::RateExtU32,
    fugit::MicrosDuration,
    clocks::init_clocks_and_plls,
    gpio::{Pins, FunctionPio0, FunctionPwm, FunctionI2C, PullUp},
    pac,
    i2c::I2C,
    pwm::{Slices, Pwm0, Slice, FreeRunning},
    pio::PIOExt,
    sio::Sio,
    timer::Timer,
    watchdog::Watchdog,
    Clock
};
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_pio::Ws2812;

use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

use cortex_m_rt::entry;
use panic_halt as _;

#[unsafe(link_section = ".boot2")]
#[unsafe(no_mangle)]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

#[entry]
fn main() -> ! {
    let core = cortex_m::Peripherals::take().unwrap();
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    let clocks = init_clocks_and_plls(
        12_000_000,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    ).ok().unwrap();
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().raw());
    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let pins = Pins::new(pac.IO_BANK0, pac.PADS_BANK0, sio.gpio_bank0, &mut pac.RESETS);

    /* PWM */

    let pwm_slices = Slices::new(pac.PWM, &mut pac.RESETS);
    let mut pwm = pwm_slices.pwm0;
    pwm.set_div_int(100);
    pwm.set_top(25_000 - 1);
    pwm.channel_a.output_to(pins.gpio0);
    pwm.enable();

    /* USB */

    let usb_bus = UsbBusAllocator::new(rp2040_hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut serial = SerialPort::new(&usb_bus);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::default()
            .manufacturer("Custom")
            .product("Servo2040 Serial")
            .serial_number("TEST")])
        .unwrap()
        .device_class(usbd_serial::USB_CLASS_CDC)
        .build();


    let channel = Rp2040PwmChannel {
        pwm: pwm
    };

    let mut driver = PwmServoDriver {
        channel: channel,
        min_pulse: 1250,
        max_pulse: 2500,
        min_angle: 0,
        max_angle: 180,
        default_angle: 90
    };

    let mut adapter = PwmServoModbusAdapter {
        base_register: 0x00,
        cmd_reg_off: 0,
        angle_reg_off: 1,
        driver: &mut driver
    };

    let mut rt = RegisterTable::default();
    let mut rv = RegisterView {
        table: &rt,
        base_register: 0,
        nb_register: 2
    };


    let slave = Slave {
        address: 0x01,
    };


    let mut rx_buf = [0u8; 256];
    let mut rx_len = 0;
    let mut rx_active_timer = false;
    let mut last_rx_micros: u64 = 0;

    loop {
        let now = timer.get_counter().ticks();

        if usb_dev.poll(&mut [&mut serial]) {
            let mut tmp_buf = [0u8; 64];

            while let Ok(count) = serial.read(&mut tmp_buf) {
                if count == 0 {
                    break;
                }
                if rx_len + count <= rx_buf.len() {
                    rx_buf[rx_len..rx_len + count].copy_from_slice(&tmp_buf[..count]);
                    rx_len += count;
                    last_rx_micros = now;
                    rx_active_timer = true;
                } else {
                    rx_active_timer = false;
                    rx_len = 0;
                    break;
                }
            }
        }
        if rx_active_timer && (now.saturating_sub(last_rx_micros) >= 3000) {
            rx_active_timer = false;
            if rx_len > 4 {
                let raw_request = parse_modbus_frame(&slave, &rx_buf[..rx_len]);
                match raw_request {
                    Ok(request) => {
                        serial.write(b"Request received and processed\n");
                        route_modbus_request(&rt, request);
                    }
                    Err(error) => match error {
                        ModbusError::InvalidAddress => {
                            serial.write(b"InvalidAddress\n");
                        }
                        ModbusError::InvalidFrame => {
                            serial.write(b"InvalidFrame\n");
                        }
                        ModbusError::CrcError => {
                            serial.write(b"CrcError\n");
                        }
                        ModbusError::UnknownOpcode => {
                            serial.write(b"UnknownOpcode\n");
                        }
                        _ => {
                            serial.write(b"Error\n");
                        }
                    }
                }
            } else {
                serial.write(b"Skipping");
            }
            rx_len = 0;
        }

        adapter.tick(&mut rv);
    }
}
