//! 通讯命令处理
//!
//! 配置查询、日志过滤、配置写入。

use servo_robot_protocol::config::{BoardConfigSnapshot, Config, ConfigType};
use servo_robot_protocol::event::StateChangeFlags;
use servo_robot_protocol::log::LogLevel;

// ============================================================================
// 日志过滤
// ============================================================================

/// 检查日志等级是否应该发送
pub fn should_send_log(tx_log_level: LogLevel, msg_level: LogLevel) -> bool {
    if tx_log_level == LogLevel::OFF {
        return false;
    }
    (msg_level as u8) >= (tx_log_level as u8)
}

// ============================================================================
// 配置查询
// ============================================================================

/// 获取配置值
pub fn get_config_value(c: &BoardConfigSnapshot, ct: ConfigType) -> Config {
    match ct {
        ConfigType::SwitchServoPower => Config::SwitchPowerServo(c.power_servo_on),
        ConfigType::Switch5VPower => Config::SwitchPower5V(c.power_5v_on),
        ConfigType::SwitchCharge => Config::SwitchCharge(c.charge_on),
        ConfigType::SwitchBatExtOut => Config::SwitchBatExtOut(c.bat_ext_out_on),
        ConfigType::PowerServoCurrentLimitMa => {
            Config::PowerServoCurrentLimitMa(c.servo_current_limit_ma)
        }
        ConfigType::PowerServoTempLimit => Config::PowerServoTempLimit(c.servo_temp_limit),
        ConfigType::Power5vTempLimit => Config::Power5vTempLimit(c.temp_5v_limit),
        ConfigType::ChargeMaxCurrentMa => Config::ChargeMaxCurrentMa(c.charge_max_current_ma),
        ConfigType::ChargeTempDerating => Config::ChargeTempDerating(c.charge_temp_derating),
        ConfigType::ChargeTempLimit => Config::ChargeTempLimit(c.charge_temp_limit),
        ConfigType::ChargeStopVoltageMv => Config::ChargeStopVoltageMv(c.charge_stop_voltage_mv),
        ConfigType::ChargeStopSoc => Config::ChargeStopSoc(c.charge_stop_percentage),
        ConfigType::TxLogLevel => Config::TxLogLevel(c.tx_log_level),
        ConfigType::ServoBaudRate => Config::ServoBaudRate(c.servo_baud_rate),
    }
}

// ============================================================================
// 配置写入
// ============================================================================

/// Side effects from applying a config change.
///
/// The caller (RTIC task) is responsible for applying GPIO/protection changes
/// since those require RTIC lock access.
pub struct ConfigEffect {
    /// State change flags to set on board_event
    pub state_flag: Option<(StateChangeFlags, bool)>,
    /// Whether to reset servo protection manager
    pub reset_servo_protection: bool,
    /// Whether to reset 5V protection manager
    pub reset_5v_protection: bool,
    /// GPIO action for servo power
    pub servo_power: Option<bool>,
    /// GPIO action for bat ext out
    pub bat_ext_out: Option<bool>,
    /// GPIO action for 5V power
    pub power_5v: Option<bool>,
}

/// Persistent snapshot and board-side effects produced by a config write.
pub struct ConfigWriteResult {
    pub effect: ConfigEffect,
    pub snapshot: BoardConfigSnapshot,
}

/// Apply a Config variant to the config snapshot and return side effects.
///
/// Pure config mutations are applied directly. GPIO and protection changes
/// are returned as `ConfigEffect` for the caller to execute under RTIC locks.
pub fn apply_config(cfg: &Config, config: &mut BoardConfigSnapshot) -> ConfigEffect {
    let mut effect = ConfigEffect {
        state_flag: None,
        reset_servo_protection: false,
        reset_5v_protection: false,
        servo_power: None,
        bat_ext_out: None,
        power_5v: None,
    };

    match cfg {
        Config::SwitchPowerServo(on) => {
            config.power_servo_on = *on;
            effect.servo_power = Some(*on);
            if *on {
                effect.reset_servo_protection = true;
            }
            effect.state_flag = Some((StateChangeFlags::SERVO_POWER_ON, *on));
        }
        Config::SwitchPower5V(on) => {
            config.power_5v_on = *on;
            effect.power_5v = Some(*on);
            if *on {
                effect.reset_5v_protection = true;
            }
            effect.state_flag = Some((StateChangeFlags::POWER_5V_ON, *on));
        }
        Config::SwitchCharge(on) => {
            config.charge_on = *on;
        }
        Config::SwitchBatExtOut(on) => {
            config.bat_ext_out_on = *on;
            effect.bat_ext_out = Some(*on);
            effect.state_flag = Some((StateChangeFlags::BAT_EXT_OUT_ON, *on));
        }
        Config::PowerServoCurrentLimitMa(v) => config.servo_current_limit_ma = *v,
        Config::PowerServoTempLimit(v) => config.servo_temp_limit = *v,
        Config::Power5vTempLimit(v) => config.temp_5v_limit = *v,
        Config::ChargeMaxCurrentMa(v) => config.charge_max_current_ma = *v,
        Config::ChargeTempDerating(v) => config.charge_temp_derating = *v,
        Config::ChargeTempLimit(v) => config.charge_temp_limit = *v,
        Config::ChargeStopVoltageMv(v) => config.charge_stop_voltage_mv = *v,
        Config::ChargeStopSoc(v) => config.charge_stop_percentage = *v,
        Config::TxLogLevel(level) => config.tx_log_level = *level,
        Config::ServoBaudRate(v) => {
            #[cfg(feature = "servo")]
            {
                config.servo_baud_rate = *v;
            }
            #[cfg(not(feature = "servo"))]
            {
                let _ = v;
            }
        }
    }

    effect
}

/// Apply a config write and return the snapshot that must be persisted.
///
/// The caller still performs GPIO/protection operations under RTIC locks and
/// writes `snapshot` only after releasing the config lock.
pub fn apply_config_write(cfg: &Config, config: &mut BoardConfigSnapshot) -> ConfigWriteResult {
    let effect = apply_config(cfg, config);
    ConfigWriteResult {
        effect,
        snapshot: config.clone(),
    }
}
