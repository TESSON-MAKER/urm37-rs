#![allow(unused_imports)]

use urm37::protocol::{Command, EepromRegister, Frame, MeasureMode, SerialLevelMode};

// Ensure we have access to std even though the crate is no_std
extern crate std;
use std::vec::Vec;

// ===== Checksum tests =====

#[test]
fn checksum_simple() {
    let sum = Frame::checksum(0x22, 0x00, 0x00);
    assert_eq!(sum, 0x22);
}

#[test]
fn checksum_with_data() {
    let sum = Frame::checksum(0x33, 0x02, 0x00);
    assert_eq!(sum, 0x35);
}

#[test]
fn checksum_wrapping() {
    let sum = Frame::checksum(0xFF, 0xFF, 0xFF);
    assert_eq!(sum, 0xFD);
}

#[test]
fn checksum_zero() {
    let sum = Frame::checksum(0x00, 0x00, 0x00);
    assert_eq!(sum, 0x00);
}

// ===== Frame building tests =====

#[test]
fn build_distance_request() {
    let frame = Frame::distance_request();
    let data = frame.raw_data();
    assert_eq!(data[0], 0x22);
    assert_eq!(data[1], 0x00);
    assert_eq!(data[2], 0x00);
    assert_eq!(data[3], 0x22);
}

#[test]
fn build_temperature_request() {
    let frame = Frame::temperature_request();
    let data = frame.raw_data();
    assert_eq!(data[0], 0x11);
    assert_eq!(data[1], 0x00);
    assert_eq!(data[2], 0x00);
    assert_eq!(data[3], 0x11);
}

#[test]
fn build_eeprom_read_request() {
    let frame = Frame::eeprom_read_request(EepromRegister::MeasureMode);
    let data = frame.raw_data();
    assert_eq!(data[0], 0x33);
    assert_eq!(data[1], 0x02);
    assert_eq!(data[2], 0x00);
    assert_eq!(data[3], 0x35);
}

#[test]
fn build_eeprom_write_request() {
    let frame = Frame::eeprom_write_request(EepromRegister::SerialLevelMode, 0x01);
    let data = frame.raw_data();
    assert_eq!(data[0], 0x44);
    assert_eq!(data[1], 0x03);
    assert_eq!(data[2], 0x01);
    assert_eq!(data[3], 0x48);
}

#[test]
fn build_custom_command() {
    let frame = Frame::build(Command::EepromWrite, 0xFF, 0xAA);
    let data = frame.raw_data();
    assert_eq!(data[0], 0x44);
    assert_eq!(data[1], 0xFF);
    assert_eq!(data[2], 0xAA);
    // 0x44 + 0xFF + 0xAA = 0x1ED, wraps to 0xED
    assert_eq!(data[3], 0xED);
}

// ===== Frame parsing tests =====

#[test]
fn parse_valid_distance_response() {
    // Distance 300 cm = 0x012C, checksum = 0x22 + 0x01 + 0x2C = 0x4F
    let raw = [0x22, 0x01, 0x2C, 0x4F];
    let frame = Frame::parse(raw).expect("should parse valid frame");
    assert_eq!(frame.raw_data(), &raw);
}

#[test]
fn parse_valid_temperature_response() {
    let raw = [0x11, 0x00, 0xF9, 0x0A];
    let frame = Frame::parse(raw).expect("should parse valid frame");
    assert_eq!(frame.raw_data(), &raw);
}

#[test]
fn parse_invalid_checksum_too_high() {
    let raw = [0x22, 0x00, 0x00, 0x23];
    let err = Frame::parse(raw).expect_err("should reject invalid checksum");
    assert_eq!(err, (0x22, 0x23));
}

#[test]
fn parse_invalid_checksum_too_low() {
    let raw = [0x22, 0x00, 0x00, 0x21];
    let err = Frame::parse(raw).expect_err("should reject invalid checksum");
    assert_eq!(err, (0x22, 0x21));
}

#[test]
fn parse_all_zeros_with_valid_checksum() {
    let raw = [0x00, 0x00, 0x00, 0x00];
    let frame = Frame::parse(raw).expect("should parse all-zeros frame");
    assert_eq!(frame.raw_data(), &raw);
}

#[test]
fn parse_max_values() {
    let raw = [0xFF, 0xFF, 0xFF, 0xFD];
    let frame = Frame::parse(raw).expect("should parse max-values frame");
    assert_eq!(frame.raw_data(), &raw);
}

// ===== Distance decoding tests =====

#[test]
fn decode_distance_100cm() {
    let raw = [0x22, 0x00, 0x64, 0x86];
    let frame = Frame::parse(raw).expect("valid frame");
    let distance = frame.decode_distance();
    assert_eq!(distance, Some(100));
}

#[test]
fn decode_distance_zero() {
    let raw = [0x22, 0x00, 0x00, 0x22];
    let frame = Frame::parse(raw).expect("valid frame");
    let distance = frame.decode_distance();
    assert_eq!(distance, Some(0));
}

#[test]
fn decode_distance_800cm() {
    let raw = [0x22, 0x03, 0x20, 0x45];
    let frame = Frame::parse(raw).expect("valid frame");
    let distance = frame.decode_distance();
    assert_eq!(distance, Some(800));
}

#[test]
fn decode_distance_invalid_sentinel() {
    let raw = [0x22, 0xFF, 0xFF, 0x20];
    let frame = Frame::parse(raw).expect("valid frame");
    let distance = frame.decode_distance();
    assert_eq!(distance, None);
}

#[test]
fn decode_distance_big_endian() {
    let raw = [0x22, 0x12, 0x34, 0x68];
    let frame = Frame::parse(raw).expect("valid frame");
    let distance = frame.decode_distance();
    assert_eq!(distance, Some(0x1234));
}

// ===== Temperature decoding tests =====

#[test]
fn decode_temperature_positive_25_5() {
    let raw = [0x11, 0x00, 0xFF, 0x10];
    let frame = Frame::parse(raw).expect("valid frame");
    let temp = frame.decode_temperature();
    assert_eq!(temp, Some(25.5));
}

#[test]
fn decode_temperature_positive_zero() {
    let raw = [0x11, 0x00, 0x00, 0x11];
    let frame = Frame::parse(raw).expect("valid frame");
    let temp = frame.decode_temperature();
    assert_eq!(temp, Some(0.0));
}

#[test]
fn decode_temperature_negative_10_5() {
    // Negative temperature: -10.5°C = -105 in 0.1°C units
    // Data0 high bits 0xF (sign), low bits contain MSB of value
    // checksum = 0x11 + 0xF0 + 0x69 = 0x16A → 0x6A (wrapping)
    let raw = [0x11, 0xF0, 0x69, 0x6A];
    let frame = Frame::parse(raw).expect("valid frame");
    let temp = frame.decode_temperature();
    assert_eq!(temp, Some(-10.5));
}

#[test]
fn decode_temperature_high_value() {
    // Temperature: 120.5°C = 1205 in 0.1°C units
    // checksum = 0x11 + 0x04 + 0xB5 = 0xCA
    let raw = [0x11, 0x04, 0xB5, 0xCA];
    let frame = Frame::parse(raw).expect("valid frame");
    let temp = frame.decode_temperature();
    assert_eq!(temp, Some(120.5));
}

#[test]
fn decode_temperature_large_positive() {
    let raw = [0x11, 0x02, 0x58, 0x6B];
    let frame = Frame::parse(raw).expect("valid frame");
    let temp = frame.decode_temperature();
    assert_eq!(temp, Some(60.0));
}

// ===== EEPROM register tests =====

#[test]
fn eeprom_registers_values() {
    assert_eq!(EepromRegister::LargerDist as u8, 0x00);
    assert_eq!(EepromRegister::LessDist as u8, 0x01);
    assert_eq!(EepromRegister::MeasureMode as u8, 0x02);
    assert_eq!(EepromRegister::SerialLevelMode as u8, 0x03);
    assert_eq!(EepromRegister::AutoMeasureTime as u8, 0x04);
}

#[test]
fn measure_mode_enum_values() {
    assert_eq!(MeasureMode::Auto as u8, 0xAA);
    assert_eq!(MeasureMode::Passive as u8, 0xBB);
}

#[test]
fn serial_level_mode_enum_values() {
    assert_eq!(SerialLevelMode::Ttl as u8, 0x00);
    assert_eq!(SerialLevelMode::Rs232 as u8, 0x01);
}

// ===== Integration tests =====

#[test]
fn build_and_parse_distance_request() {
    let built = Frame::distance_request();
    let data = built.raw_data();
    let parsed = Frame::parse(*data).expect("should parse");
    assert_eq!(built, parsed);
}

#[test]
fn build_and_parse_eeprom_write() {
    let built = Frame::eeprom_write_request(EepromRegister::MeasureMode, 0xAA);
    let data = built.raw_data();
    let parsed = Frame::parse(*data).expect("should parse");
    assert_eq!(built, parsed);
}

#[test]
fn frame_equality() {
    let f1 = Frame::distance_request();
    let f2 = Frame::distance_request();
    assert_eq!(f1, f2);
}

#[test]
fn frame_clone() {
    let f1 = Frame::distance_request();
    let f2 = f1;
    assert_eq!(f1, f2);
}
