//! # urm37
//!
//! `no_std` embedded driver for the **DFRobot URM37 V5.0** ultrasonic distance sensor
//! (SKU: SEN0001).
//!
//! ## Supported modes
//!
//! | Mode | Feature | HAL interface |
//! |------|---------|---------------|
//! | Synchronous UART | `uart` | `embedded-io` |
//! | Asynchronous UART | `uart-async` | `embedded-io-async` |
//! | PWM trigger | `pwm` | GPIO + timer (caller-managed) |
//! | Analog | `analog` | ADC (caller-managed) |
//!
//! ## Quick start (async UART with Embassy)
//!
//! ```toml
//! [dependencies]
//! urm37 = { version = "0.1", features = ["uart-async"] }
//! ```
//!
//! ```ignore
//! use urm37::uart_async::Urm37UartAsync;
//!
//! let mut sensor = Urm37UartAsync::new(uart);
//! let dist = sensor.read_distance().await?;    // cm
//! let temp = sensor.read_temperature().await?; // tenths of °C
//! ```
//!
//! ## PWM mode (Embassy, STM32)
//!
//! The driver manages the TRIG pin. Measuring the ECHO pulse width is the
//! caller's responsibility and depends on the HAL and timer available.
//!
//! The recommended approach on STM32 with Embassy uses two input-capture
//! channels on the same timer with opposite polarities, joined concurrently:
//!
//! ```ignore
//! use embassy_futures::join::join;
//! use embassy_stm32::timer::input_capture::{CapturePin, InputCapture, InputCapturePolarity};
//! use embassy_stm32::timer::low_level::CountingMode;
//! use embassy_stm32::timer::Channel;
//! use embassy_stm32::time::hz;
//! use embassy_time::{Delay, Timer};
//! use urm37::pwm::Urm37Pwm;
//!
//! // TRIG → PA0 (output), ECHO → PA5 (TIM2_CH1 AF1) + PA1 (TIM2_CH2 AF1)
//! let trig = Output::new(p.PA0, Level::High, Speed::Low);
//! let mut sensor = Urm37Pwm::new(trig).unwrap();
//!
//! let mut ic = InputCapture::new(
//!     p.TIM2,
//!     Some(CapturePin::new_ch1(p.PA5)), // front montant
//!     Some(CapturePin::new_ch2(p.PA1)), // front descendant
//!     None,
//!     None,
//!     hz(1_000_000), // 1 tick = 1 µs
//!     CountingMode::EdgeAlignedUp,
//! );
//!
//! ic.set_input_capture_polarity(Channel::Ch1, InputCapturePolarity::Rising);
//! ic.set_input_capture_polarity(Channel::Ch2, InputCapturePolarity::Falling);
//!
//! loop {
//!     sensor.trigger(&mut Delay).await.unwrap();
//!
//!     let (t_rise, t_fall) = join(
//!         ic.capture(Channel::Ch1),
//!         ic.capture(Channel::Ch2),
//!     ).await;
//!
//!     let pulse_us = t_fall.wrapping_sub(t_rise);
//!
//!     match sensor.calculate_distance(pulse_us) {
//!         Some(cm) => defmt::info!("Distance: {} cm", cm),
//!         None     => defmt::warn!("Hors plage ou lecture invalide"),
//!     }
//!
//!     Timer::after_millis(100).await;
//! }
//! ```
//!
//! ## Analog mode
//!
//! The driver provides the ADC-to-distance conversion. Reading the ADC is the
//! caller's responsibility.
//!
//! The formula is: `distance_cm = (raw / max_raw) * VCC / 0.006 V`  
//! which simplifies to roughly **2 cm per LSB** on a 12-bit / 3.3 V system.
//!
//! ```ignore
//! use urm37::analog::adc_to_distance_cm;
//!
//! // 12-bit ADC (max = 4095), VCC = 3.3 V
//! let raw: u16 = adc.read(&mut pin)?;
//! match adc_to_distance_cm(raw, 4095) {
//!     Some(cm) => defmt::info!("Distance: {} cm", cm),
//!     None     => defmt::warn!("Hors plage"),
//! }
//!
//! // 10-bit ADC (max = 1023), VCC = 5 V
//! let raw: u16 = adc.read(&mut pin)?;
//! match adc_to_distance_cm(raw, 1023) {
//!     Some(cm) => defmt::info!("Distance: {} cm", cm),
//!     None     => defmt::warn!("Hors plage"),
//! }
//! ```

#![no_std]
#![deny(missing_docs)]

// ── Always-present modules (no feature gate required) ────────────────────────

/// Low-level UART frame encoding and decoding (protocol layer).
pub mod protocol;

/// Driver error types.
pub mod error;

/// URM37 internal EEPROM register map and helpers.
pub mod eeprom;

// ── Feature-gated modules ─────────────────────────────────────────────────────

/// **Synchronous** UART driver (`feature = "uart"`).
///
/// Requires `embedded_io::Read + Write`.
#[cfg(feature = "uart")]
pub mod uart;

/// **Asynchronous** UART driver (`feature = "uart-async"`).
///
/// Requires `embedded_io_async::Read + Write`.
#[cfg(feature = "uart-async")]
pub mod uart_async;

/// Utilities for **PWM trigger** mode (`feature = "pwm"`).
///
/// Pulse width measurement is the caller's responsibility.
#[cfg(feature = "pwm")]
pub mod pwm;

/// Utilities for **analog ADC** mode (`feature = "analog"`).
///
/// ADC reading is the caller's responsibility.
#[cfg(feature = "analog")]
pub mod analog;

// ── Convenient re-exports ─────────────────────────────────────────────────────

pub use error::Error;
pub use eeprom::EepromRegister;