//! ASDU information object parser.
//!
//! This module provides parsing of information objects from ASDU raw data
//! into structured `DataPoint` values.

use crate::error::{Iec104Error, Result};
use crate::types::{Asdu, Cp56Time2a, DataPoint, DataValue, DoublePointValue, Quality, TypeId};

/// Parse an ASDU into a list of data points.
///
/// This function extracts information objects from the ASDU and converts them
/// into structured `DataPoint` values.
///
/// # Example
///
/// ```rust,ignore
/// let asdu = /* received from server */;
/// let points = parse_asdu(&asdu)?;
/// for point in points {
///     println!("IOA {}: {:?} ({})", point.ioa, point.value, point.quality);
/// }
/// ```
pub fn parse_asdu(asdu: &Asdu) -> Result<Vec<DataPoint>> {
    let data = asdu.raw_data.as_ref();
    let type_id = asdu.header.type_id;
    let count = asdu.header.vsq.count as usize;
    let sequence = asdu.header.vsq.sequence;

    if data.is_empty() && count > 0 {
        return Err(Iec104Error::invalid_asdu("Empty data for non-zero count"));
    }

    match type_id {
        // Single-point information
        TypeId::SinglePoint => parse_single_point(data, count, sequence, false),
        TypeId::SinglePointTime56 => parse_single_point(data, count, sequence, true),

        // Double-point information
        TypeId::DoublePoint => parse_double_point(data, count, sequence, false),
        TypeId::DoublePointTime56 => parse_double_point(data, count, sequence, true),

        // Step position
        TypeId::StepPosition => parse_step_position(data, count, sequence, false),

        // Bitstring
        TypeId::Bitstring32 => parse_bitstring(data, count, sequence, false),

        // Measured values - normalized
        TypeId::MeasuredNormalized => parse_measured_normalized(data, count, sequence, false),
        TypeId::MeasuredNormalizedTime24 => parse_measured_normalized(data, count, sequence, false),

        // Measured values - scaled
        TypeId::MeasuredScaled => parse_measured_scaled(data, count, sequence, false),
        TypeId::MeasuredScaledTime24 => parse_measured_scaled(data, count, sequence, false),

        // Measured values - float
        TypeId::MeasuredFloat => parse_measured_float(data, count, sequence, false),
        TypeId::MeasuredFloatTime24 => parse_measured_float(data, count, sequence, false),
        TypeId::MeasuredFloatTime56 => parse_measured_float(data, count, sequence, true),

        // Integrated totals
        TypeId::IntegratedTotals => parse_integrated_totals(data, count, sequence, false),

        // Commands and system types - return empty (not data points)
        TypeId::SingleCommand
        | TypeId::DoubleCommand
        | TypeId::RegulatingStep
        | TypeId::SetpointNormalized
        | TypeId::SetpointScaled
        | TypeId::SetpointFloat
        | TypeId::Bitstring32Command
        | TypeId::SingleCommandTime56
        | TypeId::DoubleCommandTime56
        | TypeId::SetpointFloatTime56
        | TypeId::EndOfInit
        | TypeId::InterrogationCommand
        | TypeId::CounterInterrogation
        | TypeId::ReadCommand
        | TypeId::ClockSync
        | TypeId::TestCommand
        | TypeId::ResetProcess
        | TypeId::TestCommandTime56 => Ok(Vec::new()),

        // Time-tagged variants without CP56Time2a
        TypeId::SinglePointTime24 | TypeId::DoublePointTime24 => {
            // These have 3-byte time tag (CP24Time2a), not full timestamp
            // Parse as regular without timestamp for now
            match type_id {
                TypeId::SinglePointTime24 => parse_single_point_time24(data, count, sequence),
                TypeId::DoublePointTime24 => parse_double_point_time24(data, count, sequence),
                _ => unreachable!(),
            }
        }
    }
}

/// Parse single-point information (M_SP_NA_1, M_SP_TB_1).
fn parse_single_point(
    data: &[u8],
    count: usize,
    sequence: bool,
    with_time: bool,
) -> Result<Vec<DataPoint>> {
    let mut points = Vec::with_capacity(count);

    // Calculate element size
    let element_size = if with_time { 1 + 7 } else { 1 }; // SIQ + optional CP56Time2a

    // First IOA (always present)
    if data.len() < 3 {
        return Err(Iec104Error::invalid_asdu("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    let mut offset = 3;

    for i in 0..count {
        // Get IOA
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu("Data too short for IOA"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        // Check data length
        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu("Data too short for element"));
        }

        // Parse SIQ (Single-point Information with Quality)
        let siq = data[offset];
        let value = (siq & 0x01) != 0;
        let quality = Quality::from_siq(siq);
        offset += 1;

        // Parse timestamp if present
        let timestamp = if with_time {
            let ts = Cp56Time2a::from_bytes(&data[offset..offset + 7])?;
            offset += 7;
            Some(ts)
        } else {
            None
        };

        points.push(DataPoint {
            ioa,
            value: DataValue::Single(value),
            quality,
            timestamp,
        });
    }

    Ok(points)
}

/// Parse single-point with CP24Time2a (M_SP_TA_1).
fn parse_single_point_time24(data: &[u8], count: usize, sequence: bool) -> Result<Vec<DataPoint>> {
    let mut points = Vec::with_capacity(count);
    let mut offset;

    // Element size: SIQ (1) + CP24Time2a (3)
    let element_size = 4;

    if data.len() < 3 {
        return Err(Iec104Error::invalid_asdu("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu("Data too short for element"));
        }

        let siq = data[offset];
        let value = (siq & 0x01) != 0;
        let quality = Quality::from_siq(siq);
        offset += 4; // Skip SIQ + CP24Time2a (we don't parse short timestamp)

        points.push(DataPoint {
            ioa,
            value: DataValue::Single(value),
            quality,
            timestamp: None,
        });
    }

    Ok(points)
}

/// Parse double-point information (M_DP_NA_1, M_DP_TB_1).
fn parse_double_point(
    data: &[u8],
    count: usize,
    sequence: bool,
    with_time: bool,
) -> Result<Vec<DataPoint>> {
    let mut points = Vec::with_capacity(count);
    let mut offset;

    let element_size = if with_time { 1 + 7 } else { 1 };

    if data.len() < 3 {
        return Err(Iec104Error::invalid_asdu("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu("Data too short for element"));
        }

        // Parse DIQ (Double-point Information with Quality)
        let diq = data[offset];
        let dp_value = match diq & 0x03 {
            0 => DoublePointValue::Indeterminate,
            1 => DoublePointValue::Off,
            2 => DoublePointValue::On,
            3 => DoublePointValue::IndeterminateOrFaulty,
            _ => unreachable!(),
        };
        let quality = Quality::from_diq(diq);
        offset += 1;

        let timestamp = if with_time {
            let ts = Cp56Time2a::from_bytes(&data[offset..offset + 7])?;
            offset += 7;
            Some(ts)
        } else {
            None
        };

        points.push(DataPoint {
            ioa,
            value: DataValue::Double(dp_value),
            quality,
            timestamp,
        });
    }

    Ok(points)
}

/// Parse double-point with CP24Time2a (M_DP_TA_1).
fn parse_double_point_time24(data: &[u8], count: usize, sequence: bool) -> Result<Vec<DataPoint>> {
    let mut points = Vec::with_capacity(count);
    let mut offset;

    let element_size = 4; // DIQ (1) + CP24Time2a (3)

    if data.len() < 3 {
        return Err(Iec104Error::invalid_asdu("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu("Data too short for element"));
        }

        let diq = data[offset];
        let dp_value = match diq & 0x03 {
            0 => DoublePointValue::Indeterminate,
            1 => DoublePointValue::Off,
            2 => DoublePointValue::On,
            3 => DoublePointValue::IndeterminateOrFaulty,
            _ => unreachable!(),
        };
        let quality = Quality::from_diq(diq);
        offset += 4; // Skip DIQ + CP24Time2a

        points.push(DataPoint {
            ioa,
            value: DataValue::Double(dp_value),
            quality,
            timestamp: None,
        });
    }

    Ok(points)
}

/// Parse step position information (M_ST_NA_1).
fn parse_step_position(
    data: &[u8],
    count: usize,
    sequence: bool,
    _with_time: bool,
) -> Result<Vec<DataPoint>> {
    let mut points = Vec::with_capacity(count);
    let mut offset;

    let element_size = 2; // VTI (1) + QDS (1)

    if data.len() < 3 {
        return Err(Iec104Error::invalid_asdu("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu("Data too short for element"));
        }

        // VTI: Value with Transient Indicator
        let vti = data[offset];
        // Value is in bits 0-6, bit 7 is transient indicator
        let value = ((vti & 0x7F) as i8) - 64; // Convert to -64..+63
        offset += 1;

        // QDS: Quality Descriptor
        let qds = data[offset];
        let quality = Quality::from_qds(qds);
        offset += 1;

        points.push(DataPoint {
            ioa,
            value: DataValue::StepPosition(value),
            quality,
            timestamp: None,
        });
    }

    Ok(points)
}

/// Parse bitstring of 32 bits (M_BO_NA_1).
fn parse_bitstring(
    data: &[u8],
    count: usize,
    sequence: bool,
    _with_time: bool,
) -> Result<Vec<DataPoint>> {
    let mut points = Vec::with_capacity(count);
    let mut offset;

    let element_size = 5; // BSI (4) + QDS (1)

    if data.len() < 3 {
        return Err(Iec104Error::invalid_asdu("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu("Data too short for element"));
        }

        // BSI: Bitstring of 32 bit
        let value = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;

        let qds = data[offset];
        let quality = Quality::from_qds(qds);
        offset += 1;

        points.push(DataPoint {
            ioa,
            value: DataValue::Bitstring(value),
            quality,
            timestamp: None,
        });
    }

    Ok(points)
}

/// Parse measured value, normalized (M_ME_NA_1).
fn parse_measured_normalized(
    data: &[u8],
    count: usize,
    sequence: bool,
    _with_time: bool,
) -> Result<Vec<DataPoint>> {
    let mut points = Vec::with_capacity(count);
    let mut offset;

    let element_size = 3; // NVA (2) + QDS (1)

    if data.len() < 3 {
        return Err(Iec104Error::invalid_asdu("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu("Data too short for element"));
        }

        // NVA: Normalized Value (16-bit signed, -1.0 to ~+1.0)
        let raw = i16::from_le_bytes([data[offset], data[offset + 1]]);
        let value = raw as f32 / 32768.0;
        offset += 2;

        let qds = data[offset];
        let quality = Quality::from_qds(qds);
        offset += 1;

        points.push(DataPoint {
            ioa,
            value: DataValue::Normalized(value),
            quality,
            timestamp: None,
        });
    }

    Ok(points)
}

/// Parse measured value, scaled (M_ME_NB_1).
fn parse_measured_scaled(
    data: &[u8],
    count: usize,
    sequence: bool,
    _with_time: bool,
) -> Result<Vec<DataPoint>> {
    let mut points = Vec::with_capacity(count);
    let mut offset;

    let element_size = 3; // SVA (2) + QDS (1)

    if data.len() < 3 {
        return Err(Iec104Error::invalid_asdu("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu("Data too short for element"));
        }

        // SVA: Scaled Value
        let value = i16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;

        let qds = data[offset];
        let quality = Quality::from_qds(qds);
        offset += 1;

        points.push(DataPoint {
            ioa,
            value: DataValue::Scaled(value),
            quality,
            timestamp: None,
        });
    }

    Ok(points)
}

/// Parse measured value, short floating point (M_ME_NC_1, M_ME_TF_1).
fn parse_measured_float(
    data: &[u8],
    count: usize,
    sequence: bool,
    with_time: bool,
) -> Result<Vec<DataPoint>> {
    let mut points = Vec::with_capacity(count);
    let mut offset;

    let element_size = if with_time { 5 + 7 } else { 5 }; // IEEE float (4) + QDS (1) + optional CP56Time2a

    if data.len() < 3 {
        return Err(Iec104Error::invalid_asdu("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu("Data too short for element"));
        }

        // IEEE 754 short floating point
        let value = f32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;

        let qds = data[offset];
        let quality = Quality::from_qds(qds);
        offset += 1;

        let timestamp = if with_time {
            let ts = Cp56Time2a::from_bytes(&data[offset..offset + 7])?;
            offset += 7;
            Some(ts)
        } else {
            None
        };

        points.push(DataPoint {
            ioa,
            value: DataValue::Float(value),
            quality,
            timestamp,
        });
    }

    Ok(points)
}

/// Parse integrated totals (M_IT_NA_1).
fn parse_integrated_totals(
    data: &[u8],
    count: usize,
    sequence: bool,
    _with_time: bool,
) -> Result<Vec<DataPoint>> {
    let mut points = Vec::with_capacity(count);
    let mut offset;

    let element_size = 5; // BCR (4) + sequence/flags (1)

    if data.len() < 3 {
        return Err(Iec104Error::invalid_asdu("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu("Data too short for element"));
        }

        // BCR: Binary Counter Reading
        let value = i32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;

        // Sequence number and flags
        let flags = data[offset];
        let seq_number = flags & 0x1F;
        let carry = (flags & 0x20) != 0;
        let adjusted = (flags & 0x40) != 0;
        let invalid = (flags & 0x80) != 0;
        offset += 1;

        let quality = Quality {
            invalid,
            ..Default::default()
        };

        points.push(DataPoint {
            ioa,
            value: DataValue::BinaryCounter {
                value,
                sequence: seq_number,
                carry,
                adjusted,
                invalid,
            },
            quality,
            timestamp: None,
        });
    }

    Ok(points)
}

/// Parse IOA from 3 bytes (little-endian).
#[inline(always)]
fn parse_ioa(bytes: &[u8]) -> Result<u32> {
    if bytes.len() < 3 {
        return Err(Iec104Error::invalid_asdu("IOA too short"));
    }
    Ok(read_ioa_le(bytes))
}

/// Read IOA as little-endian u24 (assumes bytes.len() >= 3).
#[inline(always)]
fn read_ioa_le(bytes: &[u8]) -> u32 {
    bytes[0] as u32 | ((bytes[1] as u32) << 8) | ((bytes[2] as u32) << 16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AsduHeader, Cot, Vsq};
    use bytes::Bytes;

    fn make_asdu(type_id: TypeId, count: u8, sequence: bool, data: &[u8]) -> Asdu {
        Asdu {
            header: AsduHeader {
                type_id,
                vsq: Vsq::new(count, sequence),
                cot: Cot::Spontaneous,
                test: false,
                negative: false,
                originator: 0,
                common_address: 1,
            },
            objects: Vec::new(),
            raw_data: Bytes::copy_from_slice(data),
        }
    }

    #[test]
    fn test_parse_single_point() {
        // IOA=1001 (0xE9 0x03 0x00), SIQ=0x01 (ON, good quality)
        let data = [0xE9, 0x03, 0x00, 0x01];
        let asdu = make_asdu(TypeId::SinglePoint, 1, false, &data);

        let points = parse_asdu(&asdu).unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].ioa, 1001);
        assert_eq!(points[0].value, DataValue::Single(true));
        assert!(points[0].is_good());
    }

    #[test]
    fn test_parse_single_point_sequence() {
        // IOA=100 (start), 3 points in sequence
        // SIQ values: 0x00 (OFF), 0x01 (ON), 0x80 (OFF, invalid)
        let data = [0x64, 0x00, 0x00, 0x00, 0x01, 0x80];
        let asdu = make_asdu(TypeId::SinglePoint, 3, true, &data);

        let points = parse_asdu(&asdu).unwrap();
        assert_eq!(points.len(), 3);

        assert_eq!(points[0].ioa, 100);
        assert_eq!(points[0].value, DataValue::Single(false));
        assert!(points[0].is_good());

        assert_eq!(points[1].ioa, 101);
        assert_eq!(points[1].value, DataValue::Single(true));
        assert!(points[1].is_good());

        assert_eq!(points[2].ioa, 102);
        assert_eq!(points[2].value, DataValue::Single(false));
        assert!(!points[2].is_good());
        assert!(points[2].quality.invalid);
    }

    #[test]
    fn test_parse_double_point() {
        // IOA=2000, DIQ=0x02 (ON)
        let data = [0xD0, 0x07, 0x00, 0x02];
        let asdu = make_asdu(TypeId::DoublePoint, 1, false, &data);

        let points = parse_asdu(&asdu).unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].ioa, 2000);
        assert_eq!(points[0].value, DataValue::Double(DoublePointValue::On));
    }

    #[test]
    fn test_parse_measured_float() {
        // IOA=3000, value=23.5f32, QDS=0x00 (good)
        let value_bytes = 23.5f32.to_le_bytes();
        let mut data = vec![0xB8, 0x0B, 0x00]; // IOA=3000
        data.extend_from_slice(&value_bytes);
        data.push(0x00); // QDS

        let asdu = make_asdu(TypeId::MeasuredFloat, 1, false, &data);

        let points = parse_asdu(&asdu).unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].ioa, 3000);
        if let DataValue::Float(v) = points[0].value {
            assert!((v - 23.5).abs() < 0.001);
        } else {
            panic!("Expected Float value");
        }
        assert!(points[0].is_good());
    }

    #[test]
    fn test_parse_measured_scaled() {
        // IOA=4000, value=1000 (i16), QDS=0x00
        let data = [
            0xA0, 0x0F, 0x00, // IOA=4000
            0xE8, 0x03, // 1000 in little-endian
            0x00, // QDS
        ];
        let asdu = make_asdu(TypeId::MeasuredScaled, 1, false, &data);

        let points = parse_asdu(&asdu).unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].ioa, 4000);
        assert_eq!(points[0].value, DataValue::Scaled(1000));
    }

    #[test]
    fn test_parse_measured_normalized() {
        // IOA=5000, value=16384 (0.5 normalized), QDS=0x00
        let data = [
            0x88, 0x13, 0x00, // IOA=5000
            0x00, 0x40, // 16384 = 0.5 * 32768
            0x00, // QDS
        ];
        let asdu = make_asdu(TypeId::MeasuredNormalized, 1, false, &data);

        let points = parse_asdu(&asdu).unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].ioa, 5000);
        if let DataValue::Normalized(v) = points[0].value {
            assert!((v - 0.5).abs() < 0.001);
        } else {
            panic!("Expected Normalized value");
        }
    }

    #[test]
    fn test_parse_integrated_totals() {
        // IOA=6000, counter=123456, seq=5, no flags
        let data = [
            0x70, 0x17, 0x00, // IOA=6000
            0x40, 0xE2, 0x01, 0x00, // 123456 in little-endian
            0x05, // sequence=5, no carry/adjust/invalid
        ];
        let asdu = make_asdu(TypeId::IntegratedTotals, 1, false, &data);

        let points = parse_asdu(&asdu).unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].ioa, 6000);
        if let DataValue::BinaryCounter {
            value,
            sequence,
            carry,
            adjusted,
            invalid,
        } = points[0].value
        {
            assert_eq!(value, 123456);
            assert_eq!(sequence, 5);
            assert!(!carry);
            assert!(!adjusted);
            assert!(!invalid);
        } else {
            panic!("Expected BinaryCounter value");
        }
    }

    #[test]
    fn test_parse_with_bad_quality() {
        // IOA=1000, value=10.0f32, QDS=0x81 (invalid + overflow)
        let value_bytes = 10.0f32.to_le_bytes();
        let mut data = vec![0xE8, 0x03, 0x00]; // IOA=1000
        data.extend_from_slice(&value_bytes);
        data.push(0x81); // QDS: IV + OV

        let asdu = make_asdu(TypeId::MeasuredFloat, 1, false, &data);

        let points = parse_asdu(&asdu).unwrap();
        assert!(!points[0].is_good());
        assert!(points[0].quality.invalid);
        assert!(points[0].quality.overflow);
    }
}
