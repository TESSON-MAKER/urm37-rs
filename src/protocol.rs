//! Encoding and decoding of URM37 UART frames using strongly-typed abstractions.
//!
//! Protocol: every command and response is exactly 4 bytes.
//! Format: [CMD, DATA0, DATA1, SUM]
//! SUM = low byte of (CMD + DATA0 + DATA1)

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

/// EEPROM register addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EepromRegister {
    /// Larger than set distance.
    LargerDist = 0x00,
    /// Less than set distance.
    LessDist = 0x01,
    /// Measurement mode: 0x00 = passive, 0x01 = auto.
    MeasureMode = 0x02,
    /// Serial level mode TTL/RS232: 0x00 = TTL, 0x01 = RS232.
    SerialLevelMode = 0x03,
    /// Automatically measure time span: 0x64 = 100ms.
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
    #[inline]
    pub fn distance_request() -> Self {
        Self::build(Command::Distance, 0x00, 0x00)
    }

    /// Builds a temperature request frame.
    #[inline]
    pub fn temperature_request() -> Self {
        Self::build(Command::Temperature, 0x00, 0x00)
    }

    /// Builds an EEPROM read request frame.
    #[inline]
    pub fn eeprom_read_request(register: EepromRegister) -> Self {
        Self::build(Command::EepromRead, register as u8, 0x00)
    }

    /// Builds an EEPROM write request frame.
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
