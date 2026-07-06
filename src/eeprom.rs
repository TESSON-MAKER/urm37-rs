/// Addresses and helpers for the URM37's internal EEPROM registers.
///
/// The URM37 has 123 bytes of non-volatile memory (EEPROM).
/// Addresses 0x00..=0x04 are documented by DFRobot.

pub use crate::protocol::EepromRegister;

/// Encodes a distance threshold (in cm) into two EEPROM bytes.
///
/// Returns `(high_byte, low_byte)` to be written to `LargerDist` and `LessDist`.
#[inline]
pub fn encode_threshold(distance_cm: u16) -> (u8, u8) {
    ((distance_cm >> 8) as u8, (distance_cm & 0xFF) as u8)
}

/// Decodes a distance threshold from two EEPROM bytes.
#[inline]
pub fn decode_threshold(high: u8, low: u8) -> u16 {
    u16::from_be_bytes([high, low])
}
