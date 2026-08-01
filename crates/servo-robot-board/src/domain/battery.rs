//! 电池数据采集辅助函数
//!
//! 使用 embedded-bq40z50 crate 读取 BQ40Z50 电池状态数据。

use embedded_bq40z50::Bq40z50;
use servo_robot_protocol::battery_state::{
    BatteryChargeStatus, BatteryHealth, BatteryState, BatteryTechnology,
};

/// 从 BQ40Z50 读取完整电池状态
///
/// 使用 embedded-bq40z50 crate 的 API 读取所有电池数据，
/// 转换为协议中的 BatteryState 格式。
pub fn read_bq40z50_data<I2C, E>(gauge: &mut Bq40z50<I2C>) -> BatteryState
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    let voltage_mv = gauge.voltage_mv().unwrap_or(0);
    let current_ma = gauge.current_ma().unwrap_or(0);
    let soc = gauge.relative_soc().unwrap_or(0);
    let temperature = gauge.temperature_c().unwrap_or(250);
    let remaining_cap = gauge.remaining_capacity_mah().unwrap_or(0);
    let full_charge_cap = gauge.full_charge_capacity_mah().unwrap_or(0);
    let design_cap = gauge.design_capacity_mah().unwrap_or(0);
    let cell_voltages = gauge.cell_voltages_mv().unwrap_or([0; 4]);
    let battery_status = gauge.battery_status().unwrap_or(0);

    // 解析电池状态
    let charge_status = if battery_status & (1 << 6) != 0 {
        BatteryChargeStatus::Discharging
    } else if battery_status & (1 << 5) != 0 {
        BatteryChargeStatus::Full
    } else if current_ma != 0 {
        BatteryChargeStatus::Charging
    } else {
        BatteryChargeStatus::NotCharging
    };

    BatteryState {
        voltage_mv,
        current_ma,
        capacity_mah: remaining_cap,
        design_capacity_mah: design_cap,
        percentage: soc,
        temperature,
        charge_status,
        // 健康状态: 从 battery_status 标志位推导
        health: if battery_status & embedded_bq40z50::STATUS_OVER_TEMP_ALARM != 0 {
            BatteryHealth::Overheat
        } else if battery_status & embedded_bq40z50::STATUS_OVER_CHARGE_ALARM != 0 {
            BatteryHealth::Overvoltage
        } else {
            BatteryHealth::Good
        },
        // 技术类型: 从 DeviceChemistry 读取
        technology: match gauge.device_chemistry_u16() {
            Ok(0x4F4C) => BatteryTechnology::LiOn, // "LO" (LION)
            Ok(0x504C) => BatteryTechnology::LiPo, // "LP" (LIPO)
            Ok(0x464C) => BatteryTechnology::LiFe, // "LF" (LIFE)
            _ => BatteryTechnology::Unknown,
        },
        // 在位检测: 电压读取成功即为在位
        present: voltage_mv > 0,
        // 序列号
        serial_number: gauge.serial_number().unwrap_or(0),
        cell_voltages_mv: alloc::vec![
            cell_voltages[0],
            cell_voltages[1],
            cell_voltages[2],
            cell_voltages[3],
        ],
        cell_temperatures: alloc::vec![temperature; 4],
    }
}

/// 从 BQ40Z50 读取电池温度 (用于充电管理)
pub fn read_battery_temp<I2C, E>(gauge: &mut Bq40z50<I2C>) -> i16
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    gauge.temperature_c().unwrap_or(250)
}

/// 从 BQ40Z50 读取电池电压 (mV)
pub fn read_battery_voltage<I2C, E>(gauge: &mut Bq40z50<I2C>) -> u16
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    gauge.voltage_mv().unwrap_or(16800)
}

/// 从 BQ40Z50 读取电池 SOC (%)
pub fn read_battery_soc<I2C, E>(gauge: &mut Bq40z50<I2C>) -> u8
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    gauge.relative_soc().unwrap_or(50)
}
