//! Data point types for IEC 60870-5-104.
//!
//! This module defines the unified data structures for representing
//! information objects parsed from ASDUs.

use super::{Cp56Time2a, DoublePointValue, MeasuredQuality, QualityDescriptor};

/// Unified data point representing an information object.
#[derive(Debug, Clone, PartialEq)]
pub struct DataPoint {
    /// Information object address (IOA)
    pub ioa: u32,
    /// Data value
    pub value: DataValue,
    /// Quality flags
    pub quality: Quality,
    /// Timestamp (if present)
    pub timestamp: Option<Cp56Time2a>,
}

impl DataPoint {
    /// Create a new data point.
    #[inline]
    pub const fn new(ioa: u32, value: DataValue) -> Self {
        Self {
            ioa,
            value,
            quality: Quality::Good,
            timestamp: None,
        }
    }

    /// Create a data point with quality.
    #[inline]
    pub const fn with_quality(ioa: u32, value: DataValue, quality: Quality) -> Self {
        Self {
            ioa,
            value,
            quality,
            timestamp: None,
        }
    }

    /// Create a data point with timestamp.
    #[inline]
    pub const fn with_timestamp(
        ioa: u32,
        value: DataValue,
        quality: Quality,
        timestamp: Cp56Time2a,
    ) -> Self {
        Self {
            ioa,
            value,
            quality,
            timestamp: Some(timestamp),
        }
    }

    /// Check if the data point has good quality.
    #[inline]
    pub const fn is_good(&self) -> bool {
        self.quality.is_good()
    }

    /// Get the value as f64 if numeric.
    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        self.value.as_f64()
    }

    /// Get the value as bool if boolean.
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        self.value.as_bool()
    }
}

/// Data value types.
#[derive(Debug, Clone, PartialEq)]
pub enum DataValue {
    /// Single-point information (M_SP_NA_1, M_SP_TB_1)
    Single(bool),

    /// Double-point information (M_DP_NA_1, M_DP_TB_1)
    Double(DoublePointValue),

    /// Normalized value -1.0 to +1.0 (M_ME_NA_1, M_ME_TD_1)
    Normalized(f32),

    /// Scaled value (M_ME_NB_1, M_ME_TE_1)
    Scaled(i16),

    /// Short floating point (M_ME_NC_1, M_ME_TF_1)
    Float(f32),

    /// Integrated totals / counter (M_IT_NA_1, M_IT_TB_1)
    Counter(i32),

    /// Bitstring of 32 bits (M_BO_NA_1, M_BO_TB_1)
    Bitstring(u32),

    /// Step position (-64 to +63) (M_ST_NA_1, M_ST_TB_1)
    StepPosition(i8),

    /// Binary counter reading with sequence and flags
    BinaryCounter {
        value: i32,
        sequence: u8,
        carry: bool,
        adjusted: bool,
        invalid: bool,
    },
}

impl DataValue {
    /// Convert to f64 if numeric.
    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Single(v) => Some(if *v { 1.0 } else { 0.0 }),
            Self::Double(v) => Some(match v {
                DoublePointValue::Off => 0.0,
                DoublePointValue::On => 1.0,
                _ => f64::NAN,
            }),
            Self::Normalized(v) => Some(*v as f64),
            Self::Scaled(v) => Some(*v as f64),
            Self::Float(v) => Some(*v as f64),
            Self::Counter(v) => Some(*v as f64),
            Self::Bitstring(v) => Some(*v as f64),
            Self::StepPosition(v) => Some(*v as f64),
            Self::BinaryCounter { value, .. } => Some(*value as f64),
        }
    }

    /// Convert to bool if boolean type.
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Single(v) => Some(*v),
            Self::Double(v) => match v {
                DoublePointValue::Off => Some(false),
                DoublePointValue::On => Some(true),
                _ => None,
            },
            _ => None,
        }
    }

    /// Check if this is a boolean type.
    #[inline]
    pub const fn is_boolean(&self) -> bool {
        matches!(self, Self::Single(_) | Self::Double(_))
    }

    /// Check if this is a numeric type.
    #[inline]
    pub const fn is_numeric(&self) -> bool {
        matches!(
            self,
            Self::Normalized(_)
                | Self::Scaled(_)
                | Self::Float(_)
                | Self::Counter(_)
                | Self::StepPosition(_)
                | Self::BinaryCounter { .. }
        )
    }
}

/// Quality flags for data points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Quality {
    /// Overflow (OV) - value exceeds predefined range
    pub overflow: bool,
    /// Blocked (BL) - value is blocked for transmission
    pub blocked: bool,
    /// Substituted (SB) - value is substituted
    pub substituted: bool,
    /// Not topical (NT) - value is not topical (outdated)
    pub not_topical: bool,
    /// Invalid (IV) - value is invalid
    pub invalid: bool,
    /// Elapsed time invalid (for counter values)
    pub elapsed_time_invalid: bool,
}

impl Quality {
    /// Good quality (all flags false).
    #[allow(non_upper_case_globals)]
    pub const Good: Self = Self {
        overflow: false,
        blocked: false,
        substituted: false,
        not_topical: false,
        invalid: false,
        elapsed_time_invalid: false,
    };

    /// Invalid quality.
    #[allow(non_upper_case_globals)]
    pub const Invalid: Self = Self {
        overflow: false,
        blocked: false,
        substituted: false,
        not_topical: false,
        invalid: true,
        elapsed_time_invalid: false,
    };

    /// Create from QualityDescriptor (for single/double point).
    #[inline]
    pub const fn from_quality_descriptor(qd: QualityDescriptor) -> Self {
        Self {
            overflow: false,
            blocked: qd.blocked,
            substituted: qd.substituted,
            not_topical: qd.not_topical,
            invalid: qd.invalid,
            elapsed_time_invalid: false,
        }
    }

    /// Create from MeasuredQuality (for measured values).
    #[inline]
    pub const fn from_measured_quality(mq: MeasuredQuality) -> Self {
        Self {
            overflow: mq.overflow,
            blocked: mq.blocked,
            substituted: mq.substituted,
            not_topical: mq.not_topical,
            invalid: mq.invalid,
            elapsed_time_invalid: false,
        }
    }

    /// Parse from QDS byte (Quality Descriptor for measured values).
    #[inline(always)]
    pub const fn from_qds(byte: u8) -> Self {
        Self {
            overflow: (byte & 0x01) != 0,
            blocked: (byte & 0x10) != 0,
            substituted: (byte & 0x20) != 0,
            not_topical: (byte & 0x40) != 0,
            invalid: (byte & 0x80) != 0,
            elapsed_time_invalid: false,
        }
    }

    /// Parse from SIQ byte (Single-point Information with Quality).
    #[inline(always)]
    pub const fn from_siq(byte: u8) -> Self {
        Self {
            overflow: false,
            blocked: (byte & 0x10) != 0,
            substituted: (byte & 0x20) != 0,
            not_topical: (byte & 0x40) != 0,
            invalid: (byte & 0x80) != 0,
            elapsed_time_invalid: false,
        }
    }

    /// Parse from DIQ byte (Double-point Information with Quality).
    #[inline(always)]
    pub const fn from_diq(byte: u8) -> Self {
        Self {
            overflow: false,
            blocked: (byte & 0x10) != 0,
            substituted: (byte & 0x20) != 0,
            not_topical: (byte & 0x40) != 0,
            invalid: (byte & 0x80) != 0,
            elapsed_time_invalid: false,
        }
    }

    /// Parse from BCR flags (Binary Counter Reading).
    #[inline(always)]
    pub const fn from_bcr_flags(byte: u8) -> Self {
        Self {
            overflow: false,
            blocked: false,
            substituted: false,
            not_topical: false,
            invalid: (byte & 0x80) != 0,
            elapsed_time_invalid: (byte & 0x40) != 0,
        }
    }

    /// Check if quality is good (no flags set).
    #[inline(always)]
    pub const fn is_good(&self) -> bool {
        !self.overflow
            && !self.blocked
            && !self.substituted
            && !self.not_topical
            && !self.invalid
            && !self.elapsed_time_invalid
    }
}

impl std::fmt::Display for Quality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_good() {
            return f.write_str("Good");
        }

        let mut first = true;
        let mut write_flag = |f: &mut std::fmt::Formatter<'_>, flag: &str| -> std::fmt::Result {
            if !first {
                f.write_str("|")?;
            }
            first = false;
            f.write_str(flag)
        };

        if self.overflow {
            write_flag(f, "OV")?;
        }
        if self.blocked {
            write_flag(f, "BL")?;
        }
        if self.substituted {
            write_flag(f, "SB")?;
        }
        if self.not_topical {
            write_flag(f, "NT")?;
        }
        if self.invalid {
            write_flag(f, "IV")?;
        }
        if self.elapsed_time_invalid {
            write_flag(f, "EI")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_point_creation() {
        let dp = DataPoint::new(1001, DataValue::Float(23.5));
        assert_eq!(dp.ioa, 1001);
        assert!(dp.is_good());
        assert_eq!(dp.as_f64(), Some(23.5));
    }

    #[test]
    fn test_data_value_conversions() {
        assert_eq!(DataValue::Single(true).as_bool(), Some(true));
        assert_eq!(DataValue::Single(false).as_bool(), Some(false));
        assert_eq!(DataValue::Float(1.5).as_f64(), Some(1.5));
        assert_eq!(DataValue::Scaled(100).as_f64(), Some(100.0));
        assert_eq!(DataValue::Counter(12345).as_f64(), Some(12345.0));
    }

    #[test]
    fn test_quality_flags() {
        assert!(Quality::Good.is_good());
        assert!(!Quality::Invalid.is_good());

        let q = Quality::from_qds(0x81); // IV + OV
        assert!(q.invalid);
        assert!(q.overflow);
        assert!(!q.blocked);
    }

    #[test]
    fn test_quality_display() {
        assert_eq!(Quality::Good.to_string(), "Good");
        assert_eq!(Quality::Invalid.to_string(), "IV");

        let q = Quality {
            overflow: true,
            invalid: true,
            ..Default::default()
        };
        assert_eq!(q.to_string(), "OV|IV");
    }

    // ============ Additional Data Tests ============

    #[test]
    fn test_data_point_with_quality() {
        let dp = DataPoint::with_quality(
            1001,
            DataValue::Float(50.0),
            Quality::Invalid,
        );
        assert_eq!(dp.ioa, 1001);
        assert!(!dp.is_good());
        assert!(dp.quality.invalid);
    }

    #[test]
    fn test_data_point_with_timestamp() {
        let ts = Cp56Time2a {
            milliseconds: 30000,
            minutes: 30,
            hours: 12,
            day: 15,
            day_of_week: 3,
            month: 6,
            year: 24,
            invalid: false,
            summer_time: false,
        };
        let dp = DataPoint::with_timestamp(
            1002,
            DataValue::Single(true),
            Quality::Good,
            ts,
        );
        assert_eq!(dp.ioa, 1002);
        assert!(dp.timestamp.is_some());
        assert_eq!(dp.timestamp.unwrap().hours, 12);
    }

    #[test]
    fn test_data_value_as_f64_all_types() {
        // Test Single
        assert_eq!(DataValue::Single(true).as_f64(), Some(1.0));
        assert_eq!(DataValue::Single(false).as_f64(), Some(0.0));

        // Test Double
        assert_eq!(DataValue::Double(DoublePointValue::Off).as_f64(), Some(0.0));
        assert_eq!(DataValue::Double(DoublePointValue::On).as_f64(), Some(1.0));
        assert!(DataValue::Double(DoublePointValue::Indeterminate).as_f64().unwrap().is_nan());

        // Test Normalized
        assert_eq!(DataValue::Normalized(0.5).as_f64(), Some(0.5));
        assert_eq!(DataValue::Normalized(-1.0).as_f64(), Some(-1.0));

        // Test Scaled
        assert_eq!(DataValue::Scaled(100).as_f64(), Some(100.0));
        assert_eq!(DataValue::Scaled(-32768).as_f64(), Some(-32768.0));

        // Test Float (use approximate comparison due to f32->f64 precision)
        if let Some(v) = DataValue::Float(123.456).as_f64() {
            assert!((v - 123.456).abs() < 0.001);
        } else {
            panic!("Expected Some");
        }

        // Test Counter
        assert_eq!(DataValue::Counter(1000000).as_f64(), Some(1000000.0));

        // Test Bitstring
        assert_eq!(DataValue::Bitstring(0xDEADBEEF).as_f64(), Some(0xDEADBEEFu32 as f64));

        // Test StepPosition
        assert_eq!(DataValue::StepPosition(-10).as_f64(), Some(-10.0));
        assert_eq!(DataValue::StepPosition(63).as_f64(), Some(63.0));

        // Test BinaryCounter
        let bc = DataValue::BinaryCounter {
            value: 12345,
            sequence: 5,
            carry: false,
            adjusted: false,
            invalid: false,
        };
        assert_eq!(bc.as_f64(), Some(12345.0));
    }

    #[test]
    fn test_data_value_as_bool() {
        // Single values
        assert_eq!(DataValue::Single(true).as_bool(), Some(true));
        assert_eq!(DataValue::Single(false).as_bool(), Some(false));

        // Double values
        assert_eq!(DataValue::Double(DoublePointValue::On).as_bool(), Some(true));
        assert_eq!(DataValue::Double(DoublePointValue::Off).as_bool(), Some(false));
        assert_eq!(DataValue::Double(DoublePointValue::Indeterminate).as_bool(), None);
        assert_eq!(DataValue::Double(DoublePointValue::IndeterminateOrFaulty).as_bool(), None);

        // Non-boolean types return None
        assert_eq!(DataValue::Float(1.0).as_bool(), None);
        assert_eq!(DataValue::Scaled(1).as_bool(), None);
        assert_eq!(DataValue::Counter(1).as_bool(), None);
    }

    #[test]
    fn test_data_value_is_boolean() {
        assert!(DataValue::Single(true).is_boolean());
        assert!(DataValue::Double(DoublePointValue::On).is_boolean());
        assert!(!DataValue::Float(1.0).is_boolean());
        assert!(!DataValue::Scaled(1).is_boolean());
        assert!(!DataValue::Normalized(0.5).is_boolean());
    }

    #[test]
    fn test_data_value_is_numeric() {
        assert!(DataValue::Normalized(0.5).is_numeric());
        assert!(DataValue::Scaled(100).is_numeric());
        assert!(DataValue::Float(1.0).is_numeric());
        assert!(DataValue::Counter(1000).is_numeric());
        assert!(DataValue::StepPosition(10).is_numeric());
        assert!(DataValue::BinaryCounter {
            value: 1,
            sequence: 0,
            carry: false,
            adjusted: false,
            invalid: false,
        }.is_numeric());

        // Boolean types are not numeric
        assert!(!DataValue::Single(true).is_numeric());
        assert!(!DataValue::Double(DoublePointValue::On).is_numeric());

        // Bitstring is not classified as numeric
        assert!(!DataValue::Bitstring(0).is_numeric());
    }

    #[test]
    fn test_quality_from_quality_descriptor() {
        let qd = QualityDescriptor {
            blocked: true,
            substituted: true,
            not_topical: false,
            invalid: false,
        };
        let q = Quality::from_quality_descriptor(qd);
        assert!(q.blocked);
        assert!(q.substituted);
        assert!(!q.not_topical);
        assert!(!q.invalid);
        assert!(!q.overflow); // Not in QualityDescriptor
    }

    #[test]
    fn test_quality_from_measured_quality() {
        let mq = MeasuredQuality {
            overflow: true,
            blocked: false,
            substituted: true,
            not_topical: false,
            invalid: true,
        };
        let q = Quality::from_measured_quality(mq);
        assert!(q.overflow);
        assert!(!q.blocked);
        assert!(q.substituted);
        assert!(!q.not_topical);
        assert!(q.invalid);
    }

    #[test]
    fn test_quality_from_bcr_flags() {
        // Test BCR flags: bits 5=carry, 6=adjusted, 7=invalid
        let q = Quality::from_bcr_flags(0xC0); // invalid + elapsed_time_invalid
        assert!(q.invalid);
        assert!(q.elapsed_time_invalid);
        assert!(!q.overflow);
        assert!(!q.blocked);
    }

    #[test]
    fn test_quality_display_all_flags() {
        let q = Quality {
            overflow: true,
            blocked: true,
            substituted: true,
            not_topical: true,
            invalid: true,
            elapsed_time_invalid: true,
        };
        let display = q.to_string();
        assert!(display.contains("OV"));
        assert!(display.contains("BL"));
        assert!(display.contains("SB"));
        assert!(display.contains("NT"));
        assert!(display.contains("IV"));
        assert!(display.contains("EI"));
    }

    #[test]
    fn test_quality_display_single_flag() {
        let q = Quality { overflow: true, ..Default::default() };
        assert_eq!(q.to_string(), "OV");

        let q = Quality { blocked: true, ..Default::default() };
        assert_eq!(q.to_string(), "BL");

        let q = Quality { substituted: true, ..Default::default() };
        assert_eq!(q.to_string(), "SB");

        let q = Quality { not_topical: true, ..Default::default() };
        assert_eq!(q.to_string(), "NT");

        let q = Quality { elapsed_time_invalid: true, ..Default::default() };
        assert_eq!(q.to_string(), "EI");
    }

    #[test]
    fn test_quality_from_qds_all_combinations() {
        // Test all individual QDS flags
        assert!(Quality::from_qds(0x01).overflow);
        assert!(Quality::from_qds(0x10).blocked);
        assert!(Quality::from_qds(0x20).substituted);
        assert!(Quality::from_qds(0x40).not_topical);
        assert!(Quality::from_qds(0x80).invalid);

        // Test combination
        let q = Quality::from_qds(0xF1);
        assert!(q.overflow);
        assert!(q.blocked);
        assert!(q.substituted);
        assert!(q.not_topical);
        assert!(q.invalid);
    }

    #[test]
    fn test_quality_from_siq_diq_equivalence() {
        // SIQ and DIQ have same quality bit layout
        for byte in [0x00, 0x10, 0x20, 0x40, 0x80, 0xF0] {
            let siq = Quality::from_siq(byte);
            let diq = Quality::from_diq(byte);
            assert_eq!(siq.blocked, diq.blocked);
            assert_eq!(siq.substituted, diq.substituted);
            assert_eq!(siq.not_topical, diq.not_topical);
            assert_eq!(siq.invalid, diq.invalid);
        }
    }

    #[test]
    fn test_data_point_as_f64_method() {
        let dp = DataPoint::new(1, DataValue::Float(99.9));
        assert!((dp.as_f64().unwrap() - 99.9).abs() < 0.001);

        let dp = DataPoint::new(2, DataValue::Single(true));
        assert_eq!(dp.as_f64(), Some(1.0));
    }

    #[test]
    fn test_data_point_as_bool_method() {
        let dp = DataPoint::new(1, DataValue::Single(true));
        assert_eq!(dp.as_bool(), Some(true));

        let dp = DataPoint::new(2, DataValue::Float(1.0));
        assert_eq!(dp.as_bool(), None);
    }
}
