//! Register definitions, constants, option types, and error types for the BQ24725.

// ============================================================================
// I2C Address (7-bit)
// ============================================================================

/// BQ24725 7-bit I2C address
pub const BQ24725_ADDR: u8 = 0x09;

// ============================================================================
// Register Addresses
// ============================================================================

/// Charger Options Control register
pub const REG_CHARGE_OPTION: u8 = 0x12;
/// 7-bit Charge Current Setting register
pub const REG_CHARGE_CURRENT: u8 = 0x14;
/// 11-bit Charge Voltage Setting register
pub const REG_CHARGE_VOLTAGE: u8 = 0x15;
/// 6-bit Input Current Setting register
pub const REG_INPUT_CURRENT: u8 = 0x3F;
/// Device ID register (read-only)
pub const REG_DEVICE_ID: u8 = 0xFF;
/// Manufacturer ID register (read-only)
pub const REG_MANUFACTURE_ID: u8 = 0xFE;

// ============================================================================
// Register Masks (hardware bit positions)
// ============================================================================

/// ChargeCurrent() register mask: bits [12:6]
pub const CHARGE_CURRENT_MASK: u16 = 0x1FC0;
/// ChargeVoltage() register mask: bits [14:4]
pub const CHARGE_VOLTAGE_MASK: u16 = 0x7FF0;
/// InputCurrent() register mask: bits [12:7]
pub const INPUT_CURRENT_MASK: u16 = 0x1F80;

// ============================================================================
// Charge Current constants
// ============================================================================

/// Minimum charge current in mA (128mA, register value 0x0040)
pub const CHARGE_CURRENT_MIN_MA: u16 = 128;
/// Maximum charge current in mA (8128mA, register value 0x1FC0)
pub const CHARGE_CURRENT_MAX_MA: u16 = 8128;
/// Charge current step in mA (64mA per LSB)
pub const CHARGE_CURRENT_STEP_MA: u16 = 64;

// ============================================================================
// Charge Voltage constants
// ============================================================================

/// Minimum charge voltage in mV (1024mV, register value 0x0010)
pub const CHARGE_VOLTAGE_MIN_MV: u16 = 1024;
/// Maximum charge voltage in mV (19200mV, register value 0x7FF0)
pub const CHARGE_VOLTAGE_MAX_MV: u16 = 19200;
/// Charge voltage step in mV (16mV per LSB)
pub const CHARGE_VOLTAGE_STEP_MV: u16 = 16;

// ============================================================================
// Input Current constants
// ============================================================================

/// Minimum input current in mA (128mA, register value 0x0080)
pub const INPUT_CURRENT_MIN_MA: u16 = 128;
/// Maximum input current in mA (8064mA, register value 0x1F80)
pub const INPUT_CURRENT_MAX_MA: u16 = 8064;
/// Input current step in mA (128mA per LSB)
pub const INPUT_CURRENT_STEP_MA: u16 = 128;

// ============================================================================
// Charge Option bit field definitions
// ============================================================================

/// ACOK deglitch time
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
#[repr(u16)]
pub enum AcokDeglitchTime {
    /// ACO rising edge deglitch time 150ms (default at POR)
    T150ms = 0x0000,
    /// ACO rising edge deglitch time 1.3s
    T1300ms = 0x8000,
}

/// Watchdog timer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
#[repr(u16)]
pub enum WatchdogTimer {
    /// Disable Watchdog Timer
    Disabled = 0x0000,
    /// Enabled, 44 sec
    T44s = 0x2000,
    /// Enabled, 88 sec
    T88s = 0x4000,
    /// Enable Watchdog Timer 175s (default at POR)
    T175s = 0x6000,
}

/// BAT depletion threshold
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
#[repr(u16)]
pub enum BatDepletionThreshold {
    /// Falling Threshold = 59.19% of voltage regulation limit (~2.486V/cell)
    Ft59_19pct = 0x0000,
    /// Falling Threshold = 62.65% (~2.631V/cell) — POR default
    Ft62_65pct = 0x0800,
    /// Falling Threshold = 66.55% (~2.795V/cell)
    Ft66_55pct = 0x1000,
    /// Falling Threshold = 70.97% (~2.981V/cell)
    Ft70_97pct = 0x1800,
}

/// EMI switching frequency adjustment direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
#[repr(u16)]
pub enum EmiSwFreqAdj {
    /// Reduce PWM switching frequency by 18% (default at POR)
    Dec18pct = 0x0000,
    /// Increase PWM switching frequency by 18%
    Inc18pct = 0x0400,
}

/// EMI switching frequency adjustment enable
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
#[repr(u16)]
pub enum EmiSwFreqAdjEn {
    /// Disable adjust PWM switching frequency (default at POR)
    Disabled = 0x0000,
    /// Enable adjust PWM switching frequency
    Enabled = 0x0200,
}

/// IFAULT_HI threshold (short circuit protection high side MOSFET voltage drop)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
#[repr(u16)]
pub enum IfaultHiThreshold {
    /// 300mV
    L300mV = 0x0000,
    /// 500mV
    L500mV = 0x0080,
    /// 700mV (default at POR)
    L700mV = 0x0100,
    /// 900mV
    L900mV = 0x0180,
}

/// LEARN enable (battery learn cycle)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
#[repr(u16)]
pub enum LearnEn {
    /// Disable LEARN Cycle (default at POR)
    Disabled = 0x0000,
    /// Enable LEARN Cycle (auto-resets after cycle completes)
    Enabled = 0x0040,
}

/// IOUT pin output selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
#[repr(u16)]
pub enum Iout {
    /// IOUT is the 20x adapter current amplifier output (default at POR)
    AdapterCurrent = 0x0000,
    /// IOUT is the 20x charge current amplifier output
    ChargeCurrent = 0x0020,
}

/// ACOC threshold (input over current protection)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
#[repr(u16)]
pub enum AcocThreshold {
    /// Disable ACOC
    Disabled = 0x0000,
    /// 1.33X of input current regulation limit
    L1_33X = 0x0002,
    /// 1.66X of input current regulation limit (default at POR)
    L1_66X = 0x0004,
    /// 2.22X of input current regulation limit
    L2_22X = 0x0006,
}

/// Charge inhibit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
#[repr(u16)]
pub enum ChargeInhibit {
    /// Enable Charge (default at POR)
    ChargeEnable = 0x0000,
    /// Inhibit Charge
    ChargeInhibit = 0x0001,
}

// ============================================================================
// Charge Options struct
// ============================================================================

/// BQ24725 charge option configuration
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct ChargeOptions {
    pub acok_deglitch_time: AcokDeglitchTime,
    pub watchdog_timer: WatchdogTimer,
    pub bat_depletion_threshold: BatDepletionThreshold,
    pub emi_sw_freq_adj: EmiSwFreqAdj,
    pub emi_sw_freq_adj_en: EmiSwFreqAdjEn,
    pub ifault_hi_threshold: IfaultHiThreshold,
    pub learn_en: LearnEn,
    pub iout: Iout,
    pub acoc_threshold: AcocThreshold,
    pub charge_inhibit: ChargeInhibit,
}

impl ChargeOptions {
    /// Pack options into a 16-bit register value.
    /// Bits [4:3] are reserved and always 0.
    pub fn to_u16(&self) -> u16 {
        self.acok_deglitch_time as u16
            | self.watchdog_timer as u16
            | self.bat_depletion_threshold as u16
            | self.emi_sw_freq_adj as u16
            | self.emi_sw_freq_adj_en as u16
            | self.ifault_hi_threshold as u16
            | self.learn_en as u16
            | self.iout as u16
            | self.acoc_threshold as u16
            | self.charge_inhibit as u16
    }

    /// Unpack a 16-bit register value into options
    pub fn from_u16(data: u16) -> Self {
        Self {
            acok_deglitch_time: if data & 0x8000 != 0 {
                AcokDeglitchTime::T1300ms
            } else {
                AcokDeglitchTime::T150ms
            },
            watchdog_timer: match data & 0x6000 {
                0x2000 => WatchdogTimer::T44s,
                0x4000 => WatchdogTimer::T88s,
                0x6000 => WatchdogTimer::T175s,
                _ => WatchdogTimer::Disabled,
            },
            bat_depletion_threshold: match data & 0x1800 {
                0x0800 => BatDepletionThreshold::Ft62_65pct,
                0x1000 => BatDepletionThreshold::Ft66_55pct,
                0x1800 => BatDepletionThreshold::Ft70_97pct,
                _ => BatDepletionThreshold::Ft59_19pct,
            },
            emi_sw_freq_adj: if data & 0x0400 != 0 {
                EmiSwFreqAdj::Inc18pct
            } else {
                EmiSwFreqAdj::Dec18pct
            },
            emi_sw_freq_adj_en: if data & 0x0200 != 0 {
                EmiSwFreqAdjEn::Enabled
            } else {
                EmiSwFreqAdjEn::Disabled
            },
            ifault_hi_threshold: match data & 0x0180 {
                0x0080 => IfaultHiThreshold::L500mV,
                0x0100 => IfaultHiThreshold::L700mV,
                0x0180 => IfaultHiThreshold::L900mV,
                _ => IfaultHiThreshold::L300mV,
            },
            learn_en: if data & 0x0040 != 0 {
                LearnEn::Enabled
            } else {
                LearnEn::Disabled
            },
            iout: if data & 0x0020 != 0 {
                Iout::ChargeCurrent
            } else {
                Iout::AdapterCurrent
            },
            acoc_threshold: match data & 0x0006 {
                0x0002 => AcocThreshold::L1_33X,
                0x0004 => AcocThreshold::L1_66X,
                0x0006 => AcocThreshold::L2_22X,
                _ => AcocThreshold::Disabled,
            },
            charge_inhibit: if data & 0x0001 != 0 {
                ChargeInhibit::ChargeInhibit
            } else {
                ChargeInhibit::ChargeEnable
            },
        }
    }

    /// Default power-on-reset configuration (POR = 0x7904)
    pub const fn por_default() -> Self {
        Self {
            acok_deglitch_time: AcokDeglitchTime::T150ms,
            watchdog_timer: WatchdogTimer::T44s,
            bat_depletion_threshold: BatDepletionThreshold::Ft62_65pct,
            emi_sw_freq_adj: EmiSwFreqAdj::Dec18pct,
            emi_sw_freq_adj_en: EmiSwFreqAdjEn::Disabled,
            ifault_hi_threshold: IfaultHiThreshold::L700mV,
            learn_en: LearnEn::Disabled,
            iout: Iout::AdapterCurrent,
            acoc_threshold: AcocThreshold::Disabled,
            charge_inhibit: ChargeInhibit::ChargeEnable,
        }
    }
}

// ============================================================================
// Physical-value conversion helpers
// ============================================================================

/// Encode a charge-current setting as its register value.
pub fn encode_charge_current_ma(ma: u16) -> Result<u16, ValueError> {
    if ma == 0 {
        return Ok(0);
    }
    if !(CHARGE_CURRENT_MIN_MA..=CHARGE_CURRENT_MAX_MA).contains(&ma)
        || !ma.is_multiple_of(CHARGE_CURRENT_STEP_MA)
    {
        return Err(ValueError::Unsupported);
    }
    Ok((ma / CHARGE_CURRENT_STEP_MA) << 6)
}

/// Decode a charge-current register value in mA.
pub const fn decode_charge_current_ma(raw: u16) -> u16 {
    ((raw & CHARGE_CURRENT_MASK) >> 6) * CHARGE_CURRENT_STEP_MA
}

/// Encode a charge-voltage setting as its register value.
pub fn encode_charge_voltage_mv(mv: u16) -> Result<u16, ValueError> {
    if mv == 0 {
        return Ok(0);
    }
    if !(CHARGE_VOLTAGE_MIN_MV..=CHARGE_VOLTAGE_MAX_MV).contains(&mv)
        || !mv.is_multiple_of(CHARGE_VOLTAGE_STEP_MV)
    {
        return Err(ValueError::Unsupported);
    }
    Ok((mv / CHARGE_VOLTAGE_STEP_MV) << 4)
}

/// Decode a charge-voltage register value in mV.
pub const fn decode_charge_voltage_mv(raw: u16) -> u16 {
    ((raw & CHARGE_VOLTAGE_MASK) >> 4) * CHARGE_VOLTAGE_STEP_MV
}

/// Encode an input-current setting as its register value.
pub fn encode_input_current_ma(ma: u16) -> Result<u16, ValueError> {
    if ma == 0 {
        return Ok(0);
    }
    if !(INPUT_CURRENT_MIN_MA..=INPUT_CURRENT_MAX_MA).contains(&ma)
        || !ma.is_multiple_of(INPUT_CURRENT_STEP_MA)
    {
        return Err(ValueError::Unsupported);
    }
    Ok((ma / INPUT_CURRENT_STEP_MA) << 7)
}

/// Decode an input-current register value in mA.
pub const fn decode_input_current_ma(raw: u16) -> u16 {
    ((raw & INPUT_CURRENT_MASK) >> 7) * INPUT_CURRENT_STEP_MA
}

/// A physical value cannot be represented by the device register.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub enum ValueError {
    /// Value is outside the documented range or is not aligned to the register step.
    Unsupported,
}

// ============================================================================
// Error type
// ============================================================================

/// Driver error
#[derive(Debug)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub enum Error<I2cError> {
    /// I2C communication error
    I2c(I2cError),
    /// Charge current out of valid range (128–8128 mA)
    ChargeCurrentOutOfRange,
    /// Charge voltage out of valid range (1024–19200 mV)
    ChargeVoltageOutOfRange,
    /// Input current out of valid range (128–8064 mA)
    InputCurrentOutOfRange,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charge_current_uses_direct_dac_scaling() {
        assert_eq!(encode_charge_current_ma(0), Ok(0));
        assert_eq!(encode_charge_current_ma(128), Ok(0x0080));
        assert_eq!(encode_charge_current_ma(8128), Ok(CHARGE_CURRENT_MASK));
        assert_eq!(decode_charge_current_ma(0x0080), 128);
        assert_eq!(decode_charge_current_ma(CHARGE_CURRENT_MASK), 8128);
        assert_eq!(encode_charge_current_ma(129), Err(ValueError::Unsupported));
        assert_eq!(encode_charge_current_ma(8192), Err(ValueError::Unsupported));
    }

    #[test]
    fn input_current_and_voltage_reject_ambiguous_values() {
        assert_eq!(encode_input_current_ma(128), Ok(0x0080));
        assert_eq!(decode_input_current_ma(INPUT_CURRENT_MASK), 8064);
        assert_eq!(encode_input_current_ma(192), Err(ValueError::Unsupported));
        assert_eq!(encode_charge_voltage_mv(1024), Ok(0x0400));
        assert_eq!(decode_charge_voltage_mv(0x0400), 1024);
        assert_eq!(encode_charge_voltage_mv(1025), Err(ValueError::Unsupported));
    }
}
