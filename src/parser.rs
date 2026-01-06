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
        return Err(Iec104Error::invalid_asdu_static("Empty data for non-zero count"));
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
        return Err(Iec104Error::invalid_asdu_static("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    let mut offset = 3;

    for i in 0..count {
        // Get IOA
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu_static("Data too short for IOA"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        // Check data length
        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu_static("Data too short for element"));
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
        return Err(Iec104Error::invalid_asdu_static("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu_static("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu_static("Data too short for element"));
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
        return Err(Iec104Error::invalid_asdu_static("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu_static("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu_static("Data too short for element"));
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
        return Err(Iec104Error::invalid_asdu_static("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu_static("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu_static("Data too short for element"));
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
        return Err(Iec104Error::invalid_asdu_static("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu_static("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu_static("Data too short for element"));
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
        return Err(Iec104Error::invalid_asdu_static("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu_static("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu_static("Data too short for element"));
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
        return Err(Iec104Error::invalid_asdu_static("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu_static("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu_static("Data too short for element"));
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
        return Err(Iec104Error::invalid_asdu_static("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu_static("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu_static("Data too short for element"));
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
        return Err(Iec104Error::invalid_asdu_static("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu_static("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu_static("Data too short for element"));
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
        return Err(Iec104Error::invalid_asdu_static("Data too short for IOA"));
    }
    let first_ioa = parse_ioa(&data[0..3])?;
    offset = 3;

    for i in 0..count {
        let ioa = if sequence {
            first_ioa + i as u32
        } else if i > 0 {
            if offset + 3 > data.len() {
                return Err(Iec104Error::invalid_asdu_static("Data too short"));
            }
            let ioa = parse_ioa(&data[offset..offset + 3])?;
            offset += 3;
            ioa
        } else {
            first_ioa
        };

        if offset + element_size > data.len() {
            return Err(Iec104Error::invalid_asdu_static("Data too short for element"));
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

        let quality = Quality::with_invalid(invalid);

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
        return Err(Iec104Error::invalid_asdu_static("IOA too short"));
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
        assert!(points[2].quality.invalid());
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
        assert!(points[0].quality.invalid());
        assert!(points[0].quality.overflow());
    }

    // ============ Additional Tests ============

    #[test]
    fn test_parse_empty_data_zero_count() {
        // Empty data with count=0 - current behavior requires IOA
        // This test verifies that commands with empty data return Ok([])
        let asdu = make_asdu(TypeId::InterrogationCommand, 0, false, &[]);
        let points = parse_asdu(&asdu).unwrap();
        assert!(points.is_empty());
    }

    #[test]
    fn test_parse_empty_data_nonzero_count() {
        // Empty data with count>0 should fail
        let asdu = make_asdu(TypeId::SinglePoint, 1, false, &[]);
        let result = parse_asdu(&asdu);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_single_point_time56() {
        // IOA=500, SIQ=0x01 (ON), CP56Time2a (7 bytes)
        let mut data = vec![0xF4, 0x01, 0x00]; // IOA=500
        data.push(0x01); // SIQ: ON
        // CP56Time2a: 2024-06-15 12:30:30.000
        data.extend_from_slice(&[0x30, 0x75, 0x1E, 0x8C, 0x6F, 0x06, 0x18]);

        let asdu = make_asdu(TypeId::SinglePointTime56, 1, false, &data);
        let points = parse_asdu(&asdu).unwrap();

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].ioa, 500);
        assert_eq!(points[0].value, DataValue::Single(true));
        assert!(points[0].timestamp.is_some());
    }

    #[test]
    fn test_parse_double_point_all_values() {
        // Test all 4 double-point values
        for (diq, expected) in [
            (0x00, DoublePointValue::Indeterminate),
            (0x01, DoublePointValue::Off),
            (0x02, DoublePointValue::On),
            (0x03, DoublePointValue::IndeterminateOrFaulty),
        ] {
            let data = [0x01, 0x00, 0x00, diq]; // IOA=1
            let asdu = make_asdu(TypeId::DoublePoint, 1, false, &data);
            let points = parse_asdu(&asdu).unwrap();
            assert_eq!(points[0].value, DataValue::Double(expected));
        }
    }

    #[test]
    fn test_parse_double_point_time56() {
        // IOA=600, DIQ=0x02 (ON), CP56Time2a
        let mut data = vec![0x58, 0x02, 0x00]; // IOA=600
        data.push(0x02); // DIQ: ON
        data.extend_from_slice(&[0x30, 0x75, 0x1E, 0x8C, 0x6F, 0x06, 0x18]);

        let asdu = make_asdu(TypeId::DoublePointTime56, 1, false, &data);
        let points = parse_asdu(&asdu).unwrap();

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].ioa, 600);
        assert_eq!(points[0].value, DataValue::Double(DoublePointValue::On));
        assert!(points[0].timestamp.is_some());
    }

    #[test]
    fn test_parse_single_point_time24() {
        // IOA=700, SIQ=0x01 (ON), CP24Time2a (3 bytes - we skip it)
        let data = [
            0xBC, 0x02, 0x00, // IOA=700
            0x01, // SIQ: ON
            0x00, 0x00, 0x00, // CP24Time2a (ignored)
        ];
        let asdu = make_asdu(TypeId::SinglePointTime24, 1, false, &data);
        let points = parse_asdu(&asdu).unwrap();

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].ioa, 700);
        assert_eq!(points[0].value, DataValue::Single(true));
        assert!(points[0].timestamp.is_none()); // CP24Time2a not parsed
    }

    #[test]
    fn test_parse_double_point_time24() {
        // IOA=800, DIQ=0x01 (OFF), CP24Time2a (3 bytes)
        let data = [
            0x20, 0x03, 0x00, // IOA=800
            0x01, // DIQ: OFF
            0x00, 0x00, 0x00, // CP24Time2a (ignored)
        ];
        let asdu = make_asdu(TypeId::DoublePointTime24, 1, false, &data);
        let points = parse_asdu(&asdu).unwrap();

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].ioa, 800);
        assert_eq!(points[0].value, DataValue::Double(DoublePointValue::Off));
    }

    #[test]
    fn test_parse_step_position() {
        // IOA=900, VTI=0x60 (value=-4, transient=false), QDS=0x00
        let data = [
            0x84, 0x03, 0x00, // IOA=900
            0x3C, // VTI: 60 - 64 = -4
            0x00, // QDS
        ];
        let asdu = make_asdu(TypeId::StepPosition, 1, false, &data);
        let points = parse_asdu(&asdu).unwrap();

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].ioa, 900);
        assert_eq!(points[0].value, DataValue::StepPosition(-4));
    }

    #[test]
    fn test_parse_step_position_range() {
        // Test min/max step position values
        // VTI=0 means value=-64, VTI=127 means value=+63
        for (vti, expected) in [(0x00, -64), (0x40, 0), (0x7F, 63)] {
            let data = [0x01, 0x00, 0x00, vti, 0x00];
            let asdu = make_asdu(TypeId::StepPosition, 1, false, &data);
            let points = parse_asdu(&asdu).unwrap();
            assert_eq!(points[0].value, DataValue::StepPosition(expected));
        }
    }

    #[test]
    fn test_parse_bitstring() {
        // IOA=1000, BSI=0xDEADBEEF, QDS=0x00
        let data = [
            0xE8, 0x03, 0x00, // IOA=1000
            0xEF, 0xBE, 0xAD, 0xDE, // BSI (little-endian)
            0x00, // QDS
        ];
        let asdu = make_asdu(TypeId::Bitstring32, 1, false, &data);
        let points = parse_asdu(&asdu).unwrap();

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].ioa, 1000);
        assert_eq!(points[0].value, DataValue::Bitstring(0xDEADBEEF));
    }

    #[test]
    fn test_parse_measured_normalized_boundary() {
        // Test boundary values: -1.0, 0.0, ~+1.0
        for (raw, expected_approx) in [
            ([0x00, 0x80], -1.0), // -32768 / 32768 = -1.0
            ([0x00, 0x00], 0.0),  // 0 / 32768 = 0.0
            ([0xFF, 0x7F], 0.999969), // 32767 / 32768 ≈ 1.0
        ] {
            let data = [0x01, 0x00, 0x00, raw[0], raw[1], 0x00];
            let asdu = make_asdu(TypeId::MeasuredNormalized, 1, false, &data);
            let points = parse_asdu(&asdu).unwrap();
            if let DataValue::Normalized(v) = points[0].value {
                assert!((v - expected_approx).abs() < 0.001);
            } else {
                panic!("Expected Normalized value");
            }
        }
    }

    #[test]
    fn test_parse_measured_scaled_boundary() {
        // Test boundary values: i16::MIN, 0, i16::MAX
        for (raw, expected) in [
            ([0x00, 0x80], i16::MIN), // -32768
            ([0x00, 0x00], 0i16),
            ([0xFF, 0x7F], i16::MAX), // 32767
        ] {
            let data = [0x01, 0x00, 0x00, raw[0], raw[1], 0x00];
            let asdu = make_asdu(TypeId::MeasuredScaled, 1, false, &data);
            let points = parse_asdu(&asdu).unwrap();
            assert_eq!(points[0].value, DataValue::Scaled(expected));
        }
    }

    #[test]
    fn test_parse_measured_float_time56() {
        // IOA=1100, value=100.5f32, QDS=0x00, CP56Time2a
        let value_bytes = 100.5f32.to_le_bytes();
        let mut data = vec![0x4C, 0x04, 0x00]; // IOA=1100
        data.extend_from_slice(&value_bytes);
        data.push(0x00); // QDS
        data.extend_from_slice(&[0x30, 0x75, 0x1E, 0x8C, 0x6F, 0x06, 0x18]); // CP56Time2a

        let asdu = make_asdu(TypeId::MeasuredFloatTime56, 1, false, &data);
        let points = parse_asdu(&asdu).unwrap();

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].ioa, 1100);
        if let DataValue::Float(v) = points[0].value {
            assert!((v - 100.5).abs() < 0.001);
        } else {
            panic!("Expected Float value");
        }
        assert!(points[0].timestamp.is_some());
    }

    #[test]
    fn test_parse_integrated_totals_with_flags() {
        // Test with carry, adjusted, and invalid flags
        let data = [
            0x01, 0x00, 0x00, // IOA=1
            0x01, 0x00, 0x00, 0x00, // BCR=1
            0xE5, // seq=5, carry=true, adjusted=true, invalid=true
        ];
        let asdu = make_asdu(TypeId::IntegratedTotals, 1, false, &data);
        let points = parse_asdu(&asdu).unwrap();

        if let DataValue::BinaryCounter {
            value,
            sequence,
            carry,
            adjusted,
            invalid,
        } = points[0].value
        {
            assert_eq!(value, 1);
            assert_eq!(sequence, 5);
            assert!(carry);
            assert!(adjusted);
            assert!(invalid);
        } else {
            panic!("Expected BinaryCounter value");
        }
    }

    #[test]
    fn test_parse_multiple_non_sequential_ioas() {
        // Multiple points with different IOAs (non-sequential)
        // IOA1=100, IOA2=200, IOA3=300
        let data = [
            0x64, 0x00, 0x00, 0x01, // IOA=100, SIQ=ON
            0xC8, 0x00, 0x00, 0x00, // IOA=200, SIQ=OFF
            0x2C, 0x01, 0x00, 0x01, // IOA=300, SIQ=ON
        ];
        let asdu = make_asdu(TypeId::SinglePoint, 3, false, &data);
        let points = parse_asdu(&asdu).unwrap();

        assert_eq!(points.len(), 3);
        assert_eq!(points[0].ioa, 100);
        assert_eq!(points[1].ioa, 200);
        assert_eq!(points[2].ioa, 300);
        assert_eq!(points[0].value, DataValue::Single(true));
        assert_eq!(points[1].value, DataValue::Single(false));
        assert_eq!(points[2].value, DataValue::Single(true));
    }

    #[test]
    fn test_parse_quality_all_flags() {
        // Test all quality flags: OV|BL|SB|NT|IV = 0xF1
        let value_bytes = 0.0f32.to_le_bytes();
        let mut data = vec![0x01, 0x00, 0x00]; // IOA=1
        data.extend_from_slice(&value_bytes);
        data.push(0xF1); // QDS: all flags except reserved bits

        let asdu = make_asdu(TypeId::MeasuredFloat, 1, false, &data);
        let points = parse_asdu(&asdu).unwrap();

        let q = &points[0].quality;
        assert!(q.overflow());
        assert!(q.blocked());
        assert!(q.substituted());
        assert!(q.not_topical());
        assert!(q.invalid());
    }

    #[test]
    fn test_parse_command_types_return_empty() {
        // Command types should return empty Vec
        for type_id in [
            TypeId::SingleCommand,
            TypeId::DoubleCommand,
            TypeId::RegulatingStep,
            TypeId::SetpointNormalized,
            TypeId::SetpointScaled,
            TypeId::SetpointFloat,
            TypeId::InterrogationCommand,
            TypeId::CounterInterrogation,
            TypeId::ReadCommand,
            TypeId::ClockSync,
            TypeId::TestCommand,
            TypeId::ResetProcess,
        ] {
            let asdu = make_asdu(type_id, 1, false, &[0x00, 0x00, 0x00, 0x00]);
            let points = parse_asdu(&asdu).unwrap();
            assert!(points.is_empty(), "Type {:?} should return empty", type_id);
        }
    }

    #[test]
    fn test_parse_data_too_short_for_element() {
        // Data has IOA but not enough for element
        let data = [0x01, 0x00, 0x00]; // IOA only, no SIQ
        let asdu = make_asdu(TypeId::SinglePoint, 1, false, &data);
        let result = parse_asdu(&asdu);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ioa_too_short() {
        // Data too short even for IOA
        let data = [0x01, 0x00]; // Only 2 bytes
        let asdu = make_asdu(TypeId::SinglePoint, 1, false, &data);
        let result = parse_asdu(&asdu);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_large_ioa_value() {
        // Test maximum IOA value (24-bit: 0xFFFFFF = 16777215)
        let data = [0xFF, 0xFF, 0xFF, 0x01]; // IOA=16777215, SIQ=ON
        let asdu = make_asdu(TypeId::SinglePoint, 1, false, &data);
        let points = parse_asdu(&asdu).unwrap();

        assert_eq!(points[0].ioa, 0xFFFFFF);
    }

    #[test]
    fn test_parse_measured_float_nan() {
        // Test NaN value
        let nan_bytes = f32::NAN.to_le_bytes();
        let mut data = vec![0x01, 0x00, 0x00];
        data.extend_from_slice(&nan_bytes);
        data.push(0x00);

        let asdu = make_asdu(TypeId::MeasuredFloat, 1, false, &data);
        let points = parse_asdu(&asdu).unwrap();

        if let DataValue::Float(v) = points[0].value {
            assert!(v.is_nan());
        } else {
            panic!("Expected Float value");
        }
    }

    #[test]
    fn test_parse_measured_float_infinity() {
        // Test infinity values
        for inf in [f32::INFINITY, f32::NEG_INFINITY] {
            let inf_bytes = inf.to_le_bytes();
            let mut data = vec![0x01, 0x00, 0x00];
            data.extend_from_slice(&inf_bytes);
            data.push(0x00);

            let asdu = make_asdu(TypeId::MeasuredFloat, 1, false, &data);
            let points = parse_asdu(&asdu).unwrap();

            if let DataValue::Float(v) = points[0].value {
                assert!(v.is_infinite());
                assert_eq!(v.is_sign_positive(), inf.is_sign_positive());
            } else {
                panic!("Expected Float value");
            }
        }
    }

    #[test]
    fn test_parse_sequence_float_multiple() {
        // Multiple float values in sequence mode
        let mut data = vec![0x64, 0x00, 0x00]; // First IOA=100
        for i in 0..5 {
            let value = (i as f32) * 10.0;
            data.extend_from_slice(&value.to_le_bytes());
            data.push(0x00); // QDS
        }

        let asdu = make_asdu(TypeId::MeasuredFloat, 5, true, &data);
        let points = parse_asdu(&asdu).unwrap();

        assert_eq!(points.len(), 5);
        for i in 0..5 {
            assert_eq!(points[i].ioa, 100 + i as u32);
            if let DataValue::Float(v) = points[i].value {
                assert!((v - (i as f32) * 10.0).abs() < 0.001);
            } else {
                panic!("Expected Float value");
            }
        }
    }
}
