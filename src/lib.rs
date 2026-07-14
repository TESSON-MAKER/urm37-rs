//! # urm37
//!
//! **`no_std` embedded driver for the DFRobot URM37 V4.0 ultrasonic distance sensor.**
//!
//! ![DFRobot URM37 V4.0](https://raw.githubusercontent.com/TESSON-MAKER/urm37-rs/main/urm37v4-0.png)
//!
//! This crate provides a platform-agnostic driver supporting all sensor interface modes:
//! UART (sync & async), PWM trigger, and analog ADC.
//!
//! - **No allocations**: Stack-only, suitable for embedded systems with limited memory
//! - **HAL-agnostic**: Works with any `embedded-io` / `embedded-hal` implementation
//! - **Feature-gated**: Include only what you need
//! - **Comprehensive**: EEPROM configuration, temperature reading, multiple output modes
//! - **Tested**: 45 unit and integration tests covering all protocol operations
//!
//! ## Supported Modes
//!
//! | Mode | Feature | Traits | Use Case |
//! |------|---------|--------|----------|
//! | **Synchronous UART** | `uart` | `embedded-io::Read + Write` | Simple blocking I/O |
//! | **Asynchronous UART** | `uart-async` | `embedded-io-async::Read + Write` | Embassy, RTIC, async/await |
//! | **PWM Trigger** | `pwm` | GPIO output + your timer | Maximum flexibility |
//! | **Analog ADC** | `analog` | None (math only) | Direct voltage measurement |
//!
//! ## Quick start (async UART with Embassy)
//!
//! ```toml
//! [dependencies]
//! urm37 = { version = "0.6", features = ["uart-async"] }
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
//! The driver manages the TRIG pin and exposes a `measure()` method
//! that accepts an async closure for the ECHO pulse measurement.
//! Measuring the pulse width is the caller's responsibility and depends on the
//! HAL and timer peripheral available.
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
//!     Some(CapturePin::new_ch1(p.PA5)), // rising edge
//!     Some(CapturePin::new_ch2(p.PA1)), // falling edge
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
//!     let distance = sensor.measure(&mut Delay, || async {
//!         // Capture both edges concurrently and compute the pulse width.
//!         let (t_rise, t_fall) = join(
//!             ic.capture(Channel::Ch1),
//!             ic.capture(Channel::Ch2),
//!         ).await;
//!         t_fall.wrapping_sub(t_rise)
//!     }).await.unwrap();
//!
//!     match distance {
//!         Some(cm) => defmt::info!("Distance: {} cm", cm),
//!         None     => defmt::warn!("Out of range or invalid reading"),
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
//!     None     => defmt::warn!("Out of range"),
//! }
//!
//! // 10-bit ADC (max = 1023), VCC = 5 V
//! let raw: u16 = adc.read(&mut pin)?;
//! match adc_to_distance_cm(raw, 1023) {
//!     Some(cm) => defmt::info!("Distance: {} cm", cm),
//!     None     => defmt::warn!("Out of range"),
//! }
//! ```

#![no_std]
#![deny(missing_docs)]

// Always-present modules (no feature gate required)

/// Low-level UART frame encoding and decoding (protocol layer).
///
/// This module contains the URM37 protocol implementation:
/// - `Frame`: 4-byte command/response structure
/// - `Command`: enum of all possible commands
/// - `EepromRegister`: EEPROM register addresses
/// - Frame building, checksum calculation, and parsing
/// - EEPROM threshold encoding/decoding helpers
pub mod protocol;

/// Driver error types.
///
/// Errors that can occur during sensor communication and data reading.
pub mod error;

// Re-export common EEPROM types and functions from protocol for convenience
pub use protocol::{encode_threshold, decode_threshold, EepromRegister};

// Feature-gated modules

/// **Synchronous (Blocking)** UART driver (`feature = "blocking"`).
///
/// Provides the `Urm37Uart<T>` driver for blocking UART communication.
/// Requires `embedded_io::Read + Write`.
/// Works with any blocking UART implementation.
///
/// # Example
/// ```ignore
/// use urm37::uart::Urm37Uart;
///
/// let mut sensor = Urm37Uart::new(uart_peripheral);
/// let distance = sensor.read_distance()?;
/// let temp = sensor.read_temperature()?;
/// ```
#[cfg(feature = "blocking")]
pub mod uart;

/// **Asynchronous (Non-blocking)** UART driver (`feature = "async"`).
///
/// Provides the `Urm37UartAsync<T>` driver for async/await UART communication.
/// Requires `embedded_io_async::Read + Write`.
/// Works with any async UART implementation.
///
/// # Example
/// ```ignore
/// use urm37::uart_async::Urm37UartAsync;
///
/// let mut sensor = Urm37UartAsync::new(uart_peripheral);
/// let distance = sensor.read_distance().await?;
/// let temp = sensor.read_temperature().await?;
/// ```
#[cfg(feature = "async")]
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

