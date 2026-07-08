//! Error types for URM37 sensor operations.
//!
//! Errors can arise from two sources:
//! 1. **Transport errors** (wrapped from the underlying HAL)
//! 2. **Protocol errors** (invalid checksums, timeouts, out-of-range readings)

/// All possible errors returned by the URM37 driver.
///
/// This is a generic error type parameterized over the underlying transport error `E`.
/// This allows the driver to be agnostic to the specific UART or communication layer
/// without sacrificing error type information.
///
/// # Example
/// ```ignore
/// use urm37::Error;
///
/// match sensor.read_distance() {
///     Ok(cm) => println!("Distance: {} cm", cm),
///     Err(Error::Bus(e)) => eprintln!("UART error: {:?}", e),
///     Err(Error::Timeout) => eprintln!("No response from sensor"),
///     Err(Error::InvalidReading) => eprintln!("Sensor out of range"),
///     _ => eprintln!("Other error"),
/// }
/// ```
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<E> {
    /// Bus communication error (UART, SPI, etc.) — wraps the underlying HAL error.
    ///
    /// This occurs when the transport layer (UART read/write) fails.
    /// The wrapped error `E` is the HAL-specific error type.
    Bus(E),

    /// The received checksum does not match the computed one.
    ///
    /// This indicates data corruption or a communication error.
    /// Contains both the expected and received checksum for debugging.
    ChecksumMismatch {
        /// The checksum value that was expected based on the received data.
        expected: u8,
        /// The checksum value that was actually received in the frame.
        got: u8,
    },

    /// The response received does not match the command that was sent.
    ///
    /// This can occur if:
    /// - A response from a previous command is read
    /// - The sensor is in an unexpected state
    /// - Multiple sensors on the same bus are interfering
    UnexpectedResponse,

    /// The sensor returned an invalid reading (e.g. out of range or no echo detected).
    ///
    /// The sensor uses the sentinel value `0xFFFF` to indicate an invalid reading.
    /// This typically occurs when:
    /// - No object is detected within the sensor's range
    /// - The object is too close or too far
    /// - Environmental conditions prevent accurate measurement
    InvalidReading,

    /// Timeout: no response received within the expected window (passive UART mode).
    ///
    /// In passive mode, the sensor only responds to explicit commands.
    /// If no response arrives, this error is returned.
    Timeout,
}

impl<E> From<E> for Error<E> {
    fn from(e: E) -> Self {
        Error::Bus(e)
    }
}
