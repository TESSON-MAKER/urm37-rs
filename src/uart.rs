//! URM37 driver in **synchronous UART** mode via `embedded-io`.
//!
//! Works with any peripheral implementing `embedded_io::Read + Write`.
//!
//! # Example
//! ```ignore
//! let mut sensor = Urm37Uart::new(serial);
//! let dist_cm        = sensor.read_distance()?;
//! let temp_tenth_c   = sensor.read_temperature()?;
//! ```

use embedded_io::{Read, ReadExactError, Write};

use crate::eeprom::EepromRegister;
use crate::error::Error;
use crate::protocol::{self, decode_distance, decode_temperature, validate_checksum};

/// URM37 synchronous UART driver.
pub struct Urm37Uart<UART> {
    uart: UART,
}

impl<UART, E> Urm37Uart<UART>
where
    UART: Read<Error = E> + Write<Error = E>,
{
    /// Creates a new driver from a UART peripheral.
    pub fn new(uart: UART) -> Self {
        Self { uart }
    }

    /// Releases the underlying UART peripheral.
    pub fn release(self) -> UART {
        self.uart
    }

    // Measurements

    /// Measures distance in centimetres.
    ///
    /// Returns `Err(Error::InvalidReading)` when the sensor is out of range.
    pub fn read_distance(&mut self) -> Result<u16, Error<E>> {
        let cmd = protocol::frame_distance();
        let resp = self.transact(&cmd)?;
        decode_distance(&resp).ok_or(Error::InvalidReading)
    }

    /// Measures temperature in tenths of degrees Celsius (235 = 23.5 °C).
    ///
    /// Returns `Err(Error::InvalidReading)` when the reading is invalid.
    pub fn read_temperature(&mut self) -> Result<i16, Error<E>> {
        let cmd = protocol::frame_temperature();
        let resp = self.transact(&cmd)?;
        decode_temperature(&resp).ok_or(Error::InvalidReading)
    }

    // EEPROM

    /// Reads an internal EEPROM register.
    pub fn eeprom_read(&mut self, reg: EepromRegister) -> Result<u8, Error<E>> {
        let cmd = protocol::frame_eeprom_read(reg.address());
        let resp = self.transact(&cmd)?;
        // DATA0 holds the value read back
        Ok(resp[1])
    }

    /// Writes an internal EEPROM register.
    ///
    /// WARNING: EEPROM values persist across power cycles.
    /// Avoid writing in a tight loop — EEPROM endurance is limited.
    pub fn eeprom_write(&mut self, reg: EepromRegister, value: u8) -> Result<(), Error<E>> {
        let cmd = protocol::frame_eeprom_write(reg.address(), value);
        let _resp = self.transact(&cmd)?;
        Ok(())
    }

    // High-level configuration

    /// Sets the COMP/Switch distance threshold (in cm).
    pub fn set_comp_threshold(&mut self, distance_cm: u16) -> Result<(), Error<E>> {
        let (high, low) = crate::eeprom::encode_threshold(distance_cm);
        self.eeprom_write(EepromRegister::ThresholdHigh, high)?;
        self.eeprom_write(EepromRegister::ThresholdLow, low)?;
        Ok(())
    }

    /// Enables auto-measurement mode with the given interval (unit: 25 ms).
    /// `interval = 1` → every 25 ms, `interval = 40` → every second.
    pub fn set_auto_mode(&mut self, interval: u8) -> Result<(), Error<E>> {
        self.eeprom_write(EepromRegister::MeasureMode, 0x01)?;
        self.eeprom_write(EepromRegister::AutoInterval, interval)?;
        Ok(())
    }

    /// Switches back to passive (on-demand) measurement mode.
    pub fn set_passive_mode(&mut self) -> Result<(), Error<E>> {
        self.eeprom_write(EepromRegister::MeasureMode, 0x00)
    }

    // Internal

    /// Sends a command and reads the 4-byte response, validating the checksum.
    fn transact(&mut self, cmd: &[u8; 4]) -> Result<[u8; 4], Error<E>> {
        self.uart.write_all(cmd).map_err(Error::Bus)?;

        let mut buf = [0u8; 4];
        self.uart.read_exact(&mut buf).map_err(|e| match e {
            ReadExactError::Other(e) => Error::Bus(e),
            ReadExactError::UnexpectedEof => Error::Timeout,
        })?;

        validate_checksum(&buf).map_err(|(expected, got)| Error::ChecksumMismatch { expected, got })?;
        Ok(buf)
    }
}