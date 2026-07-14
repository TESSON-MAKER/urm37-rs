//! # URM37 Analog ADC Example for STM32F767ZI
//!
//! Demonstrates analog distance measurement via ADC without UART.
//!
//! ## Hardware Setup
//! - **ADC1 (PA4)**: URM37 analog voltage output
//! - **STM32F767ZI (Nucleo)**
//!
//! ## Output Format
//! ```text
//! [DISTANCE] X.X cm
//! ```
//!
//! ## Build & Run
//! ```bash
//! cargo run --example stm32_analog --features analog --release
//! ```

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::adc::{Adc, SampleTime};
use embassy_time::Timer;
use urm37::analog::adc_to_distance_cm;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let mut adc = Adc::new(p.ADC1);
    let mut pin = p.PA4;

    loop {
        // Read raw ADC value
        let raw: u16 = adc.blocking_read(&mut pin, SampleTime::CYCLES112);

        // Convert to distance (0-800 cm linear mapping)
        let distance = adc_to_distance_cm(raw, 4095);
        let cm_int = distance as u32;
        let cm_frac = ((distance * 10.0) as u32) % 10;

        info!("[DISTANCE] {}.{} cm", cm_int, cm_frac);

        Timer::after_millis(500).await;
    }
}
