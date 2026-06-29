//! URM37 driver in **PWM trigger** mode.
//!
//! How it works:
//! 1. Pull COMP/TRIG low → triggers a measurement.
//! 2. Read the pulse width on the ECHO pin.
//! 3. Convert: 1 cm = 50 µs of pulse width.
//!
//! This module provides the conversion primitives only. Measuring the actual
//! pulse width depends on the HAL in use (timer, input capture, etc.) and is
//! left to the caller.
//!
//! # Example
//! ```ignore
//! // Trigger the measurement
//! trig_pin.set_low();
//! // ... wait > 1 µs ...
//! trig_pin.set_high();
//!
//! // Measure the ECHO pulse width in µs with your timer
//! let pulse_us: u32 = measure_echo_width_us();
//!
//! // Convert to centimetres
//! match urm37::pwm::pulse_to_distance_cm(pulse_us) {
//!     Some(cm) => defmt::info!("Distance: {} cm", cm),
//!     None     => defmt::warn!("Invalid reading"),
//! }
//! ```

// ── PWM protocol constants ────────────────────────────────────────────────────

/// Pulse width representing 1 cm in PWM mode (µs per cm).
pub const US_PER_CM: u32 = 50;

/// Maximum valid pulse width per datasheet (800 cm × 50 µs/cm).
pub const MAX_VALID_PULSE_US: u32 = 800 * US_PER_CM; // 40 000 µs

/// Pulse width returned by the sensor for an invalid (out-of-range) reading.
/// Per the datasheet: 50 000 µs = out of range.
pub const INVALID_PULSE_US: u32 = 50_000;

// ── Conversion ────────────────────────────────────────────────────────────────

/// Converts a PWM pulse width (in µs) to a distance (in cm).
///
/// # Arguments
/// * `pulse_us` - Width of the ECHO pulse measured in microseconds.
///
/// # Returns
/// * `Some(cm)` — valid distance in centimetres.
/// * `None`     — invalid reading (out of range or 50 000 µs sentinel).
#[inline]
pub fn pulse_to_distance_cm(pulse_us: u32) -> Option<u16> {
    if pulse_us == INVALID_PULSE_US || pulse_us > MAX_VALID_PULSE_US || pulse_us == 0 {
        None
    } else {
        Some((pulse_us / US_PER_CM) as u16)
    }
}

/// Converts a distance in cm to the expected pulse width in µs.
/// Useful for tests or for configuring the COMP threshold.
#[inline]
pub fn distance_cm_to_pulse_us(distance_cm: u16) -> u32 {
    distance_cm as u32 * US_PER_CM
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_pulse() {
        assert_eq!(pulse_to_distance_cm(5000), Some(100)); // 100 cm
    }

    #[test]
    fn test_invalid_pulse_marker() {
        assert_eq!(pulse_to_distance_cm(50_000), None);
    }

    #[test]
    fn test_zero_pulse() {
        assert_eq!(pulse_to_distance_cm(0), None);
    }

    #[test]
    fn test_max_valid_pulse() {
        assert_eq!(pulse_to_distance_cm(MAX_VALID_PULSE_US), Some(800));
    }

    #[test]
    fn test_roundtrip() {
        let original_cm: u16 = 350;
        let pulse = distance_cm_to_pulse_us(original_cm);
        assert_eq!(pulse_to_distance_cm(pulse), Some(original_cm));
    }
}