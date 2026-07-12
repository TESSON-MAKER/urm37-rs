//! **PWM echo measurement with two URM37 sensor modes** (`feature = "pwm"`).
//!
//! Distance measurement via echo pulse width. Behavior depends on URM37 sensor mode
//! (configured in EEPROM register `MeasureMode`):
//!
//! - **Autonomous mode (0xAA)** [`read_distance()`]: Sensor auto-measures and provides echo.
//!   Driver reads echo pulse only. Simple one-call measurement.
//!
//! - **Passive mode (0xBB)** [`read_distance_manual()`]: Sensor waits for TRIG pulse.
//!   Driver sends TRIG, then reads echo. Requires explicit triggering.
//!
//! # Design
//!
//! The driver encapsulates TRIG pin control and accepts a generic pulse reader.
//! The pulse reader (typically an input capture) measures the echo pulse width.
//!
//! To configure sensor mode, use UART drivers in [`crate::uart`] or [`crate::uart_async`]:
//! ```ignore
//! eeprom_write(&mut uart, EepromRegister::MeasureMode, 0xAA).await?;  // autonomous
//! eeprom_write(&mut uart, EepromRegister::MeasureMode, 0xBB).await?;  // passive
//! ```

use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;

use crate::error::Error;

/// Trait for automatic echo pulse measurement.
///
/// Implementers measure the width of the LOW pulse on the ECHO pin
/// and return the duration in microseconds.
///
/// **Pulse behavior:**
/// - ECHO line is normally HIGH (idle state)
/// - During measurement, ECHO goes LOW for a duration proportional to distance
/// - Duration LOW = (distance_cm * 50) microseconds
///
/// **Recommended implementation** (synchronization for async/autonomous sensors):
/// To reliably measure pulses in autonomous mode or async environments, synchronize
/// on the idle state first, then capture the measurement pulse:
/// 1. Wait for rising edge (ensures ECHO is HIGH and stable)
/// 2. Wait for falling edge (start of LOW pulse)
/// 3. Wait for rising edge (end of LOW pulse)
/// 4. Calculate: duration_us = t_rise - t_fall
///
/// This prevents misalignment when calling asynchronously.
pub trait PulseReader {
    /// Measure one complete LOW pulse width on ECHO pin.
    ///
    /// Returns `Some(duration_us)` on successful capture, `None` if timeout or error.
    /// A valid reading is typically in the range [0, 50000] microseconds.
    fn measure_pulse(&mut self) -> impl core::future::Future<Output = Option<u32>> + '_;
}

/// **Automatic PWM trigger driver** with embedded TRIG control and echo measurement.
///
/// Encapsulates the TRIG GPIO pin, an echo pulse reader, and a delay provider.
/// Handles the trigger pulse automatically; the caller simply invokes `read_distance()`.
pub struct Urm37Pwm<TRIG, READER, DELAY>
where
    TRIG: OutputPin,
    READER: PulseReader,
    DELAY: DelayNs,
{
    trig: TRIG,
    pulse_reader: READER,
    delay: DELAY,
    /// Trigger pulse duration in milliseconds (default: 10 ms).
    trigger_duration_ms: u32,
    /// Accept readings beyond this timeout as invalid.
    /// If pulse reader returns None, treated as timeout.
    max_timeout_us: u32,
}

impl<TRIG, READER, DELAY> Urm37Pwm<TRIG, READER, DELAY>
where
    TRIG: OutputPin,
    READER: PulseReader,
    DELAY: DelayNs,
{
    /// Create a new PWM driver.
    ///
    /// # Parameters
    /// - `trig`: GPIO pin connected to TRIG (output).
    /// - `pulse_reader`: Pulse reader (e.g., input capture).
    /// - `delay`: Delay provider for trigger pulse timing.
    ///
    /// # Errors
    /// Returns pin configuration errors if the TRIG pin cannot be set.
    pub fn new(mut trig: TRIG, pulse_reader: READER, delay: DELAY) -> Result<Self, TRIG::Error> {
        // TRIG starts HIGH; the sensor triggers on a falling edge
        trig.set_high()?;

        Ok(Self {
            trig,
            pulse_reader,
            delay,
            trigger_duration_ms: 10,
            max_timeout_us: 50000,
        })
    }

    /// Set the trigger pulse duration in milliseconds.
    ///
    /// The default duration is 10 milliseconds, which is suitable for most applications.
    /// Adjust this if needed for your specific sensor configuration.
    ///
    /// # Arguments
    /// * `ms` - Trigger pulse duration in milliseconds
    pub fn set_trigger_duration(&mut self, ms: u32) {
        self.trigger_duration_ms = ms;
    }

    /// Get the trigger pulse duration in milliseconds.
    ///
    /// # Returns
    /// The current trigger pulse duration in milliseconds.
    pub fn trigger_duration(&self) -> u32 {
        self.trigger_duration_ms
    }

    /// Release the TRIG pin, pulse reader, and delay provider.
    ///
    /// Consumes the driver and returns ownership of its components.
    ///
    /// # Returns
    /// A tuple containing:
    /// - `TRIG`: The GPIO output pin
    /// - `READER`: The pulse reader (e.g., input capture)
    /// - `DELAY`: The delay provider
    pub fn release(self) -> (TRIG, READER, DELAY) {
        (self.trig, self.pulse_reader, self.delay)
    }

    /// **Autonomous mode**: Read distance (sensor auto-triggers internally).
    ///
    /// Use when URM37 is configured in autonomous mode (`MeasureMode = 0xAA`).
    /// The sensor continuously measures and provides echo pulses automatically.
    /// This method simply reads the latest echo pulse.
    ///
    /// # Returns
    /// - `Ok(Some(cm))`: Valid reading (distance in centimeters)
    /// - `Ok(None)`: Echo out of range
    /// - `Err(Error::Timeout)`: No echo detected or pulse reader timeout
    /// - `Err(Error::Bus)`: GPIO pin control error
    ///
    /// # Timing
    /// Echo read timeout: configured via struct field `max_timeout_us` (default: 50000 µs)
    ///
    /// # Example
    /// ```ignore
    /// // Autonomous mode: sensor measures continuously
    /// loop {
    ///     match sensor.read_distance().await {
    ///         Ok(Some(cm)) => println!("Distance: {} cm", cm),
    ///         Ok(None) => println!("Out of range"),
    ///         Err(e) => println!("Error: {:?}", e),
    ///     }
    ///     Timer::after_millis(100).await;
    /// }
    /// ```
    pub async fn read_distance(&mut self) -> Result<Option<u16>, Error<TRIG::Error>> {
        // Autonomous mode: sensor auto-triggers, just read the echo
        self._measure_echo().await
    }

    /// **Passive mode**: Measure distance with manual TRIG pulse.
    ///
    /// Use when URM37 is configured in passive mode (`MeasureMode = 0xBB`).
    /// The sensor waits for an explicit TRIG pulse before measuring.
    /// This method sends the TRIG pulse and reads the resulting echo.
    ///
    /// # Returns
    /// Same as [`read_distance()`]
    ///
    /// # Trigger pulse
    /// - Pulse duration: configured via [`set_trigger_duration()`] (default: 10 ms)
    /// - Sequence: HIGH → LOW (delay) → HIGH
    ///
    /// # Example
    /// ```ignore
    /// // Passive mode: manual triggering
    /// loop {
    ///     match sensor.read_distance_manual().await {
    ///         Ok(Some(cm)) => println!("Distance: {} cm", cm),
    ///         Ok(None) => println!("Out of range"),
    ///         Err(e) => println!("Error: {:?}", e),
    ///     }
    ///     Timer::after_millis(100).await;
    /// }
    /// ```
    pub async fn read_distance_manual(&mut self) -> Result<Option<u16>, Error<TRIG::Error>> {
        // Passive mode: send TRIG pulse, then read echo
        self.trig.set_low().map_err(Error::Bus)?;
        self.delay.delay_ms(self.trigger_duration_ms as u32).await;
        self.trig.set_high().map_err(Error::Bus)?;

        self._measure_echo().await
    }

    /// Private helper: measure echo pulse and convert to distance.
    ///
    /// **Conversion formula:** distance_cm = (pulse_duration_µs) / 50
    /// - 50 µs of ECHO LOW = 1 cm distance
    /// - Valid range: ~50 µs (1 cm) to ~5000 µs (~100 cm)
    ///
    /// Returns:
    /// - `Ok(Some(cm))`: Valid measurement
    /// - `Ok(None)`: Measurement received but out of valid range
    /// - `Err(Timeout)`: No pulse detected or pulse reader timeout
    async fn _measure_echo(&mut self) -> Result<Option<u16>, Error<TRIG::Error>> {
        match self.pulse_reader.measure_pulse().await {
            Some(duration_us) => {
                // URM37: 50 µs of ECHO LOW = 1 cm
                if duration_us > 0 && duration_us < self.max_timeout_us {
                    Ok(Some((duration_us / 50) as u16))
                } else {
                    // Out of range but measurement succeeded
                    Ok(None)
                }
            }
            None => {
                // Timeout: pulse reader returned None (no echo or timeout)
                Err(Error::Timeout)
            }
        }
    }
}
