//! URM37 driver in **synchronous UART** mode via `embedded-io`.
//!
//! Works with any peripheral implementing `embedded_io::Read + Write`.
//!
//! # Example
//! ```ignore
//! let mut sensor = Urm37Uart::new(serial);
//! let dist_cm      = sensor.read_distance()?;
//! let temp_tenths  = sensor.read_temperature()?;
//! ```

use embedded_io::{Read, ReadExactError, Write};

use crate::eeprom::EepromRegister;
use crate::error::Error;
use crate::protocol::Frame;

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

    /// Measures distance in centimetres.
    ///
    /// Returns `Err(Error::InvalidReading)` when the sensor is out of range.
    pub fn read_distance(&mut self) -> Result<u16, Error<E>> {
        let cmd = Frame::distance_request();
        let resp = self.transact(cmd)?;
        resp.decode_distance().ok_or(Error::InvalidReading)
    }

    /// Measures temperature in tenths of degrees Celsius (235 = 23.5 °C).
    ///
    /// Returns `Err(Error::InvalidReading)` when the reading is invalid.
    pub fn read_temperature(&mut self) -> Result<f32, Error<E>> {
        let cmd = Frame::temperature_request();
        let resp = self.transact(cmd)?;
        resp.decode_temperature().ok_or(Error::InvalidReading)
    }

    /// Reads an internal EEPROM register.
    pub fn eeprom_read(&mut self, reg: EepromRegister) -> Result<u8, Error<E>> {
        let cmd = Frame::eeprom_read_request(reg);
        let resp = self.transact(cmd)?;
        Ok(resp.raw_data()[1])
    }

    /// Writes an internal EEPROM register.
    ///
    /// WARNING: EEPROM values persist across power cycles.
    /// Avoid writing in a tight loop — EEPROM endurance is limited.
    pub fn eeprom_write(&mut self, reg: EepromRegister, value: u8) -> Result<(), Error<E>> {
        let cmd = Frame::eeprom_write_request(reg, value);
        let _resp = self.transact(cmd)?;
        Ok(())
    }

    /// Sets the larger distance threshold (in cm).
    pub fn set_larger_dist(&mut self, distance_cm: u16) -> Result<(), Error<E>> {
        let (high, low) = crate::eeprom::encode_threshold(distance_cm);
        self.eeprom_write(EepromRegister::LargerDist, high)?;
        self.eeprom_write(EepromRegister::LessDist, low)?;
        Ok(())
    }

    /// Reads the measurement mode.
    pub fn read_measure_mode(&mut self) -> Result<u8, Error<E>> {
        self.eeprom_read(EepromRegister::MeasureMode)
    }

    /// Writes the measurement mode.
    pub fn write_measure_mode(&mut self, mode: u8) -> Result<(), Error<E>> {
        self.eeprom_write(EepromRegister::MeasureMode, mode)
    }

    fn transact(&mut self, cmd: Frame) -> Result<Frame, Error<E>> {
        let raw_cmd = cmd.raw_data();
        self.uart.write_all(raw_cmd).map_err(Error::Bus)?;

        let mut buf = [0u8; 4];
        self.uart.read_exact(&mut buf).map_err(|e| match e {
            ReadExactError::Other(e) => Error::Bus(e),
            ReadExactError::UnexpectedEof => Error::Timeout,
        })?;

        Frame::parse(buf).map_err(|(expected, got)| Error::ChecksumMismatch { expected, got })
    }
}
