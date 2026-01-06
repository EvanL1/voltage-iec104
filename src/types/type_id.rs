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
    pub fn is_monitoring(&self) -> bool {
        matches!(self.as_u8(), 1..=70)
    }

    /// Check if this type is in the control direction (from master to RTU).
    #[inline]
    pub fn is_control(&self) -> bool {
        matches!(self.as_u8(), 45..=51 | 58..=63 | 100..=107)
    }

    /// Check if this type contains a time tag.
    #[inline]
    pub fn has_time_tag(&self) -> bool {
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
    pub fn standard_name(&self) -> &'static str {
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
}
