//! Synchronous UART example for URM37 sensor.
//!
//! This example demonstrates how to use the URM37 driver in synchronous UART mode
//! with embedded-io.
//!
//! # Hardware setup
//! - UART RXD (pin 8) → MCU RX
//! - UART TXD (pin 9) → MCU TX
//! - VCC (pin 1) → 3.3V or 5V
//! - GND (pin 2) → GND

// Example uses protocol layer directly, no need to import Urm37Uart for demo

fn main() {
    // Mock UART for demonstration purposes
    // In real code, this would be your HAL's UART peripheral
    println!("URM37 Synchronous UART Example");
    println!("==============================\n");

    println!("Usage on real hardware:");
    println!("  1. Create UART with your HAL");
    println!("  2. Create Urm37Uart: let mut sensor = Urm37Uart::new(uart);");
    println!("  3. Read distance: let dist = sensor.read_distance()?;");
    println!("  4. Read temperature: let temp = sensor.read_temperature()?;\n");

    println!("Features:");
    println!("  ✓ Zero-copy protocol handling");
    println!("  ✓ Works with any embedded-io UART");
    println!("  ✓ EEPROM configuration support");
    println!("  ✓ Temperature sensing\n");

    example_protocol();
}

fn example_protocol() {
    use urm37::protocol;

    println!("Protocol Example (no hardware required):");
    println!("----------------------------------------");

    // Build a distance request frame
    let frame = protocol::frame_distance();
    println!("Distance request frame: {:02X?}", frame);

    // Build a temperature request frame
    let frame = protocol::frame_temperature();
    println!("Temperature request frame: {:02X?}", frame);

    // Simulate a distance response: 100 cm
    let response = [0x02u8, 0x00, 0x64, 0x66];
    if let Ok(()) = protocol::validate_checksum(&response) {
        if let Some(distance) = protocol::decode_distance(&response) {
            println!("Decoded distance: {} cm", distance);
        }
    }

    // Simulate a temperature response: 23.5°C
    let response = [0x01u8, 0x00, 0xEB, 0xEC];
    if let Ok(()) = protocol::validate_checksum(&response) {
        if let Some(temperature) = protocol::decode_temperature(&response) {
            println!("Decoded temperature: {}.{} °C", temperature / 10, temperature % 10);
        }
    }
}
