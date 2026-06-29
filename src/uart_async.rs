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
use crate::protocol::{self, decode_distance, decode_temperature, validate_checksum};

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

    // Measurements

    /// Measures distance in centimetres (async).
    ///
    /// Returns `Err(Error::InvalidReading)` when the sensor is out of range.
    pub async fn read_distance(&mut self) -> Result<u16, Error<E>> {
        let cmd = protocol::frame_distance();
        let resp = self.transact(&cmd).await?;
        decode_distance(&resp).ok_or(Error::InvalidReading)
    }

    /// Measures temperature in tenths of degrees Celsius (async).
    ///
    /// Returns `Err(Error::InvalidReading)` when the reading is invalid.
    pub async fn read_temperature(&mut self) -> Result<i16, Error<E>> {
        let cmd = protocol::frame_temperature();
        let resp = self.transact(&cmd).await?;
        decode_temperature(&resp).ok_or(Error::InvalidReading)
    }

    // EEPROM

    /// Reads an internal EEPROM register (async).
    pub async fn eeprom_read(&mut self, reg: EepromRegister) -> Result<u8, Error<E>> {
        let cmd = protocol::frame_eeprom_read(reg.address());
        let resp = self.transact(&cmd).await?;
        Ok(resp[1])
    }

    /// Writes an internal EEPROM register (async).
    ///
    /// WARNING: EEPROM values persist across power cycles.
    /// Avoid writing in a tight loop — EEPROM endurance is limited.
    pub async fn eeprom_write(&mut self, reg: EepromRegister, value: u8) -> Result<(), Error<E>> {
        let cmd = protocol::frame_eeprom_write(reg.address(), value);
        let _resp = self.transact(&cmd).await?;
        Ok(())
    }

    // High-level configuration

    /// Sets the COMP/Switch distance threshold in cm (async).
    pub async fn set_comp_threshold(&mut self, distance_cm: u16) -> Result<(), Error<E>> {
        let (high, low) = crate::eeprom::encode_threshold(distance_cm);
        self.eeprom_write(EepromRegister::ThresholdHigh, high).await?;
        self.eeprom_write(EepromRegister::ThresholdLow, low).await?;
        Ok(())
    }

    /// Enables auto-measurement mode (async).
    /// `interval` in units of 25 ms (e.g. 40 = every second).
    pub async fn set_auto_mode(&mut self, interval: u8) -> Result<(), Error<E>> {
        self.eeprom_write(EepromRegister::MeasureMode, 0x01).await?;
        self.eeprom_write(EepromRegister::AutoInterval, interval).await?;
        Ok(())
    }

    /// Switches back to passive (on-demand) measurement mode (async).
    pub async fn set_passive_mode(&mut self) -> Result<(), Error<E>> {
        self.eeprom_write(EepromRegister::MeasureMode, 0x00).await
    }

    // Internal

    async fn transact(&mut self, cmd: &[u8; 4]) -> Result<[u8; 4], Error<E>> {
        self.uart.write_all(cmd).await.map_err(Error::Bus)?;

        let mut buf = [0u8; 4];
        self.uart.read_exact(&mut buf).await.map_err(|e| match e {
            ReadExactError::Other(e) => Error::Bus(e),
            ReadExactError::UnexpectedEof => Error::Timeout,
        })?;

        validate_checksum(&buf).map_err(|(expected, got)| Error::ChecksumMismatch { expected, got })?;
        Ok(buf)
    }
}