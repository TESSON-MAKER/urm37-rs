//! Encoding and decoding of URM37 UART frames using strongly-typed abstractions.
//!
//! # Protocol Overview
//!
//! Every URM37 command and response is exactly **4 bytes**:
//! ```text
//! [CMD, DATA0, DATA1, CHECKSUM]
//! ```
//!
//! where `CHECKSUM = low byte of (CMD + DATA0 + DATA1)`.
//!
//! This module provides:
//! - **Frame building**: construct commands with correct checksums
//! - **Frame parsing**: validate checksums and deserialize responses
//! - **Data decoding**: extract distance and temperature from responses
//! - **EEPROM helpers**: encode/decode threshold values
//!
//! # Example
//!
//! ```ignore
//! use urm37::protocol::{Frame, Command};
//!
//! // Build a distance request
//! let frame = Frame::distance_request();
//! assert_eq!(frame.raw_data(), &[0x22, 0x00, 0x00, 0x22]);
//!
//! // Parse a distance response (300 cm = 0x012C)
//! let response = [0x22, 0x01, 0x2C, 0x4F];
//! let frame = Frame::parse(response).expect("valid frame");
//! let distance = frame.decode_distance().expect("valid reading");
//! assert_eq!(distance, 300);
//! ```

/// Value returned by the sensor when a distance/temperature reading is invalid.
pub const INVALID_DIST_TEMP: u16 = 0xFFFF;

/// Command bytes represented as a structured enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    /// Request a temperature measurement.
    Temperature = 0x11,
    /// Request a distance measurement.
    Distance = 0x22,
    /// Read an internal EEPROM register.
    EepromRead = 0x33,
    /// Write an internal EEPROM register.
    EepromWrite = 0x44,
}

/// EEPROM register addresses and their purposes.
///
/// The URM37 stores configuration in non-volatile EEPROM at these addresses.
/// Values persist across power cycles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EepromRegister {
    /// **Address 0x00**: Larger distance threshold (comparator mode).
    /// COMP pin pulls low when measured distance **≥** this value.
    /// Range: 0–255 cm (use with [`encode_threshold`] / [`decode_threshold`] for 16-bit values).
    LargerDist = 0x00,

    /// **Address 0x01**: Smaller distance threshold (comparator mode).
    /// COMP pin pulls low when measured distance **≤** this value.
    /// Range: 0–255 cm.
    LessDist = 0x01,

    /// **Address 0x02**: Measurement mode.
    /// - `0xAA` = Autonomous (sensor continuously measures at set interval)
    /// - `0xBB` = Passive (sensor measures only when requested via UART)
    MeasureMode = 0x02,

    /// **Address 0x03**: Serial interface level mode.
    /// - `0x00` = TTL (3.3 V logic, standard for microcontrollers)
    /// - `0x01` = RS232 (12 V levels, rarely used in embedded systems)
    /// **Warning:** Changing this requires sensor power cycle to take effect.
    SerialLevelMode = 0x03,

    /// **Address 0x04**: Autonomous measurement interval.
    /// Each unit = 25 ms.
    /// - `0x01` = 25 ms
    /// - `0x04` = 100 ms (common)
    /// - `0x14` = 500 ms
    /// - Max: `0xFF` = 6375 ms (~6.4 seconds)
    AutoMeasureTime = 0x04,
}

/// Values possible for the measurement mode EEPROM register.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MeasureMode {
    /// Automatic measurement mode (sensor measures distance automatically at intervals).
    Auto = 0xAA,
    /// Passive measurement mode (sensor measures distance only when requested).
    Passive = 0xBB,
}

/// Values possible for the serial level mode EEPROM register.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SerialLevelMode {
    /// TTL serial level mode (3.3V logic).
    Ttl = 0x00,
    /// RS232 serial level mode (12V logic).
    Rs232 = 0x01,
}

// ===== EEPROM helpers =====

/// Encodes a distance threshold (in cm) into two EEPROM bytes.
///
/// The URM37 stores distance thresholds as 16-bit big-endian values.
/// This function splits a `u16` distance into high and low bytes suitable
/// for writing to the sensor's EEPROM registers.
///
/// # Arguments
/// * `distance_cm` - Distance in centimetres (0..=65535)
///
/// # Returns
/// A tuple `(high_byte, low_byte)` to be written to consecutive EEPROM locations.
///
/// # Example
/// ```
/// # use urm37::protocol::encode_threshold;
/// let (high, low) = encode_threshold(500);
/// assert_eq!(high, 0x01);
/// assert_eq!(low, 0xF4);
/// ```
#[inline]
pub fn encode_threshold(distance_cm: u16) -> (u8, u8) {
    ((distance_cm >> 8) as u8, (distance_cm & 0xFF) as u8)
}

/// Decodes a distance threshold from two EEPROM bytes.
///
/// Reconstructs a 16-bit big-endian distance value from two EEPROM bytes.
///
/// # Arguments
/// * `high` - High byte (most significant)
/// * `low`  - Low byte (least significant)
///
/// # Returns
/// The decoded distance in centimetres (0..=65535).
///
/// # Example
/// ```
/// # use urm37::protocol::decode_threshold;
/// let distance = decode_threshold(0x01, 0xF4);
/// assert_eq!(distance, 500);
/// ```
#[inline]
pub fn decode_threshold(high: u8, low: u8) -> u16 {
    u16::from_be_bytes([high, low])
}

/// A validated 4-byte URM37 frame.
///
/// Encapsulates the raw byte array to prevent direct index manipulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame([u8; 4]);

impl Frame {
    // Indexes of the frame bytes sent and received are kept as internal constants
    const INDEX_CMD: usize = 0;
    const INDEX_DATA0: usize = 1;
    const INDEX_DATA1: usize = 2;
    const INDEX_CHECKSUM: usize = 3;

    /// Calculates the checksum for a given command and data bytes.
    ///
    /// # Arguments
    /// * `cmd`   - Command byte
    /// * `data0` - First data byte
    /// * `data1` - Second data byte
    /// # Returns
    /// The low byte of the sum of the three input bytes.
    #[inline]
    pub fn checksum(cmd: u8, data0: u8, data1: u8) -> u8 {
        cmd.wrapping_add(data0).wrapping_add(data1)
    }

    /// Builds a 4-byte command frame and automatically computes the checksum.
    ///
    /// # Arguments
    /// * `cmd`   - Command variant (`Command`)
    /// * `data0` - First data byte
    /// * `data1` - Second data byte
    ///
    /// # Returns
    /// A new validated `Frame` instance.
    #[inline]
    pub fn build(cmd: Command, data0: u8, data1: u8) -> Self {
        let cmd_byte = cmd as u8;
        let sum = Self::checksum(cmd_byte, data0, data1);
        Self([cmd_byte, data0, data1, sum])
    }

    /// Builds a distance request frame.
    ///
    /// Constructs a command to request the current distance measurement from the sensor.
    ///
    /// # Returns
    /// A frame ready to be sent to the sensor via UART.
    #[inline]
    pub fn distance_request() -> Self {
        Self::build(Command::Distance, 0x00, 0x00)
    }

    /// Builds a temperature request frame.
    ///
    /// Constructs a command to request the current temperature reading from the sensor.
    ///
    /// # Returns
    /// A frame ready to be sent to the sensor via UART.
    #[inline]
    pub fn temperature_request() -> Self {
        Self::build(Command::Temperature, 0x00, 0x00)
    }

    /// Builds an EEPROM read request frame.
    ///
    /// Constructs a command to read a value from one of the sensor's EEPROM registers.
    ///
    /// # Arguments
    /// * `register` - The EEPROM register to read
    ///
    /// # Returns
    /// A frame ready to be sent to the sensor via UART.
    #[inline]
    pub fn eeprom_read_request(register: EepromRegister) -> Self {
        Self::build(Command::EepromRead, register as u8, 0x00)
    }

    /// Builds an EEPROM write request frame.
    ///
    /// Constructs a command to write a value to one of the sensor's EEPROM registers.
    ///
    /// # Arguments
    /// * `register` - The EEPROM register to write
    /// * `value` - The value to write to the register
    ///
    /// # Returns
    /// A frame ready to be sent to the sensor via UART.
    #[inline]
    pub fn eeprom_write_request(register: EepromRegister, value: u8) -> Self {
        Self::build(Command::EepromWrite, register as u8, value)
    }

    /// Validates the checksum of a received raw 4-byte buffer.
    ///
    /// Returns `Ok(Frame)` if the checksum is correct, `Err((expected, got))` otherwise.
    #[inline]
    pub fn parse(raw: [u8; 4]) -> Result<Self, (u8, u8)> {
        let expected = Self::checksum(raw[Self::INDEX_CMD], raw[Self::INDEX_DATA0], raw[Self::INDEX_DATA1]);
        let got = raw[Self::INDEX_CHECKSUM];
        
        if expected == got {
            Ok(Self(raw))
        } else {
            Err((expected, got))
        }
    }

    /// Decodes a distance response from a validated frame.
    ///
    /// DATA0:DATA1 form the distance in cm (big-endian).
    /// Returns `None` if the value is the invalid-reading sentinel (0xFFFF).
    #[inline]
    pub fn decode_distance(&self) -> Option<u16> {
        let raw = u16::from_be_bytes([self.0[Self::INDEX_DATA0], self.0[Self::INDEX_DATA1]]);
        if raw == INVALID_DIST_TEMP {
            None
        } else {
            Some(raw)
        }
    }

    /// Decodes a temperature response from a validated frame.
    ///
    /// Format: bit 15..12 = sign (0xF0 = negative), bits 11..0 = value in 0.1 °C.
    /// Returns `None` if the value is the invalid-reading sentinel.
    ///
    /// # Returns
    /// Temperature in tenths of degrees Celsius.
    #[inline]
    pub fn decode_temperature(&self) -> Option<f32> {
        let is_negative = (self.0[Self::INDEX_DATA0] & 0xF0) != 0;

        let raw = u16::from_be_bytes([self.0[Self::INDEX_DATA0] & 0x0F, self.0[Self::INDEX_DATA1]]);
        if raw == INVALID_DIST_TEMP {
            None
        } else {
            let value = raw as i16;
            if is_negative {
                Some(-value as f32 / 10.0)
            } else {
                Some(value as f32 / 10.0)
            }
        }
    }

    /// Returns the raw 4-byte frame data.
    #[inline]
    pub fn raw_data(&self) -> &[u8; 4] {
        &self.0
    }
}
