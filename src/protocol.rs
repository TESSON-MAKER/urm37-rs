//! Encoding and decoding of URM37 UART frames.
//!
//! Protocol: every command and response is exactly 4 bytes.
//! Format: [CMD, DATA0, DATA1, SUM]
//! SUM = low byte of (CMD + DATA0 + DATA1)

// ── Command bytes ─────────────────────────────────────────────────────────────

/// Request a distance measurement.
pub const CMD_DISTANCE: u8 = 0x02;
/// Request a temperature measurement.
pub const CMD_TEMPERATURE: u8 = 0x01;
/// Read an internal EEPROM register.
pub const CMD_EEPROM_READ: u8 = 0x03;
/// Write an internal EEPROM register.
pub const CMD_EEPROM_WRITE: u8 = 0x04;

// ── EEPROM register addresses ─────────────────────────────────────────────────

/// High byte of the COMP/Switch distance threshold.
pub const EEPROM_THRESHOLD_HIGH: u8 = 0x00;
/// Low byte of the COMP/Switch distance threshold.
pub const EEPROM_THRESHOLD_LOW: u8 = 0x01;
/// Measurement mode: 0x00 = passive, 0x01 = auto.
pub const EEPROM_MEASURE_MODE: u8 = 0x02;
/// Auto-measurement interval (unit: 25 ms).
pub const EEPROM_AUTO_INTERVAL: u8 = 0x03;
/// Internal gain (reserved, default 0x00).
pub const EEPROM_GAIN: u8 = 0x04;

// ── Invalid reading sentinels ─────────────────────────────────────────────────

/// Value returned by the sensor when a distance reading is invalid.
pub const INVALID_DISTANCE: u16 = 0xFFFF;
/// Value returned by the sensor when a temperature reading is invalid.
pub const INVALID_TEMPERATURE_RAW: u16 = 0x8000;

// ── Frame construction ────────────────────────────────────────────────────────

/// Builds a 4-byte command frame.
///
/// # Arguments
/// * `cmd`   - Command byte (CMD_*)
/// * `data0` - First data byte
/// * `data1` - Second data byte
///
/// # Returns
/// Array `[cmd, data0, data1, checksum]`
#[inline]
pub fn build_frame(cmd: u8, data0: u8, data1: u8) -> [u8; 4] {
    let sum = cmd.wrapping_add(data0).wrapping_add(data1);
    [cmd, data0, data1, sum]
}

/// Builds a distance request frame.
#[inline]
pub fn frame_distance() -> [u8; 4] {
    build_frame(CMD_DISTANCE, 0x00, 0x00)
}

/// Builds a temperature request frame.
#[inline]
pub fn frame_temperature() -> [u8; 4] {
    build_frame(CMD_TEMPERATURE, 0x00, 0x00)
}

/// Builds an EEPROM read request frame.
#[inline]
pub fn frame_eeprom_read(address: u8) -> [u8; 4] {
    build_frame(CMD_EEPROM_READ, address, 0x00)
}

/// Builds an EEPROM write request frame.
#[inline]
pub fn frame_eeprom_write(address: u8, value: u8) -> [u8; 4] {
    build_frame(CMD_EEPROM_WRITE, address, value)
}

// ── Response decoding ─────────────────────────────────────────────────────────

/// Validates the checksum of a received frame.
///
/// Returns `Ok(())` if the checksum is correct, `Err((expected, got))` otherwise.
#[inline]
pub fn validate_checksum(frame: &[u8; 4]) -> Result<(), (u8, u8)> {
    let expected = frame[0].wrapping_add(frame[1]).wrapping_add(frame[2]);
    if expected == frame[3] {
        Ok(())
    } else {
        Err((expected, frame[3]))
    }
}

/// Decodes a distance response from a validated frame.
///
/// DATA0:DATA1 form the distance in cm (big-endian).
/// Returns `None` if the value is the invalid-reading sentinel (0xFFFF).
#[inline]
pub fn decode_distance(frame: &[u8; 4]) -> Option<u16> {
    let raw = u16::from_be_bytes([frame[1], frame[2]]);
    if raw == INVALID_DISTANCE {
        None
    } else {
        Some(raw)
    }
}

/// Decodes a temperature response from a validated frame.
///
/// Format: bit 15 = sign (1 = negative), bits 11..0 = value in 0.1 °C.
/// Returns `None` if the value is the invalid-reading sentinel.
///
/// # Returns
/// Temperature in tenths of degrees Celsius (e.g. 235 = 23.5 °C, -15 = -1.5 °C).
#[inline]
pub fn decode_temperature(frame: &[u8; 4]) -> Option<i16> {
    let raw = u16::from_be_bytes([frame[1], frame[2]]);
    if raw == INVALID_TEMPERATURE_RAW {
        return None;
    }
    // Bit 15 indicates sign per the DFRobot datasheet.
    // Bits 11..0 hold the absolute value in 0.1 °C.
    let value = (raw & 0x0FFF) as i16;
    if raw & 0x8000 != 0 {
        Some(-value)
    } else {
        Some(value)
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_frame_checksum() {
        let f = build_frame(0x02, 0x00, 0x00);
        assert_eq!(f, [0x02, 0x00, 0x00, 0x02]);
    }

    #[test]
    fn test_validate_checksum_ok() {
        let f = [0x02u8, 0x00, 0x00, 0x02];
        assert!(validate_checksum(&f).is_ok());
    }

    #[test]
    fn test_validate_checksum_fail() {
        let f = [0x02u8, 0x00, 0x00, 0xFF];
        assert!(validate_checksum(&f).is_err());
    }

    #[test]
    fn test_decode_distance_normal() {
        // 0x0064 = 100 cm
        let f = [0x02u8, 0x00, 0x64, 0x66];
        assert_eq!(decode_distance(&f), Some(100));
    }

    #[test]
    fn test_decode_distance_invalid() {
        let f = [0x02u8, 0xFF, 0xFF, 0x00]; // intentionally wrong checksum
        assert_eq!(decode_distance(&f), None);
    }

    #[test]
    fn test_decode_temperature_positive() {
        // 0x00EB = 235 → 23.5 °C
        let f = [0x01u8, 0x00, 0xEB, 0xEC];
        assert_eq!(decode_temperature(&f), Some(235));
    }

    #[test]
    fn test_decode_temperature_negative() {
        // bit15=1, value=15 → -1.5 °C
        let raw: u16 = 0x800F;
        let f = [0x01u8, (raw >> 8) as u8, (raw & 0xFF) as u8, 0x00];
        assert_eq!(decode_temperature(&f), Some(-15));
    }

    #[test]
    fn test_eeprom_read_frame() {
        let f = frame_eeprom_read(0x02);
        assert_eq!(f[0], CMD_EEPROM_READ);
        assert_eq!(f[1], 0x02);
        assert_eq!(validate_checksum(&f), Ok(()));
    }
}