//! **Synchronous (Blocking)** URM37 driver.
//!
//! **Feature**: `blocking` (default)
//!
//! Simple blocking UART interface. Works with any `embedded_io::Read + Write`.

use embedded_io::ReadExactError;
use embedded_io::{Read, Write};

use crate::error::Error;
use crate::protocol::{Frame, EepromRegister, encode_threshold};

/// **Synchronous (Blocking)** URM37 UART driver.
///
/// This driver provides blocking UART communication with the URM37 sensor.
/// All operations are synchronous (blocking) and require `embedded_io::Read + Write`.
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
    /// Create a new synchronous UART driver.
    ///
    /// # Arguments
    /// * `uart` - A type implementing `embedded_io::Read + Write`
    ///
    /// # Example
    /// ```ignore
    /// use urm37::uart::Urm37Uart;
    ///
    /// let mut sensor = Urm37Uart::new(uart_peripheral);
    /// let distance = sensor.read_distance()?;
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

    /// Read distance in centimetres (blocking).
    ///
    /// Sends a distance request to the sensor and waits for the response.
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
    /// use urm37::uart::Urm37Uart;
    ///
    /// match sensor.read_distance() {
    ///     Ok(cm) => println!("Distance: {} cm", cm),
    ///     Err(e) => eprintln!("Error: {:?}", e),
    /// }
    /// ```
    pub fn read_distance(&mut self) -> Result<u16, Error<E>> {
        let resp = self.transact(Frame::distance_request())?;
        resp.decode_distance().ok_or(Error::InvalidReading)
    }

    /// Read temperature in degrees Celsius (blocking).
    ///
    /// Sends a temperature request to the sensor and waits for the response.
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
    /// use urm37::uart::Urm37Uart;
    ///
    /// match sensor.read_temperature() {
    ///     Ok(temp) => println!("Temperature: {} °C", temp),
    ///     Err(e) => eprintln!("Error: {:?}", e),
    /// }
    /// ```
    pub fn read_temperature(&mut self) -> Result<f32, Error<E>> {
        let resp = self.transact(Frame::temperature_request())?;
        resp.decode_temperature().ok_or(Error::InvalidReading)
    }

    /// Read an EEPROM register value (blocking).
    ///
    /// Reads the current value stored in one of the sensor's internal EEPROM registers.
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
    /// use urm37::{uart::Urm37Uart, EepromRegister};
    ///
    /// let mode = sensor.eeprom_read(EepromRegister::MeasureMode)?;
    /// println!("Measurement mode: 0x{:02X}", mode);
    /// ```
    pub fn eeprom_read(&mut self, reg: EepromRegister) -> Result<u8, Error<E>> {
        let resp = self.transact(Frame::eeprom_read_request(reg))?;
        Ok(resp.raw_data()[1])
    }

    /// Write an EEPROM register value (blocking).
    ///
    /// Writes a new value to one of the sensor's internal EEPROM registers.
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
    /// use urm37::{uart::Urm37Uart, EepromRegister, MeasureMode};
    ///
    /// sensor.eeprom_write(EepromRegister::MeasureMode, MeasureMode::Autonomous as u8)?;
    /// ```
    pub fn eeprom_write(&mut self, reg: EepromRegister, value: u8) -> Result<(), Error<E>> {
        self.transact(Frame::eeprom_write_request(reg, value))?;
        Ok(())
    }

    /// Set the comparator threshold for distance triggering (blocking).
    ///
    /// Configures both the upper and lower distance thresholds for the COMP output pin.
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
    /// use urm37::uart::Urm37Uart;
    ///
    /// sensor.set_comp_threshold(100)?; // Trigger when distance < 100 cm
    /// ```
    pub fn set_comp_threshold(&mut self, distance_cm: u16) -> Result<(), Error<E>> {
        let (high, low) = encode_threshold(distance_cm);
        self.eeprom_write(EepromRegister::LargerDist, high)?;
        self.eeprom_write(EepromRegister::LessDist, low)?;
        Ok(())
    }
}
