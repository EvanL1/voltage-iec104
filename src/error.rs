//! Error types for IEC 60870-5-104 protocol.

use thiserror::Error;

/// Result type alias for IEC 104 operations.
pub type Result<T> = std::result::Result<T, Iec104Error>;

/// IEC 60870-5-104 protocol error types.
#[derive(Debug, Error)]
pub enum Iec104Error {
    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Not connected to remote
    #[error("Not connected")]
    NotConnected,

    /// Connection timeout
    #[error("Connection timeout")]
    ConnectionTimeout,

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Invalid frame format
    #[error("Invalid frame: {0}")]
    InvalidFrame(String),

    /// Invalid ASDU
    #[error("Invalid ASDU: {0}")]
    InvalidAsdu(String),

    /// Unknown type identifier
    #[error("Unknown type ID: {0}")]
    UnknownTypeId(u8),

    /// Sequence number mismatch
    #[error("Sequence number mismatch: expected {expected}, got {actual}")]
    SequenceMismatch { expected: u16, actual: u16 },

    /// T1 timeout (send confirmation)
    #[error("T1 timeout: no confirmation received")]
    T1Timeout,

    /// T2 timeout (no data acknowledgment)
    #[error("T2 timeout: acknowledgment timeout")]
    T2Timeout,

    /// T3 timeout (test frame)
    #[error("T3 timeout: connection test failed")]
    T3Timeout,

    /// Too many unconfirmed frames
    #[error("Too many unconfirmed frames (K={0})")]
    TooManyUnconfirmed(u16),

    /// Channel closed
    #[error("Channel closed")]
    ChannelClosed,

    /// Codec error
    #[error("Codec error: {0}")]
    Codec(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Iec104Error {
    /// Create a protocol error with a message.
    pub fn protocol(msg: impl Into<String>) -> Self {
        Self::Protocol(msg.into())
    }

    /// Create an invalid frame error.
    pub fn invalid_frame(msg: impl Into<String>) -> Self {
        Self::InvalidFrame(msg.into())
    }

    /// Create an invalid ASDU error.
    pub fn invalid_asdu(msg: impl Into<String>) -> Self {
        Self::InvalidAsdu(msg.into())
    }

    /// Check if this error indicates a connection problem.
    pub fn is_connection_error(&self) -> bool {
        matches!(
            self,
            Self::Connection(_)
                | Self::NotConnected
                | Self::ConnectionTimeout
                | Self::T3Timeout
        )
    }

    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::ConnectionTimeout | Self::T1Timeout | Self::T2Timeout | Self::T3Timeout
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Iec104Error::NotConnected;
        assert_eq!(err.to_string(), "Not connected");

        let err = Iec104Error::UnknownTypeId(255);
        assert_eq!(err.to_string(), "Unknown type ID: 255");

        let err = Iec104Error::SequenceMismatch {
            expected: 10,
            actual: 5,
        };
        assert_eq!(
            err.to_string(),
            "Sequence number mismatch: expected 10, got 5"
        );
    }

    #[test]
    fn test_is_connection_error() {
        assert!(Iec104Error::NotConnected.is_connection_error());
        assert!(Iec104Error::ConnectionTimeout.is_connection_error());
        assert!(Iec104Error::T3Timeout.is_connection_error());
        assert!(!Iec104Error::T1Timeout.is_connection_error());
    }

    #[test]
    fn test_is_retryable() {
        assert!(Iec104Error::ConnectionTimeout.is_retryable());
        assert!(Iec104Error::T1Timeout.is_retryable());
        assert!(!Iec104Error::NotConnected.is_retryable());
    }
}
