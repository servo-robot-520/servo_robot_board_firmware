//! Charging state machine, phase definitions, and current calculation.
//!
//! Pure logic with no I2C or hardware dependencies.

// ============================================================================
// Temperature threshold (deg C) -- hardcoded protection thresholds
// ============================================================================

/// Battery low-temperature limit: stop charging below 0 deg C
const BAT_TEMP_COLD_LIMIT: i16 = 0;
/// Low-temperature slow charging below 10 deg C
const BAT_TEMP_COOL_LIMIT: i16 = 100;
/// Battery room temperature upper limit: 45 deg C
const BAT_TEMP_WARM_LIMIT: i16 = 450;
/// Battery high-temperature limit: stop charging above 50 deg C
const BAT_TEMP_HOT_LIMIT: i16 = 500;

// ============================================================================
// Charging current limit (mA)
// ============================================================================

/// Minimum charging current: 448mA (BQ24725 step 64mA)
const CHARGE_CURRENT_MIN_MA: u16 = 448;
/// Default maximum charging current: 8.0A (BQ24725 limit 8128mA, step 64mA)
const CHARGE_CURRENT_MAX_DEFAULT_MA: u16 = 8000;
/// BQ24725 driver allowed charging current upper limit
pub(crate) const BQ24725_CHARGE_CURRENT_MAX_MA: u16 = 8128;
/// Default charging voltage (4S: 16.8V)
const CHARGE_VOLTAGE_DEFAULT_MV: u16 = 16800;

/// BQ24725 charge current quantized to 64mA step (round down)
pub fn quantize_charge_current(ma: u16) -> u16 {
    (ma / 64) * 64
}

/// BQ24725 input current quantized to 128mA step (round down)
pub fn quantize_input_current(ma: u16) -> u16 {
    (ma / 128) * 128
}

/// BQ24725 charge voltage quantized to 16mV step (round down)
pub fn quantize_charge_voltage(mv: u16) -> u16 {
    (mv / 16) * 16
}

// ============================================================================
// Charging phase
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[repr(u8)]
pub enum ChargePhase {
    Unknown = 0,
    NotCharging = 1,
    PreCharge = 2,
    Cc = 3,
    Cv = 4,
    Full = 5,
    HusbFault = 6,
    Unsupported = 7,
    ThermalProtect = 8,
}

// ============================================================================
// Charger status
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum ChargerStatus {
    Disconnected,
    Connected,
    Fault,
    Unsupported,
}

// ============================================================================
// Charging current calculation
// ============================================================================

/// Calculate target charging current (mA) based on temperature and power budget.
pub fn calc_charge_current_ma(
    batt_temp: i16,
    charger_temp: i16,
    husb_power_mw: f32,
    batt_voltage_mv: u16,
    max_current_ma: u16,
    charge_temp_derating: i16,
    charge_temp_limit: i16,
) -> u16 {
    // Battery temperature protection
    if batt_temp < BAT_TEMP_COLD_LIMIT || batt_temp > BAT_TEMP_HOT_LIMIT {
        return 0;
    }

    let current_ma: u16;

    if batt_temp < BAT_TEMP_COOL_LIMIT {
        // Low-temperature, slow charging
        current_ma = CHARGE_CURRENT_MIN_MA;
    } else if batt_temp < BAT_TEMP_WARM_LIMIT {
        // Room temperature: dynamic calculation
        if husb_power_mw > 0.0 && batt_voltage_mv > 0 {
            let calc = husb_power_mw * 0.85 / batt_voltage_mv as f32;
            current_ma = if calc > max_current_ma as f32 {
                max_current_ma
            } else {
                calc as u16
            };
        } else {
            current_ma = 0;
        }
    } else {
        // High-temperature derating: 45~50 deg C linear down to CHARGE_CURRENT_MIN_MA
        let range = BAT_TEMP_HOT_LIMIT - BAT_TEMP_WARM_LIMIT; // 50 (i16)
        let delta = BAT_TEMP_HOT_LIMIT - batt_temp; // 0~50 (i16)
        // ratio_10000: 0 = hottest (50 deg C), 10000 = coolest (45 deg C)
        let ratio_10000 = ((delta as i32 * 10000) / range as i32).clamp(0, 10000) as u16;
        let span = max_current_ma.saturating_sub(CHARGE_CURRENT_MIN_MA);
        current_ma = CHARGE_CURRENT_MIN_MA + (span as u32 * ratio_10000 as u32 / 10000) as u16;
    }

    // Charge circuit temperature protection (thresholds from Config)
    if charger_temp > charge_temp_limit {
        return 0;
    }
    let current_ma = if charger_temp > charge_temp_derating {
        let temp_range = charge_temp_limit - charge_temp_derating;
        if temp_range <= 0 {
            // Prevent division by zero: if thresholds misconfigured, stop charging
            return 0;
        }
        let ratio = (charge_temp_limit - charger_temp) as f32 / temp_range as f32;
        let ratio = ratio.clamp(0.0, 1.0);
        let reduced = (CHARGE_CURRENT_MIN_MA as f32
            + ratio * (current_ma as f32 - CHARGE_CURRENT_MIN_MA as f32))
            as u16;
        reduced.min(current_ma)
    } else {
        current_ma
    };

    // Clamp
    current_ma.min(max_current_ma)
}

// ============================================================================
// BQ24725 command
// ============================================================================

pub enum Bq24725Command {
    Disable,
    Enable { current_ma: u16, voltage_mv: u16 },
}

// ============================================================================
// Charge manager
// ============================================================================

pub struct ChargeManager {
    current_ma: u16,
    phase: ChargePhase,
    charger_status: ChargerStatus,
    max_current_ma: u16,
    charge_voltage_mv: u16,
    charge_enabled: bool,
    charge_temp_derating: i16,
    charge_temp_limit: i16,
}

impl ChargeManager {
    pub fn new() -> Self {
        Self {
            current_ma: 0,
            phase: ChargePhase::Unknown,
            charger_status: ChargerStatus::Disconnected,
            max_current_ma: CHARGE_CURRENT_MAX_DEFAULT_MA,
            charge_voltage_mv: CHARGE_VOLTAGE_DEFAULT_MV,
            charge_enabled: true,
            charge_temp_derating: 650,
            charge_temp_limit: 800,
        }
    }

    pub fn phase(&self) -> ChargePhase {
        self.phase
    }

    pub fn charger_status(&self) -> ChargerStatus {
        self.charger_status
    }

    pub fn current_ma(&self) -> u16 {
        self.current_ma
    }

    pub fn set_charge_voltage(&mut self, mv: u16) {
        self.charge_voltage_mv = mv;
    }

    pub fn set_max_current(&mut self, ma: u16) {
        self.max_current_ma = ma;
    }

    pub fn set_charge_enabled(&mut self, enabled: bool) {
        self.charge_enabled = enabled;
    }

    pub fn set_temp_thresholds(&mut self, derating: i16, limit: i16) {
        self.charge_temp_derating = derating;
        self.charge_temp_limit = limit;
    }

    /// Charging state machine update (1Hz)
    pub fn update(
        &mut self,
        husb_attached: bool,
        husb_fault: bool,
        husb_support_charge: bool,
        husb_voltage_mv: u16,
        husb_current_ma: f32,
        batt_temp: i16,
        batt_voltage_mv: u16,
        batt_soc: u8,
        charger_temp: i16,
    ) -> (ChargePhase, u16) {
        let husb_power_mw = husb_voltage_mv as f32 * husb_current_ma / 1000.0;

        // Charger status
        if !husb_attached {
            self.charger_status = ChargerStatus::Disconnected;
            self.current_ma = 0;
            self.phase = ChargePhase::NotCharging;
            return (self.phase, 0);
        }
        if husb_fault {
            self.charger_status = ChargerStatus::Fault;
            self.current_ma = 0;
            self.phase = ChargePhase::HusbFault;
            return (self.phase, 0);
        }
        if !husb_support_charge {
            self.charger_status = ChargerStatus::Unsupported;
            self.current_ma = 0;
            self.phase = ChargePhase::Unsupported;
            return (self.phase, 0);
        }

        self.charger_status = ChargerStatus::Connected;

        if !self.charge_enabled {
            self.current_ma = 0;
            self.phase = ChargePhase::NotCharging;
            return (self.phase, 0);
        }

        // Calculate target current
        let target = calc_charge_current_ma(
            batt_temp,
            charger_temp,
            husb_power_mw,
            batt_voltage_mv,
            self.max_current_ma,
            self.charge_temp_derating,
            self.charge_temp_limit,
        );

        // Battery full
        if batt_soc >= 100 {
            self.current_ma = 0;
            self.phase = ChargePhase::Full;
            return (self.phase, 0);
        }

        // Temperature protection
        if target == 0 {
            self.current_ma = 0;
            self.phase = ChargePhase::ThermalProtect;
            return (self.phase, 0);
        }

        // Current hysteresis (200mA)
        if target > self.current_ma + 200 || target < self.current_ma.saturating_sub(200) {
            self.current_ma = target;
        }

        // Charging phase determination
        if self.current_ma > 0 {
            if batt_voltage_mv >= self.charge_voltage_mv - 200 {
                self.phase = ChargePhase::Cv;
            } else if batt_voltage_mv < 12000 {
                self.phase = ChargePhase::PreCharge;
            } else {
                self.phase = ChargePhase::Cc;
            }
        } else {
            self.phase = ChargePhase::NotCharging;
        }

        (self.phase, self.current_ma)
    }

    /// Get BQ24725 command for the current state.
    pub fn bq24725_command(&self) -> Bq24725Command {
        if self.current_ma == 0 {
            Bq24725Command::Disable
        } else {
            Bq24725Command::Enable {
                current_ma: self.current_ma,
                voltage_mv: self.charge_voltage_mv,
            }
        }
    }
}
