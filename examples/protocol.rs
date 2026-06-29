//! Low-level protocol example for URM37 sensor.
//!
//! This example demonstrates the underlying protocol layer without any HAL dependencies.
//! Useful for understanding the frame structure or debugging communication issues.

use urm37::protocol::*;

fn main() {
    println!("URM37 Protocol Layer Example");
    println!("===========================\n");

    println!("Command Frames");
    println!("--------------");
    show_frame("Distance request", frame_distance());
    show_frame("Temperature request", frame_temperature());
    show_frame("EEPROM read (addr 0x02)", frame_eeprom_read(0x02));
    show_frame("EEPROM write (addr 0x03, value 0x20)", frame_eeprom_write(0x03, 0x20));

    println!("\nChecksum Validation");
    println!("-------------------");

    // Valid frame
    let valid = [0x02u8, 0x00, 0x00, 0x02];
    match validate_checksum(&valid) {
        Ok(()) => println!("✓ Frame {:02X?} has valid checksum", valid),
        Err((exp, got)) => println!("✗ Frame checksum mismatch: expected 0x{:02X}, got 0x{:02X}", exp, got),
    }

    // Invalid frame
    let invalid = [0x02u8, 0x00, 0x00, 0xFF];
    match validate_checksum(&invalid) {
        Ok(()) => println!("✓ Frame {:02X?} has valid checksum", invalid),
        Err((exp, got)) => println!("✗ Frame checksum mismatch: expected 0x{:02X}, got 0x{:02X}", exp, got),
    }

    println!("\nResponse Decoding");
    println!("-----------------");

    // Simulate distance response: 100 cm
    let dist_response = [0x02u8, 0x00, 0x64, 0x66];
    println!("Distance response: {:02X?}", dist_response);
    if let Ok(()) = validate_checksum(&dist_response) {
        if let Some(dist) = decode_distance(&dist_response) {
            println!("  ✓ Valid checksum");
            println!("  → Distance: {} cm", dist);
        }
    }

    // Simulate temperature response: 23.5°C
    let temp_response = [0x01u8, 0x00, 0xEB, 0xEC];
    println!("\nTemperature response: {:02X?}", temp_response);
    if let Ok(()) = validate_checksum(&temp_response) {
        if let Some(temp) = decode_temperature(&temp_response) {
            println!("  ✓ Valid checksum");
            println!("  → Temperature: {}.{} °C", temp / 10, (temp % 10).abs());
        }
    }

    // Simulate negative temperature: -1.5°C
    let raw: u16 = 0x800F;
    let temp_negative = [0x01u8, (raw >> 8) as u8, (raw & 0xFF) as u8, 0x10];
    println!("\nNegative temperature response: {:02X?}", temp_negative);
    if let Ok(()) = validate_checksum(&temp_negative) {
        if let Some(temp) = decode_temperature(&temp_negative) {
            println!("  ✓ Valid checksum");
            println!("  → Temperature: {}.{} °C", temp / 10, (temp % 10).abs());
        }
    }

    // Invalid reading response (0xFFFF)
    let invalid_dist = [0x02u8, 0xFF, 0xFF, 0x01];
    println!("\nInvalid distance response: {:02X?}", invalid_dist);
    if let Ok(()) = validate_checksum(&invalid_dist) {
        match decode_distance(&invalid_dist) {
            Some(dist) => println!("  → Distance: {} cm", dist),
            None => println!("  → Out of range or invalid reading"),
        }
    }

    println!("\nConstants");
    println!("---------");
    println!("CMD_DISTANCE:           0x{:02X}", CMD_DISTANCE);
    println!("CMD_TEMPERATURE:        0x{:02X}", CMD_TEMPERATURE);
    println!("CMD_EEPROM_READ:        0x{:02X}", CMD_EEPROM_READ);
    println!("CMD_EEPROM_WRITE:       0x{:02X}", CMD_EEPROM_WRITE);
    println!();
    println!("INVALID_DISTANCE:       0x{:04X}", INVALID_DISTANCE);
    println!("INVALID_TEMPERATURE_RAW: 0x{:04X}", INVALID_TEMPERATURE_RAW);
    println!();
    println!("EEPROM_THRESHOLD_HIGH:  0x{:02X}", EEPROM_THRESHOLD_HIGH);
    println!("EEPROM_THRESHOLD_LOW:   0x{:02X}", EEPROM_THRESHOLD_LOW);
    println!("EEPROM_MEASURE_MODE:    0x{:02X}", EEPROM_MEASURE_MODE);
    println!("EEPROM_AUTO_INTERVAL:   0x{:02X}", EEPROM_AUTO_INTERVAL);
    println!("EEPROM_GAIN:            0x{:02X}", EEPROM_GAIN);
}

fn show_frame(label: &str, frame: [u8; 4]) {
    println!("{:30} → {:02X} {:02X} {:02X} {:02X}", label, frame[0], frame[1], frame[2], frame[3]);
}
