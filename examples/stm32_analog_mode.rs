//! Real-world example: URM37 analog mode on STM32 with Embassy
//!
//! This example shows how to read the URM37 via the analog DAC_OUT pin.
//! The sensor outputs a voltage proportional to distance (0V = 0cm, Vcc = 800cm).
//!
//! # Hardware setup
//! - DAC_OUT (pin 7) → PA6 (ADC1_IN6)
//! - VCC (pin 1) → 3.3V
//! - GND (pin 2) → GND
//!
//! For 12-bit ADC at 3.3V:
//! - ADC max = 4095
//! - Resolution = 800 cm / 4095 ≈ 0.2 cm per LSB
//! - ~2 cm per 10 LSBs
//!
//! # Build and flash
//! ```bash
//! cargo build --example stm32_analog_mode --target thumbv7em-none-eabihf --features analog
//! cargo flash --example stm32_analog_mode --target thumbv7em-none-eabihf
//! ```

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::adc::{Adc, SampleTime};
use embassy_stm32::Peripherals;
use embassy_time::Timer;
use urm37::analog::adc_to_distance_cm;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = Peripherals::take();

    // Configure ADC1
    let mut adc = Adc::new(p.ADC1);

    defmt::info!("URM37 analog mode initialized (12-bit ADC)");

    loop {
        // Read ADC channel 6 (PA6 = DAC_OUT)
        let raw: u16 = adc.read(&mut p.PA6);

        // Convert ADC value to distance
        // For 12-bit ADC: max = 4095
        // For 10-bit ADC: max = 1023
        let distance = adc_to_distance_cm(raw, 4095);

        // Calculate approximate voltage (3.3V supply)
        let voltage_mv = (raw as u32 * 3300) / 4095;

        defmt::info!(
            "Raw: {}, Voltage: {}mV, Distance: {} cm",
            raw,
            voltage_mv,
            distance
        );

        // Wait 500ms before next reading
        Timer::after_millis(500).await;
    }
}

// Panic handler for no_std environment
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
