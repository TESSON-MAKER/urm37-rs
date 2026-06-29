//! URM37 driver in **analog** mode (DAC_OUT pin).
//!
//! The voltage on DAC_OUT is proportional to the measured distance.
//!
//! Range: 0 V (0 cm) → Vcc (800 cm)
//! Formula: `distance_cm = (adc_raw / adc_max) × 800`
//!
//! This module provides the ADC-to-distance conversion helpers.
//! The ADC read itself is done via `embedded-hal::adc` and is left to the caller.
//!
//! # Example
//! ```ignore
//! // Read a 12-bit ADC value (0..=4095 on an STM32)
//! let raw: u16 = adc.read(&mut pin).unwrap();
//!
//! // Convert to cm (12-bit ADC, Vcc = 3.3 V)
//! let cm = urm37::analog::adc_to_distance_cm(raw, 4095);
//! ```

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum sensor range in analog mode (cm).
pub const ANALOG_MAX_RANGE_CM: u16 = 800;

// ── Conversion ────────────────────────────────────────────────────────────────

/// Converts a raw ADC reading to a distance (cm).
///
/// # Arguments
/// * `adc_raw` - Value read by the ADC (e.g. 0..=4095 for 12-bit).
/// * `adc_max` - Full-scale ADC value (e.g. 4095, 1023, 255…).
///
/// # Returns
/// Distance in centimetres (0..=800).
#[inline]
pub fn adc_to_distance_cm(adc_raw: u16, adc_max: u16) -> u16 {
    if adc_max == 0 {
        return 0;
    }
    ((adc_raw as u32 * ANALOG_MAX_RANGE_CM as u32) / adc_max as u32) as u16
}

/// Converts a distance (cm) to the expected ADC value.
/// Useful for tests or for calibrating a hardware threshold.
#[inline]
pub fn distance_cm_to_adc(distance_cm: u16, adc_max: u16) -> u16 {
    if ANALOG_MAX_RANGE_CM == 0 {
        return 0;
    }
    ((distance_cm as u32 * adc_max as u32) / ANALOG_MAX_RANGE_CM as u32) as u16
}

/// Converts a voltage (in millivolts) to a distance (cm).
/// Useful when the exact supply voltage is known.
///
/// # Arguments
/// * `voltage_mv` - Voltage read on DAC_OUT in millivolts.
/// * `vcc_mv`     - Supply voltage in millivolts (e.g. 3300 or 5000).
#[inline]
pub fn voltage_mv_to_distance_cm(voltage_mv: u32, vcc_mv: u32) -> u16 {
    if vcc_mv == 0 {
        return 0;
    }
    ((voltage_mv * ANALOG_MAX_RANGE_CM as u32) / vcc_mv) as u16
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adc_midscale_12bit() {
        // 12-bit ADC, mid-scale → ~400 cm
        let cm = adc_to_distance_cm(2048, 4095);
        assert!((cm as i32 - 400).abs() <= 2); // integer rounding tolerance
    }

    #[test]
    fn test_adc_max() {
        assert_eq!(adc_to_distance_cm(4095, 4095), 800);
    }

    #[test]
    fn test_adc_zero() {
        assert_eq!(adc_to_distance_cm(0, 4095), 0);
    }

    #[test]
    fn test_voltage_half_vcc() {
        // 1650 mV out of 3300 mV → 400 cm
        let cm = voltage_mv_to_distance_cm(1650, 3300);
        assert_eq!(cm, 400);
    }

    #[test]
    fn test_roundtrip_8bit() {
        let distance: u16 = 200;
        let raw = distance_cm_to_adc(distance, 255);
        let back = adc_to_distance_cm(raw, 255);
        // Integer rounding tolerance
        assert!((back as i32 - distance as i32).abs() <= 4);
    }
}