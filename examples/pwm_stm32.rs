//! URM37 PWM Trigger example for STM32F767ZI with Embassy
//!
//! This example demonstrates:
//! - PWM mode (autonomous: sensor auto-triggers internally)
//! - Echo pulse measurement with InputCapture
//! - High precision timing (µs resolution)
//!
//! Hardware:
//! - STM32F767ZI (Nucleo F767ZI)
//! - GPIO PA0: TRIG output (for passive mode if needed)
//! - TIM2 CH1 PA5: ECHO input (InputCapture)
//! - Timer: TIM2 at 1 MHz (1 tick = 1 µs)
//!
//! Setup:
//! 1. Configure URM37 in passive mode (MeasureMode = 0xBB) via UART
//! 2. Call read_distance_manual() which sends TRIG + reads echo
//!
//! Run:
//! ```
//! cargo run --example pwm_stm32 --features pwm --release
//! ```

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::timer::input_capture::{CapturePin, InputCapture};
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::{CaptureCompareInterruptHandler, Channel};
use embassy_stm32::{bind_interrupts, peripherals};
use embassy_stm32::time::Hertz;
use embassy_time::Timer;
use urm37::pwm::{Urm37Pwm, PulseReader};
use embedded_hal_async::delay::DelayNs;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    TIM2 => CaptureCompareInterruptHandler<peripherals::TIM2>;
});

/// Implementation of PulseReader for STM32 InputCapture
/// Synchronizes on rising edge (idle state), then measures LOW pulse
struct StmPulseReader<'d> {
    ic: InputCapture<'d, peripherals::TIM2>,
}

impl<'d> PulseReader for StmPulseReader<'d> {
    fn measure_pulse(&mut self) -> impl core::future::Future<Output = Option<u32>> + '_ {
        async move {
            // Synchronize: wait for rising edge (ECHO HIGH = idle)
            self.ic.wait_for_rising_edge(Channel::Ch1).await;

            // Measure LOW pulse: wait for falling edge (start)
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
}

/// Implementation of DelayNs for embassy_time
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
        Some(CapturePin::new(p.PA5, Pull::None)),
        None,
        None,
        None,
        Irqs,
        Hertz(1_000_000),  // 1 MHz = 1 µs per tick
        CountingMode::EdgeAlignedUp,
    );

    // Initialize PWM driver
    let pulse_reader = StmPulseReader { ic };
    let delay = EmbassyDelay;
    let mut sensor = Urm37Pwm::new(trig, pulse_reader, delay).expect("PWM init failed");

    // Configure trigger pulse duration
    sensor.set_trigger_duration(10);  // 10 ms pulse

    info!("=== URM37 PWM Passive Mode Example ===");
    info!("TRIG: PA0 (GPIO), ECHO: PA5 (TIM2 CH1), Timer: 1 MHz");

    loop {
        // In passive mode: driver sends TRIG + reads echo
        match sensor.read_distance_manual().await {
            Ok(Some(cm)) => {
                info!("Distance: {} cm", cm);
            }
            Ok(None) => {
                warn!("Out of range (no echo)");
            }
            Err(e) => {
                error!("Error: {:?}", e);
            }
        }

        Timer::after_millis(100).await;
    }
}
