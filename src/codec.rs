//! IEC 60870-5-104 codec for tokio.
//!
//! This module provides a codec implementation for encoding and decoding
//! IEC 104 APDUs using the tokio-util codec framework.

use bytes::{Buf, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use crate::error::Iec104Error;
use crate::types::{Apci, Asdu, MAX_APDU_LENGTH, MIN_APDU_LENGTH, START_BYTE};

/// An IEC 104 APDU (Application Protocol Data Unit).
///
/// Contains the APCI header and optionally an ASDU (for I-frames).
#[derive(Debug, Clone, PartialEq)]
pub struct Apdu {
    /// APCI (Application Protocol Control Information)
    pub apci: Apci,
    /// ASDU (Application Service Data Unit) - only present in I-frames
    pub asdu: Option<Asdu>,
}

impl Apdu {
    /// Create a new I-frame APDU with ASDU.
    pub fn i_frame(send_seq: u16, recv_seq: u16, asdu: Asdu) -> Self {
        Self {
            apci: Apci::i_frame(send_seq, recv_seq),
            asdu: Some(asdu),
        }
    }

    /// Create a new S-frame APDU.
    pub fn s_frame(recv_seq: u16) -> Self {
        Self {
            apci: Apci::s_frame(recv_seq),
            asdu: None,
        }
    }

    /// Create a new U-frame APDU.
    pub fn u_frame(function: crate::types::UFunction) -> Self {
        Self {
            apci: Apci::u_frame(function),
            asdu: None,
        }
    }

    /// Check if this is an I-frame.
    pub fn is_i_frame(&self) -> bool {
        self.apci.is_i_frame()
    }

    /// Check if this is an S-frame.
    pub fn is_s_frame(&self) -> bool {
        self.apci.is_s_frame()
    }

    /// Check if this is a U-frame.
    pub fn is_u_frame(&self) -> bool {
        self.apci.is_u_frame()
    }
}

impl std::fmt::Display for Apdu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.apci)?;
        if let Some(asdu) = &self.asdu {
            write!(
                f,
                " [{}] COT={} CA={}",
                asdu.header.type_id, asdu.header.cot, asdu.header.common_address
            )?;
        }
        Ok(())
    }
}

/// IEC 60870-5-104 codec.
///
/// This codec handles framing and parsing of IEC 104 APDUs.
///
/// # Example
///
/// ```rust,ignore
/// use tokio_util::codec::Framed;
/// use voltage_iec104::codec::Iec104Codec;
///
/// let stream = TcpStream::connect("192.168.1.100:2404").await?;
/// let mut framed = Framed::new(stream, Iec104Codec::new());
///
/// // Send U-frame
/// framed.send(Apdu::u_frame(UFunction::StartDtAct)).await?;
///
/// // Receive response
/// while let Some(apdu) = framed.next().await {
///     println!("Received: {:?}", apdu?);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct Iec104Codec {
    // State for handling partial frames
    state: DecodeState,
}

#[derive(Debug, Clone, Default)]
#[allow(clippy::enum_variant_names)]
enum DecodeState {
    #[default]
    WaitingForStart,
    WaitingForLength,
    WaitingForData {
        length: usize,
    },
}

impl Iec104Codec {
    /// Create a new IEC 104 codec.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Decoder for Iec104Codec {
    type Item = Apdu;
    type Error = Iec104Error;

    fn decode(
        &mut self,
        src: &mut BytesMut,
    ) -> std::result::Result<Option<Self::Item>, Self::Error> {
        loop {
            match &self.state {
                DecodeState::WaitingForStart => {
                    // Find start byte
                    if src.is_empty() {
                        return Ok(None);
                    }

                    if src[0] != START_BYTE {
                        // Skip bytes until we find the start byte (fast-path: advance once)
                        let start_pos = src.iter().position(|&b| b == START_BYTE);
                        match start_pos {
                            Some(pos) => src.advance(pos),
                            None => {
                                src.clear();
                                return Ok(None);
                            }
                        }
                    }

                    self.state = DecodeState::WaitingForLength;
                }

                DecodeState::WaitingForLength => {
                    // Need at least 2 bytes (start + length)
                    if src.len() < 2 {
                        return Ok(None);
                    }

                    let length = src[1] as usize;

                    // Validate length
                    if length < MIN_APDU_LENGTH {
                        // Invalid length, skip start byte and restart
                        src.advance(1);
                        self.state = DecodeState::WaitingForStart;
                        continue;
                    }

                    if length > MAX_APDU_LENGTH {
                        // Invalid length, skip start byte and restart
                        src.advance(1);
                        self.state = DecodeState::WaitingForStart;
                        continue;
                    }

                    self.state = DecodeState::WaitingForData { length };
                }

                DecodeState::WaitingForData { length } => {
                    let total_length = 2 + length; // start + length byte + APDU content

                    if src.len() < total_length {
                        return Ok(None);
                    }

                    // We have a complete frame
                    let frame = src.split_to(total_length).freeze();
                    self.state = DecodeState::WaitingForStart;

                    // Parse the frame
                    // Frame structure: [0x68] [length] [control1] [control2] [control3] [control4] [ASDU...]
                    let control = &frame[2..6];
                    let apci = Apci::parse(control)?;

                    let asdu = if apci.is_i_frame() && frame.len() > 6 {
                        Some(Asdu::parse_bytes(frame.slice(6..))?)
                    } else {
                        None
                    };

                    return Ok(Some(Apdu { apci, asdu }));
                }
            }
        }
    }
}

impl Encoder<Apdu> for Iec104Codec {
    type Error = Iec104Error;

    fn encode(&mut self, item: Apdu, dst: &mut BytesMut) -> std::result::Result<(), Self::Error> {
        // Calculate ASDU length without encoding yet
        let asdu_len = item.asdu.as_ref().map(|a| a.encoded_len()).unwrap_or(0);

        // Validate total length
        if asdu_len > MAX_APDU_LENGTH - 4 {
            return Err(Iec104Error::Codec(std::borrow::Cow::Borrowed("ASDU too large")));
        }

        // Reserve capacity for the entire frame
        dst.reserve(6 + asdu_len);

        // Write header
        let header = item.apci.encode_header(asdu_len);
        dst.extend_from_slice(&header);

        // Write ASDU directly to dst if present (zero-copy)
        if let Some(asdu) = &item.asdu {
            asdu.encode_to(dst);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AsduHeader, Cot, TypeId, UFunction};

    #[test]
    fn test_decode_u_frame() {
        let mut codec = Iec104Codec::new();
        let mut buf = BytesMut::from(&[0x68, 0x04, 0x07, 0x00, 0x00, 0x00][..]);

        let apdu = codec.decode(&mut buf).unwrap().unwrap();
        assert!(apdu.is_u_frame());

        if let Apci::UFrame { function } = apdu.apci {
            assert_eq!(function, UFunction::StartDtAct);
        } else {
            panic!("Expected U-frame");
        }
    }

    #[test]
    fn test_decode_s_frame() {
        let mut codec = Iec104Codec::new();
        // S-frame with recv_seq = 100
        let mut buf = BytesMut::from(&[0x68, 0x04, 0x01, 0x00, 0xC8, 0x00][..]);

        let apdu = codec.decode(&mut buf).unwrap().unwrap();
        assert!(apdu.is_s_frame());
        assert_eq!(apdu.apci.recv_seq(), Some(100));
    }

    #[test]
    fn test_encode_u_frame() {
        let mut codec = Iec104Codec::new();
        let mut buf = BytesMut::new();

        let apdu = Apdu::u_frame(UFunction::StartDtAct);
        codec.encode(apdu, &mut buf).unwrap();

        assert_eq!(&buf[..], &[0x68, 0x04, 0x07, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_encode_s_frame() {
        let mut codec = Iec104Codec::new();
        let mut buf = BytesMut::new();

        let apdu = Apdu::s_frame(100);
        codec.encode(apdu, &mut buf).unwrap();

        assert_eq!(&buf[..], &[0x68, 0x04, 0x01, 0x00, 0xC8, 0x00]);
    }

    #[test]
    fn test_encode_i_frame() {
        let mut codec = Iec104Codec::new();
        let mut buf = BytesMut::new();

        let asdu = Asdu::new(AsduHeader::new(
            TypeId::InterrogationCommand,
            1,
            Cot::Activation,
            1,
        ));
        let apdu = Apdu::i_frame(10, 5, asdu);
        codec.encode(apdu, &mut buf).unwrap();

        // Verify header structure
        assert_eq!(buf[0], START_BYTE);
        // Length should be 4 (control) + 6 (ASDU header) = 10
        assert_eq!(buf[1], 10);
    }

    #[test]
    fn test_decode_partial_frame() {
        let mut codec = Iec104Codec::new();

        // Send first part
        let mut buf = BytesMut::from(&[0x68, 0x04][..]);
        assert!(codec.decode(&mut buf).unwrap().is_none());

        // Send remaining part
        buf.extend_from_slice(&[0x07, 0x00, 0x00, 0x00]);
        let apdu = codec.decode(&mut buf).unwrap().unwrap();
        assert!(apdu.is_u_frame());
    }

    #[test]
    fn test_decode_skip_garbage() {
        let mut codec = Iec104Codec::new();
        // Garbage bytes before valid frame
        let mut buf = BytesMut::from(&[0xFF, 0xAA, 0x68, 0x04, 0x07, 0x00, 0x00, 0x00][..]);

        let apdu = codec.decode(&mut buf).unwrap().unwrap();
        assert!(apdu.is_u_frame());
    }

    #[test]
    fn test_roundtrip() {
        let mut codec = Iec104Codec::new();

        // Test U-frame roundtrip
        for func in [
            UFunction::StartDtAct,
            UFunction::StartDtCon,
            UFunction::StopDtAct,
            UFunction::StopDtCon,
            UFunction::TestFrAct,
            UFunction::TestFrCon,
        ] {
            let mut buf = BytesMut::new();
            codec.encode(Apdu::u_frame(func), &mut buf).unwrap();

            let decoded = codec.decode(&mut buf).unwrap().unwrap();
            assert_eq!(decoded.apci, Apci::u_frame(func));
        }

        // Test S-frame roundtrip
        for recv_seq in [0, 100, 32767] {
            let mut buf = BytesMut::new();
            codec.encode(Apdu::s_frame(recv_seq), &mut buf).unwrap();

            let decoded = codec.decode(&mut buf).unwrap().unwrap();
            assert_eq!(decoded.apci, Apci::s_frame(recv_seq));
        }
    }

    // ============ Additional Codec Tests ============

    #[test]
    fn test_decode_empty_buffer() {
        let mut codec = Iec104Codec::new();
        let mut buf = BytesMut::new();
        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_decode_invalid_length_too_small() {
        let mut codec = Iec104Codec::new();
        // Length byte = 3, which is less than MIN_APDU_LENGTH (4)
        let mut buf = BytesMut::from(&[0x68, 0x03, 0x00, 0x00, 0x00][..]);
        // Should skip this start byte and restart
        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_decode_invalid_length_too_large() {
        let mut codec = Iec104Codec::new();
        // Length byte = 254, which is greater than MAX_APDU_LENGTH (253)
        let mut buf = BytesMut::from(&[0x68, 0xFE, 0x00, 0x00, 0x00, 0x00][..]);
        // Should skip this start byte and restart
        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_decode_multiple_frames_in_buffer() {
        let mut codec = Iec104Codec::new();
        // Two U-frames in one buffer
        let mut buf = BytesMut::from(&[
            0x68, 0x04, 0x07, 0x00, 0x00, 0x00, // STARTDT act
            0x68, 0x04, 0x0B, 0x00, 0x00, 0x00, // STARTDT con
        ][..]);

        let apdu1 = codec.decode(&mut buf).unwrap().unwrap();
        assert!(apdu1.is_u_frame());
        if let crate::types::Apci::UFrame { function } = apdu1.apci {
            assert_eq!(function, UFunction::StartDtAct);
        }

        let apdu2 = codec.decode(&mut buf).unwrap().unwrap();
        assert!(apdu2.is_u_frame());
        if let crate::types::Apci::UFrame { function } = apdu2.apci {
            assert_eq!(function, UFunction::StartDtCon);
        }

        // Buffer should be empty now
        assert!(buf.is_empty());
    }

    #[test]
    fn test_decode_i_frame_with_asdu() {
        let mut codec = Iec104Codec::new();
        // I-frame with simple ASDU
        let mut buf = BytesMut::from(&[
            0x68, 0x0E, // Start + length (14 bytes)
            0x00, 0x00, 0x00, 0x00, // Control: I-frame S=0, R=0
            // ASDU header (6 bytes):
            0x64, // TypeId: InterrogationCommand (100)
            0x01, // VSQ: 1 object
            0x06, // COT: Activation
            0x00, // Originator
            0x01, 0x00, // Common address: 1
            // Information object (4 bytes):
            0x00, 0x00, 0x00, // IOA: 0
            0x14, // QOI: 20 (station interrogation)
        ][..]);

        let apdu = codec.decode(&mut buf).unwrap().unwrap();
        assert!(apdu.is_i_frame());
        assert!(apdu.asdu.is_some());

        let asdu = apdu.asdu.unwrap();
        assert_eq!(asdu.header.type_id, TypeId::InterrogationCommand);
        assert_eq!(asdu.header.cot, Cot::Activation);
        assert_eq!(asdu.header.common_address, 1);
    }

    #[test]
    fn test_decode_skip_multiple_garbage_bytes() {
        let mut codec = Iec104Codec::new();
        // Many garbage bytes before valid frame
        let mut buf = BytesMut::from(&[
            0xFF, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE,
            0x68, 0x04, 0x07, 0x00, 0x00, 0x00, // Valid U-frame
        ][..]);

        let apdu = codec.decode(&mut buf).unwrap().unwrap();
        assert!(apdu.is_u_frame());
    }

    #[test]
    fn test_decode_partial_then_complete() {
        let mut codec = Iec104Codec::new();

        // First: only start byte
        let mut buf = BytesMut::from(&[0x68][..]);
        assert!(codec.decode(&mut buf).unwrap().is_none());

        // Add length byte
        buf.extend_from_slice(&[0x04]);
        assert!(codec.decode(&mut buf).unwrap().is_none());

        // Add partial control field
        buf.extend_from_slice(&[0x07, 0x00]);
        assert!(codec.decode(&mut buf).unwrap().is_none());

        // Complete the frame
        buf.extend_from_slice(&[0x00, 0x00]);
        let apdu = codec.decode(&mut buf).unwrap().unwrap();
        assert!(apdu.is_u_frame());
    }

    #[test]
    fn test_encode_i_frame_with_asdu() {
        let mut codec = Iec104Codec::new();
        let mut buf = BytesMut::new();

        let asdu = Asdu::new(AsduHeader::new(
            TypeId::MeasuredFloat,
            2,
            Cot::Spontaneous,
            100,
        ));
        let apdu = Apdu::i_frame(50, 25, asdu);
        codec.encode(apdu, &mut buf).unwrap();

        // Verify structure
        assert_eq!(buf[0], START_BYTE);
        // Length should be 4 (control) + 6 (ASDU header) = 10
        assert_eq!(buf[1], 10);

        // Decode and verify
        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        assert!(decoded.is_i_frame());
        assert_eq!(decoded.apci.send_seq(), Some(50));
        assert_eq!(decoded.apci.recv_seq(), Some(25));
    }

    #[test]
    fn test_encode_decode_s_frame_sequence_numbers() {
        let mut codec = Iec104Codec::new();

        // Test various sequence numbers
        for recv_seq in [0, 1, 100, 1000, 16383, 32767] {
            let mut buf = BytesMut::new();
            let apdu = Apdu::s_frame(recv_seq);
            codec.encode(apdu.clone(), &mut buf).unwrap();

            let decoded = codec.decode(&mut buf).unwrap().unwrap();
            assert!(decoded.is_s_frame());
            assert_eq!(decoded.apci.recv_seq(), Some(recv_seq));
        }
    }

    #[test]
    fn test_encode_decode_i_frame_sequence_numbers() {
        let mut codec = Iec104Codec::new();

        // Test various sequence number combinations
        let test_cases = [
            (0, 0),
            (1, 1),
            (100, 50),
            (16383, 16383),
            (32767, 32767),
            (0, 32767),
            (32767, 0),
        ];

        for (send_seq, recv_seq) in test_cases {
            let mut buf = BytesMut::new();
            let asdu = Asdu::new(AsduHeader::new(
                TypeId::SinglePoint,
                1,
                Cot::Spontaneous,
                1,
            ));
            let apdu = Apdu::i_frame(send_seq, recv_seq, asdu);
            codec.encode(apdu, &mut buf).unwrap();

            let decoded = codec.decode(&mut buf).unwrap().unwrap();
            assert!(decoded.is_i_frame());
            assert_eq!(decoded.apci.send_seq(), Some(send_seq));
            assert_eq!(decoded.apci.recv_seq(), Some(recv_seq));
        }
    }

    #[test]
    fn test_apdu_display() {
        let apdu = Apdu::u_frame(UFunction::TestFrAct);
        let display = format!("{}", apdu);
        assert!(display.contains("TESTFR"));

        let apdu = Apdu::s_frame(100);
        let display = format!("{}", apdu);
        assert!(display.contains("100"));

        let asdu = Asdu::new(AsduHeader::new(
            TypeId::MeasuredFloat,
            1,
            Cot::Spontaneous,
            1,
        ));
        let apdu = Apdu::i_frame(10, 5, asdu);
        let display = format!("{}", apdu);
        assert!(display.contains("M_ME_NC_1"));
        assert!(display.contains("Spontaneous"));
    }

    #[test]
    fn test_apdu_frame_type_helpers() {
        let u_frame = Apdu::u_frame(UFunction::StartDtAct);
        assert!(u_frame.is_u_frame());
        assert!(!u_frame.is_s_frame());
        assert!(!u_frame.is_i_frame());

        let s_frame = Apdu::s_frame(0);
        assert!(!s_frame.is_u_frame());
        assert!(s_frame.is_s_frame());
        assert!(!s_frame.is_i_frame());

        let asdu = Asdu::new(AsduHeader::new(TypeId::SinglePoint, 1, Cot::Spontaneous, 1));
        let i_frame = Apdu::i_frame(0, 0, asdu);
        assert!(!i_frame.is_u_frame());
        assert!(!i_frame.is_s_frame());
        assert!(i_frame.is_i_frame());
    }

    #[test]
    fn test_codec_state_reset_on_invalid() {
        let mut codec = Iec104Codec::new();

        // Invalid length followed by valid frame
        let mut buf = BytesMut::from(&[
            0x68, 0x01, // Invalid: length too small
            0x68, 0x04, 0x07, 0x00, 0x00, 0x00, // Valid U-frame
        ][..]);

        // First decode should skip invalid and find valid
        let apdu = codec.decode(&mut buf).unwrap().unwrap();
        assert!(apdu.is_u_frame());
    }

    #[test]
    fn test_decode_only_start_byte_no_length() {
        let mut codec = Iec104Codec::new();
        let mut buf = BytesMut::from(&[0x68][..]);

        // Should return None, waiting for more data
        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_none());
        // Start byte should still be in buffer
        assert_eq!(buf.len(), 1);
    }
}
