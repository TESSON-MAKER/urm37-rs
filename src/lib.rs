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
//! | **Synchronous UART** | `blocking` | `embedded-io::Read + Write` | Simple blocking I/O |
//! | **Asynchronous UART** | `async` | `embedded-io-async::Read + Write` | Embassy, RTIC, async/await |
//! | **PWM Trigger** | `pwm` | GPIO output + your timer | Maximum flexibility |
//! | **Analog ADC** | `analog` | None (math only) | Direct voltage measurement |
//!
//! ## Standardized Output Format
//!
//! All examples follow this output format for easy parsing:
//! ```text
//! [DISTANCE] X cm              # Successful distance measurement
//! [TEMPERATURE] X.X °C         # Temperature reading
//! [OUT_OF_RANGE]               # Sensor reading out of valid range
//! [ERROR]                       # Communication or sensor error
//! ```
//!
//! ## Examples
//!
//! Ready-to-use examples for popular platforms:
//!
//! ### Arduino Mega 2560
//! - **[mega2560_uart](examples/mega2560_uart.rs)**: UART communication (dual USART)
//! - **[mega2560_pwm](examples/mega2560_pwm.rs)**: PWM pulse measurement
//! - **[mega2560_analog](examples/mega2560_analog.rs)**: Analog ADC reading
//!
//! ### STM32F767ZI (Nucleo)
//! - **[stm32_uart_async](examples/stm32_uart_async.rs)**: Async UART with Embassy
//! - **[stm32_pwm](examples/stm32_pwm.rs)**: Async PWM with InputCapture
//! - **[stm32_analog](examples/stm32_analog.rs)**: Async ADC with Embassy
//!
//! All examples output distance/temperature in the standardized format above.
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
//! ## PWM mode
//!
//! Distance measurement via ECHO pulse width. The driver manages the TRIG pin
//! and supports both synchronous and asynchronous implementations.
//!
//! ### Async Mode (Embassy-based, recommended)
//!
//! Use `Urm37PwmAsync` for non-blocking async/await code with Embassy:
//!
//! ```ignore
//! use embassy_stm32::timer::input_capture::{CapturePin, InputCapture};
//! use embassy_stm32::timer::Channel;
//! use embassy_time::Timer;
//! use urm37::pwm_async::{Urm37PwmAsync, PulseReaderAsync};
//!
//! // Async pulse reader using InputCapture
//! struct AsyncPulseReader<'d> { ic: InputCapture<'d, peripherals::TIM2> }
//!
//! impl<'d> PulseReaderAsync for AsyncPulseReader<'d> {
//!     async fn measure_pulse(&mut self) -> Option<u32> {
//!         self.ic.wait_for_rising_edge(Channel::Ch1).await;
//!         let t_fall = self.ic.wait_for_falling_edge(Channel::Ch1).await;
//!         let t_rise = self.ic.wait_for_rising_edge(Channel::Ch1).await;
//!         let duration_us = t_rise.wrapping_sub(t_fall);
//!         (duration_us > 0 && duration_us < 50000).then_some(duration_us)
//!     }
//! }
//!
//! let trig = Output::new(p.PA0, Level::High, Speed::Low);
//! let mut sensor = Urm37PwmAsync::new(trig, AsyncPulseReader { ic }, Delay)?;
//! match sensor.read_distance_manual().await {
//!     Ok(Some(cm)) => defmt::info!("Distance: {} cm", cm),
//!     _ => {}
//! }
//! ```
//!
//! ### Sync Mode (Blocking)
//!
//! Use `Urm37Pwm` for simple blocking code without async:
//!
//! ```ignore
//! use urm37::pwm::{Urm37Pwm, PulseReader};
//!
//! struct SyncPulseReader { echo: Pin, timer: Timer }
//!
//! impl PulseReader for SyncPulseReader {
//!     fn measure_pulse(&mut self) -> Option<u32> {
//!         while self.echo.is_low() { }
//!         while self.echo.is_high() { }
//!         let t_fall = self.timer.counter();
//!         while self.echo.is_low() { }
//!         let duration_us = self.timer.counter().wrapping_sub(t_fall);
//!         (duration_us > 0 && duration_us < 50000).then_some(duration_us)
//!     }
//! }
//!
//! let mut sensor = Urm37Pwm::new(trig, SyncPulseReader { ... }, delay)?;
//! match sensor.read_distance_manual() {
//!     Ok(Some(cm)) => println!("Distance: {} cm", cm),
//!     _ => {}
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

/// **Synchronous (Blocking)** PWM trigger driver (`feature = "pwm"`).
///
/// Provides the `Urm37Pwm` driver for blocking PWM pulse measurement.
///
/// Use this when you want a simple blocking API without async/await overhead.
/// The pulse reader implementation should use busy-waiting or a blocking timer.
///
/// Supports both sensor modes:
/// - **Autonomous (0xAA):** `read_distance()` - sensor auto-triggers
/// - **Passive (0xBB):** `read_distance_manual()` - MCU sends TRIG pulse
///
/// # Example
/// ```ignore
/// let mut sensor = Urm37Pwm::new(trig_pin, pulse_reader, delay)?;
/// match sensor.read_distance_manual() {
///     Ok(Some(cm)) => println!("Distance: {} cm", cm),
///     Ok(None) => println!("Out of range"),
///     Err(e) => println!("Error: {:?}", e),
/// }
/// ```
#[cfg(feature = "pwm")]
pub mod pwm;

/// **Asynchronous (Non-blocking)** PWM trigger driver (`feature = "pwm"`).
///
/// Provides the `Urm37PwmAsync` driver for async/await PWM pulse measurement.
/// Recommended for Embassy and other async runtimes.
///
/// The pulse reader implementation should return async futures for non-blocking operation.
///
/// Supports both sensor modes:
/// - **Autonomous (0xAA):** `read_distance().await` - sensor auto-triggers
/// - **Passive (0xBB):** `read_distance_manual().await` - MCU sends TRIG pulse
///
/// # Example
/// ```ignore
/// let mut sensor = Urm37PwmAsync::new(trig_pin, pulse_reader, delay)?;
/// match sensor.read_distance_manual().await {
///     Ok(Some(cm)) => println!("Distance: {} cm", cm),
///     Ok(None) => println!("Out of range"),
///     Err(e) => println!("Error: {:?}", e),
/// }
/// ```
#[cfg(feature = "pwm")]
pub mod pwm_async;

/// Utilities for **analog ADC** mode (`feature = "analog"`).
///
/// ADC reading is the caller's responsibility.
#[cfg(feature = "analog")]
pub mod analog;

