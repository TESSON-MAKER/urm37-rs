//! Pseudo-code example: URM37 PWM trigger mode on STM32
//!
//! Shows how to use PWM mode with input capture for precise timing.
//!
//! # Hardware setup
//! - TRIG (pin 6) → GPIO output (PA0 example)
//! - ECHO (pin 4) → Timer input capture (TIM2_CH1 rising, TIM2_CH2 falling)
//! - VCC → 3.3V, GND → GND
//!
//! # Pattern
//! ```rust,no_run
//! # use urm37::pwm::Urm37Pwm;
//! # async fn example() {
//! use embassy_futures::join::join;
//!
//! // 1. Configure TRIG pin as GPIO output
//! let trig = /* GPIO output setup */;
//!
//! // 2. Configure input capture on timer (dual-edge mode)
//! let mut input_capture = /* Timer input capture setup */;
//!
//! // 3. Create PWM driver
//! let mut sensor = Urm37Pwm::new(trig).unwrap();
//!
//! // 4. Trigger measurement with pulse capture
//! let distance = sensor.measure(&mut delay, || async {
//!     // Measure ECHO pulse width using input capture
//!     // Rising edge on CH1, falling edge on CH2
//!     let (t_rise, t_fall) = join(
//!         input_capture.capture(Channel::Ch1),
//!         input_capture.capture(Channel::Ch2),
//!     ).await;
//!
//!     // Return pulse width in microseconds
//!     t_fall.wrapping_sub(t_rise)
//! }).await;
//! # }
//! ```
//!
//! # Key points
//! - Timer resolution: 1 µs recommended (1 MHz clock)
//! - Pulse width formula: distance (cm) = pulse width (µs) / 50
//! - Dual-edge capture: concurrent rising/falling measurement
//! - Requires accurate timer: use input capture, not GPIO polling

fn main() {
    println!("This is a pseudo-code example.");
    println!("See the documentation above for the PWM trigger pattern.");
    println!("Adapt to your STM32 board: https://docs.embassy.dev/");
}
