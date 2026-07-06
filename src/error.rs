/// All possible errors returned by the URM37 driver.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<E> {
    /// Bus communication error (UART, etc.) — wraps the underlying HAL error.
    Bus(E),

    /// The received checksum does not match the computed one.
    ChecksumMismatch {
        /// The checksum value that was expected.
        expected: u8,
        /// The checksum value that was actually received.
        got: u8,
    },

    /// The response received does not match the command that was sent.
    UnexpectedResponse,

    /// The sensor returned an invalid reading (e.g. out of range).
    InvalidReading,

    /// Timeout: no response received within the expected window (passive UART mode).
    Timeout,
}

impl<E> From<E> for Error<E> {
    fn from(e: E) -> Self {
        Error::Bus(e)
    }
}
