//! URM37 driver in **PWM trigger** mode.
//!
//! # How it works
//!
//! 1. Pull TRIG low for ≥ 1 µs → triggers a measurement.
//! 2. The sensor drives ECHO high for a duration proportional to the distance.
//! 3. Convert: distance (cm) = pulse width (µs) / 50.
//!
//! # Design
//!
//! Measuring the ECHO pulse width accurately requires hardware support
//! (input capture, timer peripheral, etc.) that varies across HALs.
//! This driver handles the TRIG side and accepts a user-provided async closure
//! for the ECHO measurement, keeping the driver HAL-agnostic and `no_std`.
//!
//! # Example
//!
//! ```ignore
//! let mut sensor = Urm37Pwm::new(trig_pin)?;
//!
//! let distance = sensor.measure(&mut delay, || async {
//!     // Measure the ECHO pulse width using your HAL's input capture / timer.
//!     my_timer.measure_pulse_us().await
//! }).await?;
//!
//! match distance {
//!     Some(cm) => defmt::info!("Distance: {} cm", cm),
//!     None     => defmt::warn!("Out of range or invalid reading"),
//! }
//! ```

use core::future::Future;

use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;

// ── PWM protocol constants ────────────────────────────────────────────────────

/// Pulse width per centimetre in PWM mode (µs/cm).
pub const US_PER_CM: u32 = 50;

/// Minimum TRIG pulse width required to start a measurement (µs).
pub const TRIG_PULSE_US: u32 = 15;

/// Maximum valid ECHO pulse width (800 cm × 50 µs/cm = 40 000 µs).
pub const MAX_VALID_PULSE_US: u32 = 800 * US_PER_CM;

/// Sentinel pulse width indicating an out-of-range reading (per datasheet).
pub const INVALID_PULSE_US: u32 = 50_000;

// ── Driver ────────────────────────────────────────────────────────────────────

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

    /// Triggers a measurement and returns the distance in centimetres.
    ///
    /// # Arguments
    ///
    /// * `delay`        — An async delay provider (`embedded_hal_async::delay::DelayNs`).
    /// * `measure_echo` — An async closure that measures the ECHO pulse width in µs.
    ///                    Use your HAL's input capture or timer peripheral here.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(cm))` — Valid distance in centimetres (1–800 cm).
    /// * `Ok(None)`     — Out-of-range or invalid reading.
    /// * `Err(e)`       — GPIO error while driving the TRIG pin.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let cm = sensor.measure(&mut delay, || async {
    ///     timer.measure_pulse_us().await
    /// }).await?;
    /// ```
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
        self.trigger(delay).await?;
        let pulse_us = measure_echo().await;
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

// ── Conversion helpers ────────────────────────────────────────────────────────

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

// ── Tests ─────────────────────────────────────────────────────────────────────

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