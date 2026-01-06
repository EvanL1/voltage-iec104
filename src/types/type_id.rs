//! IEC 60870-5-104 Type Identification.
//!
//! Type identification defines the structure and meaning of information objects.

use crate::error::{Iec104Error, Result};

/// IEC 60870-5-104 Type Identification.
///
/// Defines the type of information contained in an ASDU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TypeId {
    // ============================================
    // Process information in monitoring direction
    // ============================================
    /// Single-point information (M_SP_NA_1)
    SinglePoint = 1,

    /// Single-point information with time tag (M_SP_TA_1)
    SinglePointTime24 = 2,

    /// Double-point information (M_DP_NA_1)
    DoublePoint = 3,

    /// Double-point information with time tag (M_DP_TA_1)
    DoublePointTime24 = 4,

    /// Step position information (M_ST_NA_1)
    StepPosition = 5,

    /// Bitstring of 32 bit (M_BO_NA_1)
    Bitstring32 = 7,

    /// Measured value, normalized (M_ME_NA_1)
    MeasuredNormalized = 9,

    /// Measured value, normalized with time tag (M_ME_TA_1)
    MeasuredNormalizedTime24 = 10,

    /// Measured value, scaled (M_ME_NB_1)
    MeasuredScaled = 11,

    /// Measured value, scaled with time tag (M_ME_TB_1)
    MeasuredScaledTime24 = 12,

    /// Measured value, short floating point (M_ME_NC_1)
    MeasuredFloat = 13,

    /// Measured value, short floating point with time tag (M_ME_TC_1)
    MeasuredFloatTime24 = 14,

    /// Integrated totals (M_IT_NA_1)
    IntegratedTotals = 15,

    /// Single-point information with time tag CP56Time2a (M_SP_TB_1)
    SinglePointTime56 = 30,

    /// Double-point information with time tag CP56Time2a (M_DP_TB_1)
    DoublePointTime56 = 31,

    /// Measured value, short floating point with time tag CP56Time2a (M_ME_TF_1)
    MeasuredFloatTime56 = 36,

    // ============================================
    // Process information in control direction
    // ============================================
    /// Single command (C_SC_NA_1)
    SingleCommand = 45,

    /// Double command (C_DC_NA_1)
    DoubleCommand = 46,

    /// Regulating step command (C_RC_NA_1)
    RegulatingStep = 47,

    /// Set-point command, normalized (C_SE_NA_1)
    SetpointNormalized = 48,

    /// Set-point command, scaled (C_SE_NB_1)
    SetpointScaled = 49,

    /// Set-point command, short floating point (C_SE_NC_1)
    SetpointFloat = 50,

    /// Bitstring of 32 bit command (C_BO_NA_1)
    Bitstring32Command = 51,

    /// Single command with time tag CP56Time2a (C_SC_TA_1)
    SingleCommandTime56 = 58,

    /// Double command with time tag CP56Time2a (C_DC_TA_1)
    DoubleCommandTime56 = 59,

    /// Set-point command, short floating point with time tag CP56Time2a (C_SE_TC_1)
    SetpointFloatTime56 = 63,

    // ============================================
    // System information in monitoring direction
    // ============================================
    /// End of initialization (M_EI_NA_1)
    EndOfInit = 70,

    // ============================================
    // System information in control direction
    // ============================================
    /// Interrogation command (C_IC_NA_1)
    InterrogationCommand = 100,

    /// Counter interrogation command (C_CI_NA_1)
    CounterInterrogation = 101,

    /// Read command (C_RD_NA_1)
    ReadCommand = 102,

    /// Clock synchronization command (C_CS_NA_1)
    ClockSync = 103,

    /// Test command (C_TS_NA_1)
    TestCommand = 104,

    /// Reset process command (C_RP_NA_1)
    ResetProcess = 105,

    /// Test command with time tag CP56Time2a (C_TS_TA_1)
    TestCommandTime56 = 107,
}

impl TypeId {
    /// Create TypeId from raw byte value.
    #[inline]
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::SinglePoint),
            2 => Ok(Self::SinglePointTime24),
            3 => Ok(Self::DoublePoint),
            4 => Ok(Self::DoublePointTime24),
            5 => Ok(Self::StepPosition),
            7 => Ok(Self::Bitstring32),
            9 => Ok(Self::MeasuredNormalized),
            10 => Ok(Self::MeasuredNormalizedTime24),
            11 => Ok(Self::MeasuredScaled),
            12 => Ok(Self::MeasuredScaledTime24),
            13 => Ok(Self::MeasuredFloat),
            14 => Ok(Self::MeasuredFloatTime24),
            15 => Ok(Self::IntegratedTotals),
            30 => Ok(Self::SinglePointTime56),
            31 => Ok(Self::DoublePointTime56),
            36 => Ok(Self::MeasuredFloatTime56),
            45 => Ok(Self::SingleCommand),
            46 => Ok(Self::DoubleCommand),
            47 => Ok(Self::RegulatingStep),
            48 => Ok(Self::SetpointNormalized),
            49 => Ok(Self::SetpointScaled),
            50 => Ok(Self::SetpointFloat),
            51 => Ok(Self::Bitstring32Command),
            58 => Ok(Self::SingleCommandTime56),
            59 => Ok(Self::DoubleCommandTime56),
            63 => Ok(Self::SetpointFloatTime56),
            70 => Ok(Self::EndOfInit),
            100 => Ok(Self::InterrogationCommand),
            101 => Ok(Self::CounterInterrogation),
            102 => Ok(Self::ReadCommand),
            103 => Ok(Self::ClockSync),
            104 => Ok(Self::TestCommand),
            105 => Ok(Self::ResetProcess),
            107 => Ok(Self::TestCommandTime56),
            _ => Err(Iec104Error::UnknownTypeId(value)),
        }
    }

    /// Convert to raw byte value.
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Check if this type is in the monitoring direction (from RTU to master).
    #[inline]
    pub const fn is_monitoring(&self) -> bool {
        matches!(self.as_u8(), 1..=70)
    }

    /// Check if this type is in the control direction (from master to RTU).
    #[inline]
    pub const fn is_control(&self) -> bool {
        matches!(self.as_u8(), 45..=51 | 58..=63 | 100..=107)
    }

    /// Check if this type contains a time tag.
    #[inline]
    pub const fn has_time_tag(&self) -> bool {
        matches!(
            self,
            Self::SinglePointTime24
                | Self::DoublePointTime24
                | Self::MeasuredNormalizedTime24
                | Self::MeasuredScaledTime24
                | Self::MeasuredFloatTime24
                | Self::SinglePointTime56
                | Self::DoublePointTime56
                | Self::MeasuredFloatTime56
                | Self::SingleCommandTime56
                | Self::DoubleCommandTime56
                | Self::SetpointFloatTime56
                | Self::TestCommandTime56
        )
    }

    /// Get the IEC standard name (e.g., "M_SP_NA_1").
    #[inline]
    pub const fn standard_name(&self) -> &'static str {
        match self {
            Self::SinglePoint => "M_SP_NA_1",
            Self::SinglePointTime24 => "M_SP_TA_1",
            Self::DoublePoint => "M_DP_NA_1",
            Self::DoublePointTime24 => "M_DP_TA_1",
            Self::StepPosition => "M_ST_NA_1",
            Self::Bitstring32 => "M_BO_NA_1",
            Self::MeasuredNormalized => "M_ME_NA_1",
            Self::MeasuredNormalizedTime24 => "M_ME_TA_1",
            Self::MeasuredScaled => "M_ME_NB_1",
            Self::MeasuredScaledTime24 => "M_ME_TB_1",
            Self::MeasuredFloat => "M_ME_NC_1",
            Self::MeasuredFloatTime24 => "M_ME_TC_1",
            Self::IntegratedTotals => "M_IT_NA_1",
            Self::SinglePointTime56 => "M_SP_TB_1",
            Self::DoublePointTime56 => "M_DP_TB_1",
            Self::MeasuredFloatTime56 => "M_ME_TF_1",
            Self::SingleCommand => "C_SC_NA_1",
            Self::DoubleCommand => "C_DC_NA_1",
            Self::RegulatingStep => "C_RC_NA_1",
            Self::SetpointNormalized => "C_SE_NA_1",
            Self::SetpointScaled => "C_SE_NB_1",
            Self::SetpointFloat => "C_SE_NC_1",
            Self::Bitstring32Command => "C_BO_NA_1",
            Self::SingleCommandTime56 => "C_SC_TA_1",
            Self::DoubleCommandTime56 => "C_DC_TA_1",
            Self::SetpointFloatTime56 => "C_SE_TC_1",
            Self::EndOfInit => "M_EI_NA_1",
            Self::InterrogationCommand => "C_IC_NA_1",
            Self::CounterInterrogation => "C_CI_NA_1",
            Self::ReadCommand => "C_RD_NA_1",
            Self::ClockSync => "C_CS_NA_1",
            Self::TestCommand => "C_TS_NA_1",
            Self::ResetProcess => "C_RP_NA_1",
            Self::TestCommandTime56 => "C_TS_TA_1",
        }
    }
}

impl std::fmt::Display for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.standard_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_id_from_u8() {
        assert_eq!(TypeId::from_u8(1).unwrap(), TypeId::SinglePoint);
        assert_eq!(TypeId::from_u8(13).unwrap(), TypeId::MeasuredFloat);
        assert_eq!(TypeId::from_u8(100).unwrap(), TypeId::InterrogationCommand);
        assert!(TypeId::from_u8(255).is_err());
    }

    #[test]
    fn test_type_id_direction() {
        assert!(TypeId::SinglePoint.is_monitoring());
        assert!(!TypeId::SinglePoint.is_control());

        assert!(TypeId::SingleCommand.is_control());
        assert!(TypeId::InterrogationCommand.is_control());
    }

    #[test]
    fn test_type_id_time_tag() {
        assert!(!TypeId::SinglePoint.has_time_tag());
        assert!(TypeId::SinglePointTime24.has_time_tag());
        assert!(TypeId::MeasuredFloatTime56.has_time_tag());
    }

    #[test]
    fn test_type_id_standard_name() {
        assert_eq!(TypeId::SinglePoint.standard_name(), "M_SP_NA_1");
        assert_eq!(TypeId::MeasuredFloat.standard_name(), "M_ME_NC_1");
        assert_eq!(TypeId::SingleCommand.standard_name(), "C_SC_NA_1");
    }

    // ============ Additional TypeId Tests ============

    #[test]
    fn test_type_id_all_values_roundtrip() {
        let valid_values = [
            1, 2, 3, 4, 5, 7, 9, 10, 11, 12, 13, 14, 15,
            30, 31, 36,
            45, 46, 47, 48, 49, 50, 51,
            58, 59, 63,
            70,
            100, 101, 102, 103, 104, 105, 107,
        ];

        for val in valid_values {
            let type_id = TypeId::from_u8(val).unwrap();
            assert_eq!(type_id.as_u8(), val, "Roundtrip failed for value {}", val);
        }
    }

    #[test]
    fn test_type_id_invalid_values() {
        // Test some invalid type IDs
        let invalid_values = [0, 6, 8, 16, 17, 29, 32, 44, 52, 60, 71, 99, 106, 108, 200, 255];

        for val in invalid_values {
            let result = TypeId::from_u8(val);
            assert!(result.is_err(), "Expected error for TypeId value {}", val);
        }
    }

    #[test]
    fn test_type_id_monitoring_types() {
        // All M_ types should be monitoring
        let monitoring_types = [
            TypeId::SinglePoint,
            TypeId::SinglePointTime24,
            TypeId::DoublePoint,
            TypeId::DoublePointTime24,
            TypeId::StepPosition,
            TypeId::Bitstring32,
            TypeId::MeasuredNormalized,
            TypeId::MeasuredNormalizedTime24,
            TypeId::MeasuredScaled,
            TypeId::MeasuredScaledTime24,
            TypeId::MeasuredFloat,
            TypeId::MeasuredFloatTime24,
            TypeId::IntegratedTotals,
            TypeId::SinglePointTime56,
            TypeId::DoublePointTime56,
            TypeId::MeasuredFloatTime56,
            TypeId::EndOfInit,
        ];

        for type_id in monitoring_types {
            assert!(type_id.is_monitoring(), "{:?} should be monitoring", type_id);
        }
    }

    #[test]
    fn test_type_id_control_types() {
        // All C_ types should be control
        let control_types = [
            TypeId::SingleCommand,
            TypeId::DoubleCommand,
            TypeId::RegulatingStep,
            TypeId::SetpointNormalized,
            TypeId::SetpointScaled,
            TypeId::SetpointFloat,
            TypeId::Bitstring32Command,
            TypeId::SingleCommandTime56,
            TypeId::DoubleCommandTime56,
            TypeId::SetpointFloatTime56,
            TypeId::InterrogationCommand,
            TypeId::CounterInterrogation,
            TypeId::ReadCommand,
            TypeId::ClockSync,
            TypeId::TestCommand,
            TypeId::ResetProcess,
            TypeId::TestCommandTime56,
        ];

        for type_id in control_types {
            assert!(type_id.is_control(), "{:?} should be control", type_id);
        }
    }

    #[test]
    fn test_type_id_time_tagged_types() {
        // Types with time tags
        let time_tagged = [
            TypeId::SinglePointTime24,
            TypeId::DoublePointTime24,
            TypeId::MeasuredNormalizedTime24,
            TypeId::MeasuredScaledTime24,
            TypeId::MeasuredFloatTime24,
            TypeId::SinglePointTime56,
            TypeId::DoublePointTime56,
            TypeId::MeasuredFloatTime56,
            TypeId::SingleCommandTime56,
            TypeId::DoubleCommandTime56,
            TypeId::SetpointFloatTime56,
            TypeId::TestCommandTime56,
        ];

        for type_id in time_tagged {
            assert!(type_id.has_time_tag(), "{:?} should have time tag", type_id);
        }

        // Types without time tags
        let no_time_tag = [
            TypeId::SinglePoint,
            TypeId::DoublePoint,
            TypeId::MeasuredFloat,
            TypeId::SingleCommand,
            TypeId::InterrogationCommand,
        ];

        for type_id in no_time_tag {
            assert!(!type_id.has_time_tag(), "{:?} should not have time tag", type_id);
        }
    }

    #[test]
    fn test_type_id_display() {
        // Display should use standard name
        assert_eq!(format!("{}", TypeId::SinglePoint), "M_SP_NA_1");
        assert_eq!(format!("{}", TypeId::MeasuredFloat), "M_ME_NC_1");
        assert_eq!(format!("{}", TypeId::InterrogationCommand), "C_IC_NA_1");
    }

    #[test]
    fn test_type_id_all_standard_names() {
        // Verify all types have proper standard names
        let types_and_names = [
            (TypeId::SinglePoint, "M_SP_NA_1"),
            (TypeId::SinglePointTime24, "M_SP_TA_1"),
            (TypeId::DoublePoint, "M_DP_NA_1"),
            (TypeId::DoublePointTime24, "M_DP_TA_1"),
            (TypeId::StepPosition, "M_ST_NA_1"),
            (TypeId::Bitstring32, "M_BO_NA_1"),
            (TypeId::MeasuredNormalized, "M_ME_NA_1"),
            (TypeId::MeasuredNormalizedTime24, "M_ME_TA_1"),
            (TypeId::MeasuredScaled, "M_ME_NB_1"),
            (TypeId::MeasuredScaledTime24, "M_ME_TB_1"),
            (TypeId::MeasuredFloat, "M_ME_NC_1"),
            (TypeId::MeasuredFloatTime24, "M_ME_TC_1"),
            (TypeId::IntegratedTotals, "M_IT_NA_1"),
            (TypeId::SinglePointTime56, "M_SP_TB_1"),
            (TypeId::DoublePointTime56, "M_DP_TB_1"),
            (TypeId::MeasuredFloatTime56, "M_ME_TF_1"),
            (TypeId::SingleCommand, "C_SC_NA_1"),
            (TypeId::DoubleCommand, "C_DC_NA_1"),
            (TypeId::RegulatingStep, "C_RC_NA_1"),
            (TypeId::SetpointNormalized, "C_SE_NA_1"),
            (TypeId::SetpointScaled, "C_SE_NB_1"),
            (TypeId::SetpointFloat, "C_SE_NC_1"),
            (TypeId::Bitstring32Command, "C_BO_NA_1"),
            (TypeId::SingleCommandTime56, "C_SC_TA_1"),
            (TypeId::DoubleCommandTime56, "C_DC_TA_1"),
            (TypeId::SetpointFloatTime56, "C_SE_TC_1"),
            (TypeId::EndOfInit, "M_EI_NA_1"),
            (TypeId::InterrogationCommand, "C_IC_NA_1"),
            (TypeId::CounterInterrogation, "C_CI_NA_1"),
            (TypeId::ReadCommand, "C_RD_NA_1"),
            (TypeId::ClockSync, "C_CS_NA_1"),
            (TypeId::TestCommand, "C_TS_NA_1"),
            (TypeId::ResetProcess, "C_RP_NA_1"),
            (TypeId::TestCommandTime56, "C_TS_TA_1"),
        ];

        for (type_id, expected_name) in types_and_names {
            assert_eq!(type_id.standard_name(), expected_name, "Wrong name for {:?}", type_id);
        }
    }

    #[test]
    fn test_type_id_numeric_values() {
        // Verify specific numeric values
        assert_eq!(TypeId::SinglePoint.as_u8(), 1);
        assert_eq!(TypeId::DoublePoint.as_u8(), 3);
        assert_eq!(TypeId::MeasuredFloat.as_u8(), 13);
        assert_eq!(TypeId::SinglePointTime56.as_u8(), 30);
        assert_eq!(TypeId::SingleCommand.as_u8(), 45);
        assert_eq!(TypeId::EndOfInit.as_u8(), 70);
        assert_eq!(TypeId::InterrogationCommand.as_u8(), 100);
        assert_eq!(TypeId::TestCommandTime56.as_u8(), 107);
    }
}
