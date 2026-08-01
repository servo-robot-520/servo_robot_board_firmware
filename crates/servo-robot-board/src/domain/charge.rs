//! Charging management
//!
//! Charging state machine + current calculation + BQ24725/HUSB238A/BQ40Z50 operation。

// ============================================================================
// Temperature threshold (°C) — hardcoded protection threshold
// ============================================================================

/// Battery low-temperature limit: Stop charging below this temperature => 0°C
const BAT_TEMP_COLD_LIMIT: i16 = 0;
/// Limits on low-temperature slow charging => 10°C
const BAT_TEMP_COOL_LIMIT: i16 = 100;
/// Battery room temperature upper limit => 45°C
const BAT_TEMP_WARM_LIMIT: i16 = 450;
/// Battery high-temperature limit: Stop charging above this temperature => 50°C
const BAT_TEMP_HOT_LIMIT: i16 = 500;

// ============================================================================
// Charging current limit (mA)
// ============================================================================

/// Minimum charging current => 0.5A
const CHARGE_CURRENT_MIN_MA: u16 = 500;
/// Default maximum charging current => 9A
const CHARGE_CURRENT_MAX_DEFAULT_MA: u16 = 9000;
/// Default charging voltage => (4S: 16.8V)
const CHARGE_VOLTAGE_DEFAULT_MV: u16 = 16800;

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
// 充电器状态
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum ChargerStatus {
    Disconnected,
    Connected,
    Fault,
    Unsupported,
}

// ============================================================================
// 充电电流计算
// ============================================================================

/// 根据温度和功率计算目标充电电流 (mA)
pub fn calc_charge_current_ma(
    batt_temp: i16,
    charger_temp: i16,
    husb_power_mw: f32,
    batt_voltage_mv: u16,
    max_current_ma: u16,
    charge_temp_derating: i16,
    charge_temp_limit: i16,
) -> u16 {
    // 电池温度保护
    if batt_temp < BAT_TEMP_COLD_LIMIT || batt_temp > BAT_TEMP_HOT_LIMIT {
        return 0;
    }

    let current_ma: u16;

    if batt_temp < BAT_TEMP_COOL_LIMIT {
        // Low-temperature, slow charging
        current_ma = CHARGE_CURRENT_MIN_MA;
    } else if batt_temp < BAT_TEMP_WARM_LIMIT {
        // Room temperature: Dynamic calculation
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
        // 高温降流: 45~50°C 线性降至 CHARGE_CURRENT_MIN_MA
        let range = BAT_TEMP_HOT_LIMIT - BAT_TEMP_WARM_LIMIT; // 50 (i16)
        let delta = BAT_TEMP_HOT_LIMIT - batt_temp;            // 0~50 (i16)
        // ratio_10000: 0 = 最热(50°C), 10000 = 最冷(45°C)
        let ratio_10000 = ((delta as i32 * 10000) / range as i32).clamp(0, 10000) as u16;
        let span = max_current_ma.saturating_sub(CHARGE_CURRENT_MIN_MA);
        current_ma = CHARGE_CURRENT_MIN_MA + (span as u32 * ratio_10000 as u32 / 10000) as u16;
    }

    // 充电电路温度保护 (阈值来自 Config)
    if charger_temp > charge_temp_limit {
        return 0;
    }
    let current_ma = if charger_temp > charge_temp_derating {
        let temp_range = charge_temp_limit - charge_temp_derating;
        if temp_range <= 0 {
            // 防止除零: 如果阈值配置错误，直接停止充电
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

    // 钳位
    current_ma.min(max_current_ma)
}

// ============================================================================
// BQ24725 操作指令
// ============================================================================

pub enum Bq24725Command {
    Disable,
    Enable { current_ma: u16, voltage_mv: u16 },
}

// ============================================================================
// 充电管理器
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
            charge_temp_derating: 600,
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

    /// 充电状态机更新 (1Hz)
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

        // 充电器状态判断
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

        // 计算目标电流
        let target = calc_charge_current_ma(
            batt_temp,
            charger_temp,
            husb_power_mw,
            batt_voltage_mv,
            self.max_current_ma,
            self.charge_temp_derating,
            self.charge_temp_limit,
        );

        // 电池已满
        if batt_soc >= 100 {
            self.current_ma = 0;
            self.phase = ChargePhase::Full;
            return (self.phase, 0);
        }

        // 温度保护
        if target == 0 {
            self.current_ma = 0;
            self.phase = ChargePhase::ThermalProtect;
            return (self.phase, 0);
        }

        // 电流迟滞 (200mA)
        if target > self.current_ma + 200 || target < self.current_ma.saturating_sub(200) {
            self.current_ma = target;
        }

        // 充电阶段判断
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

    /// 获取 BQ24725 操作指令
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

// ============================================================================
// 充电管理辅助函数
// ============================================================================

/// HUSB238A 状态数据
#[derive(Debug, Clone, Copy, Default)]
pub struct HusbStatus {
    pub attached: bool,
    pub fault: bool,
    pub support_charge: bool,
    pub voltage_mv: u16,
    pub current_ma: f32,
}

/// BQ40Z50 电池数据
#[derive(Debug, Clone, Copy, Default)]
pub struct BatteryData {
    pub temp_c: i16,
    pub voltage_mv: u16,
    pub soc: u8,
}

/// 充电更新结果
#[derive(Debug, Clone, Copy)]
pub struct ChargeUpdateResult {
    pub phase: ChargePhase,
    pub target_current_ma: u16,
    pub husb: HusbStatus,
    pub battery: BatteryData,
}

/// 读取 HUSB238A 状态
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

/// 读取 BQ40Z50 电池数据
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

/// 设置 BQ24725 充电参数
pub fn set_bq24725_charge<I2C, E>(
    i2c: &mut I2C,
    target_current_ma: u16,
    voltage_mv: u16,
    input_current_ma: u16,
) -> Result<(), E>
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    use embedded_bq24725::Bq24725;

    // 预验证输入范围，避免驱动返回范围错误导致 panic
    if target_current_ma < 128 || target_current_ma > 8128 {
        defmt::error!("BQ24725: charge current {} out of range (128-8128)", target_current_ma);
        return Ok(());
    }
    if voltage_mv < 1024 || voltage_mv > 19200 {
        defmt::error!("BQ24725: charge voltage {} out of range (1024-19200)", voltage_mv);
        return Ok(());
    }
    if input_current_ma > 0 && (input_current_ma < 128 || input_current_ma > 8064) {
        defmt::error!("BQ24725: input current {} out of range (128-8064)", input_current_ma);
        return Ok(());
    }

    let mut charger = Bq24725::new(i2c);
    // 范围错误不应发生（输入已预验证），但安全处理而非 panic
    charger
        .set_charge_current_ma(target_current_ma)
        .map_err(|e| match e {
            embedded_bq24725::Error::I2c(e) => e,
            _ => unreachable!("BQ24725 range error with validated inputs"),
        })?;
    charger
        .set_charge_voltage_mv(voltage_mv)
        .map_err(|e| match e {
            embedded_bq24725::Error::I2c(e) => e,
            _ => unreachable!("BQ24725 range error with validated inputs"),
        })?;
    if input_current_ma > 0 {
        charger
            .set_input_current_ma(input_current_ma)
            .map_err(|e| match e {
                embedded_bq24725::Error::I2c(e) => e,
                _ => unreachable!("BQ24725 range error with validated inputs"),
            })?;
    }
    Ok(())
}

/// 完整的充电状态更新
///
/// 1. 读取 HUSB238A 状态
/// 2. 读取 BQ40Z50 电池数据
/// 3. 运行充电状态机
/// 4. 设置 BQ24725 充电参数
/// 5. 返回充电结果
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
    // 读取 HUSB238A 状态
    let husb = read_husb_status(i2c);

    // 读取 BQ40Z50 电池数据
    let battery = read_battery_data(i2c);

    // 更新充电状态机
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

    // 设置 BQ24725 充电参数
    if target_current > 0 {
        let charge_voltage = charge_voltage_mv;
        let input_current = if husb.current_ma > 0.0 {
            husb.current_ma as u16
        } else {
            0
        };
        if let Err(_e) = set_bq24725_charge(i2c, target_current, charge_voltage, input_current) {
            defmt::warn!("BQ24725 charge set failed");
            super::error_stats::ERROR_STATS.inc_charge();
        }
    }

    ChargeUpdateResult {
        phase,
        target_current_ma: target_current,
        husb,
        battery,
    }
}
