//! **Asynchronous (Non-blocking)** URM37 driver.
//!
//! **Feature**: `async`
//!
//! Non-blocking async interface. Works with any `embedded_io_async::Read + Write`.

use embedded_io::ReadExactError;
use embedded_io_async::{Read, Write};

use crate::error::Error;
use crate::protocol::{Frame, EepromRegister, encode_threshold};

/// URM37 asynchronous UART driver.
pub struct Urm37UartAsync<UART, E>
where
    UART: Read<Error = E> + Write<Error = E>,
{
    uart: UART,
}

impl<UART, E> Urm37UartAsync<UART, E>
where
    UART: Read<Error = E> + Write<Error = E>,
{
    /// Create a new async driver.
    pub fn new(uart: UART) -> Self {
        Self { uart }
    }

    /// Release the underlying UART.
    pub fn release(self) -> UART {
        self.uart
    }

    async fn transact(&mut self, cmd: Frame) -> Result<Frame, Error<E>> {
        // Write command
        self.uart
            .write_all(cmd.raw_data())
            .await
            .map_err(Error::Bus)?;

        // Read response
        let mut buf = [0u8; 4];
        match self.uart.read_exact(&mut buf).await {
            Ok(()) => {}
            Err(ReadExactError::UnexpectedEof) => return Err(Error::Timeout),
            Err(ReadExactError::Other(e)) => return Err(Error::Bus(e)),
        }

        Frame::parse(buf).map_err(|(expected, got)| Error::ChecksumMismatch { expected, got })
    }

    /// Read distance in centimetres (async).
    pub async fn read_distance(&mut self) -> Result<u16, Error<E>> {
        let resp = self.transact(Frame::distance_request()).await?;
        resp.decode_distance().ok_or(Error::InvalidReading)
    }

    /// Read temperature in degrees Celsius (async).
    pub async fn read_temperature(&mut self) -> Result<f32, Error<E>> {
        let resp = self.transact(Frame::temperature_request()).await?;
        resp.decode_temperature().ok_or(Error::InvalidReading)
    }

    /// Read EEPROM register (async).
    pub async fn eeprom_read(&mut self, reg: EepromRegister) -> Result<u8, Error<E>> {
        let resp = self.transact(Frame::eeprom_read_request(reg)).await?;
        Ok(resp.raw_data()[1])
    }

    /// Write EEPROM register (async).
    pub async fn eeprom_write(&mut self, reg: EepromRegister, value: u8) -> Result<(), Error<E>> {
        self.transact(Frame::eeprom_write_request(reg, value))
            .await?;
        Ok(())
    }

    /// Set comparator threshold (async).
    pub async fn set_comp_threshold(&mut self, distance_cm: u16) -> Result<(), Error<E>> {
        let (high, low) = encode_threshold(distance_cm);
        self.eeprom_write(EepromRegister::LargerDist, high).await?;
        self.eeprom_write(EepromRegister::LessDist, low).await?;
        Ok(())
    }
}
