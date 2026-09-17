#![no_std]
#![no_main]

use core::fmt::Write;
mod config;
mod channel;
use config::config;

use panic_halt as _;
use cortex_m::peripheral::SCB;
use cortex_m_rt::entry;

use embedded_hal::{delay::DelayNs, digital::OutputPin, pwm::SetDutyCycle, i2c::I2c};

use lucy_embedded_firmware_core::{
    pwm::{PwmChannel},
    uart::UartChannel,
    drivers::{pwm_servo::{PwmServoDriver, PwmServoConfig, PwmServoModbusAdapter}, bus_servo::{BusServoDriver, BusServoConfig, BusServoModbusAdapter}},
    modbus::{ModbusError, ModbusAdapter, RegisterView, RegisterTable, Slave, parse_modbus_frame, route_modbus_request}
};

use rp2040_hal::{
    uart::{Writer, Reader, UartDevice, ValidUartPinout},
    uart::{DataBits, StopBits, UartConfig, UartPeripheral, State},
    pac, pac::{PIO0}, fugit::{RateExtU32, MicrosDuration},
    clocks::init_clocks_and_plls,
    gpio::{bank0, Pins, Pin, FunctionPio0, FunctionUart, FunctionPwm, FunctionI2C, PullUp, PullDown},
    i2c::I2C, pwm::{Slices, Pwm0, Pwm4, Slice, FreeRunning},
    pio::{PIOExt, SM0},
    sio::{Sio},
    timer::{Timer, CountDown},
    watchdog::Watchdog, Clock
};

use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_pio::Ws2812;
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

enum UartError {

}

struct Rp2040UartChannel<DIR>
where
    DIR: OutputPin,
{
    uart: UartPeripheral<rp2040_hal::uart::Enabled, pac::UART0, (rp2040_hal::gpio::Pin<bank0::Gpio0, FunctionUart, PullDown>, rp2040_hal::gpio::Pin<bank0::Gpio1, FunctionUart, PullDown>)>,
    dir: DIR
}

impl<DIR> UartChannel for Rp2040UartChannel<DIR>
where
    DIR: OutputPin,
{
    type Error = UartError;

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.dir.set_high();

        self.uart.write_full_blocking(bytes);
        while self.uart.uart_is_busy() {}

        self.dir.set_low();
        Ok(())
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(0)
    }
}

struct Init;
struct Idle;
struct Active;
struct Error;

struct Board<State> {
    _state: core::marker::PhantomData<State>,
    ws: Ws2812<PIO0, SM0, CountDown, Pin<bank0::Gpio18, FunctionPio0, PullDown>>,
}

impl Board<Init> {
    pub fn new(mut ws: Ws2812<PIO0, SM0, CountDown, Pin<bank0::Gpio18, FunctionPio0, PullDown>>) -> Self {
        let mut leds = [RGB8::default(); 3];
        leds[0] = RGB8 { r: 100, g: 100, b: 0 };
        ws.write(leds.iter().cloned());


        Board { _state: core::marker::PhantomData, ws: ws}
    }

    pub fn initialize(self) -> Board<Idle> {
        Board { _state: core::marker::PhantomData, ws: self.ws }
    }
}

impl Board<Idle> {
    pub fn tick(self) -> Board<Active> {
        Board { _state: core::marker::PhantomData, ws: self.ws }
    }
}

impl Board<Active> {
    pub fn process(self) -> Result<Board<Idle>, Board<Error>> {
        Ok(Board { _state: core::marker::PhantomData, ws: self.ws })
    }

}

impl Board<Error> {
    pub fn handle_error(self) -> Board<Idle> {
        Board { _state: core::marker::PhantomData, ws: self.ws }
    }

    pub fn reset(self) -> ! {
        SCB::sys_reset();
    }
}


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

    let uart_tx = pins.gpio0.into_function::<FunctionUart>();
    let uart_rx = pins.gpio1.into_function::<FunctionUart>();
    let mut dir_pin = pins.gpio2.into_push_pull_output();
    dir_pin.set_low().unwrap();

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);

    let mut ws = Ws2812::new(
        pins.gpio18.into_function(),
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
        timer.count_down(),
    );

    let board = Board::<Init>::new(ws);

    let mut rt = RegisterTable::default();

    let pwm_slices = Slices::new(pac.PWM, &mut pac.RESETS);
    let mut robot = config(&mut rt, pwm_slices,
        pins.gpio6, pins.gpio7, pins.gpio8, pins.gpio9, pins.gpio10, pins.gpio11);

    // LeRobot
    let uart = UartPeripheral::new(
        pac.UART0,
        (uart_tx, uart_rx),
        &mut pac.RESETS
    ).enable(
        UartConfig::new(
            1_000_000.Hz(),
            DataBits::Eight,
            None,
            StopBits::One,
        ),
        clocks.peripheral_clock.freq(),
    ).unwrap();

    let channel_uart = Rp2040UartChannel {
        dir: dir_pin,
        uart: uart
    };

    let mut driver_config = BusServoConfig {
        min_pulse: 0,
        max_pulse: 4096,
        min_angle: 0,
        max_angle: 360,
        default_angle: 90
    };

    let mut driver = BusServoDriver {
        config: driver_config,
        channel: channel_uart
    };

    let mut adapter7 = BusServoModbusAdapter {
        base_register: 0x00,
        cmd_reg_off: 0,
        id_reg_off: 1,
        angle_reg_off: 2,
        driver: &mut driver
    };

    let mut rv7 = RegisterView {
        table: &rt,
        base_register: 0x00,
        nb_register: 2
    };

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
            let mut read_buf = [0u8; 64];

            while let Ok(count) = serial.read(&mut read_buf) {
                if count == 0 {
                    break;
                }
                if rx_len + count <= rx_buf.len() {
                    rx_buf[rx_len..rx_len + count].copy_from_slice(&read_buf[..count]);
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
        if rx_active_timer && (now.wrapping_sub(last_rx_micros) >= 3000) {
            rx_active_timer = false;
            if rx_len >= 8 {
                let raw_request = parse_modbus_frame(&slave, &rx_buf[..rx_len]);
                match raw_request {
                    Ok(request) => {
                        let tmp = route_modbus_request(&rt, request).unwrap_or(0);
                    }
                    Err(error) => match error {
                        ModbusError::InvalidAddress => {
                        }
                        ModbusError::InvalidFrame => {
                        }
                        ModbusError::CrcError => {
                        }
                        ModbusError::UnknownOpcode => {
                        }
                        _ => {
                        }
                    }
                }
            }
            rx_len = 0;
        }
        robot.tick(&rt);

        //adapter7.tick(&mut rv7);
    }
}
