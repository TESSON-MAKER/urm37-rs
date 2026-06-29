//! EEPROM configuration example for URM37 sensor.
//!
//! This example demonstrates how to configure the URM37's internal EEPROM registers.
//! All configuration options are available in both synchronous and asynchronous modes.
//!
//! # Warning
//! EEPROM writes are non-volatile and persist across power cycles.
//! Avoid writing in tight loops — EEPROM has limited endurance (~100k cycles).

use urm37::eeprom::{decode_threshold, encode_threshold, EepromRegister};

fn main() {
    println!("URM37 EEPROM Configuration Example");
    println!("==================================\n");

    println!("Available EEPROM Registers:");
    println!("  0x00 - THRESHOLD_HIGH   (COMP distance threshold - high byte)");
    println!("  0x01 - THRESHOLD_LOW    (COMP distance threshold - low byte)");
    println!("  0x02 - MEASURE_MODE     (0=passive, 1=auto)");
    println!("  0x03 - AUTO_INTERVAL    (measurement interval in units of 25ms)");
    println!("  0x04 - GAIN             (internal gain, reserved)\n");

    println!("Configuration Examples:");
    println!("----------------------\n");

    // Example 1: COMP/Switch threshold
    println!("1. Setting COMP threshold to 50 cm:");
    println!("   ");
    let (high, low) = encode_threshold(50);
    println!("   encode_threshold(50) → high=0x{:02X}, low=0x{:02X}", high, low);

    let decoded = decode_threshold(high, low);
    println!("   decode_threshold(0x{:02X}, 0x{:02X}) → {} cm\n", high, low, decoded);

    // Example 2: Threshold encoding for various distances
    println!("2. Threshold encoding table:");
    for dist in [0, 50, 100, 200, 500, 800].iter() {
        let (high, low) = encode_threshold(*dist);
        println!("   {} cm  → 0x{:02X}{:02X}", dist, high, low);
    }
    println!();

    // Example 3: Auto-measurement intervals
    println!("3. Auto-measurement interval examples:");
    println!("   (unit: 25ms)");
    let intervals = [
        (1, "25 ms"),
        (2, "50 ms"),
        (4, "100 ms"),
        (10, "250 ms"),
        (20, "500 ms"),
        (40, "1 second"),
        (200, "5 seconds"),
    ];
    for (val, desc) in intervals.iter() {
        println!("   {} → {}", val, desc);
    }
    println!();

    // Example 4: Register enum usage
    println!("4. Using EepromRegister enum:");
    let regs = [
        EepromRegister::ThresholdHigh,
        EepromRegister::ThresholdLow,
        EepromRegister::MeasureMode,
        EepromRegister::AutoInterval,
        EepromRegister::Gain,
    ];

    for reg in regs.iter() {
        let addr = reg.address();
        let is_std = reg.is_standard();
        println!("   {:?}", reg);
        println!("     Address: 0x{:02X}, Standard: {}\n", addr, is_std);
    }

    println!("5. Raw register access:");
    println!("   EepromRegister::Raw(0x42) for undocumented registers\n");

    // Example 5: Measurement modes
    println!("6. Measurement modes:");
    println!("   Passive mode:  MeasureMode = 0x00");
    println!("                  Sensor responds only to explicit read requests\n");
    println!("   Auto mode:     MeasureMode = 0x01");
    println!("                  Sensor takes measurements at AutoInterval rate\n");

    println!("Usage in code:");
    println!("  Synchronous:   sensor.set_comp_threshold(50)?;");
    println!("  Asynchronous:  sensor.set_comp_threshold(50).await?;");
    println!("                 sensor.set_auto_mode(40).await?;");
    println!("                 sensor.set_passive_mode().await?;");
}
