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
//! Use an ADC to read the voltage on DAC_OUT, average multiple readings for stability,
//! then convert to distance with the helper functions in this module.
//! ADC reading and averaging are left to the caller for maximum HAL compatibility.
//!
//! # Example
//!
//! ```ignore
//! use urm37::analog::adc_to_distance_cm;
//!
//! // Collect and average multiple ADC readings
//! let mut sum: u32 = 0;
//! for _ in 0..10 {
//!     sum += adc.blocking_read(&mut pin, SampleTime::CYCLES112) as u32;
//! }
//! let average = (sum / 10) as u16;
//!
//! // Convert to distance (12-bit ADC, 4095 max value)
//! let distance = adc_to_distance_cm(average, 4095);
//! println!("Distance: {} cm", distance);
//! ```
//!
//! # Conversion functions
//!
//! - [`adc_to_distance_cm`]: Convert averaged ADC reading to distance
//! - [`distance_cm_to_adc`]: Reverse conversion (for calibration)
//! - [`voltage_mv_to_distance_cm`]: Direct voltage-to-distance conversion

/// Maximum sensor range in analog mode (cm).
pub const ANALOG_MAX_RANGE_CM: u16 = 800;

/// Maximum ADC value for 12-bit resolution.
pub const ANALOG_MAX_ADC_VALUE: u16 = 4095;

/// Generic ADC reader trait for analog measurements.
///
/// Implement this trait for your ADC peripheral to use with `AnalogSensor`.
/// This trait abstracts the ADC reading logic, allowing the driver to work with
/// any ADC implementation.
#[cfg(feature = "analog")]
pub trait AdcReader {
    /// Error type for ADC reading operations.
    type Error;

    /// Read a single ADC sample.
    ///
    /// # Returns
    /// - `Ok(raw_value)`: The raw ADC reading (0 to `adc_max`)
    /// - `Err(Error)`: An error from the ADC peripheral
    fn read(&mut self) -> Result<u16, Self::Error>;
}

/// Analog sensor driver that reads and averages ADC values.
///
/// This driver simplifies distance measurements from the URM37 analog output.
/// It handles multiple ADC samples, averaging, and conversion to distance.
/// Generic over any type implementing `AdcReader`.
#[cfg(feature = "analog")]
pub struct AnalogSensor<R: AdcReader> {
    reader: R,
}

#[cfg(feature = "analog")]
impl<R: AdcReader> AnalogSensor<R> {
    /// Create a new analog sensor with an ADC reader.
    ///
    /// # Arguments
    /// * `reader` - A type implementing the `AdcReader` trait
    ///
    /// # Example
    /// ```ignore
    /// use urm37::analog::AnalogSensor;
    ///
    /// let mut sensor = AnalogSensor::new(adc_reader);
    /// let distance = sensor.read_distance(4095, 10)?;
    /// ```
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Read multiple ADC samples, average them, and convert to distance in centimetres.
    ///
    /// This method reads `num_samples` ADC values from the DAC_OUT pin,
    /// computes their average, and converts the result to distance using
    /// the linear relationship defined by the sensor.
    ///
    /// # Arguments
    /// * `adc_max` - Maximum ADC value (e.g., 4095 for 12-bit, 1023 for 10-bit)
    /// * `num_samples` - Number of ADC samples to read and average
    ///
    /// # Returns
    /// - `Ok(distance_cm)`: Distance in centimetres (0.0 to 800.0)
    /// - `Err(Error)`: ADC reading error
    ///
    /// # Example
    /// ```ignore
    /// use urm37::analog::AnalogSensor;
    ///
    /// match sensor.read_distance(4095, 10) {
    ///     Ok(cm) => println!("Distance: {:.1} cm", cm),
    ///     Err(e) => eprintln!("ADC error: {:?}", e),
    /// }
    /// ```
    pub fn read_distance(&mut self, adc_max: u16, num_samples: usize) -> Result<f32, R::Error> {
        let mut sum: u32 = 0;
        for _ in 0..num_samples {
            let reading = self.reader.read()?;
            sum += reading as u32;
        }
        let average = (sum / num_samples as u32) as u16;
        Ok(adc_to_distance_cm(average, adc_max))
    }
}

/// Converts a raw ADC reading to a distance in centimetres.
///
/// This performs a linear interpolation from the ADC's 0–max_raw range
/// to the sensor's 0–800 cm range.
#[inline]
pub fn adc_to_distance_cm(adc_raw: u16, adc_max: u16) -> f32 {
    if adc_max == 0 {
        return 0.0;
    }
    let ratio = ANALOG_MAX_RANGE_CM as f32 / adc_max as f32;
    adc_raw as f32 * ratio
}

/// Converts a distance (cm) to the expected ADC value — reverse operation.
///
/// Useful for hardware threshold configuration or calibration routines.
#[inline]
pub fn distance_cm_to_adc(distance_cm: u16, adc_max: u16) -> u16 {
    if ANALOG_MAX_RANGE_CM == 0 {
        return 0;
    }
    ((distance_cm as u32 * adc_max as u32) / ANALOG_MAX_RANGE_CM as u32) as u16
}

/// Converts a measured voltage (in millivolts) directly to distance in centimetres.
///
/// Useful when you measure the DAC voltage with an external ADC or multimeter.
#[inline]
pub fn voltage_mv_to_distance_cm(voltage_mv: u32, vcc_mv: u32) -> u16 {
    if vcc_mv == 0 {
        return 0;
    }
    ((voltage_mv * ANALOG_MAX_RANGE_CM as u32) / vcc_mv) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adc_midscale_12bit() {
        let cm = adc_to_distance_cm(2048, 4095);
        assert!((cm - 400.0).abs() < 2.5);
    }

    #[test]
    fn test_adc_max() {
        let cm = adc_to_distance_cm(4095, 4095);
        assert!((cm - 800.0).abs() < 0.1);
    }

    #[test]
    fn test_adc_zero() {
        let cm = adc_to_distance_cm(0, 4095);
        assert_eq!(cm, 0.0);
    }

    #[test]
    fn test_voltage_half_vcc() {
        let cm = voltage_mv_to_distance_cm(1650, 3300);
        assert_eq!(cm, 400);
    }
}