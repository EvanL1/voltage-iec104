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
    pub fn control_byte(&self) -> u8 {
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
    pub fn from_control_byte(byte: u8) -> Result<Self> {
        match byte {
            0x07 => Ok(Self::StartDtAct),
            0x0B => Ok(Self::StartDtCon),
            0x13 => Ok(Self::StopDtAct),
            0x23 => Ok(Self::StopDtCon),
            0x43 => Ok(Self::TestFrAct),
            0x83 => Ok(Self::TestFrCon),
            _ => Err(Iec104Error::invalid_frame(format!(
                "Unknown U-frame function: 0x{:02X}",
                byte
            ))),
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
    pub fn i_frame(send_seq: u16, recv_seq: u16) -> Self {
        Self::IFrame { send_seq, recv_seq }
    }

    /// Create a new S-frame APCI.
    pub fn s_frame(recv_seq: u16) -> Self {
        Self::SFrame { recv_seq }
    }

    /// Create a new U-frame APCI.
    pub fn u_frame(function: UFunction) -> Self {
        Self::UFrame { function }
    }

    /// Get the frame type.
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
    pub fn parse(control: &[u8]) -> Result<Self> {
        if control.len() < 4 {
            return Err(Iec104Error::invalid_frame("Control field too short"));
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
            Err(Iec104Error::invalid_frame(format!(
                "Invalid control field: 0x{:02X}",
                cf1
            )))
        }
    }

    /// Encode APCI to 4 bytes of control field.
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
    pub fn encode_header(&self, asdu_len: usize) -> [u8; 6] {
        let control = self.encode();
        let apdu_len = (4 + asdu_len) as u8;
        [START_BYTE, apdu_len, control[0], control[1], control[2], control[3]]
    }

    /// Check if this is an I-frame.
    pub fn is_i_frame(&self) -> bool {
        matches!(self, Self::IFrame { .. })
    }

    /// Check if this is an S-frame.
    pub fn is_s_frame(&self) -> bool {
        matches!(self, Self::SFrame { .. })
    }

    /// Check if this is a U-frame.
    pub fn is_u_frame(&self) -> bool {
        matches!(self, Self::UFrame { .. })
    }

    /// Get the send sequence number (I-frame only).
    pub fn send_seq(&self) -> Option<u16> {
        match self {
            Self::IFrame { send_seq, .. } => Some(*send_seq),
            _ => None,
        }
    }

    /// Get the receive sequence number (I-frame and S-frame).
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
}
