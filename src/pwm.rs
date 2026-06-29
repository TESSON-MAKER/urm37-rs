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

use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;

// ── PWM protocol constants ────────────────────────────────────────────────────

/// Pulse width representing 1 cm in PWM mode (µs per cm).
pub const US_PER_CM: u32 = 50;

/// Maximum valid pulse width per datasheet (800 cm × 50 µs/cm).
pub const MAX_VALID_PULSE_US: u32 = 800 * US_PER_CM; // 40 000 µs

/// Pulse width returned by the sensor for an invalid (out-of-range) reading.
/// Per the datasheet: 50 000 µs = out of range.
pub const INVALID_PULSE_US: u32 = 50_000;

// ── Driver Structure ──────────────────────────────────────────────────────────

/// High-level driver wrapper for the URM37 sensor using PWM/Pulse mode.
pub struct Urm37Pwm<TRIG> {
    trig_pin: TRIG,
}

impl<TRIG> Urm37Pwm<TRIG>
where
    TRIG: OutputPin,
{
    /// Creates a new `Urm37Pwm` instance.
    pub fn new(mut trig_pin: TRIG) -> Result<Self, TRIG::Error> {
        // Enforce the initial High state required by the protocol
        trig_pin.set_high()?;
        Ok(Self { trig_pin })
    }

    /// Triggers the sensor measurement by driving the TRIG pin low for 15 µs.
    pub async fn trigger<D>(&mut self, delay: &mut D) -> Result<(), TRIG::Error>
    where
        D: DelayNs,
    {
        self.trig_pin.set_low()?;
        delay.delay_us(15).await; // Safely covers the > 1 µs requirement
        self.trig_pin.set_high()?;
        Ok(())
    }

    /// Convenience method to convert an externally measured pulse width.
    pub fn calculate_distance(&self, pulse_us: u32) -> Option<u16> {
        pulse_to_distance_cm(pulse_us)
    }
}

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