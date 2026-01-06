//! Error types for IEC 60870-5-104 protocol.

use std::borrow::Cow;
use thiserror::Error;

/// Result type alias for IEC 104 operations.
pub type Result<T> = std::result::Result<T, Iec104Error>;

/// IEC 60870-5-104 protocol error types.
///
/// Uses `Cow<'static, str>` to avoid allocations for static error messages.
#[derive(Debug, Error)]
pub enum Iec104Error {
    /// Connection error
    #[error("Connection error: {0}")]
    Connection(Cow<'static, str>),

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
    Protocol(Cow<'static, str>),

    /// Invalid frame format
    #[error("Invalid frame: {0}")]
    InvalidFrame(Cow<'static, str>),

    /// Invalid ASDU
    #[error("Invalid ASDU: {0}")]
    InvalidAsdu(Cow<'static, str>),

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
    Codec(Cow<'static, str>),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(Cow<'static, str>),
}

impl Iec104Error {
    /// Create a protocol error with a static message (zero allocation).
    #[inline]
    pub const fn protocol_static(msg: &'static str) -> Self {
        Self::Protocol(Cow::Borrowed(msg))
    }

    /// Create a protocol error with a dynamic message.
    #[inline]
    pub fn protocol(msg: impl Into<String>) -> Self {
        Self::Protocol(Cow::Owned(msg.into()))
    }

    /// Create an invalid frame error with a static message (zero allocation).
    #[inline]
    pub const fn invalid_frame_static(msg: &'static str) -> Self {
        Self::InvalidFrame(Cow::Borrowed(msg))
    }

    /// Create an invalid frame error with a dynamic message.
    #[inline]
    pub fn invalid_frame(msg: impl Into<String>) -> Self {
        Self::InvalidFrame(Cow::Owned(msg.into()))
    }

    /// Create an invalid ASDU error with a static message (zero allocation).
    #[inline]
    pub const fn invalid_asdu_static(msg: &'static str) -> Self {
        Self::InvalidAsdu(Cow::Borrowed(msg))
    }

    /// Create an invalid ASDU error with a dynamic message.
    #[inline]
    pub fn invalid_asdu(msg: impl Into<String>) -> Self {
        Self::InvalidAsdu(Cow::Owned(msg.into()))
    }

    /// Check if this error indicates a connection problem.
    #[inline]
    pub fn is_connection_error(&self) -> bool {
        matches!(
            self,
            Self::Connection(_) | Self::NotConnected | Self::ConnectionTimeout | Self::T3Timeout
        )
    }

    /// Check if this error is retryable.
    #[inline]
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

    // ============ Additional Error Tests ============

    #[test]
    fn test_error_helper_constructors() {
        let err = Iec104Error::protocol("test protocol error");
        assert!(err.to_string().contains("test protocol error"));

        let err = Iec104Error::invalid_frame("bad frame");
        assert!(err.to_string().contains("bad frame"));

        let err = Iec104Error::invalid_asdu("bad asdu");
        assert!(err.to_string().contains("bad asdu"));
    }

    #[test]
    fn test_error_display_all_variants() {
        // Test all error variants have proper Display
        let errors = [
            Iec104Error::Connection(Cow::Borrowed("test")),
            Iec104Error::NotConnected,
            Iec104Error::ConnectionTimeout,
            Iec104Error::Protocol(Cow::Borrowed("test")),
            Iec104Error::InvalidFrame(Cow::Borrowed("test")),
            Iec104Error::InvalidAsdu(Cow::Borrowed("test")),
            Iec104Error::UnknownTypeId(255),
            Iec104Error::SequenceMismatch { expected: 10, actual: 20 },
            Iec104Error::T1Timeout,
            Iec104Error::T2Timeout,
            Iec104Error::T3Timeout,
            Iec104Error::TooManyUnconfirmed(100),
            Iec104Error::ChannelClosed,
            Iec104Error::Codec(Cow::Borrowed("test")),
            Iec104Error::Internal(Cow::Borrowed("test")),
        ];

        for err in errors {
            let display = err.to_string();
            assert!(!display.is_empty(), "Display for {:?} should not be empty", err);
        }
    }

    #[test]
    fn test_connection_error_variants() {
        assert!(Iec104Error::Connection(Cow::Borrowed("addr")).is_connection_error());
        assert!(Iec104Error::NotConnected.is_connection_error());
        assert!(Iec104Error::ConnectionTimeout.is_connection_error());
        assert!(Iec104Error::T3Timeout.is_connection_error());

        // Non-connection errors
        assert!(!Iec104Error::T1Timeout.is_connection_error());
        assert!(!Iec104Error::T2Timeout.is_connection_error());
        assert!(!Iec104Error::protocol_static("test").is_connection_error());
        assert!(!Iec104Error::invalid_frame_static("test").is_connection_error());
        assert!(!Iec104Error::ChannelClosed.is_connection_error());
    }

    #[test]
    fn test_retryable_error_variants() {
        assert!(Iec104Error::ConnectionTimeout.is_retryable());
        assert!(Iec104Error::T1Timeout.is_retryable());
        assert!(Iec104Error::T2Timeout.is_retryable());
        assert!(Iec104Error::T3Timeout.is_retryable());

        // Non-retryable errors
        assert!(!Iec104Error::NotConnected.is_retryable());
        assert!(!Iec104Error::Connection(Cow::Borrowed("test")).is_retryable());
        assert!(!Iec104Error::protocol_static("test").is_retryable());
        assert!(!Iec104Error::invalid_frame_static("test").is_retryable());
        assert!(!Iec104Error::invalid_asdu_static("test").is_retryable());
        assert!(!Iec104Error::UnknownTypeId(1).is_retryable());
        assert!(!Iec104Error::SequenceMismatch { expected: 1, actual: 2 }.is_retryable());
        assert!(!Iec104Error::TooManyUnconfirmed(10).is_retryable());
        assert!(!Iec104Error::ChannelClosed.is_retryable());
        assert!(!Iec104Error::Codec(Cow::Borrowed("test")).is_retryable());
        assert!(!Iec104Error::Internal(Cow::Borrowed("test")).is_retryable());
    }

    #[test]
    fn test_io_error_conversion() {
        use std::io::{Error as IoError, ErrorKind};
        let io_err = IoError::new(ErrorKind::ConnectionRefused, "connection refused");
        let iec_err: Iec104Error = io_err.into();

        if let Iec104Error::Io(e) = iec_err {
            assert_eq!(e.kind(), ErrorKind::ConnectionRefused);
        } else {
            panic!("Expected Io variant");
        }
    }

    #[test]
    fn test_sequence_mismatch_display() {
        let err = Iec104Error::SequenceMismatch {
            expected: 100,
            actual: 50,
        };
        let display = err.to_string();
        assert!(display.contains("100"));
        assert!(display.contains("50"));
    }

    #[test]
    fn test_too_many_unconfirmed_display() {
        let err = Iec104Error::TooManyUnconfirmed(12);
        let display = err.to_string();
        assert!(display.contains("12"));
    }

    #[test]
    fn test_unknown_type_id_display() {
        let err = Iec104Error::UnknownTypeId(99);
        let display = err.to_string();
        assert!(display.contains("99"));
    }

    #[test]
    fn test_error_debug() {
        // Ensure Debug is implemented
        let err = Iec104Error::NotConnected;
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotConnected"));
    }
}
