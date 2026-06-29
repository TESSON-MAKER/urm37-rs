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
//! ## PWM mode
//!
//! ```ignore
//! use urm37::pwm::pulse_to_distance_cm;
//!
//! // Trigger + measure the ECHO pulse with your timer
//! let pulse_us: u32 = /* ... */;
//! let cm = pulse_to_distance_cm(pulse_us);
//! ```
//!
//! ## Analog mode
//!
//! ```ignore
//! use urm37::analog::adc_to_distance_cm;
//!
//! let raw: u16 = adc.read(&mut pin)?;
//! let cm = adc_to_distance_cm(raw, 4095); // 12-bit ADC
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