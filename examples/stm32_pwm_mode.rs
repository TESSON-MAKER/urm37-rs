//! Real-world example: URM37 PWM trigger mode on STM32 with Embassy
//!
//! This example shows how to use the URM37 in PWM trigger mode.
//! The driver manages the TRIG pin, and we use input capture on a timer
//! to measure the ECHO pulse width.
//!
//! # Hardware setup
//! - TRIG (pin 6, COMP) → PA0 (GPIO output)
//! - ECHO (pin 4) → PA5 (TIM2_CH1, input capture rising edge)
//! - ECHO (pin 4) → PA1 (TIM2_CH2, input capture falling edge)
//! - VCC (pin 1) → 3.3V
//! - GND (pin 2) → GND
//!
//! # Build and flash
//! ```bash
//! cargo build --example stm32_pwm_mode --target thumbv7em-none-eabihf --features pwm
//! cargo flash --example stm32_pwm_mode --target thumbv7em-none-eabihf
//! ```

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::input_capture::{CapturePin, InputCapture, InputCapturePolarity};
use embassy_stm32::timer::Channel;
use embassy_stm32::time::hz;
use embassy_stm32::Peripherals;
use embassy_time::{Delay, Timer};
use urm37::pwm::Urm37Pwm;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = Peripherals::take();

    // Configure TRIG as GPIO output (PA0)
    let trig = Output::new(p.PA0, Level::High, Speed::Low);

    // Configure input capture on TIM2 (PA5 rising, PA1 falling)
    let mut ic = InputCapture::new(
        p.TIM2,
        Some(CapturePin::new_ch1(p.PA5)), // Rising edge
        Some(CapturePin::new_ch2(p.PA1)), // Falling edge
        None,
        None,
        hz(1_000_000), // 1 tick = 1 µs (for accurate pulse measurement)
        CountingMode::EdgeAlignedUp,
    );

    ic.set_input_capture_polarity(Channel::Ch1, InputCapturePolarity::Rising);
    ic.set_input_capture_polarity(Channel::Ch2, InputCapturePolarity::Falling);

    // Create URM37 driver
    let mut sensor = Urm37Pwm::new(trig).expect("Failed to initialize TRIG pin");
    let mut delay = Delay;

    defmt::info!("URM37 PWM mode initialized");

    loop {
        // Trigger measurement and read ECHO pulse width
        let distance = sensor
            .measure(&mut delay, || async {
                // Capture both edges concurrently
                let (t_rise, t_fall) = join(
                    ic.capture(Channel::Ch1),
                    ic.capture(Channel::Ch2),
                )
                .await;

                // Compute pulse width in microseconds
                t_fall.wrapping_sub(t_rise)
            })
            .await;

        match distance {
            Ok(Some(cm)) => defmt::info!("Distance: {} cm", cm),
            Ok(None) => defmt::warn!("Out of range or invalid reading"),
            Err(e) => defmt::error!("Measurement error: {:?}", e),
        }

        // Wait 500ms before next measurement
        Timer::after_millis(500).await;
    }
}

// Panic handler for no_std environment
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
