use rp2040_hal::{
    uart::{Writer, Reader, UartDevice, ValidUartPinout},
    uart::{DataBits, StopBits, UartConfig, UartPeripheral, State},
    pac,
    fugit::RateExtU32,
    fugit::MicrosDuration,
    clocks::init_clocks_and_plls,
    gpio::{bank0, Pins, FunctionPio0, FunctionUart, FunctionPwm, FunctionI2C, PullUp, PullDown},
    i2c::I2C,
    pwm::{Slices, Pwm0, Pwm4, Slice, FreeRunning},
    pio::PIOExt,
    sio::Sio,
    timer::Timer,
    watchdog::Watchdog,
    Clock
};

use embedded_hal::{
    delay::DelayNs,
    digital::OutputPin,
    pwm::SetDutyCycle,
    i2c::I2c,
};

use lucy_embedded_firmware_core::pwm::{PwmChannel};
use lucy_embedded_firmware_core::uart::UartChannel;
use lucy_embedded_firmware_core::drivers::pwm_servo::{PwmServoDriver, PwmServoConfig, PwmServoModbusAdapter};
use lucy_embedded_firmware_core::drivers::bus_servo::{BusServoDriver, BusServoConfig, BusServoModbusAdapter};
use lucy_embedded_firmware_core::modbus::{
    ModbusError,
    ModbusAdapter,
    RegisterView, RegisterTable,
    Slave,
    parse_modbus_frame, route_modbus_request
};

use crate::channel::Rp2040PwmChannel;

type Servo<'a, T> = PwmServoModbusAdapter<'a, Rp2040PwmChannel<T>>;

pub struct Robot<'a, C, D, E, F, G, H>
where
    C: SetDutyCycle,
    D: SetDutyCycle,
    E: SetDutyCycle,
    F: SetDutyCycle,
    G: SetDutyCycle,
    H: SetDutyCycle,
{
    pub servo1: Servo<'a, C>, 
    pub servo2: Servo<'a, D>, 
    pub servo3: Servo<'a, E>, 
    pub servo4: Servo<'a, F>, 
    pub servo5: Servo<'a, G>, 
    pub servo6: Servo<'a, H>,
}
