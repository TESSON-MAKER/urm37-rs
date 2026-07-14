//! URM37 PWM Trigger (Async) example for STM32F767ZI with Embassy
//!
//! This example demonstrates:
//! - Async PWM mode with InputCapture for echo measurement
//! - Supports both autonomous and passive measurement modes
//! - High precision timing (µs resolution)
//!
//! Hardware:
//! - STM32F767ZI (Nucleo F767ZI)
//! - GPIO PA0: TRIG output (for passive mode)
//! - TIM2 CH1 PA5: ECHO input (InputCapture)
//! - Timer: TIM2 at 1 MHz (1 tick = 1 µs)
//!
//! Sensor Configuration:
//! 1. Autonomous mode (0xAA): Sensor auto-measures → call `read_distance()`
//! 2. Passive mode (0xBB): Manual triggering → call `read_distance_manual()`
//!
//! Run:
//! ```
//! cargo run --example pwm_stm32 --features pwm --release
//! ```

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::timer::input_capture::{CapturePin, InputCapture};
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::{CaptureCompareInterruptHandler, Channel};
use embassy_stm32::{bind_interrupts, peripherals};
use embassy_stm32::time::Hertz;
use embassy_time::Timer;
use urm37::pwm_async::{Urm37PwmAsync, PulseReaderAsync};
use embedded_hal_async::delay::DelayNs;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    TIM2 => CaptureCompareInterruptHandler<peripherals::TIM2>;
});

/// Async implementation of PulseReaderAsync for STM32 InputCapture
/// Synchronizes on rising edge (idle state), then measures LOW pulse
struct StmPulseReaderAsync<'d> {
    ic: InputCapture<'d, peripherals::TIM2>,
}

impl<'d> PulseReaderAsync for StmPulseReaderAsync<'d> {
    async fn measure_pulse(&mut self) -> Option<u32> {
        // Synchronize: wait for rising edge (ECHO HIGH = idle state)
        self.ic.wait_for_rising_edge(Channel::Ch1).await;

        // Measure LOW pulse: wait for falling edge (start of pulse)
        let t_fall = self.ic.wait_for_falling_edge(Channel::Ch1).await;

        // Wait for rising edge (end of LOW pulse)
        let t_rise = self.ic.wait_for_rising_edge(Channel::Ch1).await;

        // Duration in µs (timer at 1 MHz)
        let duration_us = t_rise.wrapping_sub(t_fall);

        // Validate: typical range 50-5000 µs (1-100 cm)
        if duration_us > 0 && duration_us < 50000 {
            Some(duration_us)
        } else {
            None
        }
    }
}

/// Async DelayNs implementation using Embassy timers
struct EmbassyDelay;

impl DelayNs for EmbassyDelay {
    async fn delay_ns(&mut self, ns: u32) {
        if ns > 0 {
            Timer::after_nanos(ns as u64).await;
        }
    }

    async fn delay_us(&mut self, us: u32) {
        if us > 0 {
            Timer::after_micros(us as u64).await;
        }
    }

    async fn delay_ms(&mut self, ms: u32) {
        if ms > 0 {
            Timer::after_millis(ms as u64).await;
        }
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    // Setup GPIO for TRIG (PA0, active LOW)
    let trig = Output::new(p.PA0, Level::High, Speed::Low);

    // Setup InputCapture for ECHO (TIM2 CH1 on PA5)
    let ic = InputCapture::new(
        p.TIM2,
        Some(CapturePin::new(p.PA5)),
        None,
        None,
        None,
        Irqs,
        Hertz(1_000_000),  // 1 MHz = 1 µs per tick
        CountingMode::EdgeAlignedUp,
    );

    // Initialize async PWM driver
    let pulse_reader = StmPulseReaderAsync { ic };
    let delay = EmbassyDelay;
    let mut sensor = Urm37PwmAsync::new(trig, pulse_reader, delay).expect("PWM init failed");

    // Configure trigger pulse duration (default: 10 ms)
    sensor.set_trigger_duration(10);

    info!("=== URM37 PWM Async Example ===");
    info!("TRIG: PA0 (GPIO output), ECHO: PA5 (TIM2 CH1 input capture)");
    info!("Timer: 1 MHz (1 tick = 1 µs)");
    info!("Sensor mode: Passive (manual TRIG)");
    info!("");

    loop {
        // Passive mode: driver sends TRIG pulse + reads echo
        match sensor.read_distance_manual().await {
            Ok(Some(cm)) => {
                info!("✓ Distance: {} cm", cm);
            }
            Ok(None) => {
                warn!("Out of range or invalid reading");
            }
            Err(e) => {
                error!("Error: {:?}", e);
            }
        }

        Timer::after_millis(100).await;
    }
}
