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
///
/// Packed into a single byte for cache efficiency. Bit layout:
/// - Bit 0: overflow (OV)
/// - Bit 1: blocked (BL)
/// - Bit 2: substituted (SB)
/// - Bit 3: not_topical (NT)
/// - Bit 4: invalid (IV)
/// - Bit 5: elapsed_time_invalid (EI)
#[derive(Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct Quality(u8);

// Bit masks for quality flags (compile-time constants)
impl Quality {
    const OV_MASK: u8 = 0b0000_0001;
    const BL_MASK: u8 = 0b0000_0010;
    const SB_MASK: u8 = 0b0000_0100;
    const NT_MASK: u8 = 0b0000_1000;
    const IV_MASK: u8 = 0b0001_0000;
    const EI_MASK: u8 = 0b0010_0000;
}

impl Quality {
    /// Overflow (OV) - value exceeds predefined range
    #[inline(always)]
    pub const fn overflow(&self) -> bool {
        (self.0 & Self::OV_MASK) != 0
    }

    /// Set overflow flag
    #[inline(always)]
    pub const fn set_overflow(mut self, value: bool) -> Self {
        if value {
            self.0 |= Self::OV_MASK;
        } else {
            self.0 &= !Self::OV_MASK;
        }
        self
    }

    /// Blocked (BL) - value is blocked for transmission
    #[inline(always)]
    pub const fn blocked(&self) -> bool {
        (self.0 & Self::BL_MASK) != 0
    }

    /// Set blocked flag
    #[inline(always)]
    pub const fn set_blocked(mut self, value: bool) -> Self {
        if value {
            self.0 |= Self::BL_MASK;
        } else {
            self.0 &= !Self::BL_MASK;
        }
        self
    }

    /// Substituted (SB) - value is substituted
    #[inline(always)]
    pub const fn substituted(&self) -> bool {
        (self.0 & Self::SB_MASK) != 0
    }

    /// Set substituted flag
    #[inline(always)]
    pub const fn set_substituted(mut self, value: bool) -> Self {
        if value {
            self.0 |= Self::SB_MASK;
        } else {
            self.0 &= !Self::SB_MASK;
        }
        self
    }

    /// Not topical (NT) - value is not topical (outdated)
    #[inline(always)]
    pub const fn not_topical(&self) -> bool {
        (self.0 & Self::NT_MASK) != 0
    }

    /// Set not_topical flag
    #[inline(always)]
    pub const fn set_not_topical(mut self, value: bool) -> Self {
        if value {
            self.0 |= Self::NT_MASK;
        } else {
            self.0 &= !Self::NT_MASK;
        }
        self
    }

    /// Invalid (IV) - value is invalid
    #[inline(always)]
    pub const fn invalid(&self) -> bool {
        (self.0 & Self::IV_MASK) != 0
    }

    /// Set invalid flag
    #[inline(always)]
    pub const fn set_invalid(mut self, value: bool) -> Self {
        if value {
            self.0 |= Self::IV_MASK;
        } else {
            self.0 &= !Self::IV_MASK;
        }
        self
    }

    /// Elapsed time invalid (for counter values)
    #[inline(always)]
    pub const fn elapsed_time_invalid(&self) -> bool {
        (self.0 & Self::EI_MASK) != 0
    }

    /// Set elapsed_time_invalid flag
    #[inline(always)]
    pub const fn set_elapsed_time_invalid(mut self, value: bool) -> Self {
        if value {
            self.0 |= Self::EI_MASK;
        } else {
            self.0 &= !Self::EI_MASK;
        }
        self
    }

    /// Get the raw packed byte value
    #[inline(always)]
    pub const fn as_raw(&self) -> u8 {
        self.0
    }

    /// Create from raw packed byte value
    #[inline(always)]
    pub const fn from_raw(raw: u8) -> Self {
        Self(raw)
    }
}

impl Quality {
    /// Good quality (all flags false).
    #[allow(non_upper_case_globals)]
    pub const Good: Self = Self(0);

    /// Invalid quality.
    #[allow(non_upper_case_globals)]
    pub const Invalid: Self = Self(Self::IV_MASK);

    /// Create from QualityDescriptor (for single/double point).
    #[inline(always)]
    pub const fn from_quality_descriptor(qd: QualityDescriptor) -> Self {
        let mut raw = 0u8;
        if qd.blocked {
            raw |= Self::BL_MASK;
        }
        if qd.substituted {
            raw |= Self::SB_MASK;
        }
        if qd.not_topical {
            raw |= Self::NT_MASK;
        }
        if qd.invalid {
            raw |= Self::IV_MASK;
        }
        Self(raw)
    }

    /// Create from MeasuredQuality (for measured values).
    #[inline(always)]
    pub const fn from_measured_quality(mq: MeasuredQuality) -> Self {
        let mut raw = 0u8;
        if mq.overflow {
            raw |= Self::OV_MASK;
        }
        if mq.blocked {
            raw |= Self::BL_MASK;
        }
        if mq.substituted {
            raw |= Self::SB_MASK;
        }
        if mq.not_topical {
            raw |= Self::NT_MASK;
        }
        if mq.invalid {
            raw |= Self::IV_MASK;
        }
        Self(raw)
    }

    /// Parse from QDS byte (Quality Descriptor for measured values).
    /// Direct bit mapping for zero-cost parsing.
    #[inline(always)]
    pub const fn from_qds(byte: u8) -> Self {
        // QDS layout: IV(7) NT(6) SB(5) BL(4) _ _ _ OV(0)
        // Our layout:  _ _ EI(5) IV(4) NT(3) SB(2) BL(1) OV(0)
        let mut raw = 0u8;
        if (byte & 0x01) != 0 {
            raw |= Self::OV_MASK;
        }
        if (byte & 0x10) != 0 {
            raw |= Self::BL_MASK;
        }
        if (byte & 0x20) != 0 {
            raw |= Self::SB_MASK;
        }
        if (byte & 0x40) != 0 {
            raw |= Self::NT_MASK;
        }
        if (byte & 0x80) != 0 {
            raw |= Self::IV_MASK;
        }
        Self(raw)
    }

    /// Parse from SIQ byte (Single-point Information with Quality).
    #[inline(always)]
    pub const fn from_siq(byte: u8) -> Self {
        // SIQ layout: IV(7) NT(6) SB(5) BL(4) _ _ _ SPI(0)
        let mut raw = 0u8;
        if (byte & 0x10) != 0 {
            raw |= Self::BL_MASK;
        }
        if (byte & 0x20) != 0 {
            raw |= Self::SB_MASK;
        }
        if (byte & 0x40) != 0 {
            raw |= Self::NT_MASK;
        }
        if (byte & 0x80) != 0 {
            raw |= Self::IV_MASK;
        }
        Self(raw)
    }

    /// Parse from DIQ byte (Double-point Information with Quality).
    #[inline(always)]
    pub const fn from_diq(byte: u8) -> Self {
        // DIQ layout: IV(7) NT(6) SB(5) BL(4) _ _ DPI(1:0)
        // Same quality bit positions as SIQ
        Self::from_siq(byte)
    }

    /// Parse from BCR flags (Binary Counter Reading).
    #[inline(always)]
    pub const fn from_bcr_flags(byte: u8) -> Self {
        // BCR flags: IV(7) CA(6) CY(5) SQ(4:0)
        let mut raw = 0u8;
        if (byte & 0x80) != 0 {
            raw |= Self::IV_MASK;
        }
        if (byte & 0x40) != 0 {
            raw |= Self::EI_MASK; // CA maps to elapsed_time_invalid
        }
        Self(raw)
    }

    /// Check if quality is good (no flags set).
    /// Single comparison for maximum efficiency.
    #[inline(always)]
    pub const fn is_good(&self) -> bool {
        self.0 == 0
    }

    /// Create a new Quality with only the invalid flag set based on a boolean.
    #[inline(always)]
    pub const fn with_invalid(invalid: bool) -> Self {
        if invalid {
            Self(Self::IV_MASK)
        } else {
            Self(0)
        }
    }
}

impl std::fmt::Debug for Quality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Quality")
            .field("overflow", &self.overflow())
            .field("blocked", &self.blocked())
            .field("substituted", &self.substituted())
            .field("not_topical", &self.not_topical())
            .field("invalid", &self.invalid())
            .field("elapsed_time_invalid", &self.elapsed_time_invalid())
            .finish()
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

        if self.overflow() {
            write_flag(f, "OV")?;
        }
        if self.blocked() {
            write_flag(f, "BL")?;
        }
        if self.substituted() {
            write_flag(f, "SB")?;
        }
        if self.not_topical() {
            write_flag(f, "NT")?;
        }
        if self.invalid() {
            write_flag(f, "IV")?;
        }
        if self.elapsed_time_invalid() {
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
        assert!(q.invalid());
        assert!(q.overflow());
        assert!(!q.blocked());
    }

    #[test]
    fn test_quality_display() {
        assert_eq!(Quality::Good.to_string(), "Good");
        assert_eq!(Quality::Invalid.to_string(), "IV");

        let q = Quality::Good.set_overflow(true).set_invalid(true);
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
        assert!(dp.quality.invalid());
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
        assert!(q.blocked());
        assert!(q.substituted());
        assert!(!q.not_topical());
        assert!(!q.invalid());
        assert!(!q.overflow()); // Not in QualityDescriptor
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
        assert!(q.overflow());
        assert!(!q.blocked());
        assert!(q.substituted());
        assert!(!q.not_topical());
        assert!(q.invalid());
    }

    #[test]
    fn test_quality_from_bcr_flags() {
        // Test BCR flags: bits 5=carry, 6=adjusted, 7=invalid
        let q = Quality::from_bcr_flags(0xC0); // invalid + elapsed_time_invalid
        assert!(q.invalid());
        assert!(q.elapsed_time_invalid());
        assert!(!q.overflow());
        assert!(!q.blocked());
    }

    #[test]
    fn test_quality_display_all_flags() {
        let q = Quality::Good
            .set_overflow(true)
            .set_blocked(true)
            .set_substituted(true)
            .set_not_topical(true)
            .set_invalid(true)
            .set_elapsed_time_invalid(true);
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
        let q = Quality::Good.set_overflow(true);
        assert_eq!(q.to_string(), "OV");

        let q = Quality::Good.set_blocked(true);
        assert_eq!(q.to_string(), "BL");

        let q = Quality::Good.set_substituted(true);
        assert_eq!(q.to_string(), "SB");

        let q = Quality::Good.set_not_topical(true);
        assert_eq!(q.to_string(), "NT");

        let q = Quality::Good.set_elapsed_time_invalid(true);
        assert_eq!(q.to_string(), "EI");
    }

    #[test]
    fn test_quality_from_qds_all_combinations() {
        // Test all individual QDS flags
        assert!(Quality::from_qds(0x01).overflow());
        assert!(Quality::from_qds(0x10).blocked());
        assert!(Quality::from_qds(0x20).substituted());
        assert!(Quality::from_qds(0x40).not_topical());
        assert!(Quality::from_qds(0x80).invalid());

        // Test combination
        let q = Quality::from_qds(0xF1);
        assert!(q.overflow());
        assert!(q.blocked());
        assert!(q.substituted());
        assert!(q.not_topical());
        assert!(q.invalid());
    }

    #[test]
    fn test_quality_from_siq_diq_equivalence() {
        // SIQ and DIQ have same quality bit layout
        for byte in [0x00, 0x10, 0x20, 0x40, 0x80, 0xF0] {
            let siq = Quality::from_siq(byte);
            let diq = Quality::from_diq(byte);
            assert_eq!(siq.blocked(), diq.blocked());
            assert_eq!(siq.substituted(), diq.substituted());
            assert_eq!(siq.not_topical(), diq.not_topical());
            assert_eq!(siq.invalid(), diq.invalid());
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

    // ============ Quality Packed Struct Tests ============

    #[test]
    fn test_quality_packed_size() {
        // Verify Quality is only 1 byte (packed into u8)
        assert_eq!(std::mem::size_of::<Quality>(), 1);
    }

    #[test]
    fn test_quality_builder_pattern() {
        // Test chained builder pattern
        let q = Quality::Good
            .set_overflow(true)
            .set_blocked(true)
            .set_substituted(true)
            .set_not_topical(true)
            .set_invalid(true)
            .set_elapsed_time_invalid(true);

        assert!(q.overflow());
        assert!(q.blocked());
        assert!(q.substituted());
        assert!(q.not_topical());
        assert!(q.invalid());
        assert!(q.elapsed_time_invalid());
        assert!(!q.is_good());
    }

    #[test]
    fn test_quality_builder_toggle() {
        // Test setting and unsetting flags
        let q = Quality::Good.set_invalid(true);
        assert!(q.invalid());

        let q = q.set_invalid(false);
        assert!(!q.invalid());
        assert!(q.is_good());
    }

    #[test]
    fn test_quality_raw_roundtrip() {
        // Test raw byte roundtrip
        for raw in [0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x3F] {
            let q = Quality::from_raw(raw);
            assert_eq!(q.as_raw(), raw);
        }
    }

    #[test]
    fn test_quality_with_invalid_constructor() {
        // Test the with_invalid convenience constructor
        let q = Quality::with_invalid(false);
        assert!(q.is_good());

        let q = Quality::with_invalid(true);
        assert!(!q.is_good());
        assert!(q.invalid());
    }

    #[test]
    fn test_quality_const_evaluation() {
        // Verify Quality methods are const-evaluable
        const GOOD: Quality = Quality::Good;
        const INVALID: Quality = Quality::Invalid;
        const IS_GOOD: bool = GOOD.is_good();
        const IS_BAD: bool = INVALID.is_good();

        assert!(IS_GOOD);
        assert!(!IS_BAD);
    }

    #[test]
    fn test_quality_bit_isolation() {
        // Test each bit is isolated correctly
        let q = Quality::from_raw(0x01); // OV only
        assert!(q.overflow());
        assert!(!q.blocked());

        let q = Quality::from_raw(0x02); // BL only
        assert!(!q.overflow());
        assert!(q.blocked());
        assert!(!q.substituted());

        let q = Quality::from_raw(0x04); // SB only
        assert!(q.substituted());
        assert!(!q.not_topical());

        let q = Quality::from_raw(0x08); // NT only
        assert!(q.not_topical());
        assert!(!q.invalid());

        let q = Quality::from_raw(0x10); // IV only
        assert!(q.invalid());
        assert!(!q.elapsed_time_invalid());

        let q = Quality::from_raw(0x20); // EI only
        assert!(q.elapsed_time_invalid());
    }
}
