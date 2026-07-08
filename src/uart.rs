//! **Synchronous (Blocking)** URM37 driver.
//!
//! **Feature**: `blocking` (default)
//!
//! Simple blocking UART interface. Works with any `embedded_io::Read + Write`.

use embedded_io::ReadExactError;
use embedded_io::{Read, Write};

use crate::error::Error;
use crate::protocol::{Frame, EepromRegister, encode_threshold};

/// URM37 synchronous UART driver.
pub struct Urm37Uart<UART, E>
where
    UART: Read<Error = E> + Write<Error = E>,
{
    uart: UART,
}

impl<UART, E> Urm37Uart<UART, E>
where
    UART: Read<Error = E> + Write<Error = E>,
{
    /// Create a new driver.
    pub fn new(uart: UART) -> Self {
        Self { uart }
    }

    /// Release the underlying UART.
    pub fn release(self) -> UART {
        self.uart
    }

    fn transact(&mut self, cmd: Frame) -> Result<Frame, Error<E>> {
        // Write command
        self.uart
            .write_all(cmd.raw_data())
            .map_err(Error::Bus)?;

        // Read response
        let mut buf = [0u8; 4];
        match self.uart.read_exact(&mut buf) {
            Ok(()) => {}
            Err(ReadExactError::UnexpectedEof) => return Err(Error::Timeout),
            Err(ReadExactError::Other(e)) => return Err(Error::Bus(e)),
        }

        Frame::parse(buf).map_err(|(expected, got)| Error::ChecksumMismatch { expected, got })
    }

    /// Read distance in centimetres.
    pub fn read_distance(&mut self) -> Result<u16, Error<E>> {
        let resp = self.transact(Frame::distance_request())?;
        resp.decode_distance().ok_or(Error::InvalidReading)
    }

    /// Read temperature in degrees Celsius.
    pub fn read_temperature(&mut self) -> Result<f32, Error<E>> {
        let resp = self.transact(Frame::temperature_request())?;
        resp.decode_temperature().ok_or(Error::InvalidReading)
    }

    /// Read EEPROM register.
    pub fn eeprom_read(&mut self, reg: EepromRegister) -> Result<u8, Error<E>> {
        let resp = self.transact(Frame::eeprom_read_request(reg))?;
        Ok(resp.raw_data()[1])
    }

    /// Write EEPROM register.
    pub fn eeprom_write(&mut self, reg: EepromRegister, value: u8) -> Result<(), Error<E>> {
        self.transact(Frame::eeprom_write_request(reg, value))?;
        Ok(())
    }

    /// Set comparator threshold.
    pub fn set_comp_threshold(&mut self, distance_cm: u16) -> Result<(), Error<E>> {
        let (high, low) = encode_threshold(distance_cm);
        self.eeprom_write(EepromRegister::LargerDist, high)?;
        self.eeprom_write(EepromRegister::LessDist, low)?;
        Ok(())
    }
}
