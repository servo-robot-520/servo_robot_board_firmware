//! Data types for the HUSB238A driver.

use crate::registers::{
    INT1_I_ATTACH, INT1_I_DETACH, INT1_I_FAULT, INT1_I_VBUS_OV, INT2_I_TSD, INT2_I_VBUS_UV,
    PD_CONTRACT_5V, PD_CONTRACT_9V, PD_CONTRACT_12V, PD_CONTRACT_15V, PD_CONTRACT_20V,
    PD_CONTRACT_28V, PD_CONTRACT_36V, PD_CONTRACT_48V, PD_CONTRACT_AVS, PD_CONTRACT_EPR_AVS,
    PD_CONTRACT_PPS1, PD_CONTRACT_PPS2, PD_CONTRACT_PPS3, PD_CONTRACT_TYPEC_5V,
};

/// Charger protocol type
///
/// Covers all protocols supported by the HUSB238A:
/// - PD contracts (fixed PDO: 5V–48V, PPS, AVS, EPR AVS)
/// - Legacy DPM contracts (BC1.2, Divider-3, QC2, HVDCP)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub enum ChargerProtocol {
    /// No contract established
    Unknown,
    // --- PD contracts (PD_CONTRACT field, CONTRACT_STATUS0[7:4]) ---
    /// 5V Type-C contract (no PD negotiation)
    TypeC5v,
    /// 5V PD contract (PDO1)
    Pd5v,
    /// 9V PD contract (PDO2)
    Pd9v,
    /// 12V PD contract (PDO3)
    Pd12v,
    /// 15V PD contract (PDO4)
    Pd15v,
    /// 20V PD contract (PDO5)
    Pd20v,
    /// 28V PD contract (EPR PDO1)
    Pd28v,
    /// 36V PD contract (EPR PDO2)
    Pd36v,
    /// 48V PD contract (EPR PDO3)
    Pd48v,
    /// Programmable Power Supply contract (PPS1/PPS2/PPS3)
    Pps,
    /// Adjustable Voltage Supply contract
    Avs,
    /// EPR Adjustable Voltage Supply contract
    EprAvs,
    // --- Legacy DPM contracts (DPM_CONTRACT field, CONTRACT_STATUS0[3:0]) ---
    /// 5V Default Contract (BC1.2 DCP/SDP/CDP or unconfigured)
    Default5v,
    /// 5V Divider-3 Contract
    Divider3,
    /// 5V SDP (Standard Downstream Port)
    Sdp,
    /// 5V CDP (Charging Downstream Port)
    Cdp,
    /// 5V DCP (Dedicated Charging Port)
    Dcp,
    /// 5V HVDCP (High Voltage Dedicated Charging Port)
    Hvdcp,
    /// QC2 9V Contract
    Qc2_9v,
    /// QC2 12V Contract
    Qc2_12v,
}

impl ChargerProtocol {
    /// Map PD_CONTRACT code (CONTRACT_STATUS0[7:4]) to ChargerProtocol.
    pub(crate) fn from_pd_contract(code: u8) -> Self {
        match code {
            PD_CONTRACT_TYPEC_5V => Self::TypeC5v,
            PD_CONTRACT_5V => Self::Pd5v,
            PD_CONTRACT_9V => Self::Pd9v,
            PD_CONTRACT_12V => Self::Pd12v,
            PD_CONTRACT_15V => Self::Pd15v,
            PD_CONTRACT_20V => Self::Pd20v,
            PD_CONTRACT_PPS1 | PD_CONTRACT_PPS2 | PD_CONTRACT_PPS3 => Self::Pps,
            PD_CONTRACT_AVS => Self::Avs,
            PD_CONTRACT_28V => Self::Pd28v,
            PD_CONTRACT_36V => Self::Pd36v,
            PD_CONTRACT_48V => Self::Pd48v,
            PD_CONTRACT_EPR_AVS => Self::EprAvs,
            _ => Self::Unknown,
        }
    }

    /// Map DPM_CONTRACT code (CONTRACT_STATUS0[3:0]) to ChargerProtocol.
    pub(crate) fn from_dpm_contract(code: u8) -> Self {
        match code {
            0x00 => Self::Default5v,
            0x01 => Self::Divider3,
            0x02 => Self::Sdp,
            0x03 => Self::Cdp,
            0x04 => Self::Dcp,
            0x05 => Self::Hvdcp,
            0x06 => Self::Qc2_9v,
            0x07 => Self::Qc2_12v,
            _ => Self::Unknown,
        }
    }

    /// Convert PD contract code to voltage in mV.
    /// Returns 0 for PPS/AVS/EPR_AVS (voltage is negotiated separately).
    pub(crate) fn pd_voltage_mv(code: u8) -> u16 {
        match code {
            PD_CONTRACT_TYPEC_5V | PD_CONTRACT_5V => 5000,
            PD_CONTRACT_9V => 9000,
            PD_CONTRACT_12V => 12000,
            PD_CONTRACT_15V => 15000,
            PD_CONTRACT_20V => 20000,
            PD_CONTRACT_28V => 28000,
            PD_CONTRACT_36V => 36000,
            PD_CONTRACT_48V => 48000,
            _ => 0,
        }
    }
}

/// PDO information
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct PdoInfo {
    pub code: u8,
    pub protocol: ChargerProtocol,
    pub voltage_mv: u16,
    pub current_ma: u16,
}

/// Contract information from the charger
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct ContractInfo {
    pub protocol: ChargerProtocol,
    pub voltage_mv: u16,
    pub current_ma: f32,
}

/// State of an explicit PDO request.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub enum RequestStatus {
    /// Negotiation is still in progress.
    Pending,
    /// Negotiation completed and this is the active contract.
    Succeeded(ContractInfo),
}

/// Interrupt status from the three interrupt registers
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct InterruptStatus {
    pub int: u8,
    pub int1: u8,
    pub int2: u8,
}

impl InterruptStatus {
    /// Returns true when the controller reported Type-C source attachment or detachment.
    pub const fn has_attach_change(&self) -> bool {
        self.int1 & (INT1_I_ATTACH | INT1_I_DETACH) != 0
    }

    /// Returns true when the controller reported a PD sink fault condition.
    pub const fn has_fault(&self) -> bool {
        self.int1 & (INT1_I_FAULT | INT1_I_VBUS_OV) != 0
            || self.int2 & (INT2_I_VBUS_UV | INT2_I_TSD) != 0
    }
}

/// PPS max voltage codes (SRC_PPS_VOLTAGE register bits [7:6], [5:4], [3:2])
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
#[repr(u8)]
pub enum PpsMaxVoltage {
    /// 0V–7V
    V5_9 = 0,
    /// 7.02V–12V
    V11 = 1,
    /// 12.02V–17V
    V16 = 2,
    /// >17.02V
    V21 = 3,
}

impl PpsMaxVoltage {
    /// Upper voltage bound in millivolts for this PPS range code.
    pub const fn max_mv(self) -> u16 {
        match self {
            Self::V5_9 => 5900,
            Self::V11 => 11000,
            Self::V16 => 16000,
            Self::V21 => 21000,
        }
    }

    /// Decode from 2-bit raw register value
    pub fn from_raw(raw: u8) -> Self {
        match raw & 0x03 {
            0 => Self::V5_9,
            1 => Self::V11,
            2 => Self::V16,
            _ => Self::V21,
        }
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registers::{
        INT1_I_ATTACH, INT1_I_DETACH, INT1_I_FAULT, INT1_I_VBUS_OV, INT2_I_TSD, INT2_I_VBUS_UV,
    };

    #[test]
    fn interrupt_status_reports_attach_and_detach_changes() {
        assert!(
            InterruptStatus {
                int: 0,
                int1: INT1_I_ATTACH,
                int2: 0,
            }
            .has_attach_change()
        );
        assert!(
            InterruptStatus {
                int: 0,
                int1: INT1_I_DETACH,
                int2: 0,
            }
            .has_attach_change()
        );
        assert!(
            !InterruptStatus {
                int: 0,
                int1: 0,
                int2: 0,
            }
            .has_attach_change()
        );
    }

    #[test]
    fn interrupt_status_reports_all_pd_sink_fault_sources() {
        for status in [
            InterruptStatus {
                int: 0,
                int1: INT1_I_FAULT,
                int2: 0,
            },
            InterruptStatus {
                int: 0,
                int1: INT1_I_VBUS_OV,
                int2: 0,
            },
            InterruptStatus {
                int: 0,
                int1: 0,
                int2: INT2_I_VBUS_UV,
            },
            InterruptStatus {
                int: 0,
                int1: 0,
                int2: INT2_I_TSD,
            },
        ] {
            assert!(status.has_fault());
        }
        assert!(
            !InterruptStatus {
                int: 0,
                int1: 0,
                int2: 0,
            }
            .has_fault()
        );
    }
}
