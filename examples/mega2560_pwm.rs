//! # URM37 PWM Example for Arduino Mega 2560
//!
//! Demonstrates PWM mode with manual pulse width triggering.
//!
//! ## Hardware Setup
//! - **D9**: TRIG output (PWM trigger)
//! - **D2**: ECHO input (pulse measurement)
//! - **Arduino Mega 2560**
//!
//! ## Output Format
//! ```text
//! [DISTANCE] X cm
//! [OUT_OF_RANGE]
//! [ERROR]
//! ```
//!
//! ## Build & Run
//! ```bash
//! cargo build --example mega2560_pwm --features pwm
//! ```

#![no_std]
#![no_main]

use panic_halt as _;
use urm37::pwm::{Urm37Pwm, PulseReader};
use arduino_hal::port::mode::{Input, Floating};

type EchoPin = arduino_hal::port::Pin<Input<Floating>, arduino_hal::port::D2>;

/// Simple pulse reader for measuring echo duration on D2
struct SimplePulseReader {
    echo_pin: EchoPin,
}

impl SimplePulseReader {
    fn new(echo_pin: EchoPin) -> Self {
        Self { echo_pin }
    }
}

impl PulseReader for SimplePulseReader {
    fn measure_pulse(&mut self) -> Option<u32> {
        let mut timeout = 0u32;
        const MAX_TIMEOUT: u32 = 500_000;

        // Wait for echo pulse to start (falling edge)
        while self.echo_pin.is_high() && timeout < MAX_TIMEOUT {
            timeout += 1;
        }

        if timeout >= MAX_TIMEOUT {
            return None;
        }

        timeout = 0;

        // Measure pulse duration (LOW pulse)
        while !self.echo_pin.is_high() && timeout < MAX_TIMEOUT {
            timeout += 1;
        }

        Some(timeout)
    }
}

struct SimpleDelay;

impl embedded_hal::delay::DelayNs for SimpleDelay {
    fn delay_ns(&mut self, _ns: u32) {
        // No-op for this simple implementation
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    // TRIG pin (output on pin 9)
    let trig_pin = pins.d9.into_output();

    // ECHO pin (input on pin 2)
    let echo_pin = pins.d2.into_floating_input();

    let pulse_reader = SimplePulseReader::new(echo_pin);
    let delay = SimpleDelay;

    // Create the URM37 PWM sensor using the REAL crate (synchronous)
    let mut sensor = match Urm37Pwm::new(trig_pin, pulse_reader, delay) {
        Ok(s) => s,
        Err(_) => {
            ufmt::uwriteln!(&mut serial, "Failed to initialize sensor\r").unwrap();
            loop {}
        }
    };

    loop {
        match sensor.read_distance_manual() {
            Ok(Some(cm)) => {
                ufmt::uwriteln!(&mut serial, "[DISTANCE] {} cm", cm).unwrap();
            }
            Ok(None) => {
                ufmt::uwriteln!(&mut serial, "[OUT_OF_RANGE]").unwrap();
            }
            Err(_) => {
                ufmt::uwriteln!(&mut serial, "[ERROR]").unwrap();
            }
        }

        arduino_hal::delay_ms(500);
    }
}
