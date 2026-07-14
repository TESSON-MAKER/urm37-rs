//! # URM37 UART Example for Arduino Mega 2560
//!
//! Demonstrates UART communication for distance measurement.
//!
//! ## Hardware Setup
//! - **USART0** (D0/D1): Communication with computer (57600 baud)
//! - **USART1** (D18/D19): URM37 sensor (9600 baud)
//! - **Arduino Mega 2560**
//!
//! ## Output Format
//! ```text
//! [DISTANCE] X cm
//! [ERROR]
//! ```
//!
//! ## Build & Run
//! ```bash
//! cargo build --example mega2560_uart --features blocking
//! ```

#![no_std]
#![no_main]

use panic_halt as _;
use urm37::uart::Urm37Uart;
use ufmt::uwriteln;
use embedded_io::{Read, Write, ReadExactError, ErrorType};
use embedded_hal_0_2::serial as hal_serial;

/// UART adapter implementing embedded_io traits for arduino_hal USART
struct UsartAdapter<T> {
    serial: T,
}

#[derive(Debug, Copy, Clone)]
struct UsartError;

impl core::fmt::Display for UsartError {
    fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }
}

impl core::error::Error for UsartError {}

impl embedded_io::Error for UsartError {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

impl<T> ErrorType for UsartAdapter<T> {
    type Error = UsartError;
}

impl<T: hal_serial::Read<u8>> Read for UsartAdapter<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        const MAX_ATTEMPTS: u32 = 50000;
        let mut attempts = 0u32;

        loop {
            match self.serial.read() {
                Ok(byte) => {
                    buf[0] = byte;
                    return Ok(1);
                }
                Err(nb::Error::WouldBlock) => {
                    attempts += 1;
                    if attempts >= MAX_ATTEMPTS {
                        return Err(UsartError);
                    }
                }
                Err(_) => return Err(UsartError),
            }
        }
    }
}

impl<T: hal_serial::Write<u8>> Write for UsartAdapter<T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for &byte in buf {
            match nb::block!(self.serial.write(byte)) {
                Ok(()) => {}
                Err(_) => return Err(UsartError),
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[allow(dead_code)]
trait ReadExact: Read {
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), embedded_io::ReadExactError<Self::Error>> {
        let mut n = 0;
        while n < buf.len() {
            match self.read(&mut buf[n..])? {
                0 => return Err(ReadExactError::UnexpectedEof),
                m => n += m,
            }
        }
        Ok(())
    }
}

impl<T: Read> ReadExact for T {}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Serial for output to computer (USART0)
    let mut output = arduino_hal::Usart::new(
        dp.USART0,
        pins.d0,
        pins.d1.into_output(),
        arduino_hal::hal::usart::BaudrateArduinoExt::into_baudrate(57600),
    );

    // Serial for URM37 sensor (USART1)
    let sensor_serial = arduino_hal::Usart::new(
        dp.USART1,
        pins.d19,
        pins.d18.into_output(),
        arduino_hal::hal::usart::BaudrateArduinoExt::into_baudrate(9600),
    );

    let adapter = UsartAdapter { serial: sensor_serial };
    let mut sensor = Urm37Uart::new(adapter);

    loop {
        match sensor.read_distance() {
            Ok(distance) => {
                uwriteln!(&mut output, "[DISTANCE] {} cm", distance).ok();
            }
            Err(_) => {
                uwriteln!(&mut output, "[ERROR]").ok();
            }
        }
        arduino_hal::delay_ms(500);
    }
}