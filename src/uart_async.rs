//! **Asynchronous (Non-blocking)** URM37 driver.
//!
//! **Feature**: `async`
//!
//! Non-blocking async interface. Works with any `embedded_io_async::Read + Write`.
//!
//! # Timeout handling
//!
//! To add a timeout to UART read operations, implement the [`Read`] trait
//! in your wrapper using `embassy_time::with_timeout()`. See the example in `main.rs`.

use embedded_io::ReadExactError;
use embedded_io_async::{Read, Write};

use crate::error::Error;
use crate::protocol::{Frame, EepromRegister, encode_threshold};

/// **Asynchronous (Non-blocking)** URM37 UART driver.
///
/// This driver provides non-blocking async/await UART communication with the URM37 sensor.
/// All operations are asynchronous and require `embedded_io_async::Read + Write`.
/// The driver is transport-agnostic and works with any async UART implementation.
///
/// To add timeout protection, wrap your UART with a timeout-aware implementation
/// (e.g., `embassy_time::with_timeout()`).
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
    /// Create a new asynchronous UART driver.
    ///
    /// # Arguments
    /// * `uart` - A type implementing `embedded_io_async::Read + Write`
    ///
    /// # Example
    /// ```ignore
    /// use urm37::uart_async::Urm37UartAsync;
    ///
    /// let mut sensor = Urm37UartAsync::new(uart_peripheral);
    /// let distance = sensor.read_distance().await?;
    /// ```
    pub fn new(uart: UART) -> Self {
        Self { uart }
    }

    /// Release the underlying UART peripheral.
    ///
    /// Consumes the driver and returns ownership of the UART.
    ///
    /// # Returns
    /// The underlying UART peripheral.
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
    ///
    /// Sends a distance request to the sensor and asynchronously waits for the response.
    /// The sensor returns the distance to the nearest detected object.
    ///
    /// # Returns
    /// - `Ok(cm)`: Valid distance measurement in centimetres
    /// - `Err(Error::Bus)`: UART communication error
    /// - `Err(Error::Timeout)`: No response from sensor
    /// - `Err(Error::ChecksumMismatch)`: Data corruption detected
    /// - `Err(Error::InvalidReading)`: Sensor returned invalid value (out of range)
    ///
    /// # Example
    /// ```ignore
    /// use urm37::uart_async::Urm37UartAsync;
    ///
    /// match sensor.read_distance().await {
    ///     Ok(cm) => println!("Distance: {} cm", cm),
    ///     Err(e) => eprintln!("Error: {:?}", e),
    /// }
    /// ```
    pub async fn read_distance(&mut self) -> Result<u16, Error<E>> {
        let resp = self.transact(Frame::distance_request()).await?;
        resp.decode_distance().ok_or(Error::InvalidReading)
    }

    /// Read temperature in degrees Celsius (async).
    ///
    /// Sends a temperature request to the sensor and asynchronously waits for the response.
    /// Temperature is returned in tenths of degrees Celsius.
    ///
    /// # Returns
    /// - `Ok(temp)`: Valid temperature in degrees Celsius
    /// - `Err(Error::Bus)`: UART communication error
    /// - `Err(Error::Timeout)`: No response from sensor
    /// - `Err(Error::ChecksumMismatch)`: Data corruption detected
    /// - `Err(Error::InvalidReading)`: Sensor returned invalid value
    ///
    /// # Example
    /// ```ignore
    /// use urm37::uart_async::Urm37UartAsync;
    ///
    /// match sensor.read_temperature().await {
    ///     Ok(temp) => println!("Temperature: {} °C", temp),
    ///     Err(e) => eprintln!("Error: {:?}", e),
    /// }
    /// ```
    pub async fn read_temperature(&mut self) -> Result<f32, Error<E>> {
        let resp = self.transact(Frame::temperature_request()).await?;
        resp.decode_temperature().ok_or(Error::InvalidReading)
    }

    /// Read an EEPROM register value (async).
    ///
    /// Asynchronously reads the current value stored in one of the sensor's internal EEPROM registers.
    /// Use [`crate::EepromRegister`] to specify which register to read.
    ///
    /// # Arguments
    /// * `reg` - The EEPROM register to read
    ///
    /// # Returns
    /// - `Ok(value)`: The value stored in the register
    /// - `Err(Error::Bus)`: UART communication error
    /// - `Err(Error::Timeout)`: No response from sensor
    /// - `Err(Error::ChecksumMismatch)`: Data corruption detected
    ///
    /// # Example
    /// ```ignore
    /// use urm37::{uart_async::Urm37UartAsync, EepromRegister};
    ///
    /// let mode = sensor.eeprom_read(EepromRegister::MeasureMode).await?;
    /// println!("Measurement mode: 0x{:02X}", mode);
    /// ```
    pub async fn eeprom_read(&mut self, reg: EepromRegister) -> Result<u8, Error<E>> {
        let resp = self.transact(Frame::eeprom_read_request(reg)).await?;
        Ok(resp.raw_data()[1])
    }

    /// Write an EEPROM register value (async).
    ///
    /// Asynchronously writes a new value to one of the sensor's internal EEPROM registers.
    /// Changes persist across power cycles. Some registers require a sensor restart
    /// to take effect (see sensor documentation).
    ///
    /// # Arguments
    /// * `reg` - The EEPROM register to write
    /// * `value` - The value to write
    ///
    /// # Returns
    /// - `Ok(())`: Register written successfully
    /// - `Err(Error::Bus)`: UART communication error
    /// - `Err(Error::Timeout)`: No response from sensor
    /// - `Err(Error::ChecksumMismatch)`: Data corruption detected
    ///
    /// # Example
    /// ```ignore
    /// use urm37::{uart_async::Urm37UartAsync, EepromRegister, MeasureMode};
    ///
    /// sensor.eeprom_write(EepromRegister::MeasureMode, MeasureMode::Autonomous as u8).await?;
    /// ```
    pub async fn eeprom_write(&mut self, reg: EepromRegister, value: u8) -> Result<(), Error<E>> {
        self.transact(Frame::eeprom_write_request(reg, value))
            .await?;
        Ok(())
    }

    /// Set the comparator threshold for distance triggering (async).
    ///
    /// Asynchronously configures both the upper and lower distance thresholds for the COMP output pin.
    /// The COMP pin pulls low when the measured distance is within the configured range.
    /// This is a convenience function that writes two EEPROM registers.
    ///
    /// # Arguments
    /// * `distance_cm` - Threshold distance in centimetres
    ///
    /// # Returns
    /// - `Ok(())`: Thresholds written successfully
    /// - `Err(Error::Bus)`: UART communication error
    /// - `Err(Error::Timeout)`: No response from sensor
    /// - `Err(Error::ChecksumMismatch)`: Data corruption detected
    ///
    /// # Example
    /// ```ignore
    /// use urm37::uart_async::Urm37UartAsync;
    ///
    /// sensor.set_comp_threshold(100).await?; // Trigger when distance < 100 cm
    /// ```
    pub async fn set_comp_threshold(&mut self, distance_cm: u16) -> Result<(), Error<E>> {
        let (high, low) = encode_threshold(distance_cm);
        self.eeprom_write(EepromRegister::LargerDist, high).await?;
        self.eeprom_write(EepromRegister::LessDist, low).await?;
        Ok(())
    }
}
