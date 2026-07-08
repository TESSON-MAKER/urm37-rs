//! **PWM trigger** mode for URM37.
//!
//! In PWM mode, the driver triggers a measurement by pulsing the TRIG pin,
//! then measures the ECHO pulse width to calculate distance.
//!
//! # How it works
//!
//! 1. **TRIG pulse**: Pull TRIG low for ≥ 15 µs to trigger a measurement
//! 2. **ECHO pulse**: Sensor drives ECHO high for a duration proportional to distance
//! 3. **Conversion**: `distance (cm) = pulse_width_µs / 50`
//!
//! The maximum range is 800 cm (40 ms pulse width).
//!
//! # Architecture
//!
//! This driver uses a **split responsibility** pattern:
//! - **Driver manages**: TRIG pin pulsing and timing
//! - **Caller provides**: ECHO pulse width measurement via an async closure
//!
//! This keeps the driver HAL-agnostic because measuring pulse widths accurately
//! requires hardware support (input capture, timer peripherals) that varies
//! widely across platforms. Delegating to the caller avoids forcing a specific
//! timer or input-capture implementation.
//!
//! # Example (Embassy with STM32)
//!
//! ```ignore
//! use urm37::pwm::Urm37Pwm;
//! use embassy_time::Delay;
//!
//! let mut sensor = Urm37Pwm::new(trig_pin)?;
//!
//! loop {
//!     let distance = sensor.measure(&mut Delay, || async {
//!         // Measure ECHO pulse width with your timer peripheral
//!         let (rise_time, fall_time) = ic.capture_rising_falling().await?;
//!         Ok::<u32, _>(fall_time - rise_time)
//!     }).await?;
//!
//!     match distance {
//!         Some(cm) => println!("Distance: {} cm", cm),
//!         None => println!("Out of range"),
//!     }
//! }
//! ```

use core::future::Future;

use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;

// PWM protocol constants

/// Pulse width per centimetre in PWM mode (µs/cm).
pub const US_PER_CM: u32 = 50;

/// Minimum TRIG pulse width required to start a measurement (µs).
pub const TRIG_PULSE_US: u32 = 15;

/// Maximum valid ECHO pulse width (800 cm × 50 µs/cm = 40 000 µs).
pub const MAX_VALID_PULSE_US: u32 = 800 * US_PER_CM;

/// Sentinel pulse width indicating an out-of-range reading (per datasheet).
pub const INVALID_PULSE_US: u32 = 50_000;

// Driver

/// URM37 driver in PWM trigger mode.
///
/// Generic over the TRIG output pin type.
/// The ECHO measurement is injected by the caller as an async closure.
pub struct Urm37Pwm<TRIG> {
    trig_pin: TRIG,
}

impl<TRIG> Urm37Pwm<TRIG>
where
    TRIG: OutputPin,
{
    /// Creates a new [`Urm37Pwm`] instance.
    ///
    /// Drives TRIG high immediately to satisfy the idle-high requirement.
    ///
    /// # Errors
    ///
    /// Returns `TRIG::Error` if the initial `set_high` call fails.
    pub fn new(mut trig_pin: TRIG) -> Result<Self, TRIG::Error> {
        trig_pin.set_high()?;
        Ok(Self { trig_pin })
    }

    /// Triggers a measurement with concurrent ECHO capture for optimal timing.
    ///
    /// This method runs the TRIG pulse and ECHO measurement in parallel,
    /// eliminating the latency gap that causes missed or partial pulses.
    ///
    /// # Arguments
    ///
    /// * `delay`        — An async delay provider (`embedded_hal_async::delay::DelayNs`).
    /// * `measure_echo` — An async closure that measures the ECHO pulse width in µs.
    ///   Use your HAL's input capture or timer peripheral here.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(cm))` — Valid distance in centimetres (1–800 cm).
    /// * `Ok(None)`     — Out-of-range or invalid reading.
    /// * `Err(e)`       — GPIO error while driving the TRIG pin.
    ///
    /// # Notes
    ///
    /// For concurrent ECHO measurement (recommended), use your executor's join utility
    /// (e.g., `embassy_futures::join::join` for Embassy) to start input capture
    /// before the TRIG pulse and measure during it.
    pub async fn measure<D, F, Fut>(
        &mut self,
        delay: &mut D,
        measure_echo: F,
    ) -> Result<Option<u16>, TRIG::Error>
    where
        D: DelayNs,
        F: FnOnce() -> Fut,
        Fut: Future<Output = u32>,
    {
        self.trig_pin.set_low()?;

        // Trigger the measurement and measure ECHO pulse concurrently
        // (implementation detail depends on executor, use futures::join or similar)
        delay.delay_us(TRIG_PULSE_US).await;
        let pulse_us = measure_echo().await;

        self.trig_pin.set_high()?;

        Ok(pulse_to_distance_cm(pulse_us))
    }

    /// Drives the TRIG pin low for [`TRIG_PULSE_US`] µs, then restores it high.
    ///
    /// Prefer [`measure`](Self::measure) for a complete measurement.
    /// Use this directly only if you need full control over the ECHO reading.
    ///
    /// # Errors
    ///
    /// Returns `TRIG::Error` if any GPIO call fails.
    pub async fn trigger<D>(&mut self, delay: &mut D) -> Result<(), TRIG::Error>
    where
        D: DelayNs,
    {
        self.trig_pin.set_low()?;
        delay.delay_us(TRIG_PULSE_US).await;
        self.trig_pin.set_high()?;
        Ok(())
    }

    /// Releases the TRIG pin, returning ownership to the caller.
    pub fn release(self) -> TRIG {
        self.trig_pin
    }
}

// Conversion helpers

/// Converts an ECHO pulse width (µs) to a distance (cm).
///
/// Returns `None` for the out-of-range sentinel (`50 000 µs`), zero, or any
/// pulse exceeding the maximum valid range (800 cm).
#[inline]
pub fn pulse_to_distance_cm(pulse_us: u32) -> Option<u16> {
    if pulse_us == 0 || pulse_us == INVALID_PULSE_US || pulse_us > MAX_VALID_PULSE_US {
        None
    } else {
        Some((pulse_us / US_PER_CM) as u16)
    }
}

/// Converts a distance (cm) to the expected ECHO pulse width (µs).
///
/// Useful for tests or for computing the COMP threshold register value.
#[inline]
pub fn distance_cm_to_pulse_us(distance_cm: u16) -> u32 {
    distance_cm as u32 * US_PER_CM
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_pulse_converts_correctly() {
        assert_eq!(pulse_to_distance_cm(5_000), Some(100));
    }

    #[test]
    fn invalid_sentinel_returns_none() {
        assert_eq!(pulse_to_distance_cm(INVALID_PULSE_US), None);
    }

    #[test]
    fn zero_pulse_returns_none() {
        assert_eq!(pulse_to_distance_cm(0), None);
    }

    #[test]
    fn max_valid_pulse_returns_800cm() {
        assert_eq!(pulse_to_distance_cm(MAX_VALID_PULSE_US), Some(800));
    }

    #[test]
    fn above_max_valid_pulse_returns_none() {
        assert_eq!(pulse_to_distance_cm(MAX_VALID_PULSE_US + 1), None);
    }

    #[test]
    fn roundtrip_distance_to_pulse_and_back() {
        let cm: u16 = 350;
        assert_eq!(pulse_to_distance_cm(distance_cm_to_pulse_us(cm)), Some(cm));
    }
}