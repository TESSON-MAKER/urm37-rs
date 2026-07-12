//! URM37 Analog ADC example for STM32F767ZI with Embassy
//!
//! This example demonstrates:
//! - Reading distance via analog ADC (voltage output)
//! - No UART required, simple and fast
//! - Direct ADC reading and conversion to distance
//!
//! Hardware:
//! - STM32F767ZI (Nucleo F767ZI)
//! - ADC1: PA4 (analog voltage from URM37)
//! - URM37 ANALOG pin connected to PA4
//!
//! Formula: distance_cm = (ADC_reading / ADC_max) * VCC / 0.006
//! For 12-bit ADC (max=4095) at 3.3V: ~2 cm per LSB
//!
//! Run:
//! ```
//! cargo run --example analog_stm32 --features analog --release
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

    info!("=== URM37 Analog ADC Example ===");
    info!("ADC: 12-bit, VCC: 3.3V");

    loop {
        // Read raw ADC value
        let raw: u16 = adc.blocking_read(&mut pin, SampleTime::CYCLES112);

        // Convert to distance (0-800 cm linear mapping)
        let distance = adc_to_distance_cm(raw, 4095);
        let cm_int = distance as u32;
        let cm_frac = ((distance * 10.0) as u32) % 10;

        info!("ADC: {}, Distance: {}.{} cm", raw, cm_int, cm_frac);

        Timer::after_millis(200).await;
    }
}
