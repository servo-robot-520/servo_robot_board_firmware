//! Charge I/O helpers: read sensors, write BQ24725, run full update cycle.

use servo_robot_protocol::config::BoardConfigSnapshot;

use super::state::{
    BQ24725_CHARGE_CURRENT_MAX_MA, ChargeManager, ChargePhase, quantize_charge_current,
    quantize_charge_voltage, quantize_input_current,
};

// ============================================================================
// Data types
// ============================================================================

/// HUSB238A status snapshot
#[derive(Debug, Clone, Copy, Default)]
pub struct HusbStatus {
    pub attached: bool,
    pub fault: bool,
    pub support_charge: bool,
    pub voltage_mv: u16,
    pub current_ma: f32,
}

/// BQ40Z50 battery data snapshot
#[derive(Debug, Clone, Copy, Default)]
pub struct BatteryData {
    pub temp_c: i16,
    pub voltage_mv: u16,
    pub soc: u8,
}

/// Charge update result
#[derive(Debug, Clone, Copy)]
pub struct ChargeUpdateResult {
    pub phase: ChargePhase,
    /// true if BQ24725 write failed (I2C error or out-of-range)
    pub charge_error: bool,
}

// ============================================================================
// Sensor reads
// ============================================================================

/// Read HUSB238A status (attachment, fault, PD contract).
pub fn read_husb_status<I2C, E>(i2c: &mut I2C) -> HusbStatus
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    use embedded_husb238a::Husb238a;

    let mut husb = Husb238a::new(i2c);
    let attached = husb.charger_attached().unwrap_or(false);
    let mut pdo_buf = [embedded_husb238a::PdoInfo {
        code: 0,
        protocol: embedded_husb238a::ChargerProtocol::Unknown,
        voltage_mv: 0,
        current_ma: 0,
    }; 11];
    let support_charge = husb
        .source_pdos(&mut pdo_buf)
        .map(|count| pdo_buf[..count].iter().any(|pdo| pdo.voltage_mv >= 19_000))
        .unwrap_or(false);
    let _ = husb.update_contract_info();
    HusbStatus {
        attached,
        fault: husb.is_fault(),
        support_charge,
        voltage_mv: husb.contract_voltage_mv(),
        current_ma: husb.contract_current_ma(),
    }
}

/// Read BQ40Z50 battery data (temperature, voltage, SOC).
pub fn read_battery_data<I2C, E>(i2c: &mut I2C) -> BatteryData
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    use embedded_bq40z50::Bq40z50;

    let mut gauge = Bq40z50::new(i2c);
    BatteryData {
        temp_c: gauge.temperature_c().unwrap_or(250),
        voltage_mv: gauge.voltage_mv().unwrap_or(16800),
        soc: gauge.relative_soc().unwrap_or(50),
    }
}

// ============================================================================
// BQ24725 write
// ============================================================================

/// Set BQ24725 charge parameters.
///
/// Returns Ok(true) on success, Ok(false) if parameters out of range (safe stop),
/// Err on I2C failure. When target_current_ma is 0, charging is disabled.
pub fn set_bq24725_charge<I2C, E>(
    i2c: &mut I2C,
    target_current_ma: u16,
    voltage_mv: u16,
    input_current_ma: u16,
) -> Result<bool, E>
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    use embedded_bq24725::Bq24725;

    let mut charger = Bq24725::new(i2c);

    // Target current 0 => disable charging
    if target_current_ma == 0 {
        charger.set_charging_enabled(false).map_err(|e| match e {
            embedded_bq24725::Error::I2c(e) => e,
            _ => unreachable!(),
        })?;
        return Ok(true);
    }

    // Quantize to BQ24725 step sizes to prevent driver range errors
    // Charge current: 64mA step, input current: 128mA step
    let target_current = quantize_charge_current(target_current_ma);
    let input_current = quantize_input_current(input_current_ma);
    let charge_voltage = quantize_charge_voltage(voltage_mv);

    // Validate quantized values; out-of-range => safe stop
    if target_current < 128 || target_current > BQ24725_CHARGE_CURRENT_MAX_MA {
        defmt::error!(
            "BQ24725: charge current {} (quantized {}) out of range",
            target_current_ma,
            target_current
        );
        // Safe disable
        let _ = charger.set_charging_enabled(false);
        return Ok(false);
    }
    if charge_voltage < 1024 || charge_voltage > 19200 {
        defmt::error!(
            "BQ24725: charge voltage {} (quantized {}) out of range",
            voltage_mv,
            charge_voltage
        );
        let _ = charger.set_charging_enabled(false);
        return Ok(false);
    }

    // Quantized values are in valid range with correct step alignment
    charger
        .set_charge_current_ma(target_current)
        .map_err(|e| match e {
            embedded_bq24725::Error::I2c(e) => e,
            _ => unreachable!("BQ24725 range error after quantization"),
        })?;
    charger
        .set_charge_voltage_mv(charge_voltage)
        .map_err(|e| match e {
            embedded_bq24725::Error::I2c(e) => e,
            _ => unreachable!("BQ24725 range error after quantization"),
        })?;
    if input_current >= 128 {
        charger
            .set_input_current_ma(input_current)
            .map_err(|e| match e {
                embedded_bq24725::Error::I2c(e) => e,
                _ => unreachable!("BQ24725 range error after quantization"),
            })?;
    }
    // Ensure charging enabled
    charger.set_charging_enabled(true).map_err(|e| match e {
        embedded_bq24725::Error::I2c(e) => e,
        _ => unreachable!(),
    })?;
    Ok(true)
}

// ============================================================================
// Full charge update cycle
// ============================================================================

/// Complete charge state update:
///
/// 1. Read HUSB238A status
/// 2. Read BQ40Z50 battery data
/// 3. Run charge state machine
/// 4. Set BQ24725 charge parameters
/// 5. Return charge result
pub fn update_charge<I2C, E>(
    i2c: &mut I2C,
    cm: &mut ChargeManager,
    charge_enable: bool,
    max_current: u16,
    charge_voltage_mv: u16,
    temp_derating: i16,
    temp_limit: i16,
    charger_temp: i16,
) -> ChargeUpdateResult
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    // Read HUSB238A status
    let husb = read_husb_status(i2c);

    // Read BQ40Z50 battery data
    let battery = read_battery_data(i2c);

    // Update charge state machine
    cm.set_max_current(max_current);
    cm.set_charge_voltage(charge_voltage_mv);
    cm.set_charge_enabled(charge_enable);
    cm.set_temp_thresholds(temp_derating, temp_limit);

    let (phase, target_current) = cm.update(
        husb.attached,
        husb.fault,
        husb.support_charge,
        husb.voltage_mv,
        husb.current_ma,
        battery.temp_c,
        battery.voltage_mv,
        battery.soc,
        charger_temp,
    );

    // Set BQ24725 charge parameters
    // Always send hardware command regardless of target current to keep BQ24725 in sync
    let charge_voltage = charge_voltage_mv;
    let input_current = if husb.current_ma > 0.0 {
        husb.current_ma as u16
    } else {
        0
    };
    let charge_error = match set_bq24725_charge(i2c, target_current, charge_voltage, input_current)
    {
        Ok(true) => false,
        Ok(false) => {
            defmt::warn!("BQ24725: params out of range, charge stopped");
            true
        }
        Err(_e) => {
            defmt::warn!("BQ24725 charge set I2C error");
            true
        }
    };

    ChargeUpdateResult {
        phase,
        charge_error,
    }
}

/// Project the internal charge phase onto the wire protocol's event phase.
/// Result of one charge task cycle, projected to the protocol event model.
#[derive(Clone, Copy, Debug)]
pub struct ChargeTaskUpdate {
    pub phase: servo_robot_protocol::event::ChargePhase,
    pub charge_error: bool,
}

/// Run the complete charging cycle using the current configuration snapshot.
///
/// The RTIC wrapper owns the individual resource locks; this function owns
/// the charge-feature configuration mapping and state-machine update.
pub fn run_charge_cycle<I2C, E>(
    i2c: &mut I2C,
    manager: &mut ChargeManager,
    config: &BoardConfigSnapshot,
    charger_temp: i16,
) -> ChargeTaskUpdate
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    let result = update_charge(
        i2c,
        manager,
        config.charge_on,
        config.charge_max_current_ma,
        config.charge_stop_voltage_mv,
        config.charge_temp_derating as i16,
        config.charge_temp_limit as i16,
        charger_temp,
    );
    ChargeTaskUpdate {
        phase: protocol_charge_phase(result.phase),
        charge_error: result.charge_error,
    }
}

pub fn protocol_charge_phase(
    phase: super::state::ChargePhase,
) -> servo_robot_protocol::event::ChargePhase {
    use servo_robot_protocol::event::ChargePhase as EventPhase;

    match phase {
        super::state::ChargePhase::NotCharging => EventPhase::NotCharging,
        super::state::ChargePhase::PreCharge => EventPhase::PreCharge,
        super::state::ChargePhase::Cc => EventPhase::Cc,
        super::state::ChargePhase::Cv => EventPhase::Cv,
        super::state::ChargePhase::Full => EventPhase::Full,
        super::state::ChargePhase::HusbFault => EventPhase::PdSinkFault,
        super::state::ChargePhase::Unsupported => EventPhase::UnsupportedCharger,
        super::state::ChargePhase::Unknown | super::state::ChargePhase::ThermalProtect => {
            EventPhase::NotCharging
        }
    }
}

/// Update the charger-connected event bit from the BC_ACOK input level.
pub fn set_charger_connected(event: &mut servo_robot_protocol::event::BoardEvent, connected: bool) {
    event.state_change_flags.set(
        servo_robot_protocol::event::StateChangeFlags::CHARGER_CONNECTED,
        connected,
    );
}

/// Snapshot HUSB238A's latched interrupt and current attachment state.
///
/// The I²C ownership stays with the RTIC wrapper; this function contains the
/// HUSB-specific interpretation and does not know about RTIC or task spawning.
pub fn handle_husb_interrupt<I2C, E>(i2c: &mut I2C) -> Option<HusbInterrupt>
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    let mut husb = embedded_husb238a::Husb238a::new(i2c);
    let status = match husb.handle_interrupt() {
        Ok(status) => status,
        Err(_) => {
            defmt::warn!("HUSB238A handle_interrupt failed");
            return None;
        }
    };
    let attached = match husb.charger_attached() {
        Ok(attached) => attached,
        Err(_) => {
            defmt::warn!("HUSB238A read attachment state failed");
            return None;
        }
    };

    Some(HusbInterrupt {
        attached,
        attach_changed: status.has_attach_change(),
        fault: status.has_fault(),
    })
}

/// HUSB interrupt data projected into the charge feature's event model.
#[derive(Clone, Copy, Debug, defmt::Format)]
pub struct HusbInterrupt {
    pub attached: bool,
    pub attach_changed: bool,
    pub fault: bool,
}

/// Apply a HUSB interrupt snapshot to the board event.
pub fn apply_husb_interrupt(
    event: &mut servo_robot_protocol::event::BoardEvent,
    interrupt: HusbInterrupt,
) {
    set_charger_connected(event, interrupt.attached);
    if interrupt.fault {
        event.charge_phase = servo_robot_protocol::event::ChargePhase::PdSinkFault;
    }
}
