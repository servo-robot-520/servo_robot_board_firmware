//! 电源数据采集辅助函数
//!
//! 电压/电流相关转换 + INA219 驱动

use crate::hal::adc;

// ============================================================================
// INA219 (embedded-ina219 crate)
// ============================================================================

/// INA219 测量结果
#[derive(Debug, Clone, Copy, Default)]
pub struct Ina219Data {
    pub bus_voltage: f32,
    pub current_ma: f32,
}

/// 读取 INA219 数据 (分流电阻 2mΩ)
pub fn read_ina219_data<I2C, E>(i2c: &mut I2C) -> Ina219Data
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    use embedded_ina219::Ina219;
    let mut ina = Ina219::new(i2c);
    ina.set_shunt_resistor(2.0);

    match ina.read_all() {
        Ok(m) => Ina219Data {
            bus_voltage: m.bus_voltage,
            current_ma: m.current_ma,
        },
        Err(_) => Ina219Data::default(),
    }
}

// ============================================================================
// 充电电流 (INA199)
// ============================================================================

/// BC_IOUT ADC 转充电电流 (mA)
///
/// 电路: INA199, 采样电阻 10mΩ, 增益 100V/V
/// I(mA) = V_out(mV) / (10mΩ × 100) ≈ V_out(mV)
pub fn charge_current_ma(adc_val: u16) -> f32 {
    adc::adc_to_mv(adc_val)
}

// ============================================================================
// PD 电压检测
// ============================================================================

/// PD 分压比: (100K + 47K) / 47K ≈ 3.128
const PD_DIVIDER_RATIO: f32 = (100_000.0 + 47_000.0) / 47_000.0;

/// CV_ADC 转 PD 输入电压 (mV)
///
/// 电路: PD_IN → 100K → ADC → 47K → GND
pub fn pd_voltage_mv(adc_val: u16) -> f32 {
    adc::adc_to_mv(adc_val) * PD_DIVIDER_RATIO
}
