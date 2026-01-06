//! IEC 60870-5-104 ASDU (Application Service Data Unit).
//!
//! ASDU contains the actual data (measurements, commands, etc.).

use bytes::{BufMut, Bytes, BytesMut};

use crate::error::{Iec104Error, Result};
use crate::types::{Cot, TypeId};

/// Variable Structure Qualifier (VSQ).
///
/// Defines the structure of information objects in an ASDU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vsq {
    /// Number of information objects (1-127)
    pub count: u8,
    /// If true, addresses are sequential (SQ=1)
    pub sequence: bool,
}

impl Vsq {
    /// Create a new VSQ.
    #[inline]
    pub const fn new(count: u8, sequence: bool) -> Self {
        Self { count, sequence }
    }

    /// Parse VSQ from byte.
    #[inline]
    pub const fn from_u8(value: u8) -> Self {
        Self {
            count: value & 0x7F,
            sequence: (value & 0x80) != 0,
        }
    }

    /// Encode VSQ to byte.
    #[inline]
    pub const fn as_u8(&self) -> u8 {
        (self.count & 0x7F) | if self.sequence { 0x80 } else { 0 }
    }
}

/// Information Object Address (IOA).
///
/// 3-byte address identifying a specific data point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ioa(pub u32);

impl Ioa {
    /// Create IOA from u32 (lower 24 bits).
    #[inline]
    pub const fn new(value: u32) -> Self {
        Self(value & 0x00FFFFFF)
    }

    /// Parse IOA from 3 bytes (little-endian).
    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 3 {
            return Err(Iec104Error::invalid_asdu("IOA too short"));
        }
        let value = bytes[0] as u32 | ((bytes[1] as u32) << 8) | ((bytes[2] as u32) << 16);
        Ok(Self(value))
    }

    /// Encode IOA to 3 bytes (little-endian).
    #[inline]
    pub fn to_bytes(&self) -> [u8; 3] {
        [
            (self.0 & 0xFF) as u8,
            ((self.0 >> 8) & 0xFF) as u8,
            ((self.0 >> 16) & 0xFF) as u8,
        ]
    }

    /// Get the raw value.
    #[inline]
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for Ioa {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// ASDU header (fixed part).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsduHeader {
    /// Type identification
    pub type_id: TypeId,
    /// Variable structure qualifier
    pub vsq: Vsq,
    /// Cause of transmission
    pub cot: Cot,
    /// Test flag (if true, this is a test ASDU)
    pub test: bool,
    /// Negative flag (if true, negative confirmation)
    pub negative: bool,
    /// Originator address (0 if not used)
    pub originator: u8,
    /// Common address of ASDU (station address)
    pub common_address: u16,
}

impl AsduHeader {
    /// Create a new ASDU header.
    pub fn new(type_id: TypeId, count: u8, cot: Cot, common_address: u16) -> Self {
        Self {
            type_id,
            vsq: Vsq::new(count, false),
            cot,
            test: false,
            negative: false,
            originator: 0,
            common_address,
        }
    }

    /// Parse ASDU header from bytes.
    ///
    /// Returns the header and the number of bytes consumed.
    pub fn parse(data: &[u8]) -> Result<(Self, usize)> {
        if data.len() < 6 {
            return Err(Iec104Error::invalid_asdu("ASDU header too short"));
        }

        let type_id = TypeId::from_u8(data[0])?;
        let vsq = Vsq::from_u8(data[1]);

        // COT is in lower 6 bits, test flag in bit 7, negative in bit 6
        let cot = Cot::from_u8(data[2])?;
        let test = (data[2] & 0x80) != 0;
        let negative = (data[2] & 0x40) != 0;

        let originator = data[3];
        let common_address = data[4] as u16 | ((data[5] as u16) << 8);

        Ok((
            Self {
                type_id,
                vsq,
                cot,
                test,
                negative,
                originator,
                common_address,
            },
            6,
        ))
    }

    /// Encode ASDU header to bytes.
    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u8(self.type_id.as_u8());
        buf.put_u8(self.vsq.as_u8());

        let mut cot_byte = self.cot.as_u8();
        if self.test {
            cot_byte |= 0x80;
        }
        if self.negative {
            cot_byte |= 0x40;
        }
        buf.put_u8(cot_byte);
        buf.put_u8(self.originator);
        buf.put_u16_le(self.common_address);
    }

    /// Get the encoded size in bytes.
    pub fn encoded_size(&self) -> usize {
        6
    }
}

/// Single-point information value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SinglePoint {
    /// Single-point value (0 or 1)
    pub value: bool,
    /// Quality descriptor
    pub quality: QualityDescriptor,
}

impl SinglePoint {
    /// Parse from byte.
    pub fn from_u8(value: u8) -> Self {
        Self {
            value: (value & 0x01) != 0,
            quality: QualityDescriptor::from_siq(value),
        }
    }

    /// Encode to byte.
    pub fn as_u8(&self) -> u8 {
        let mut result = if self.value { 0x01 } else { 0x00 };
        result |= self.quality.to_siq();
        result
    }
}

/// Double-point information value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoublePointValue {
    /// Indeterminate or intermediate (00)
    Indeterminate = 0,
    /// Determined OFF (01)
    Off = 1,
    /// Determined ON (10)
    On = 2,
    /// Indeterminate (11)
    IndeterminateOrFaulty = 3,
}

impl DoublePointValue {
    /// Parse from byte (lower 2 bits).
    pub fn from_u8(value: u8) -> Self {
        match value & 0x03 {
            0 => Self::Indeterminate,
            1 => Self::Off,
            2 => Self::On,
            3 => Self::IndeterminateOrFaulty,
            _ => unreachable!(),
        }
    }
}

/// Double-point information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoublePoint {
    /// Double-point value
    pub value: DoublePointValue,
    /// Quality descriptor
    pub quality: QualityDescriptor,
}

impl DoublePoint {
    /// Parse from byte.
    pub fn from_u8(value: u8) -> Self {
        Self {
            value: DoublePointValue::from_u8(value),
            quality: QualityDescriptor::from_diq(value),
        }
    }
}

/// Quality descriptor for single/double point information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QualityDescriptor {
    /// Blocked (BL)
    pub blocked: bool,
    /// Substituted (SB)
    pub substituted: bool,
    /// Not topical (NT)
    pub not_topical: bool,
    /// Invalid (IV)
    pub invalid: bool,
}

impl QualityDescriptor {
    /// Create a new quality descriptor with all flags false.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a quality descriptor indicating invalid data.
    pub fn invalid() -> Self {
        Self {
            invalid: true,
            ..Default::default()
        }
    }

    /// Parse from SIQ byte (single-point information with quality).
    pub fn from_siq(value: u8) -> Self {
        Self {
            blocked: (value & 0x10) != 0,
            substituted: (value & 0x20) != 0,
            not_topical: (value & 0x40) != 0,
            invalid: (value & 0x80) != 0,
        }
    }

    /// Parse from DIQ byte (double-point information with quality).
    pub fn from_diq(value: u8) -> Self {
        Self {
            blocked: (value & 0x10) != 0,
            substituted: (value & 0x20) != 0,
            not_topical: (value & 0x40) != 0,
            invalid: (value & 0x80) != 0,
        }
    }

    /// Encode to SIQ byte (without value bits).
    pub fn to_siq(&self) -> u8 {
        let mut result = 0u8;
        if self.blocked {
            result |= 0x10;
        }
        if self.substituted {
            result |= 0x20;
        }
        if self.not_topical {
            result |= 0x40;
        }
        if self.invalid {
            result |= 0x80;
        }
        result
    }

    /// Check if the quality is good (all flags false).
    pub fn is_good(&self) -> bool {
        !self.blocked && !self.substituted && !self.not_topical && !self.invalid
    }
}

/// Quality descriptor for measured values (QDS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MeasuredQuality {
    /// Overflow (OV)
    pub overflow: bool,
    /// Blocked (BL)
    pub blocked: bool,
    /// Substituted (SB)
    pub substituted: bool,
    /// Not topical (NT)
    pub not_topical: bool,
    /// Invalid (IV)
    pub invalid: bool,
}

impl MeasuredQuality {
    /// Create a new quality descriptor with all flags false.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse from QDS byte.
    pub fn from_u8(value: u8) -> Self {
        Self {
            overflow: (value & 0x01) != 0,
            blocked: (value & 0x10) != 0,
            substituted: (value & 0x20) != 0,
            not_topical: (value & 0x40) != 0,
            invalid: (value & 0x80) != 0,
        }
    }

    /// Encode to QDS byte.
    pub fn as_u8(&self) -> u8 {
        let mut result = 0u8;
        if self.overflow {
            result |= 0x01;
        }
        if self.blocked {
            result |= 0x10;
        }
        if self.substituted {
            result |= 0x20;
        }
        if self.not_topical {
            result |= 0x40;
        }
        if self.invalid {
            result |= 0x80;
        }
        result
    }

    /// Check if the quality is good (all flags false).
    pub fn is_good(&self) -> bool {
        !self.overflow && !self.blocked && !self.substituted && !self.not_topical && !self.invalid
    }
}

/// Measured value with quality.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeasuredValue {
    /// The value
    pub value: f32,
    /// Quality descriptor
    pub quality: MeasuredQuality,
}

impl MeasuredValue {
    /// Create a new measured value.
    pub fn new(value: f32) -> Self {
        Self {
            value,
            quality: MeasuredQuality::new(),
        }
    }

    /// Create a measured value with invalid quality.
    pub fn invalid(value: f32) -> Self {
        Self {
            value,
            quality: MeasuredQuality {
                invalid: true,
                ..Default::default()
            },
        }
    }
}

/// CP56Time2a timestamp (7 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cp56Time2a {
    /// Milliseconds (0-59999)
    pub milliseconds: u16,
    /// Minutes (0-59)
    pub minutes: u8,
    /// Hours (0-23)
    pub hours: u8,
    /// Day of month (1-31)
    pub day: u8,
    /// Day of week (1-7, 1=Monday)
    pub day_of_week: u8,
    /// Month (1-12)
    pub month: u8,
    /// Year (0-99, years since 2000)
    pub year: u8,
    /// Invalid flag
    pub invalid: bool,
    /// Summer time flag
    pub summer_time: bool,
}

impl Cp56Time2a {
    /// Parse from 7 bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 7 {
            return Err(Iec104Error::invalid_asdu("CP56Time2a too short"));
        }

        let milliseconds = bytes[0] as u16 | ((bytes[1] as u16) << 8);
        let minutes = bytes[2] & 0x3F;
        let invalid = (bytes[2] & 0x80) != 0;
        let hours = bytes[3] & 0x1F;
        let summer_time = (bytes[3] & 0x80) != 0;
        let day = bytes[4] & 0x1F;
        let day_of_week = (bytes[4] >> 5) & 0x07;
        let month = bytes[5] & 0x0F;
        let year = bytes[6] & 0x7F;

        Ok(Self {
            milliseconds,
            minutes,
            hours,
            day,
            day_of_week,
            month,
            year,
            invalid,
            summer_time,
        })
    }

    /// Encode to 7 bytes.
    pub fn to_bytes(&self) -> [u8; 7] {
        let mut result = [0u8; 7];
        result[0] = (self.milliseconds & 0xFF) as u8;
        result[1] = ((self.milliseconds >> 8) & 0xFF) as u8;
        result[2] = (self.minutes & 0x3F) | if self.invalid { 0x80 } else { 0 };
        result[3] = (self.hours & 0x1F) | if self.summer_time { 0x80 } else { 0 };
        result[4] = (self.day & 0x1F) | ((self.day_of_week & 0x07) << 5);
        result[5] = self.month & 0x0F;
        result[6] = self.year & 0x7F;
        result
    }
}

/// Information object (generic container).
#[derive(Debug, Clone, PartialEq)]
pub struct InformationObject {
    /// Information object address
    pub ioa: Ioa,
    /// Raw data bytes
    pub data: Bytes,
}

impl InformationObject {
    /// Create a new information object.
    pub fn new(ioa: Ioa, data: Bytes) -> Self {
        Self { ioa, data }
    }
}

/// Complete ASDU.
#[derive(Debug, Clone, PartialEq)]
pub struct Asdu {
    /// ASDU header
    pub header: AsduHeader,
    /// Information objects
    pub objects: Vec<InformationObject>,
    /// Raw data (for unparsed types)
    pub raw_data: Bytes,
}

impl Asdu {
    /// Create a new ASDU.
    pub fn new(header: AsduHeader) -> Self {
        Self {
            header,
            objects: Vec::new(),
            raw_data: Bytes::new(),
        }
    }

    /// Create an interrogation command ASDU.
    pub fn interrogation_command(common_address: u16, qoi: u8) -> Self {
        let mut asdu = Self::new(AsduHeader::new(
            TypeId::InterrogationCommand,
            1,
            Cot::Activation,
            common_address,
        ));
        asdu.objects.push(InformationObject {
            ioa: Ioa::new(0),
            data: Bytes::copy_from_slice(&[qoi]),
        });
        asdu
    }

    /// Create a clock synchronization command ASDU.
    pub fn clock_sync_command(common_address: u16, time: Cp56Time2a) -> Self {
        let mut asdu = Self::new(AsduHeader::new(
            TypeId::ClockSync,
            1,
            Cot::Activation,
            common_address,
        ));
        asdu.objects.push(InformationObject {
            ioa: Ioa::new(0),
            data: Bytes::copy_from_slice(&time.to_bytes()),
        });
        asdu
    }

    /// Parse ASDU from bytes (after APCI).
    pub fn parse(data: &[u8]) -> Result<Self> {
        let (header, header_len) = AsduHeader::parse(data)?;
        let raw_data = Bytes::copy_from_slice(&data[header_len..]);

        Ok(Self {
            header,
            objects: Vec::new(),
            raw_data,
        })
    }

    /// Encode ASDU to bytes.
    pub fn encode(&self) -> BytesMut {
        let mut buf = BytesMut::with_capacity(self.encoded_len());
        self.encode_to(&mut buf);
        buf
    }

    /// Encode ASDU directly into the provided buffer (zero-copy).
    #[inline]
    pub fn encode_to(&self, buf: &mut BytesMut) {
        self.header.encode(buf);

        // Encode information objects
        for obj in &self.objects {
            buf.put_slice(&obj.ioa.to_bytes());
            buf.put_slice(&obj.data);
        }

        // Or raw data if no parsed objects
        if self.objects.is_empty() && !self.raw_data.is_empty() {
            buf.put_slice(&self.raw_data);
        }
    }

    /// Calculate the encoded length of this ASDU.
    #[inline]
    pub fn encoded_len(&self) -> usize {
        let mut len = self.header.encoded_size();
        for obj in &self.objects {
            len += 3 + obj.data.len(); // IOA (3 bytes) + data
        }
        if self.objects.is_empty() {
            len += self.raw_data.len();
        }
        len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vsq() {
        let vsq = Vsq::new(10, false);
        assert_eq!(vsq.as_u8(), 10);

        let vsq = Vsq::new(10, true);
        assert_eq!(vsq.as_u8(), 0x8A);

        let vsq = Vsq::from_u8(0x8A);
        assert_eq!(vsq.count, 10);
        assert!(vsq.sequence);
    }

    #[test]
    fn test_ioa() {
        let ioa = Ioa::new(0x123456);
        let bytes = ioa.to_bytes();
        assert_eq!(bytes, [0x56, 0x34, 0x12]);

        let parsed = Ioa::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.value(), 0x123456);
    }

    #[test]
    fn test_quality_descriptor() {
        let qd = QualityDescriptor::from_siq(0x90);
        assert!(qd.blocked);
        assert!(qd.invalid);
        assert!(!qd.substituted);
        assert!(!qd.not_topical);

        assert_eq!(qd.to_siq(), 0x90);
    }

    #[test]
    fn test_asdu_header() {
        let header = AsduHeader::new(TypeId::MeasuredFloat, 5, Cot::Spontaneous, 1);
        let mut buf = BytesMut::new();
        header.encode(&mut buf);

        let (parsed, _) = AsduHeader::parse(&buf).unwrap();
        assert_eq!(parsed.type_id, TypeId::MeasuredFloat);
        assert_eq!(parsed.vsq.count, 5);
        assert_eq!(parsed.cot, Cot::Spontaneous);
        assert_eq!(parsed.common_address, 1);
    }

    #[test]
    fn test_cp56time2a() {
        let time = Cp56Time2a {
            milliseconds: 30000,
            minutes: 30,
            hours: 12,
            day: 15,
            day_of_week: 3,
            month: 6,
            year: 24,
            invalid: false,
            summer_time: true,
        };

        let bytes = time.to_bytes();
        let parsed = Cp56Time2a::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.milliseconds, 30000);
        assert_eq!(parsed.minutes, 30);
        assert_eq!(parsed.hours, 12);
        assert_eq!(parsed.day, 15);
        assert_eq!(parsed.month, 6);
        assert_eq!(parsed.year, 24);
        assert!(parsed.summer_time);
        assert!(!parsed.invalid);
    }
}
