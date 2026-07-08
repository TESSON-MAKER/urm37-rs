//! **Analog** mode for URM37 (DAC_OUT pin).
//!
//! In analog mode, the sensor outputs a voltage on DAC_OUT proportional to distance.
//!
//! # How it works
//!
//! The sensor maps distance linearly to the supply voltage:
//! - **0 V** → 0 cm
//! - **Vcc** → 800 cm (maximum range)
//!
//! So: `distance (cm) = (voltage / Vcc) × 800`
//!
//! # Application
//!
//! Use an ADC to read the voltage on DAC_OUT, then convert to distance with
//! the helper functions in this module. The ADC reading itself is left to the caller
//! because ADC implementations vary widely across HALs.
//!
//! # Example
//!
//! ```ignore
//! use urm37::analog::adc_to_distance_cm;
//!
//! // Read 12-bit ADC on STM32 with 3.3V supply
//! let raw_adc = adc.read_channel(channel).unwrap();  // 0..=4095
//! let distance = adc_to_distance_cm(raw_adc, 4095);
//!
//! println!("Distance: {} cm", distance);
//! ```
//!
//! # Conversion functions
//!
//! - [`adc_to_distance_cm`]: Convert ADC raw value to distance
//! - [`distance_cm_to_adc`]: Reverse conversion (for calibration)
//! - [`voltage_mv_to_distance_cm`]: Direct voltage-to-distance conversion

// Constants

/// Maximum sensor range in analog mode (cm).
pub const ANALOG_MAX_RANGE_CM: u16 = 800;

// Conversion

/// Converts a raw ADC reading to a distance in centimetres.
///
/// This is the most common conversion function. It performs a linear interpolation
/// from the ADC's 0–max_raw range to the sensor's 0–800 cm range.
///
/// # Formula
/// ```text
/// distance (cm) = (adc_raw / adc_max) × 800
/// ```
///
/// # Arguments
/// * `adc_raw` — ADC value from the peripheral (typically 0..=4095 for 12-bit, 0..=1023 for 10-bit)
/// * `adc_max` — Maximum ADC value for your bit resolution
///   - 12-bit: 4095
///   - 10-bit: 1023
///   - 8-bit: 255
///
/// # Returns
/// Distance in centimetres (0–800 cm). Returns 0 if `adc_max` is 0 (safety check).
///
/// # Examples
///
/// **12-bit ADC with 3.3 V supply (most common):**
/// ```ignore
/// use urm37::analog::adc_to_distance_cm;
/// let raw: u16 = adc.read_channel(channel)?;
/// let distance = adc_to_distance_cm(raw, 4095); // 12-bit max
/// ```
///
/// **10-bit ADC (RP2040, AVR):**
/// ```ignore
/// use urm37::analog::adc_to_distance_cm;
/// let raw: u16 = adc.read()?;
/// let distance = adc_to_distance_cm(raw, 1023); // 10-bit max
/// ```
///
/// **8-bit ADC (rare, but possible):**
/// ```ignore
/// use urm37::analog::adc_to_distance_cm;
/// let raw: u16 = adc.read() as u16;
/// let distance = adc_to_distance_cm(raw, 255); // 8-bit max
/// ```
#[inline]
pub fn adc_to_distance_cm(adc_raw: u16, adc_max: u16) -> u16 {
    if adc_max == 0 {
        return 0;
    }
    ((adc_raw as u32 * ANALOG_MAX_RANGE_CM as u32) / adc_max as u32) as u16
}

/// Converts a distance (cm) to the expected ADC value — reverse operation.
///
/// Useful for:
/// - Hardware threshold configuration (analog comparators)
/// - Testing ADC range
/// - Calibration routines
///
/// # Formula
/// ```text
/// adc_value = (distance / 800) × adc_max
/// ```
///
/// # Arguments
/// * `distance_cm` — Target distance (0–800 cm)
/// * `adc_max` — Maximum ADC value (4095 for 12-bit, 1023 for 10-bit, etc.)
///
/// # Returns
/// Expected ADC value for the given distance.
///
/// # Example
/// ```ignore
/// use urm37::analog::distance_cm_to_adc;
/// let threshold_adc = distance_cm_to_adc(300, 4095); // What ADC value = 300 cm?
/// ```
#[inline]
pub fn distance_cm_to_adc(distance_cm: u16, adc_max: u16) -> u16 {
    if ANALOG_MAX_RANGE_CM == 0 {
        return 0;
    }
    ((distance_cm as u32 * adc_max as u32) / ANALOG_MAX_RANGE_CM as u32) as u16
}

/// Converts a measured voltage (in millivolts) directly to distance in centimetres.
///
/// Useful when you measure the DAC voltage with an external ADC or millivoltmeter.
/// The formula assumes a linear relationship: `distance = (voltage / Vcc) × 800`.
///
/// # Formula
/// ```text
/// distance (cm) = (voltage_mv / vcc_mv) × 800
/// ```
///
/// # Arguments
/// * `voltage_mv` — Measured voltage on DAC_OUT (in millivolts)
/// * `vcc_mv` — Supply voltage (in millivolts)
///   - 3300 mV for 3.3 V systems
///   - 5000 mV for 5.0 V systems
///
/// # Returns
/// Distance in centimetres (0–800 cm). Returns 0 if `vcc_mv` is 0 (safety check).
///
/// # Example
/// ```ignore
/// use urm37::analog::voltage_mv_to_distance_cm;
/// // Measured 1650 mV on 3.3V supply
/// let distance = voltage_mv_to_distance_cm(1650, 3300); // Result: 400 cm
/// ```
///
/// # Use Cases
/// - Direct voltage measurement (e.g., using a precision ADC or multimeter)
/// - Hardware design validation
/// - Debugging output voltage calibration
#[inline]
pub fn voltage_mv_to_distance_cm(voltage_mv: u32, vcc_mv: u32) -> u16 {
    if vcc_mv == 0 {
        return 0;
    }
    ((voltage_mv * ANALOG_MAX_RANGE_CM as u32) / vcc_mv) as u16
}

// Tests

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