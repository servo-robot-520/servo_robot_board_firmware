#![no_std]

//! INA219 电流/功率监测驱动
//!
//! 基于 TI INA219 datasheet (SBOS448G) 实现。
//! 支持配置寄存器、校准、读取总线电压、分流电压、电流、功率。
//!
//! # 使用示例
//!
//! ```ignore
//! let mut ina = Ina219::new(i2c);
//! // 配置：32V量程，PGA=/1 (±40mV)，12-bit 连续模式
//! ina.configure(BusVoltageRange::Range32V, PgaGain::Gain1, AdcResolution::Bits12, AdcResolution::Bits12, OperatingMode::ShuntAndBusContinuous)?;
//! // 校准：2mΩ 分流电阻，最大 15A
//! ina.calibrate(Calibration::new(2_000, 15_000_000)?)?;
//! // 读取
//! let measurement = ina.read_all()?;
//! ```

use embedded_hal::i2c::I2c;

/// INA219 默认 I2C 地址 (7-bit)
pub const INA219_ADDR: u8 = 0x40;

// ============================================================================
// 寄存器地址
// ============================================================================

/// INA219 寄存器地址
pub mod registers {
    /// 配置寄存器 (R/W)
    pub const CONFIGURATION: u8 = 0x00;
    /// 分流电压寄存器 (R, 有符号, 10µV/LSB)
    pub const SHUNT_VOLTAGE: u8 = 0x01;
    /// 总线电压寄存器 (R, 4mV/LSB, 含 CNVR/OVF 标志)
    pub const BUS_VOLTAGE: u8 = 0x02;
    /// 功率寄存器 (R, 需校准)
    pub const POWER: u8 = 0x03;
    /// 电流寄存器 (R, 需校准)
    pub const CURRENT: u8 = 0x04;
    /// 校准寄存器 (R/W)
    pub const CALIBRATION: u8 = 0x05;
}

// ============================================================================
// 配置常量
// ============================================================================

/// 总线电压量程 (BRNG, Configuration bit 13)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum BusVoltageRange {
    /// 16V 量程 (BRNG=0)
    Range16V = 0,
    /// 32V 量程 (BRNG=1, 默认)
    Range32V = 1 << 13,
}

/// PGA 增益设置 (PG, Configuration bits 12:11)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum PgaGain {
    /// 增益 ×1, ±40mV (PG=00)
    Gain1 = 0,
    /// 增益 /2, ±80mV (PG=01)
    Gain2 = 1 << 11,
    /// 增益 /4, ±160mV (PG=10)
    Gain4 = 2 << 11,
    /// 增益 /8, ±320mV (PG=11, 默认)
    Gain8 = 3 << 11,
}

impl PgaGain {
    /// 返回满量程电压 (mV)
    pub fn full_scale_mv(self) -> f32 {
        match self {
            PgaGain::Gain1 => 40.0,
            PgaGain::Gain2 => 80.0,
            PgaGain::Gain4 => 160.0,
            PgaGain::Gain8 => 320.0,
        }
    }
}

/// ADC 分辨率/平均设置 (BADC/SADC, Configuration bits 10:7 / 6:3)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum AdcResolution {
    /// 9 bit (84µs)
    Bits9 = 0b0000,
    /// 10 bit (148µs)
    Bits10 = 0b0001,
    /// 11 bit (276µs)
    Bits11 = 0b0010,
    /// 12 bit (532µs, 默认)
    Bits12 = 0b0011,
    /// 12 bit, 2 samples (1.06ms)
    Samples2 = 0b1001,
    /// 12 bit, 4 samples (2.13ms)
    Samples4 = 0b1010,
    /// 12 bit, 8 samples (4.26ms)
    Samples8 = 0b1011,
    /// 12 bit, 16 samples (8.51ms)
    Samples16 = 0b1100,
    /// 12 bit, 32 samples (17.02ms)
    Samples32 = 0b1101,
    /// 12 bit, 64 samples (34.05ms)
    Samples64 = 0b1110,
    /// 12 bit, 128 samples (68.10ms)
    Samples128 = 0b1111,
}

/// 工作模式 (MODE, Configuration bits 2:0)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum OperatingMode {
    /// 关机模式
    PowerDown = 0b000,
    /// 分流电压, 触发模式
    ShuntVoltageTriggered = 0b001,
    /// 总线电压, 触发模式
    BusVoltageTriggered = 0b010,
    /// 分流和总线, 触发模式
    ShuntAndBusTriggered = 0b011,
    /// ADC 关闭 (禁用)
    AdcOff = 0b100,
    /// 分流电压, 连续模式
    ShuntVoltageContinuous = 0b101,
    /// 总线电压, 连续模式
    BusVoltageContinuous = 0b110,
    /// 分流和总线, 连续模式 (默认)
    ShuntAndBusContinuous = 0b111,
}

/// 总线电压寄存器状态标志
#[derive(Debug, Clone, Copy)]
pub struct BusVoltageStatus {
    /// 转换就绪标志 (CNVR, bit 1)
    pub conversion_ready: bool,
    /// 溢出标志 (OVF, bit 0)
    pub overflow: bool,
}

/// Validated INA219 calibration parameters.
///
/// Inputs use integer micro-units to keep the common path usable on MCUs
/// without a floating-point unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Calibration {
    register: u16,
    current_lsb_microamps: u32,
}

impl Calibration {
    /// Create a calibration for `shunt_micro_ohms` and expected peak current.
    pub fn new(
        shunt_micro_ohms: u32,
        max_current_microamps: u32,
    ) -> Result<Self, CalibrationError> {
        if shunt_micro_ohms == 0 {
            return Err(CalibrationError::ZeroShuntResistance);
        }
        if max_current_microamps == 0 {
            return Err(CalibrationError::ZeroExpectedCurrent);
        }
        let current_lsb_microamps = max_current_microamps.div_ceil(32_768);
        let denominator = u64::from(current_lsb_microamps) * u64::from(shunt_micro_ohms);
        let register = (40_960_000_000u64 / denominator) & !1;
        if register == 0 || register > 0xFFFE {
            return Err(CalibrationError::OutOfRange);
        }
        Ok(Self {
            register: register as u16,
            current_lsb_microamps,
        })
    }

    /// Register value to write to `CALIBRATION`.
    pub const fn register(self) -> u16 {
        self.register
    }

    /// Current scale in microamps per CURRENT-register LSB.
    pub const fn current_lsb_microamps(self) -> u32 {
        self.current_lsb_microamps
    }
}

/// Calibration-input validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationError {
    /// Shunt resistance must be nonzero.
    ZeroShuntResistance,
    /// Expected maximum current must be nonzero.
    ZeroExpectedCurrent,
    /// The requested values cannot be represented by the 16-bit calibration register.
    OutOfRange,
}

/// 电源测量数据
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct PowerMeasurement {
    /// 总线电压 (V)
    pub bus_voltage: f32,
    /// 分流电压 (µV)
    pub shunt_voltage_uv: f32,
    /// 电流 (mA)
    pub current_ma: f32,
    /// 功率 (mW)
    pub power_mw: f32,
}

/// INA219 驱动
pub struct Ina219<I2C> {
    i2c: I2C,
    address: u8,
    /// 分流电阻 (mΩ)
    shunt_resistor_mohm: f32,
    /// 当前配置值
    config: u16,
    /// Calibration used for hardware CURRENT and POWER values.
    calibration: Option<Calibration>,
}

impl<I2C, I2cError> Ina219<I2C>
where
    I2C: I2c<Error = I2cError>,
{
    /// 创建 INA219 驱动实例 (默认地址 0x40)
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            address: INA219_ADDR,
            shunt_resistor_mohm: 2.0, // 默认 2mΩ
            config: 0x39F,            // 上电默认值
            calibration: None,
        }
    }

    /// 创建 INA219 驱动实例 (自定义地址)
    pub fn with_address(i2c: I2C, address: u8) -> Self {
        Self {
            i2c,
            address,
            shunt_resistor_mohm: 2.0,
            config: 0x39F,
            calibration: None,
        }
    }

    /// 设置分流电阻值 (mΩ)
    pub fn set_shunt_resistor(&mut self, mohm: f32) {
        self.shunt_resistor_mohm = mohm;
    }

    /// Consume the driver and return the I²C bus.
    pub fn destroy(self) -> I2C {
        self.i2c
    }

    // ========================================================================
    // 底层寄存器读写
    // ========================================================================

    /// 读取 16-bit 寄存器 (大端序)
    fn read_word(&mut self, reg: u8) -> Result<u16, I2cError> {
        let mut buf = [0u8; 2];
        self.i2c.write_read(self.address, &[reg], &mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    /// 写入 16-bit 寄存器 (大端序)
    fn write_word(&mut self, reg: u8, value: u16) -> Result<(), I2cError> {
        let bytes = value.to_be_bytes();
        self.i2c.write(self.address, &[reg, bytes[0], bytes[1]])
    }

    // ========================================================================
    // 复位
    // ========================================================================

    /// 软件复位 INA219 (等同于上电复位)
    ///
    /// 写入 Configuration 寄存器的 RST 位 (bit 15)。
    /// 复位后需要等待至少 40µs 才能进行下一次通信。
    pub fn reset(&mut self) -> Result<(), I2cError> {
        self.write_word(registers::CONFIGURATION, 0x8000)?;
        self.config = 0x39F; // 恢复默认值
        Ok(())
    }

    // ========================================================================
    // 配置
    // ========================================================================

    /// 配置 INA219
    ///
    /// # 参数
    /// - `bus_range`: 总线电压量程 (16V 或 32V)
    /// - `pga`: PGA 增益 (决定分流电压量程)
    /// - `bus_adc`: 总线 ADC 分辨率/平均
    /// - `shunt_adc`: 分流 ADC 分辨率/平均
    /// - `mode`: 工作模式
    pub fn configure(
        &mut self,
        bus_range: BusVoltageRange,
        pga: PgaGain,
        bus_adc: AdcResolution,
        shunt_adc: AdcResolution,
        mode: OperatingMode,
    ) -> Result<(), I2cError> {
        let config = bus_range as u16
            | pga as u16
            | ((bus_adc as u16) << 7)
            | ((shunt_adc as u16) << 3)
            | (mode as u16);

        self.write_word(registers::CONFIGURATION, config)?;
        self.config = config;
        Ok(())
    }

    /// 读取当前配置值
    pub fn read_config(&mut self) -> Result<u16, I2cError> {
        let raw = self.read_word(registers::CONFIGURATION)?;
        self.config = raw;
        Ok(raw)
    }

    // ========================================================================
    // 校准
    // ========================================================================

    /// Program and retain a validated calibration.
    pub fn calibrate(&mut self, calibration: Calibration) -> Result<(), I2cError> {
        self.write_word(registers::CALIBRATION, calibration.register())?;
        self.calibration = Some(calibration);
        Ok(())
    }

    /// Return the retained calibration, if hardware current/power scaling is enabled.
    pub const fn calibration(&self) -> Option<Calibration> {
        self.calibration
    }

    /// Directly write a raw calibration value and clear retained scaling.
    pub fn write_calibration_raw(&mut self, cal: u16) -> Result<(), I2cError> {
        self.write_word(registers::CALIBRATION, cal & 0xFFFE)?;
        self.calibration = None;
        Ok(())
    }

    // ========================================================================
    // 分流电压
    // ========================================================================

    /// 读取原始分流电压寄存器值
    pub fn read_shunt_voltage_raw(&mut self) -> Result<i16, I2cError> {
        let raw = self.read_word(registers::SHUNT_VOLTAGE)?;
        Ok(raw as i16)
    }

    /// 读取分流电压 (µV)
    ///
    /// 有符号值，LSB = 10µV。
    /// 根据 PGA 设置不同，量程不同：
    /// - PGA=/1: ±40mV (±40000µV)
    /// - PGA=/2: ±80mV (±80000µV)
    /// - PGA=/4: ±160mV (±160000µV)
    /// - PGA=/8: ±320mV (±320000µV)
    pub fn read_shunt_voltage_uv(&mut self) -> Result<f32, I2cError> {
        let raw = self.read_word(registers::SHUNT_VOLTAGE)?;
        // 有符号值, 10µV/LSB
        Ok(raw as i16 as f32 * 10.0)
    }

    /// 读取分流电压 (mV)
    pub fn read_shunt_voltage_mv(&mut self) -> Result<f32, I2cError> {
        Ok(self.read_shunt_voltage_uv()? / 1000.0)
    }

    // ========================================================================
    // 总线电压
    // ========================================================================

    /// 读取原始总线电压寄存器值
    pub fn read_bus_voltage_raw(&mut self) -> Result<u16, I2cError> {
        self.read_word(registers::BUS_VOLTAGE)
    }

    /// 读取总线电压 (V)
    ///
    /// 12-bit 数据，LSB = 4mV。
    /// - BRNG=0: 0-16V
    /// - BRNG=1: 0-32V
    ///
    /// 寄存器格式: [BD12..BD0][—][CNVR][OVF]
    /// 数据在 bits 15:3，需要右移 3 位。
    pub fn read_bus_voltage(&mut self) -> Result<f32, I2cError> {
        let raw = self.read_word(registers::BUS_VOLTAGE)?;
        // 12-bit 数据, 4mV/LSB, bit1 是 CNVR, bit0 是 OVF
        let voltage_raw = (raw >> 3) & 0x1FFF;
        Ok(voltage_raw as f32 * 0.004)
    }

    /// 读取总线电压状态标志 (CNVR, OVF)
    pub fn read_bus_voltage_status(&mut self) -> Result<BusVoltageStatus, I2cError> {
        let raw = self.read_word(registers::BUS_VOLTAGE)?;
        Ok(BusVoltageStatus {
            conversion_ready: (raw & (1 << 1)) != 0,
            overflow: (raw & 1) != 0,
        })
    }

    // ========================================================================
    // 电流（硬件计算，需先校准）
    // ========================================================================

    /// 读取原始电流寄存器值
    ///
    /// 需要先调用 `calibrate()` 写入校准值，否则返回 0。
    pub fn read_current_raw(&mut self) -> Result<i16, I2cError> {
        let raw = self.read_word(registers::CURRENT)?;
        Ok(raw as i16)
    }

    /// Read calibrated hardware current in microamps.
    ///
    /// Returns `Ok(None)` until [`calibrate`](Self::calibrate) installs a
    /// validated calibration value in both the device and this driver.
    pub fn read_current_microamps(&mut self) -> Result<Option<i64>, I2cError> {
        let Some(calibration) = self.calibration else {
            return Ok(None);
        };
        let raw = i64::from(self.read_current_raw()?);
        Ok(Some(raw * i64::from(calibration.current_lsb_microamps())))
    }

    /// 读取电流 (mA)，使用硬件 Current 寄存器
    ///
    /// 需要先调用 `calibrate()` 写入校准值。
    /// `current_lsb` 是校准时设定的电流 LSB (A/bit)。
    /// 电流 = raw × current_lsb × 1000 (mA)
    pub fn read_current_hardware(&mut self, current_lsb: f32) -> Result<f32, I2cError> {
        let raw = self.read_word(registers::CURRENT)?;
        Ok(raw as i16 as f32 * current_lsb * 1000.0)
    }

    /// 读取电流 (mA)，使用分流电压和分流电阻手动计算
    ///
    /// 不依赖校准值，始终有效。
    /// I (mA) = Vshunt (µV) / Rshunt (mΩ)
    pub fn read_current_ma(&mut self) -> Result<f32, I2cError> {
        let shunt_uv = self.read_shunt_voltage_uv()?;
        // I (mA) = Vshunt (µV) / Rshunt (mΩ)
        Ok(shunt_uv / self.shunt_resistor_mohm)
    }

    // ========================================================================
    // 功率（硬件计算，需先校准）
    // ========================================================================

    /// 读取原始功率寄存器值
    ///
    /// 需要先调用 `calibrate()` 写入校准值，否则返回 0。
    pub fn read_power_raw(&mut self) -> Result<u16, I2cError> {
        self.read_word(registers::POWER)
    }

    /// Read calibrated hardware power in microwatts.
    ///
    /// INA219 power-register LSB is 20 times the configured current-register
    /// LSB. Returns `Ok(None)` until [`calibrate`](Self::calibrate) succeeds.
    pub fn read_power_microwatts(&mut self) -> Result<Option<u64>, I2cError> {
        let Some(calibration) = self.calibration else {
            return Ok(None);
        };
        let raw = u64::from(self.read_power_raw()?);
        Ok(Some(
            raw * u64::from(calibration.current_lsb_microamps()) * 20,
        ))
    }

    /// 读取功率 (mW)，使用硬件 Power 寄存器
    ///
    /// 需要先调用 `calibrate()` 写入校准值。
    /// `power_lsb` 是校准时设定的功率 LSB (W/bit)。
    /// 功率 = raw × power_lsb × 1000 (mW)
    pub fn read_power_hardware(&mut self, power_lsb: f32) -> Result<f32, I2cError> {
        let raw = self.read_word(registers::POWER)?;
        Ok(raw as f32 * power_lsb * 1000.0)
    }

    /// 读取功率 (mW)，使用总线电压和分流电压手动计算
    ///
    /// 不依赖校准值，始终有效。
    /// P (mW) = Vbus (V) × I (mA)
    pub fn read_power_ma(&mut self) -> Result<f32, I2cError> {
        let bus_v = self.read_bus_voltage()?;
        let current_ma = self.read_current_ma()?;
        Ok(bus_v * current_ma)
    }

    // ========================================================================
    // 组合读取
    // ========================================================================

    /// 读取所有电源数据（手动计算，不依赖校准）
    pub fn read_all(&mut self) -> Result<PowerMeasurement, I2cError> {
        let bus_voltage = self.read_bus_voltage()?;
        let shunt_voltage_uv = self.read_shunt_voltage_uv()?;
        let current_ma = shunt_voltage_uv / self.shunt_resistor_mohm;
        let power_mw = bus_voltage * current_ma;

        Ok(PowerMeasurement {
            bus_voltage,
            shunt_voltage_uv,
            current_ma,
            power_mw,
        })
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum MockError {
        Injected,
    }

    impl embedded_hal::i2c::Error for MockError {
        fn kind(&self) -> embedded_hal::i2c::ErrorKind {
            embedded_hal::i2c::ErrorKind::Other
        }
    }

    #[derive(Debug)]
    struct ExpectedTransaction {
        address: u8,
        write: std::vec::Vec<u8>,
        read: std::vec::Vec<u8>,
        result: Result<(), MockError>,
    }

    impl ExpectedTransaction {
        fn write(bytes: &[u8]) -> Self {
            Self {
                address: INA219_ADDR,
                write: bytes.to_vec(),
                read: std::vec::Vec::new(),
                result: Ok(()),
            }
        }

        fn write_read(write: &[u8], read: &[u8]) -> Self {
            Self {
                address: INA219_ADDR,
                write: write.to_vec(),
                read: read.to_vec(),
                result: Ok(()),
            }
        }

        fn failing_write_read(write: &[u8], read_len: usize) -> Self {
            Self {
                address: INA219_ADDR,
                write: write.to_vec(),
                read: std::vec![0; read_len],
                result: Err(MockError::Injected),
            }
        }
    }

    #[derive(Debug)]
    struct MockI2c {
        expected: std::collections::VecDeque<ExpectedTransaction>,
    }

    impl MockI2c {
        fn new(expected: impl IntoIterator<Item = ExpectedTransaction>) -> Self {
            Self {
                expected: expected.into_iter().collect(),
            }
        }

        fn assert_complete(&self) {
            assert!(
                self.expected.is_empty(),
                "unconsumed I2C expectations: {:?}",
                self.expected
            );
        }
    }

    impl embedded_hal::i2c::ErrorType for MockI2c {
        type Error = MockError;
    }

    impl embedded_hal::i2c::I2c for MockI2c {
        fn transaction(
            &mut self,
            address: u8,
            operations: &mut [embedded_hal::i2c::Operation<'_>],
        ) -> Result<(), Self::Error> {
            let expected = self
                .expected
                .pop_front()
                .expect("unexpected I2C transaction");
            assert_eq!(address, expected.address);
            let mut write = None;
            let mut read_count = 0;

            for operation in operations {
                match operation {
                    embedded_hal::i2c::Operation::Write(bytes) => {
                        assert!(write.replace(bytes.to_vec()).is_none(), "multiple writes")
                    }
                    embedded_hal::i2c::Operation::Read(bytes) => {
                        read_count += 1;
                        assert_eq!(bytes.len(), expected.read.len());
                        bytes.copy_from_slice(&expected.read);
                    }
                }
            }

            assert_eq!(write.unwrap_or_default(), expected.write);
            assert_eq!(read_count, usize::from(!expected.read.is_empty()));
            expected.result
        }
    }

    #[test]
    fn calibrated_current_and_power_use_integer_register_scales() {
        let calibration = Calibration::new(2_000, 15_000_000).unwrap();
        let calibration_bytes = calibration.register().to_be_bytes();
        let i2c = MockI2c::new([
            ExpectedTransaction::write(&[
                registers::CALIBRATION,
                calibration_bytes[0],
                calibration_bytes[1],
            ]),
            ExpectedTransaction::write_read(&[registers::CURRENT], &[0xFF, 0xFE]),
            ExpectedTransaction::write_read(&[registers::POWER], &[0x00, 0x03]),
        ]);
        let mut monitor = Ina219::new(i2c);

        monitor.calibrate(calibration).unwrap();
        assert_eq!(monitor.read_current_microamps().unwrap(), Some(-916));
        assert_eq!(monitor.read_power_microwatts().unwrap(), Some(27_480));
        monitor.destroy().assert_complete();
    }

    #[test]
    fn uncalibrated_scaled_reads_do_not_access_the_bus() {
        let i2c = MockI2c::new([]);
        let mut monitor = Ina219::new(i2c);

        assert_eq!(monitor.read_current_microamps().unwrap(), None);
        assert_eq!(monitor.read_power_microwatts().unwrap(), None);
        monitor.destroy().assert_complete();
    }

    #[test]
    fn calibrated_current_read_propagates_i2c_errors() {
        let calibration = Calibration::new(2_000, 15_000_000).unwrap();
        let calibration_bytes = calibration.register().to_be_bytes();
        let i2c = MockI2c::new([
            ExpectedTransaction::write(&[
                registers::CALIBRATION,
                calibration_bytes[0],
                calibration_bytes[1],
            ]),
            ExpectedTransaction::failing_write_read(&[registers::CURRENT], 2),
        ]);
        let mut monitor = Ina219::new(i2c);

        monitor.calibrate(calibration).unwrap();
        assert_eq!(monitor.read_current_microamps(), Err(MockError::Injected));
        monitor.destroy().assert_complete();
    }

    #[test]
    fn calibration_rejects_zero_inputs_and_retains_integer_scale() {
        assert_eq!(
            Calibration::new(0, 1),
            Err(CalibrationError::ZeroShuntResistance)
        );
        assert_eq!(
            Calibration::new(2_000, 0),
            Err(CalibrationError::ZeroExpectedCurrent)
        );
        let calibration = Calibration::new(2_000, 15_000_000).unwrap();
        assert_ne!(calibration.register(), 0);
        assert_eq!(calibration.register() & 1, 0);
        assert_eq!(calibration.current_lsb_microamps(), 458);
    }
}
