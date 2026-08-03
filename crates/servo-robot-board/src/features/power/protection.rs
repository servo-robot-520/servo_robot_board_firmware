//! 过温/过流保护逻辑
//!
//! - 舵机电源过温 30s → 关断 PWR_SERVO_EN
//! - 5V 电源过温 30s → 关断 PWR_5V_EN
//! - 舵机过流 30s → 关断 PWR_SERVO_EN

/// 保护标志位
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct ProtectionFlags {
    pub servo_overcurrent: bool,
    pub servo_thermal: bool,
    pub dcdc_5v_thermal: bool,
    pub charge_derating: bool,
    pub charge_thermal: bool,
    pub battery_low: bool,
}

impl ProtectionFlags {
    pub const fn new() -> Self {
        Self {
            servo_overcurrent: false,
            servo_thermal: false,
            dcdc_5v_thermal: false,
            charge_derating: false,
            charge_thermal: false,
            battery_low: false,
        }
    }

    /// 转换为协议中的 u16 位域
    pub fn to_u16(&self) -> u16 {
        let mut flags = 0u16;
        if self.servo_overcurrent {
            flags |= 1 << 0;
        }
        if self.servo_thermal {
            flags |= 1 << 1;
        }
        if self.dcdc_5v_thermal {
            flags |= 1 << 2;
        }
        if self.charge_derating {
            flags |= 1 << 3;
        }
        if self.charge_thermal {
            flags |= 1 << 4;
        }
        if self.battery_low {
            flags |= 1 << 5;
        }
        flags
    }

    pub fn any(&self) -> bool {
        self.servo_overcurrent
            || self.servo_thermal
            || self.dcdc_5v_thermal
            || self.charge_derating
            || self.charge_thermal
            || self.battery_low
    }
}

/// 保护管理器
///
/// thermal_task (5Hz) 调用 `check_thermal()`
/// power_task (20Hz) 调用 `check_current()`
pub struct ProtectionManager {
    /// 舵机电源过温计数 (5Hz × 30s = 150)
    servo_thermal_count: u16,
    /// 5V 电源过温计数
    dcdc_5v_thermal_count: u16,
    /// 舵机过流计数 (20Hz × 30s = 600)
    servo_overcurrent_count: u16,
    /// 当前保护标志
    flags: ProtectionFlags,
    /// 舵机电源是否已被保护关断
    servo_power_cut: bool,
    /// 5V 电源是否已被保护关断
    power_5v_cut: bool,
    /// 配置: 舵机电源温度限制 (°C)
    servo_temp_limit: f32,
    /// 配置: 5V 电源温度限制 (°C)
    temp_5v_limit: f32,
    /// 配置: 舵机电流限制 (A)
    servo_current_limit: f32,
}

impl ProtectionManager {
    pub fn new() -> Self {
        Self {
            servo_thermal_count: 0,
            dcdc_5v_thermal_count: 0,
            servo_overcurrent_count: 0,
            flags: ProtectionFlags::new(),
            servo_power_cut: false,
            power_5v_cut: false,
            servo_temp_limit: 80.0,
            temp_5v_limit: 70.0,
            servo_current_limit: 5.0,
        }
    }

    pub fn flags(&self) -> ProtectionFlags {
        self.flags
    }

    pub fn is_servo_power_cut(&self) -> bool {
        self.servo_power_cut
    }

    pub fn is_5v_power_cut(&self) -> bool {
        self.power_5v_cut
    }

    pub fn set_servo_temp_limit(&mut self, limit: f32) {
        self.servo_temp_limit = limit;
    }

    pub fn set_5v_temp_limit(&mut self, limit: f32) {
        self.temp_5v_limit = limit;
    }

    pub fn set_servo_current_limit(&mut self, limit: f32) {
        self.servo_current_limit = limit;
    }

    /// 温度保护检查 (5Hz 调用, 30s = 150 次)
    ///
    /// 返回 `(servo_should_cut, 5v_should_cut)`
    pub fn check_thermal(&mut self, temp_servo: f32, temp_5v: f32) -> (bool, bool) {
        // 舵机电源过温
        if temp_servo > self.servo_temp_limit {
            self.servo_thermal_count += 1;
            if self.servo_thermal_count >= 150 {
                self.flags.servo_thermal = true;
                self.servo_power_cut = true;
            }
        } else {
            self.servo_thermal_count = 0;
            self.flags.servo_thermal = false;
        }

        // 5V 电源过温
        if temp_5v > self.temp_5v_limit {
            self.dcdc_5v_thermal_count += 1;
            if self.dcdc_5v_thermal_count >= 150 {
                self.flags.dcdc_5v_thermal = true;
                self.power_5v_cut = true;
            }
        } else {
            self.dcdc_5v_thermal_count = 0;
            self.flags.dcdc_5v_thermal = false;
        }

        (self.servo_power_cut, self.power_5v_cut)
    }

    /// 电流保护检查 (20Hz 调用, 30s = 600 次)
    ///
    /// 返回 `servo_should_cut`
    pub fn check_current(&mut self, servo_current_a: f32) -> bool {
        if servo_current_a > self.servo_current_limit {
            self.servo_overcurrent_count += 1;
            if self.servo_overcurrent_count >= 600 {
                self.flags.servo_overcurrent = true;
                self.servo_power_cut = true;
            }
        } else {
            self.servo_overcurrent_count = 0;
            self.flags.servo_overcurrent = false;
        }

        self.servo_power_cut
    }

    /// 上位机重新使能舵机电源
    pub fn reset_servo_power(&mut self) {
        self.servo_power_cut = false;
        self.servo_thermal_count = 0;
        self.servo_overcurrent_count = 0;
        self.flags.servo_thermal = false;
        self.flags.servo_overcurrent = false;
    }

    /// 上位机重新使能 5V 电源
    pub fn reset_5v_power(&mut self) {
        self.power_5v_cut = false;
        self.dcdc_5v_thermal_count = 0;
        self.flags.dcdc_5v_thermal = false;
    }
}
