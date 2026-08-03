//! Power-feature initialization.
//!
//! Applies persisted power policy to board outputs. Pin ownership remains in
//! `main.rs`; this module owns the meaning of the stored power configuration.

use servo_robot_protocol::config::BoardConfigSnapshot;

/// Restore power-control output levels from a persisted configuration.
pub fn restore_power_outputs(
    servo_power: &mut impl embedded_hal::digital::OutputPin,
    battery_output: &mut impl embedded_hal::digital::OutputPin,
    power_5v: &mut impl embedded_hal::digital::OutputPin,
    config: &BoardConfigSnapshot,
) {
    if config.power_servo_on {
        servo_power.set_high().ok();
    } else {
        servo_power.set_low().ok();
    }
    if config.bat_ext_out_on {
        battery_output.set_high().ok();
    } else {
        battery_output.set_low().ok();
    }
    if config.power_5v_on {
        power_5v.set_high().ok();
    } else {
        power_5v.set_low().ok();
    }
}
