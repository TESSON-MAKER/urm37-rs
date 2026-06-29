/// Addresses and helpers for the URM37's internal EEPROM registers.
///
/// The URM37 has 123 bytes of non-volatile memory (EEPROM).
/// Addresses 0x00..=0x04 are documented by DFRobot.

use crate::protocol;

/// Documented EEPROM registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum EepromRegister {
    /// High byte of the COMP distance threshold (cm).
    ThresholdHigh,
    /// Low byte of the COMP distance threshold (cm).
    ThresholdLow,
    /// Measurement mode: 0 = passive, 1 = auto.
    MeasureMode,
    /// Auto-measurement interval (unit: 25 ms, default = 1 → 25 ms).
    AutoInterval,
    /// Internal gain (reserved — do not modify).
    Gain,
    /// Raw address for undocumented registers.
    Raw(u8),
}

impl EepromRegister {
    /// Returns the numeric address of the register.
    #[inline]
    pub fn address(self) -> u8 {
        match self {
            EepromRegister::ThresholdHigh => protocol::EEPROM_THRESHOLD_HIGH,
            EepromRegister::ThresholdLow  => protocol::EEPROM_THRESHOLD_LOW,
            EepromRegister::MeasureMode   => protocol::EEPROM_MEASURE_MODE,
            EepromRegister::AutoInterval  => protocol::EEPROM_AUTO_INTERVAL,
            EepromRegister::Gain          => protocol::EEPROM_GAIN,
            EepromRegister::Raw(a)        => a,
        }
    }

    /// Returns `true` if the address falls within the documented range (0x00..=0x04).
    /// Raw addresses beyond that are allowed but not guaranteed.
    #[inline]
    pub fn is_standard(self) -> bool {
        self.address() <= 0x04
    }
}

/// Encodes a distance threshold (in cm) into two EEPROM bytes.
///
/// Returns `(high_byte, low_byte)` to be written to `ThresholdHigh` and `ThresholdLow`.
#[inline]
pub fn encode_threshold(distance_cm: u16) -> (u8, u8) {
    ((distance_cm >> 8) as u8, (distance_cm & 0xFF) as u8)
}

/// Decodes a distance threshold from two EEPROM bytes.
#[inline]
pub fn decode_threshold(high: u8, low: u8) -> u16 {
    u16::from_be_bytes([high, low])
}