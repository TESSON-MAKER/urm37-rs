#![cfg_attr(feature = "analog", allow(dead_code))]
//! Analog mode example for URM37 sensor.
//!
//! This example demonstrates how to use the URM37 driver in analog mode.
//! The sensor outputs voltage on DAC_OUT (pin 7) proportional to distance.
//!
//! Run with: cargo run --example analog_mode --features analog
//!
//! # Hardware setup
//! - DAC_OUT (pin 7) → ADC input pin
//! - VCC (pin 1) → 3.3V or 5V
//! - GND (pin 2) → GND
//!
//! # Typical measurement loop
//! ```ignore
//! use urm37::analog::adc_to_distance_cm;
//!
//! loop {
//!     let raw = adc.read(&mut pin)?;
//!     let distance = adc_to_distance_cm(raw, 4095); // for 12-bit ADC
//!     println!("Distance: {} cm", distance);
//! }
//! ```

use urm37::analog::{adc_to_distance_cm, distance_cm_to_adc, voltage_mv_to_distance_cm, ANALOG_MAX_RANGE_CM};

fn main() {
    println!("URM37 Analog Mode Example");
    println!("========================\n");

    println!("Hardware Requirements:");
    println!("  - ADC input for reading DAC_OUT voltage");
    println!("  - 10-bit, 12-bit, or other ADC resolution\n");

    println!("Key Specifications:");
    println!("  - Maximum range: {} cm", ANALOG_MAX_RANGE_CM);
    println!("  - Voltage range: 0V (0cm) → Vcc ({} cm)", ANALOG_MAX_RANGE_CM);
    println!();

    // 12-bit ADC example
    println!("Example 1: 12-bit ADC (0..4095)");
    println!("------------------------------");
    adc_conversion_table(4095);

    println!();

    // 10-bit ADC example
    println!("Example 2: 10-bit ADC (0..1023)");
    println!("------------------------------");
    adc_conversion_table(1023);

    println!();

    // Voltage-based conversion
    println!("Example 3: Voltage Conversion");
    println!("----------------------------");
    voltage_conversion_examples();

    println!();

    // Calibration helper
    println!("Example 4: Distance → ADC (calibration)");
    println!("--------------------------------------");
    let adc_max = 4095;
    for distance in [0, 100, 200, 400, 800].iter() {
        let adc_val = distance_cm_to_adc(*distance, adc_max);
        println!("{}  cm → ADC value {}", distance, adc_val);
    }
}

fn adc_conversion_table(adc_max: u16) {
    let adc_steps = [0, 256, 512, 1024, 2048, 3072, adc_max];
    for adc_val in adc_steps.iter() {
        let distance = adc_to_distance_cm(*adc_val, adc_max);
        let percent = (*adc_val as f32 / adc_max as f32) * 100.0;
        println!("ADC {:4} ({:5.1}%) → {:3} cm", adc_val, percent, distance);
    }
}

fn voltage_conversion_examples() {
    println!("3.3V supply (3300 mV):");
    for voltage in [0, 825, 1650, 3300].iter() {
        let distance = voltage_mv_to_distance_cm(*voltage, 3300);
        println!("  {} mV → {} cm", voltage, distance);
    }

    println!();
    println!("5.0V supply (5000 mV):");
    for voltage in [0, 1250, 2500, 5000].iter() {
        let distance = voltage_mv_to_distance_cm(*voltage, 5000);
        println!("  {} mV → {} cm", voltage, distance);
    }
}
