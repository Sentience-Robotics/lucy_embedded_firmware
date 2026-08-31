#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use embedded_hal::pwm::SetDutyCycle;
use embedded_hal::i2c::I2c;

use rp2040_hal::{
    fugit::RateExtU32,
    clocks::init_clocks_and_plls,
    gpio::{Pins, FunctionPio0, FunctionPwm, FunctionI2C, PullUp},
    pac,
    i2c::I2C,
    pwm::{Slices, Pwm0},
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

    let sda_pin = pins.gpio4.into_function::<FunctionI2C>().into_pull_type::<PullUp>();
    let scl_pin = pins.gpio5.into_function::<FunctionI2C>().into_pull_type::<PullUp>();
    const ADDR: u8 = 0x40;
    let mut buffer = [0u8; 1];


    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let led_pin = pins.gpio18.into_function::<FunctionPio0>();
    let mut ws = Ws2812::new(
        led_pin,
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
        timer.count_down()
    );

    let pwm_slices = Slices::new(pac.PWM, &mut pac.RESETS);
    let mut pwm = pwm_slices.pwm0;
    pwm.set_div_int(100);
    pwm.set_top(25_000 - 1);
    pwm.enable();
    let mut channel = pwm.channel_a;
    channel.output_to(pins.gpio0);

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

    let mut counter = 0;

    let mut leds = [RGB8::default(); 6];
    leds[0].r = 255;
    ws.write(leds.iter().copied()).unwrap_or(());

    loop {
        if usb_dev.poll(&mut [&mut serial]) {
            let mut buf = [0u8; 64];
            let _ = serial.read(&mut buf);
        }

        let _ = serial.write(b"Hello! Le RP2040 tourne correctement.\r\n");
        channel.set_duty_cycle(1000).unwrap();
        delay.delay_ms(1000);
        channel.set_duty_cycle(2500).unwrap();
        delay.delay_ms(1000);
    }
}
