//! BQ40Z50 data types and status flag wrappers.

use crate::flags::*;

// ============================================================================
// Error type
// ============================================================================

#[derive(Debug)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub enum Error<I2cError> {
    /// I²C communication error.
    I2c(I2cError),
    /// The SMBus count byte exceeds the protocol or caller-buffer capacity.
    InvalidBlockLength { reported: u8, capacity: u8 },
    /// A fixed-size typed response did not contain the required number of bytes.
    UnexpectedBlockLength { expected: u8, reported: u8 },
}

// ============================================================================
// Temperature detail (from DAStatus2)
// ============================================================================

/// Detailed temperature readings from DAStatus2 sub-command (0x0072).
///
/// Raw values are in 0.1°K, stored as Celsius (raw - 2732) / 10.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct TempDetail {
    /// Internal temperature (°C)
    pub int_temp: i16,
    /// TS1 temperature (°C)
    pub ts1_temp: i16,
    /// TS2 temperature (°C)
    pub ts2_temp: i16,
    /// TS3 temperature (°C)
    pub ts3_temp: i16,
    /// TS4 temperature (°C)
    pub ts4_temp: i16,
    /// Cell temperature (°C)
    pub cell_temp: i16,
    /// FET temperature (°C)
    pub fet_temp: i16,
}

// ============================================================================
// Safety flags
// ============================================================================

/// Safety alert/status flags (32-bit).
///
/// From MAC sub-commands 0x0050 (SafetyAlert) / 0x0051 (SafetyStatus).
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct SafetyFlags(pub u32);

impl SafetyFlags {
    pub fn utd(self) -> bool {
        self.0 & SAFETY_UTD != 0
    }
    pub fn utc(self) -> bool {
        self.0 & SAFETY_UTC != 0
    }
    pub fn pchgc(self) -> bool {
        self.0 & SAFETY_PCHGC != 0
    }
    pub fn chgv(self) -> bool {
        self.0 & SAFETY_CHGV != 0
    }
    pub fn chgc(self) -> bool {
        self.0 & SAFETY_CHGC != 0
    }
    pub fn oc(self) -> bool {
        self.0 & SAFETY_OC != 0
    }
    pub fn cto(self) -> bool {
        self.0 & SAFETY_CTO != 0
    }
    pub fn pto(self) -> bool {
        self.0 & SAFETY_PTO != 0
    }
    pub fn otf(self) -> bool {
        self.0 & SAFETY_OTF != 0
    }
    pub fn cuvc(self) -> bool {
        self.0 & SAFETY_CUVC != 0
    }
    pub fn otd(self) -> bool {
        self.0 & SAFETY_OTD != 0
    }
    pub fn otc(self) -> bool {
        self.0 & SAFETY_OTC != 0
    }
    pub fn ascd(self) -> bool {
        self.0 & SAFETY_ASCD != 0
    }
    pub fn ascc(self) -> bool {
        self.0 & SAFETY_ASCC != 0
    }
    pub fn aold(self) -> bool {
        self.0 & SAFETY_AOLD != 0
    }
    pub fn ocd2(self) -> bool {
        self.0 & SAFETY_OCD2 != 0
    }
    pub fn ocd1(self) -> bool {
        self.0 & SAFETY_OCD1 != 0
    }
    pub fn occ2(self) -> bool {
        self.0 & SAFETY_OCC2 != 0
    }
    pub fn occ1(self) -> bool {
        self.0 & SAFETY_OCC1 != 0
    }
    pub fn cov(self) -> bool {
        self.0 & SAFETY_COV != 0
    }
    pub fn cuv(self) -> bool {
        self.0 & SAFETY_CUV != 0
    }

    /// Check if any safety flag is set
    pub fn has_any(self) -> bool {
        self.0 != 0
    }
}

// ============================================================================
// Operation status
// ============================================================================

/// Operation status flags (32-bit).
///
/// From MAC sub-command 0x0054 (OperationStatus).
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct OperationStatus(pub u32);

/// Security mode extracted from OperationStatus SEC1:SEC0 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub enum SecurityMode {
    /// Reserved
    Reserved,
    /// Full Access — full configuration access
    FullAccess,
    /// Unsealed — most commands available
    Unsealed,
    /// Sealed — standard SBS commands only
    Sealed,
}

impl OperationStatus {
    pub fn emergency_shutdown(self) -> bool {
        self.0 & OP_STATUS_EMSHUT != 0
    }
    pub fn cell_balancing(self) -> bool {
        self.0 & OP_STATUS_CB != 0
    }
    pub fn initializing(self) -> bool {
        self.0 & OP_STATUS_INIT != 0
    }
    pub fn sleep_mode(self) -> bool {
        self.0 & OP_STATUS_SLEEP != 0
    }
    pub fn charging_disabled(self) -> bool {
        self.0 & OP_STATUS_XCHG != 0
    }
    pub fn discharging_disabled(self) -> bool {
        self.0 & OP_STATUS_XDSG != 0
    }
    pub fn permanent_failure(self) -> bool {
        self.0 & OP_STATUS_PF != 0
    }
    pub fn safety_mode(self) -> bool {
        self.0 & OP_STATUS_SS != 0
    }
    pub fn fuse_active(self) -> bool {
        self.0 & OP_STATUS_FUSE != 0
    }
    pub fn precharge_fet(self) -> bool {
        self.0 & OP_STATUS_PCHG != 0
    }
    pub fn charge_fet(self) -> bool {
        self.0 & OP_STATUS_CHG != 0
    }
    pub fn discharge_fet(self) -> bool {
        self.0 & OP_STATUS_DSG != 0
    }
    pub fn system_present(self) -> bool {
        self.0 & OP_STATUS_PRES != 0
    }

    /// Extract security mode from SEC1:SEC0 (Bits 9:8)
    pub fn security_mode(self) -> SecurityMode {
        match (self.0 >> 8) & 0x03 {
            0b01 => SecurityMode::FullAccess,
            0b10 => SecurityMode::Unsealed,
            0b11 => SecurityMode::Sealed,
            _ => SecurityMode::Reserved,
        }
    }
}

// ============================================================================
// Charging status
// ============================================================================

/// Charging status flags (32-bit).
///
/// From MAC sub-command 0x0055 (ChargingStatus).
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct ChargingStatus(pub u32);

impl ChargingStatus {
    pub fn charge_terminated(self) -> bool {
        self.0 & CHG_STATUS_VCT != 0
    }
    pub fn maintenance_charge(self) -> bool {
        self.0 & CHG_STATUS_MCHG != 0
    }
    pub fn charge_inhibit(self) -> bool {
        self.0 & CHG_STATUS_IN != 0
    }
    pub fn high_voltage_region(self) -> bool {
        self.0 & CHG_STATUS_HV != 0
    }
    pub fn mid_voltage_region(self) -> bool {
        self.0 & CHG_STATUS_MV != 0
    }
    pub fn low_voltage_region(self) -> bool {
        self.0 & CHG_STATUS_LV != 0
    }
    pub fn precharge_region(self) -> bool {
        self.0 & CHG_STATUS_PV != 0
    }
    pub fn overtemp_region(self) -> bool {
        self.0 & CHG_STATUS_OT != 0
    }
    pub fn high_temp_region(self) -> bool {
        self.0 & CHG_STATUS_HT != 0
    }
    pub fn recommended_temp_region(self) -> bool {
        self.0 & CHG_STATUS_RT != 0
    }
    pub fn low_temp_region(self) -> bool {
        self.0 & CHG_STATUS_LT != 0
    }
    pub fn under_temp_region(self) -> bool {
        self.0 & CHG_STATUS_UT != 0
    }
}

// ============================================================================
// Gauging status
// ============================================================================

/// Gauging status flags (32-bit).
///
/// From MAC sub-command 0x0056 (GaugingStatus).
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct GaugingStatus(pub u32);

impl GaugingStatus {
    pub fn it_enabled(self) -> bool {
        self.0 & GAUGE_QEN != 0
    }
    pub fn vok(self) -> bool {
        self.0 & GAUGE_VOK != 0
    }
    pub fn resistance_updates_disabled(self) -> bool {
        self.0 & GAUGE_R_DIS != 0
    }
    pub fn ocv_reading_taken(self) -> bool {
        self.0 & GAUGE_REST != 0
    }
    pub fn condition_flag(self) -> bool {
        self.0 & GAUGE_CF != 0
    }
    pub fn discharging(self) -> bool {
        self.0 & GAUGE_DSG != 0
    }
    pub fn edv_reached(self) -> bool {
        self.0 & GAUGE_EDV != 0
    }
    pub fn cell_balancing_possible(self) -> bool {
        self.0 & GAUGE_BAL_EN != 0
    }
    pub fn terminate_charge(self) -> bool {
        self.0 & GAUGE_TC != 0
    }
    pub fn terminate_discharge(self) -> bool {
        self.0 & GAUGE_TD != 0
    }
    pub fn fully_charged(self) -> bool {
        self.0 & GAUGE_FC != 0
    }
    pub fn fully_discharged(self) -> bool {
        self.0 & GAUGE_FD != 0
    }
    pub fn discharge_qualified(self) -> bool {
        self.0 & GAUGE_VDQ != 0
    }
    pub fn constant_power_load(self) -> bool {
        self.0 & GAUGE_LDMD != 0
    }
}

// ============================================================================
// State of Health
// ============================================================================

/// State of Health data from MAC 0x0077.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct StateOfHealth {
    /// SOH Full Charge Capacity (mAh)
    pub fcc_mah: u16,
    /// SOH energy (cWh)
    pub energy_cwh: u16,
}
