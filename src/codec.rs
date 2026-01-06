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

                    // Skip bytes until we find the start byte
                    while !src.is_empty() && src[0] != START_BYTE {
                        src.advance(1);
                    }

                    if src.is_empty() {
                        return Ok(None);
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
                    let frame = src.split_to(total_length);
                    self.state = DecodeState::WaitingForStart;

                    // Parse the frame
                    // Frame structure: [0x68] [length] [control1] [control2] [control3] [control4] [ASDU...]
                    let control = &frame[2..6];
                    let apci = Apci::parse(control)?;

                    let asdu = if apci.is_i_frame() && frame.len() > 6 {
                        Some(Asdu::parse(&frame[6..])?)
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
            return Err(Iec104Error::Codec("ASDU too large".into()));
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
            let original = Apdu::u_frame(func);
            codec.encode(original.clone(), &mut buf).unwrap();

            let decoded = codec.decode(&mut buf).unwrap().unwrap();
            assert_eq!(decoded.apci, original.apci);
        }

        // Test S-frame roundtrip
        for recv_seq in [0, 100, 32767] {
            let mut buf = BytesMut::new();
            let original = Apdu::s_frame(recv_seq);
            codec.encode(original.clone(), &mut buf).unwrap();

            let decoded = codec.decode(&mut buf).unwrap().unwrap();
            assert_eq!(decoded.apci, original.apci);
        }
    }
}
