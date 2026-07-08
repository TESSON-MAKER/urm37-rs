use urm37::protocol::{decode_threshold, encode_threshold};

#[test]
fn encode_threshold_zero() {
    let (high, low) = encode_threshold(0);
    assert_eq!(high, 0x00);
    assert_eq!(low, 0x00);
}

#[test]
fn encode_threshold_256() {
    let (high, low) = encode_threshold(256);
    assert_eq!(high, 0x01);
    assert_eq!(low, 0x00);
}

#[test]
fn encode_threshold_100() {
    let (high, low) = encode_threshold(100);
    assert_eq!(high, 0x00);
    assert_eq!(low, 0x64);
}

#[test]
fn encode_threshold_max() {
    let (high, low) = encode_threshold(0xFFFF);
    assert_eq!(high, 0xFF);
    assert_eq!(low, 0xFF);
}

#[test]
fn decode_threshold_zero() {
    let value = decode_threshold(0x00, 0x00);
    assert_eq!(value, 0);
}

#[test]
fn decode_threshold_256() {
    let value = decode_threshold(0x01, 0x00);
    assert_eq!(value, 256);
}

#[test]
fn decode_threshold_100() {
    let value = decode_threshold(0x00, 0x64);
    assert_eq!(value, 100);
}

#[test]
fn decode_threshold_max() {
    let value = decode_threshold(0xFF, 0xFF);
    assert_eq!(value, 0xFFFF);
}

#[test]
fn roundtrip_encode_decode() {
    let original: u16 = 500;
    let (high, low) = encode_threshold(original);
    let decoded = decode_threshold(high, low);
    assert_eq!(decoded, original);
}

#[test]
fn roundtrip_multiple_values() {
    for value in [0, 1, 100, 256, 500, 800, 1000, 0xFFFF] {
        let (high, low) = encode_threshold(value);
        let decoded = decode_threshold(high, low);
        assert_eq!(decoded, value, "roundtrip failed for value {}", value);
    }
}
