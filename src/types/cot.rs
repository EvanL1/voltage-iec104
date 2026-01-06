//! IEC 60870-5-104 Cause of Transmission (COT).
//!
//! The cause of transmission defines the reason for sending an ASDU.

use crate::error::{Iec104Error, Result};

/// Cause of Transmission (COT).
///
/// Defines the reason for transmission of an ASDU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Cot {
    /// Periodic, cyclic (1)
    Periodic = 1,

    /// Background scan (2)
    Background = 2,

    /// Spontaneous (3)
    Spontaneous = 3,

    /// Initialized (4)
    Initialized = 4,

    /// Request or requested (5)
    Request = 5,

    /// Activation (6)
    Activation = 6,

    /// Activation confirmation (7)
    ActivationConfirm = 7,

    /// Deactivation (8)
    Deactivation = 8,

    /// Deactivation confirmation (9)
    DeactivationConfirm = 9,

    /// Activation termination (10)
    ActivationTermination = 10,

    /// Return information caused by a remote command (11)
    ReturnRemoteCommand = 11,

    /// Return information caused by a local command (12)
    ReturnLocalCommand = 12,

    /// File transfer (13)
    FileTransfer = 13,

    /// Interrogated by station interrogation (20)
    InterrogatedByStation = 20,

    /// Interrogated by group 1 interrogation (21)
    InterrogatedByGroup1 = 21,

    /// Interrogated by group 2 interrogation (22)
    InterrogatedByGroup2 = 22,

    /// Interrogated by group 3 interrogation (23)
    InterrogatedByGroup3 = 23,

    /// Interrogated by group 4 interrogation (24)
    InterrogatedByGroup4 = 24,

    /// Interrogated by group 5 interrogation (25)
    InterrogatedByGroup5 = 25,

    /// Interrogated by group 6 interrogation (26)
    InterrogatedByGroup6 = 26,

    /// Interrogated by group 7 interrogation (27)
    InterrogatedByGroup7 = 27,

    /// Interrogated by group 8 interrogation (28)
    InterrogatedByGroup8 = 28,

    /// Interrogated by group 9 interrogation (29)
    InterrogatedByGroup9 = 29,

    /// Interrogated by group 10 interrogation (30)
    InterrogatedByGroup10 = 30,

    /// Interrogated by group 11 interrogation (31)
    InterrogatedByGroup11 = 31,

    /// Interrogated by group 12 interrogation (32)
    InterrogatedByGroup12 = 32,

    /// Interrogated by group 13 interrogation (33)
    InterrogatedByGroup13 = 33,

    /// Interrogated by group 14 interrogation (34)
    InterrogatedByGroup14 = 34,

    /// Interrogated by group 15 interrogation (35)
    InterrogatedByGroup15 = 35,

    /// Interrogated by group 16 interrogation (36)
    InterrogatedByGroup16 = 36,

    /// Requested by general counter request (37)
    RequestedByGeneralCounter = 37,

    /// Requested by group 1 counter request (38)
    RequestedByGroup1Counter = 38,

    /// Requested by group 2 counter request (39)
    RequestedByGroup2Counter = 39,

    /// Requested by group 3 counter request (40)
    RequestedByGroup3Counter = 40,

    /// Requested by group 4 counter request (41)
    RequestedByGroup4Counter = 41,

    /// Unknown type identification (44)
    UnknownTypeId = 44,

    /// Unknown cause of transmission (45)
    UnknownCot = 45,

    /// Unknown common address of ASDU (46)
    UnknownCommonAddress = 46,

    /// Unknown information object address (47)
    UnknownIoa = 47,
}

impl Cot {
    /// Create COT from raw byte value (lower 6 bits).
    #[inline]
    pub fn from_u8(value: u8) -> Result<Self> {
        // COT is in the lower 6 bits
        let cot_value = value & 0x3F;

        match cot_value {
            1 => Ok(Self::Periodic),
            2 => Ok(Self::Background),
            3 => Ok(Self::Spontaneous),
            4 => Ok(Self::Initialized),
            5 => Ok(Self::Request),
            6 => Ok(Self::Activation),
            7 => Ok(Self::ActivationConfirm),
            8 => Ok(Self::Deactivation),
            9 => Ok(Self::DeactivationConfirm),
            10 => Ok(Self::ActivationTermination),
            11 => Ok(Self::ReturnRemoteCommand),
            12 => Ok(Self::ReturnLocalCommand),
            13 => Ok(Self::FileTransfer),
            20 => Ok(Self::InterrogatedByStation),
            21 => Ok(Self::InterrogatedByGroup1),
            22 => Ok(Self::InterrogatedByGroup2),
            23 => Ok(Self::InterrogatedByGroup3),
            24 => Ok(Self::InterrogatedByGroup4),
            25 => Ok(Self::InterrogatedByGroup5),
            26 => Ok(Self::InterrogatedByGroup6),
            27 => Ok(Self::InterrogatedByGroup7),
            28 => Ok(Self::InterrogatedByGroup8),
            29 => Ok(Self::InterrogatedByGroup9),
            30 => Ok(Self::InterrogatedByGroup10),
            31 => Ok(Self::InterrogatedByGroup11),
            32 => Ok(Self::InterrogatedByGroup12),
            33 => Ok(Self::InterrogatedByGroup13),
            34 => Ok(Self::InterrogatedByGroup14),
            35 => Ok(Self::InterrogatedByGroup15),
            36 => Ok(Self::InterrogatedByGroup16),
            37 => Ok(Self::RequestedByGeneralCounter),
            38 => Ok(Self::RequestedByGroup1Counter),
            39 => Ok(Self::RequestedByGroup2Counter),
            40 => Ok(Self::RequestedByGroup3Counter),
            41 => Ok(Self::RequestedByGroup4Counter),
            44 => Ok(Self::UnknownTypeId),
            45 => Ok(Self::UnknownCot),
            46 => Ok(Self::UnknownCommonAddress),
            47 => Ok(Self::UnknownIoa),
            // Use static error to avoid allocation; actual value rarely needed in production
            _ => Err(Iec104Error::protocol_static("Unknown COT")),
        }
    }

    /// Convert to raw byte value.
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Check if this is a positive confirmation.
    #[inline]
    pub const fn is_positive(&self) -> bool {
        matches!(
            self,
            Self::ActivationConfirm | Self::DeactivationConfirm | Self::ActivationTermination
        )
    }

    /// Check if this is a negative confirmation.
    #[inline]
    pub const fn is_negative(&self) -> bool {
        matches!(
            self,
            Self::UnknownTypeId | Self::UnknownCot | Self::UnknownCommonAddress | Self::UnknownIoa
        )
    }

    /// Check if this COT indicates an interrogation response.
    #[inline]
    pub const fn is_interrogation_response(&self) -> bool {
        matches!(self.as_u8(), 20..=36)
    }

    /// Check if this COT indicates a counter request response.
    #[inline]
    pub const fn is_counter_response(&self) -> bool {
        matches!(self.as_u8(), 37..=41)
    }
}

impl std::fmt::Display for Cot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Periodic => write!(f, "Periodic"),
            Self::Background => write!(f, "Background"),
            Self::Spontaneous => write!(f, "Spontaneous"),
            Self::Initialized => write!(f, "Initialized"),
            Self::Request => write!(f, "Request"),
            Self::Activation => write!(f, "Activation"),
            Self::ActivationConfirm => write!(f, "ActivationConfirm"),
            Self::Deactivation => write!(f, "Deactivation"),
            Self::DeactivationConfirm => write!(f, "DeactivationConfirm"),
            Self::ActivationTermination => write!(f, "ActivationTermination"),
            Self::ReturnRemoteCommand => write!(f, "ReturnRemoteCommand"),
            Self::ReturnLocalCommand => write!(f, "ReturnLocalCommand"),
            Self::FileTransfer => write!(f, "FileTransfer"),
            Self::InterrogatedByStation => write!(f, "InterrogatedByStation"),
            Self::InterrogatedByGroup1 => write!(f, "InterrogatedByGroup1"),
            Self::InterrogatedByGroup2 => write!(f, "InterrogatedByGroup2"),
            Self::InterrogatedByGroup3 => write!(f, "InterrogatedByGroup3"),
            Self::InterrogatedByGroup4 => write!(f, "InterrogatedByGroup4"),
            Self::InterrogatedByGroup5 => write!(f, "InterrogatedByGroup5"),
            Self::InterrogatedByGroup6 => write!(f, "InterrogatedByGroup6"),
            Self::InterrogatedByGroup7 => write!(f, "InterrogatedByGroup7"),
            Self::InterrogatedByGroup8 => write!(f, "InterrogatedByGroup8"),
            Self::InterrogatedByGroup9 => write!(f, "InterrogatedByGroup9"),
            Self::InterrogatedByGroup10 => write!(f, "InterrogatedByGroup10"),
            Self::InterrogatedByGroup11 => write!(f, "InterrogatedByGroup11"),
            Self::InterrogatedByGroup12 => write!(f, "InterrogatedByGroup12"),
            Self::InterrogatedByGroup13 => write!(f, "InterrogatedByGroup13"),
            Self::InterrogatedByGroup14 => write!(f, "InterrogatedByGroup14"),
            Self::InterrogatedByGroup15 => write!(f, "InterrogatedByGroup15"),
            Self::InterrogatedByGroup16 => write!(f, "InterrogatedByGroup16"),
            Self::RequestedByGeneralCounter => write!(f, "RequestedByGeneralCounter"),
            Self::RequestedByGroup1Counter => write!(f, "RequestedByGroup1Counter"),
            Self::RequestedByGroup2Counter => write!(f, "RequestedByGroup2Counter"),
            Self::RequestedByGroup3Counter => write!(f, "RequestedByGroup3Counter"),
            Self::RequestedByGroup4Counter => write!(f, "RequestedByGroup4Counter"),
            Self::UnknownTypeId => write!(f, "UnknownTypeId"),
            Self::UnknownCot => write!(f, "UnknownCot"),
            Self::UnknownCommonAddress => write!(f, "UnknownCommonAddress"),
            Self::UnknownIoa => write!(f, "UnknownIoa"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cot_from_u8() {
        assert_eq!(Cot::from_u8(1).unwrap(), Cot::Periodic);
        assert_eq!(Cot::from_u8(3).unwrap(), Cot::Spontaneous);
        assert_eq!(Cot::from_u8(6).unwrap(), Cot::Activation);
        assert_eq!(Cot::from_u8(7).unwrap(), Cot::ActivationConfirm);
        assert_eq!(Cot::from_u8(20).unwrap(), Cot::InterrogatedByStation);
    }

    #[test]
    fn test_cot_positive_negative() {
        assert!(Cot::ActivationConfirm.is_positive());
        assert!(Cot::DeactivationConfirm.is_positive());
        assert!(!Cot::Activation.is_positive());

        assert!(Cot::UnknownTypeId.is_negative());
        assert!(Cot::UnknownIoa.is_negative());
        assert!(!Cot::Spontaneous.is_negative());
    }

    #[test]
    fn test_cot_interrogation_response() {
        assert!(Cot::InterrogatedByStation.is_interrogation_response());
        assert!(Cot::InterrogatedByGroup1.is_interrogation_response());
        assert!(!Cot::Spontaneous.is_interrogation_response());
    }

    // ============ Additional COT Tests ============

    #[test]
    fn test_cot_all_values_roundtrip() {
        // Test all valid COT values
        let valid_values = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13,
            20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36,
            37, 38, 39, 40, 41,
            44, 45, 46, 47,
        ];

        for val in valid_values {
            let cot = Cot::from_u8(val).unwrap();
            assert_eq!(cot.as_u8(), val, "Roundtrip failed for value {}", val);
        }
    }

    #[test]
    fn test_cot_invalid_values() {
        // Test invalid COT values
        let invalid_values = [0, 14, 15, 16, 17, 18, 19, 42, 43, 48, 49, 50, 63];

        for val in invalid_values {
            let result = Cot::from_u8(val);
            assert!(result.is_err(), "Expected error for COT value {}", val);
        }
    }

    #[test]
    fn test_cot_upper_bits_masked() {
        // COT uses only lower 6 bits, upper 2 bits should be masked
        // 0x43 = 0b0100_0011 -> lower 6 bits = 3 (Spontaneous)
        let cot = Cot::from_u8(0x43).unwrap();
        assert_eq!(cot, Cot::Spontaneous);

        // 0x86 = 0b1000_0110 -> lower 6 bits = 6 (Activation)
        let cot = Cot::from_u8(0x86).unwrap();
        assert_eq!(cot, Cot::Activation);
    }

    #[test]
    fn test_cot_is_counter_response() {
        assert!(Cot::RequestedByGeneralCounter.is_counter_response());
        assert!(Cot::RequestedByGroup1Counter.is_counter_response());
        assert!(Cot::RequestedByGroup2Counter.is_counter_response());
        assert!(Cot::RequestedByGroup3Counter.is_counter_response());
        assert!(Cot::RequestedByGroup4Counter.is_counter_response());
        assert!(!Cot::Spontaneous.is_counter_response());
        assert!(!Cot::InterrogatedByStation.is_counter_response());
    }

    #[test]
    fn test_cot_display_all() {
        // Test that all COT values have Display implementation
        let test_cases = [
            (Cot::Periodic, "Periodic"),
            (Cot::Background, "Background"),
            (Cot::Spontaneous, "Spontaneous"),
            (Cot::Initialized, "Initialized"),
            (Cot::Request, "Request"),
            (Cot::Activation, "Activation"),
            (Cot::ActivationConfirm, "ActivationConfirm"),
            (Cot::Deactivation, "Deactivation"),
            (Cot::DeactivationConfirm, "DeactivationConfirm"),
            (Cot::ActivationTermination, "ActivationTermination"),
            (Cot::UnknownTypeId, "UnknownTypeId"),
            (Cot::UnknownCot, "UnknownCot"),
            (Cot::UnknownCommonAddress, "UnknownCommonAddress"),
            (Cot::UnknownIoa, "UnknownIoa"),
        ];

        for (cot, expected) in test_cases {
            assert_eq!(cot.to_string(), expected);
        }
    }

    #[test]
    fn test_cot_interrogation_groups() {
        // Test all interrogation group responses
        let groups = [
            (Cot::InterrogatedByStation, 20),
            (Cot::InterrogatedByGroup1, 21),
            (Cot::InterrogatedByGroup2, 22),
            (Cot::InterrogatedByGroup3, 23),
            (Cot::InterrogatedByGroup4, 24),
            (Cot::InterrogatedByGroup5, 25),
            (Cot::InterrogatedByGroup6, 26),
            (Cot::InterrogatedByGroup7, 27),
            (Cot::InterrogatedByGroup8, 28),
            (Cot::InterrogatedByGroup9, 29),
            (Cot::InterrogatedByGroup10, 30),
            (Cot::InterrogatedByGroup11, 31),
            (Cot::InterrogatedByGroup12, 32),
            (Cot::InterrogatedByGroup13, 33),
            (Cot::InterrogatedByGroup14, 34),
            (Cot::InterrogatedByGroup15, 35),
            (Cot::InterrogatedByGroup16, 36),
        ];

        for (cot, expected_value) in groups {
            assert_eq!(cot.as_u8(), expected_value);
            assert!(cot.is_interrogation_response());
        }
    }

    #[test]
    fn test_cot_positive_cases() {
        assert!(Cot::ActivationConfirm.is_positive());
        assert!(Cot::DeactivationConfirm.is_positive());
        assert!(Cot::ActivationTermination.is_positive());

        // Non-positive COTs
        assert!(!Cot::Activation.is_positive());
        assert!(!Cot::Deactivation.is_positive());
        assert!(!Cot::Spontaneous.is_positive());
    }

    #[test]
    fn test_cot_negative_cases() {
        assert!(Cot::UnknownTypeId.is_negative());
        assert!(Cot::UnknownCot.is_negative());
        assert!(Cot::UnknownCommonAddress.is_negative());
        assert!(Cot::UnknownIoa.is_negative());

        // Non-negative COTs
        assert!(!Cot::Activation.is_negative());
        assert!(!Cot::ActivationConfirm.is_negative());
        assert!(!Cot::Spontaneous.is_negative());
    }
}
