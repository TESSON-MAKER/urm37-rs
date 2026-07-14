//! # URM37 Analog Example for Arduino Mega 2560
//!
//! Demonstrates analog distance measurement via ADC without UART.
//!
//! ## Hardware Setup
//! - **A0**: URM37 analog voltage output
//! - **Arduino Mega 2560**
//!
//! ## Output Format
//! ```text
//! [DISTANCE] X.X cm
//! ```
//!
//! ## Build & Run
//! ```bash
//! cargo build --example mega2560_analog --features analog
//! ```

#![no_std]
#![no_main]

use panic_halt as _;
use urm37::analog::{AnalogSensor, AdcReader};

/// Simple ADC reader wrapper for Arduino
struct SimpleAdcReader {
    adc: arduino_hal::Adc,
    pin: arduino_hal::port::Pin<arduino_hal::port::mode::Analog, arduino_hal::port::A0>,
}

impl AdcReader for SimpleAdcReader {
    type Error = ();

    fn read(&mut self) -> Result<u16, Self::Error> {
        Ok(self.adc.read_blocking(&mut self.pin) as u16)
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
    let a0 = pins.a0.into_analog_input(&mut adc);
    let mut sensor = AnalogSensor::new(SimpleAdcReader { adc, pin: a0 });

    loop {
        if let Ok(distance_cm) = sensor.read_distance(1023, 10) {
            let cm = distance_cm as u16;
            let dec = ((distance_cm * 10.0) as u16) % 10;
            ufmt::uwriteln!(&mut serial, "[DISTANCE] {}.{} cm", cm, dec).unwrap();
        }
        arduino_hal::delay_ms(500);
    }
}
