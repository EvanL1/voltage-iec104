//! IEC 60870-5-104 APCI (Application Protocol Control Information).
//!
//! APCI is the 6-byte header of an APDU, containing frame type and sequence numbers.

use crate::error::{Iec104Error, Result};

/// Start byte for IEC 104 frames.
pub const START_BYTE: u8 = 0x68;

/// Minimum APDU length (APCI only, no ASDU).
pub const MIN_APDU_LENGTH: usize = 4;

/// Maximum APDU length.
pub const MAX_APDU_LENGTH: usize = 253;

/// APCI frame type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    /// I-frame: Information transfer
    IFrame,
    /// S-frame: Supervisory (acknowledgment)
    SFrame,
    /// U-frame: Unnumbered (control)
    UFrame,
}

/// U-frame function codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UFunction {
    /// STARTDT act (Start Data Transfer activation)
    StartDtAct,
    /// STARTDT con (Start Data Transfer confirmation)
    StartDtCon,
    /// STOPDT act (Stop Data Transfer activation)
    StopDtAct,
    /// STOPDT con (Stop Data Transfer confirmation)
    StopDtCon,
    /// TESTFR act (Test Frame activation)
    TestFrAct,
    /// TESTFR con (Test Frame confirmation)
    TestFrCon,
}

impl UFunction {
    /// Get the control field byte for this U-function.
    #[inline]
    pub const fn control_byte(&self) -> u8 {
        match self {
            Self::StartDtAct => 0x07, // 0000 0111
            Self::StartDtCon => 0x0B, // 0000 1011
            Self::StopDtAct => 0x13,  // 0001 0011
            Self::StopDtCon => 0x23,  // 0010 0011
            Self::TestFrAct => 0x43,  // 0100 0011
            Self::TestFrCon => 0x83,  // 1000 0011
        }
    }

    /// Parse U-function from control byte.
    #[inline]
    pub fn from_control_byte(byte: u8) -> Result<Self> {
        match byte {
            0x07 => Ok(Self::StartDtAct),
            0x0B => Ok(Self::StartDtCon),
            0x13 => Ok(Self::StopDtAct),
            0x23 => Ok(Self::StopDtCon),
            0x43 => Ok(Self::TestFrAct),
            0x83 => Ok(Self::TestFrCon),
            // Use static error for common case; specific byte info is less important than perf
            _ => Err(Iec104Error::invalid_frame_static("Unknown U-frame function")),
        }
    }
}

/// APCI (Application Protocol Control Information).
///
/// The 6-byte header of an IEC 104 APDU.
///
/// ```text
/// +--------+--------+--------+--------+--------+--------+
/// | 0x68   | Length | CF1    | CF2    | CF3    | CF4    |
/// +--------+--------+--------+--------+--------+--------+
///   Start    APDU     Control Field (4 bytes)
///   Byte     Length
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Apci {
    /// I-frame with send and receive sequence numbers.
    IFrame {
        /// Send sequence number (0-32767)
        send_seq: u16,
        /// Receive sequence number (0-32767)
        recv_seq: u16,
    },
    /// S-frame with receive sequence number only.
    SFrame {
        /// Receive sequence number (0-32767)
        recv_seq: u16,
    },
    /// U-frame with function code.
    UFrame {
        /// U-frame function
        function: UFunction,
    },
}

impl Apci {
    /// Create a new I-frame APCI.
    #[inline]
    pub fn i_frame(send_seq: u16, recv_seq: u16) -> Self {
        Self::IFrame { send_seq, recv_seq }
    }

    /// Create a new S-frame APCI.
    #[inline]
    pub fn s_frame(recv_seq: u16) -> Self {
        Self::SFrame { recv_seq }
    }

    /// Create a new U-frame APCI.
    #[inline]
    pub fn u_frame(function: UFunction) -> Self {
        Self::UFrame { function }
    }

    /// Get the frame type.
    #[inline]
    pub fn frame_type(&self) -> FrameType {
        match self {
            Self::IFrame { .. } => FrameType::IFrame,
            Self::SFrame { .. } => FrameType::SFrame,
            Self::UFrame { .. } => FrameType::UFrame,
        }
    }

    /// Parse APCI from bytes (4 bytes of control field).
    ///
    /// Note: This expects 4 bytes of control field, not the full 6-byte APCI.
    #[inline]
    pub fn parse(control: &[u8]) -> Result<Self> {
        if control.len() < 4 {
            return Err(Iec104Error::invalid_frame_static("Control field too short"));
        }

        let cf1 = control[0];

        // Check frame type by LSB of first control byte
        if cf1 & 0x01 == 0 {
            // I-frame: bit 0 = 0
            let send_seq = ((control[1] as u16) << 7) | ((cf1 >> 1) as u16);
            let recv_seq = ((control[3] as u16) << 7) | ((control[2] >> 1) as u16);
            Ok(Self::IFrame { send_seq, recv_seq })
        } else if cf1 & 0x03 == 0x01 {
            // S-frame: bits 0-1 = 01
            let recv_seq = ((control[3] as u16) << 7) | ((control[2] >> 1) as u16);
            Ok(Self::SFrame { recv_seq })
        } else if cf1 & 0x03 == 0x03 {
            // U-frame: bits 0-1 = 11
            let function = UFunction::from_control_byte(cf1)?;
            Ok(Self::UFrame { function })
        } else {
            Err(Iec104Error::invalid_frame_static("Invalid control field"))
        }
    }

    /// Encode APCI to 4 bytes of control field.
    #[inline]
    pub fn encode(&self) -> [u8; 4] {
        match self {
            Self::IFrame { send_seq, recv_seq } => {
                let cf1 = ((send_seq & 0x7F) << 1) as u8;
                let cf2 = (send_seq >> 7) as u8;
                let cf3 = ((recv_seq & 0x7F) << 1) as u8;
                let cf4 = (recv_seq >> 7) as u8;
                [cf1, cf2, cf3, cf4]
            }
            Self::SFrame { recv_seq } => {
                let cf3 = ((recv_seq & 0x7F) << 1) as u8;
                let cf4 = (recv_seq >> 7) as u8;
                [0x01, 0x00, cf3, cf4]
            }
            Self::UFrame { function } => [function.control_byte(), 0x00, 0x00, 0x00],
        }
    }

    /// Encode full APDU header (6 bytes: start + length + control).
    ///
    /// `asdu_len` is the length of the ASDU that follows (0 for S-frame and U-frame).
    #[inline]
    pub fn encode_header(&self, asdu_len: usize) -> [u8; 6] {
        let control = self.encode();
        let apdu_len = (4 + asdu_len) as u8;
        [
            START_BYTE, apdu_len, control[0], control[1], control[2], control[3],
        ]
    }

    /// Check if this is an I-frame.
    #[inline]
    pub fn is_i_frame(&self) -> bool {
        matches!(self, Self::IFrame { .. })
    }

    /// Check if this is an S-frame.
    #[inline]
    pub fn is_s_frame(&self) -> bool {
        matches!(self, Self::SFrame { .. })
    }

    /// Check if this is a U-frame.
    #[inline]
    pub fn is_u_frame(&self) -> bool {
        matches!(self, Self::UFrame { .. })
    }

    /// Get the send sequence number (I-frame only).
    #[inline]
    pub fn send_seq(&self) -> Option<u16> {
        match self {
            Self::IFrame { send_seq, .. } => Some(*send_seq),
            _ => None,
        }
    }

    /// Get the receive sequence number (I-frame and S-frame).
    #[inline]
    pub fn recv_seq(&self) -> Option<u16> {
        match self {
            Self::IFrame { recv_seq, .. } | Self::SFrame { recv_seq } => Some(*recv_seq),
            _ => None,
        }
    }
}

impl std::fmt::Display for Apci {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IFrame { send_seq, recv_seq } => {
                write!(f, "I(S={}, R={})", send_seq, recv_seq)
            }
            Self::SFrame { recv_seq } => {
                write!(f, "S(R={})", recv_seq)
            }
            Self::UFrame { function } => {
                let name = match function {
                    UFunction::StartDtAct => "STARTDT act",
                    UFunction::StartDtCon => "STARTDT con",
                    UFunction::StopDtAct => "STOPDT act",
                    UFunction::StopDtCon => "STOPDT con",
                    UFunction::TestFrAct => "TESTFR act",
                    UFunction::TestFrCon => "TESTFR con",
                };
                write!(f, "U({})", name)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i_frame_encode_decode() {
        let apci = Apci::i_frame(100, 50);
        let encoded = apci.encode();
        let decoded = Apci::parse(&encoded).unwrap();

        assert_eq!(decoded, apci);
        assert_eq!(decoded.send_seq(), Some(100));
        assert_eq!(decoded.recv_seq(), Some(50));
    }

    #[test]
    fn test_s_frame_encode_decode() {
        let apci = Apci::s_frame(200);
        let encoded = apci.encode();
        let decoded = Apci::parse(&encoded).unwrap();

        assert_eq!(decoded, apci);
        assert_eq!(decoded.send_seq(), None);
        assert_eq!(decoded.recv_seq(), Some(200));
    }

    #[test]
    fn test_u_frame_encode_decode() {
        for func in [
            UFunction::StartDtAct,
            UFunction::StartDtCon,
            UFunction::StopDtAct,
            UFunction::StopDtCon,
            UFunction::TestFrAct,
            UFunction::TestFrCon,
        ] {
            let apci = Apci::u_frame(func);
            let encoded = apci.encode();
            let decoded = Apci::parse(&encoded).unwrap();
            assert_eq!(decoded, apci);
        }
    }

    #[test]
    fn test_frame_type() {
        assert_eq!(Apci::i_frame(0, 0).frame_type(), FrameType::IFrame);
        assert_eq!(Apci::s_frame(0).frame_type(), FrameType::SFrame);
        assert_eq!(
            Apci::u_frame(UFunction::StartDtAct).frame_type(),
            FrameType::UFrame
        );
    }

    #[test]
    fn test_apci_display() {
        assert_eq!(Apci::i_frame(10, 5).to_string(), "I(S=10, R=5)");
        assert_eq!(Apci::s_frame(20).to_string(), "S(R=20)");
        assert_eq!(
            Apci::u_frame(UFunction::StartDtAct).to_string(),
            "U(STARTDT act)"
        );
    }

    #[test]
    fn test_sequence_number_max() {
        // Max sequence number is 32767 (15 bits)
        let apci = Apci::i_frame(32767, 32767);
        let encoded = apci.encode();
        let decoded = Apci::parse(&encoded).unwrap();
        assert_eq!(decoded.send_seq(), Some(32767));
        assert_eq!(decoded.recv_seq(), Some(32767));
    }

    // ============ Additional APCI Tests ============

    #[test]
    fn test_u_function_control_bytes() {
        // Verify all U-function control bytes
        assert_eq!(UFunction::StartDtAct.control_byte(), 0x07);
        assert_eq!(UFunction::StartDtCon.control_byte(), 0x0B);
        assert_eq!(UFunction::StopDtAct.control_byte(), 0x13);
        assert_eq!(UFunction::StopDtCon.control_byte(), 0x23);
        assert_eq!(UFunction::TestFrAct.control_byte(), 0x43);
        assert_eq!(UFunction::TestFrCon.control_byte(), 0x83);
    }

    #[test]
    fn test_u_function_from_invalid_byte() {
        // Invalid U-function bytes should return error
        let invalid_bytes = [0x00, 0x01, 0x02, 0x03, 0x04, 0x10, 0xFF];
        for byte in invalid_bytes {
            let result = UFunction::from_control_byte(byte);
            assert!(result.is_err(), "Expected error for byte 0x{:02X}", byte);
        }
    }

    #[test]
    fn test_apci_parse_too_short() {
        // Control field less than 4 bytes
        let short_data = [0x00, 0x00, 0x00];
        let result = Apci::parse(&short_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_i_frame_bit_layout() {
        // I-frame: LSB of first byte is 0
        let apci = Apci::i_frame(0, 0);
        let encoded = apci.encode();
        assert_eq!(encoded[0] & 0x01, 0, "I-frame LSB should be 0");
    }

    #[test]
    fn test_s_frame_bit_layout() {
        // S-frame: bits 0-1 of first byte are 01
        let apci = Apci::s_frame(0);
        let encoded = apci.encode();
        assert_eq!(encoded[0] & 0x03, 0x01, "S-frame bits 0-1 should be 01");
    }

    #[test]
    fn test_u_frame_bit_layout() {
        // U-frame: bits 0-1 of first byte are 11
        let apci = Apci::u_frame(UFunction::StartDtAct);
        let encoded = apci.encode();
        assert_eq!(encoded[0] & 0x03, 0x03, "U-frame bits 0-1 should be 11");
    }

    #[test]
    fn test_encode_header_length() {
        // Test that encode_header calculates length correctly
        let apci = Apci::u_frame(UFunction::StartDtAct);
        let header = apci.encode_header(0);
        assert_eq!(header[0], START_BYTE);
        assert_eq!(header[1], 4); // APCI only, no ASDU

        let apci = Apci::i_frame(0, 0);
        let header = apci.encode_header(10); // 10 bytes ASDU
        assert_eq!(header[1], 14); // 4 + 10

        let header = apci.encode_header(100); // 100 bytes ASDU
        assert_eq!(header[1], 104); // 4 + 100
    }

    #[test]
    fn test_sequence_number_edge_cases() {
        // Test sequence numbers at various points
        let test_values = [0, 1, 127, 128, 255, 256, 1000, 16383, 16384, 32766, 32767];

        for val in test_values {
            let apci = Apci::i_frame(val, val);
            let encoded = apci.encode();
            let decoded = Apci::parse(&encoded).unwrap();
            assert_eq!(decoded.send_seq(), Some(val), "Failed for value {}", val);
            assert_eq!(decoded.recv_seq(), Some(val), "Failed for value {}", val);
        }
    }

    #[test]
    fn test_asymmetric_sequence_numbers() {
        // Test various combinations of send/recv seq
        let test_cases = [
            (0, 32767),
            (32767, 0),
            (1, 2),
            (100, 200),
            (12345, 23456),
        ];

        for (send, recv) in test_cases {
            let apci = Apci::i_frame(send, recv);
            let encoded = apci.encode();
            let decoded = Apci::parse(&encoded).unwrap();
            assert_eq!(decoded.send_seq(), Some(send));
            assert_eq!(decoded.recv_seq(), Some(recv));
        }
    }

    #[test]
    fn test_apci_u_frame_no_seq_numbers() {
        // U-frames should not have sequence numbers
        let apci = Apci::u_frame(UFunction::TestFrAct);
        assert_eq!(apci.send_seq(), None);
        assert_eq!(apci.recv_seq(), None);
    }

    #[test]
    fn test_apci_s_frame_no_send_seq() {
        // S-frames should only have recv_seq
        let apci = Apci::s_frame(100);
        assert_eq!(apci.send_seq(), None);
        assert_eq!(apci.recv_seq(), Some(100));
    }

    #[test]
    fn test_frame_type_consistency() {
        // Verify frame_type matches is_* methods
        let i_apci = Apci::i_frame(0, 0);
        assert_eq!(i_apci.frame_type(), FrameType::IFrame);
        assert!(i_apci.is_i_frame());
        assert!(!i_apci.is_s_frame());
        assert!(!i_apci.is_u_frame());

        let s_apci = Apci::s_frame(0);
        assert_eq!(s_apci.frame_type(), FrameType::SFrame);
        assert!(!s_apci.is_i_frame());
        assert!(s_apci.is_s_frame());
        assert!(!s_apci.is_u_frame());

        let u_apci = Apci::u_frame(UFunction::StartDtAct);
        assert_eq!(u_apci.frame_type(), FrameType::UFrame);
        assert!(!u_apci.is_i_frame());
        assert!(!u_apci.is_s_frame());
        assert!(u_apci.is_u_frame());
    }

    #[test]
    fn test_constants() {
        assert_eq!(START_BYTE, 0x68);
        assert_eq!(MIN_APDU_LENGTH, 4);
        assert_eq!(MAX_APDU_LENGTH, 253);
    }
}
