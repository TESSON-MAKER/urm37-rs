//! URM37 driver in **asynchronous UART** mode via `embedded-io-async`.
//!
//! Compatible with Embassy, RTIC, and any embedded async executor.
//!
//! # Example (Embassy)
//! ```ignore
//! let mut sensor = Urm37UartAsync::new(uart);
//! let dist_cm = sensor.read_distance().await?;
//! let temp    = sensor.read_temperature().await?;
//! ```

use embedded_io::ReadExactError;
use embedded_io_async::{Read, Write};

use crate::eeprom::EepromRegister;
use crate::error::Error;
use crate::protocol::Frame;

/// URM37 asynchronous UART driver.
pub struct Urm37UartAsync<UART> {
    uart: UART,
}

impl<UART, E> Urm37UartAsync<UART>
where
    UART: Read<Error = E> + Write<Error = E>,
{
    /// Creates a new driver from an async UART peripheral.
    pub fn new(uart: UART) -> Self {
        Self { uart }
    }

    /// Releases the underlying UART peripheral.
    pub fn release(self) -> UART {
        self.uart
    }

    /// Measures distance in centimetres (async).
    ///
    /// Returns `Err(Error::InvalidReading)` when the sensor is out of range.
    pub async fn read_distance(&mut self) -> Result<u16, Error<E>> {
        let cmd = Frame::distance_request();
        let resp = self.transact(cmd).await?;
        resp.decode_distance().ok_or(Error::InvalidReading)
    }

    /// Measures temperature in tenths of degrees Celsius (async).
    ///
    /// Returns `Err(Error::InvalidReading)` when the reading is invalid.
    pub async fn read_temperature(&mut self) -> Result<f32, Error<E>> {
        let cmd = Frame::temperature_request();
        let resp = self.transact(cmd).await?;
        resp.decode_temperature().ok_or(Error::InvalidReading)
    }

    /// Reads an internal EEPROM register (async).
    pub async fn eeprom_read(&mut self, reg: EepromRegister) -> Result<u8, Error<E>> {
        let cmd = Frame::eeprom_read_request(reg);
        let resp = self.transact(cmd).await?;
        Ok(resp.raw_data()[1])
    }

    /// Writes an internal EEPROM register (async).
    ///
    /// WARNING: EEPROM values persist across power cycles.
    /// Avoid writing in a tight loop — EEPROM endurance is limited.
    pub async fn eeprom_write(&mut self, reg: EepromRegister, value: u8) -> Result<(), Error<E>> {
        let cmd = Frame::eeprom_write_request(reg, value);
        let _resp = self.transact(cmd).await?;
        Ok(())
    }

    /// Sets the larger distance threshold in cm (async).
    pub async fn set_larger_dist(&mut self, distance_cm: u16) -> Result<(), Error<E>> {
        let (high, low) = crate::eeprom::encode_threshold(distance_cm);
        self.eeprom_write(EepromRegister::LargerDist, high).await?;
        self.eeprom_write(EepromRegister::LessDist, low).await?;
        Ok(())
    }

    /// Reads the measurement mode (async).
    pub async fn read_measure_mode(&mut self) -> Result<u8, Error<E>> {
        self.eeprom_read(EepromRegister::MeasureMode).await
    }

    /// Writes the measurement mode (async).
    pub async fn write_measure_mode(&mut self, mode: u8) -> Result<(), Error<E>> {
        self.eeprom_write(EepromRegister::MeasureMode, mode).await
    }

    async fn transact(&mut self, cmd: Frame) -> Result<Frame, Error<E>> {
        let raw_cmd = cmd.raw_data();
        self.uart.write_all(raw_cmd).await.map_err(Error::Bus)?;

        let mut buf = [0u8; 4];
        self.uart.read_exact(&mut buf).await.map_err(|e| match e {
            ReadExactError::Other(e) => Error::Bus(e),
            ReadExactError::UnexpectedEof => Error::Timeout,
        })?;

        Frame::parse(buf).map_err(|(expected, got)| Error::ChecksumMismatch { expected, got })
    }
}
