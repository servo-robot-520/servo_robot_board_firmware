//! Power/thermal task helper functions.
//!
//! Pure computation helpers extracted from RTIC tasks. These functions
//! encapsulate data assembly and decision logic without depending on
//! RTIC lock semantics.

use servo_robot_protocol::power::PowerData;

use super::protection::{ProtectionFlags, ProtectionManager};
use super::sampling::{Ina219Data, charge_current_ma, pd_voltage_mv};
use super::thermal;
use crate::platform::adc;

// ============================================================================
// Power data assembly
// ============================================================================

/// Build `PowerData` from INA219 reading and raw ADC values.
///
/// Called by `power_task` (20Hz) after reading INA219 via I2C and ADC DMA buffer.
pub fn build_power_data(ina: &Ina219Data, bc_iout_adc: u16, cv_adc_adc: u16) -> PowerData {
    PowerData {
        servo_voltage_mv: ina.bus_voltage as u16,
        servo_current_ma: ina.current_ma as u16,
        charge_in_current_ma: charge_current_ma(bc_iout_adc) as u16,
        charge_in_voltage_mv: pd_voltage_mv(cv_adc_adc) as u16,
        ..PowerData::default()
    }
}

/// Read the power-monitoring peripherals and assemble the wire-format sample.
///
/// Returns the sample plus the unrounded servo current used by protection
/// logic, so current-limit decisions retain INA219 precision.
pub fn sample_power<I2C, E>(
    i2c: &mut I2C,
    adc_samples: &[u16; adc::ADC_CHANNEL_COUNT],
) -> (PowerData, f32)
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    let ina = super::sampling::read_ina219_data(i2c);
    let data = build_power_data(
        &ina,
        adc_samples[adc::CH_BC_IOUT],
        adc_samples[adc::CH_CV_ADC],
    );
    (data, ina.current_ma)
}

// ============================================================================
// Thermal readings
// ============================================================================

/// Read all NTC + MCU temperatures from the ADC DMA buffer.
///
/// Returns `(temp_charge, temp_servo, temp_5v, mcu_temp)` in degrees Celsius.
pub fn read_thermal_temperatures(buf: &[u16; adc::ADC_CHANNEL_COUNT]) -> (f32, f32, f32, f32) {
    (
        thermal::ntc_temp_c(buf[adc::CH_TEMP_CHARGE]),
        thermal::ntc_temp_c(buf[adc::CH_TEMP_SERVO]),
        thermal::ntc_temp_c(buf[adc::CH_TEMP_5V]),
        thermal::mcu_temp_c(buf[adc::CH_MCU_TEMP]),
    )
}

/// Convert the charge-circuit NTC sample to the protocol's deci-degree unit.
///
/// Charge management and configuration thresholds both use 0.1°C units.
pub fn charge_temperature_deci_c(samples: &[u16; adc::ADC_CHANNEL_COUNT]) -> i16 {
    (thermal::ntc_temp_c(samples[adc::CH_TEMP_CHARGE]) * 10.0) as i16
}

// ============================================================================
// Thermal protection check
// ============================================================================

/// Run thermal protection check, returns `(flags, servo_should_cut, 5v_should_cut)`.
///
/// Caller is responsible for actually cutting power GPIO if the cut flags are set.
pub fn check_thermal_protection(
    pm: &mut ProtectionManager,
    temp_servo: f32,
    temp_5v: f32,
) -> (ProtectionFlags, bool, bool) {
    let (s_cut, v_cut) = pm.check_thermal(temp_servo, temp_5v);
    (pm.flags(), s_cut, v_cut)
}

// ============================================================================
// Fan hysteresis control
// ============================================================================

/// Compute fan on/off state with hysteresis.
///
/// - ON when any temperature exceeds `limit - 10`
/// - OFF when all temperatures are below `limit - 15`
/// - Otherwise, keep the current state (`fan_currently_on`)
pub fn check_fan_hysteresis(
    temp_servo: f32,
    temp_5v: f32,
    temp_charge: f32,
    servo_limit: f32,
    v5_limit: f32,
    charge_limit: f32,
    fan_currently_on: bool,
) -> bool {
    let should_on = temp_servo > (servo_limit - 10.0)
        || temp_5v > (v5_limit - 10.0)
        || temp_charge > (charge_limit - 10.0);

    let should_off = temp_servo < (servo_limit - 15.0)
        && temp_5v < (v5_limit - 15.0)
        && temp_charge < (charge_limit - 15.0);

    if should_on {
        true
    } else if should_off {
        false
    } else {
        fan_currently_on
    }
}

/// Evaluate servo over-current protection from an INA219 current reading.
pub fn check_servo_overcurrent(
    manager: &mut ProtectionManager,
    current_ma: f32,
) -> (ProtectionFlags, bool) {
    let should_cut = manager.check_current(current_ma / 1000.0);
    (manager.flags(), should_cut)
}

/// Temperature limits derived from the persistent power configuration.
#[derive(Clone, Copy, Debug)]
pub struct ThermalLimits {
    pub servo_c: f32,
    pub power_5v_c: f32,
    pub charge_c: f32,
}

/// Convert the protocol's deci-degree configuration fields to Celsius.
pub fn thermal_limits(config: &servo_robot_protocol::config::BoardConfigSnapshot) -> ThermalLimits {
    ThermalLimits {
        servo_c: config.servo_temp_limit as f32 / 10.0,
        power_5v_c: config.temp_5v_limit as f32 / 10.0,
        charge_c: config.charge_temp_limit as f32 / 10.0,
    }
}

/// GPIO-independent outcomes of the periodic thermal-control update.
#[derive(Clone, Copy, Debug)]
pub struct ThermalControl {
    pub protection_flags: ProtectionFlags,
    pub cut_servo_power: bool,
    pub cut_5v_power: bool,
    pub fan_on: bool,
}

/// Update thermal protection state and compute fan control for one system tick.
pub fn evaluate_thermal_control(
    manager: &mut ProtectionManager,
    temp_charge: f32,
    temp_servo: f32,
    temp_5v: f32,
    limits: ThermalLimits,
    fan_currently_on: bool,
) -> ThermalControl {
    manager.set_servo_temp_limit(limits.servo_c);
    manager.set_5v_temp_limit(limits.power_5v_c);
    let (protection_flags, cut_servo_power, cut_5v_power) =
        check_thermal_protection(manager, temp_servo, temp_5v);
    let fan_on = check_fan_hysteresis(
        temp_servo,
        temp_5v,
        temp_charge,
        limits.servo_c,
        limits.power_5v_c,
        limits.charge_c,
        fan_currently_on,
    );
    ThermalControl {
        protection_flags,
        cut_servo_power,
        cut_5v_power,
        fan_on,
    }
}
