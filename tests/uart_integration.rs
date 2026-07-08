//! Integration tests for UART drivers with a mock transport.
//!
//! This module tests the complete UART driver flow:
//! 1. Building command frames
//! 2. Sending frames to a mock UART
//! 3. Receiving and parsing response frames
//! 4. Extracting sensor data (distance, temperature)

use urm37::protocol::{Frame, Command, EepromRegister};
use urm37::error::Error;
use std::cell::RefCell;
use embedded_io::{ErrorType, Read, ReadExactError};

// ===== Mock UART implementation =====

/// A simple error type for the mock UART.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockError {
    /// End of file (no more data in read buffer)
    Eof,
}

impl std::fmt::Display for MockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MockError::Eof => write!(f, "end of file"),
        }
    }
}

impl std::error::Error for MockError {}

impl embedded_io::Error for MockError {
    fn kind(&self) -> embedded_io::ErrorKind {
        match self {
            MockError::Eof => embedded_io::ErrorKind::Other,
        }
    }
}

/// A mock UART that stores data in buffers for testing.
pub struct MockUart {
    /// Data to return on read operations
    read_buffer: RefCell<std::vec::Vec<u8>>,
    /// Data written by the driver
    write_buffer: RefCell<std::vec::Vec<u8>>,
}

impl MockUart {
    /// Create a new mock UART with predefined response data.
    pub fn new(responses: std::vec::Vec<u8>) -> Self {
        Self {
            read_buffer: RefCell::new(responses),
            write_buffer: RefCell::new(std::vec::Vec::new()),
        }
    }

    /// Get the data that was written to this mock UART.
    pub fn written_data(&self) -> std::vec::Vec<u8> {
        self.write_buffer.borrow().clone()
    }
}

impl ErrorType for MockUart {
    type Error = MockError;
}

impl embedded_io::Read for MockUart {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut buffer = self.read_buffer.borrow_mut();
        if buffer.is_empty() {
            return Err(MockError::Eof);
        }

        let len = std::cmp::min(buf.len(), buffer.len());
        buf[..len].copy_from_slice(&buffer[..len]);
        buffer.drain(..len);
        Ok(len)
    }
}

impl embedded_io::Write for MockUart {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.write_buffer.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}


// ===== Protocol-level tests (no driver needed) =====

#[test]
fn frame_distance_request_roundtrip() {
    // Build a distance request
    let frame = Frame::distance_request();
    let data = frame.raw_data();

    // Should be [0x22, 0x00, 0x00, 0x22]
    assert_eq!(data, &[0x22, 0x00, 0x00, 0x22]);

    // Parse it back
    let parsed = Frame::parse(*data).expect("should parse");
    assert_eq!(parsed, frame);
}

#[test]
fn frame_distance_response_decode() {
    // Response: 300 cm = 0x012C
    let response = [0x22, 0x01, 0x2C, 0x4F];

    // Parse the response
    let frame = Frame::parse(response).expect("should parse");

    // Decode the distance
    let distance = frame.decode_distance().expect("should decode");
    assert_eq!(distance, 300);
}

#[test]
fn frame_temperature_response_decode() {
    // Response: 25.5°C
    let response = [0x11, 0x00, 0xFF, 0x10];

    // Parse the response
    let frame = Frame::parse(response).expect("should parse");

    // Decode the temperature
    let temp = frame.decode_temperature().expect("should decode");
    assert_eq!(temp, 25.5);
}

// ===== Mock UART driver tests (requires embedded-io trait adaptation) =====

#[cfg(feature = "uart")]
mod uart_frame_tests {
    use super::*;

    #[test]
    fn mock_uart_write_and_read() {
        let mut uart = MockUart::new(vec![0x22, 0x01, 0x2C, 0x4F]);

        // Write a command
        let cmd = [0x22, 0x00, 0x00, 0x22];
        embedded_io::Write::write(&mut uart, &cmd).expect("should write");

        // Read response using our extension trait
        let mut buf = [0u8; 4];
        uart.read_exact(&mut buf).expect("should read exact");

        // Verify
        assert_eq!(buf, [0x22, 0x01, 0x2C, 0x4F]);
        assert_eq!(uart.written_data(), vec![0x22, 0x00, 0x00, 0x22]);
    }

    #[test]
    fn mock_uart_multiple_frames() {
        // Prepare responses for two distance reads
        let responses = vec![
            0x22, 0x00, 0x64, 0x86,  // 100 cm
            0x22, 0x01, 0x2C, 0x4F,  // 300 cm
        ];
        let mut uart = MockUart::new(responses);

        // First read
        let mut buf1 = [0u8; 4];
        uart.read_exact(&mut buf1).expect("should read first frame");
        let frame1 = Frame::parse(buf1).expect("should parse");
        assert_eq!(frame1.decode_distance(), Some(100));

        // Second read
        let mut buf2 = [0u8; 4];
        uart.read_exact(&mut buf2).expect("should read second frame");
        let frame2 = Frame::parse(buf2).expect("should parse");
        assert_eq!(frame2.decode_distance(), Some(300));
    }

    #[test]
    fn mock_uart_partial_read() {
        let mut uart = MockUart::new(vec![0x22, 0x01, 0x2C]);  // Only 3 bytes

        // First read should get 3 bytes
        let mut buf = [0u8; 4];
        let n = embedded_io::Read::read(&mut uart, &mut buf[..3])
            .expect("should read partial frame");
        assert_eq!(n, 3);
        assert_eq!(&buf[..3], &[0x22, 0x01, 0x2C]);
    }
}
