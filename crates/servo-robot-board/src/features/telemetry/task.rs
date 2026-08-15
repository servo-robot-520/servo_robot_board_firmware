//! 遥测任务辅助函数
//!
//! 从 `sys_info_task` 和 `event_flush_task` 中提取的可复用逻辑。

use servo_robot_protocol::event::ErrorFlags;
use servo_robot_protocol::system::SystemInfo;

use crate::features::power::task as power_task;
use crate::platform;

/// 栈水位检测: 扫描描漆区域，返回最小剩余栈空间 (KB)
///
/// Delegates to platform::startup::check_stack_watermark().
pub fn check_stack_watermark() -> u16 {
    crate::platform::startup::check_stack_watermark()
}

/// 从错误计数器计算 ErrorFlags
pub fn compute_error_flags(
    i2c_errors: u16,
    spi_errors: u16,
    uart_errors: u16,
    usb_errors: u16,
) -> ErrorFlags {
    let mut err = ErrorFlags::empty();
    if i2c_errors > 0 {
        err |= ErrorFlags::I2C1_ERROR;
    }
    if spi_errors > 0 {
        err |= ErrorFlags::SPI1_ERROR;
    }
    if uart_errors > 0 {
        err |= ErrorFlags::UART1_ERROR;
    }
    if usb_errors > 0 {
        err |= ErrorFlags::USB_ERROR;
    }
    err
}

/// 填充 SystemInfo 温度字段 (i16, 实际值 = 原始值 / 10)
pub fn fill_temperatures(
    info: &mut SystemInfo,
    temp_servo: f32,
    temp_5v: f32,
    mcu_temp: f32,
    temp_charge: f32,
    temp_battery: i16,
) {
    info.temp_servo_power = (temp_servo * 10.0) as i16;
    info.temp_5v_power = (temp_5v * 10.0) as i16;
    info.temp_mcu = (mcu_temp * 10.0) as i16;
    info.temp_charge = (temp_charge * 10.0) as i16;
    info.temp_battery = temp_battery;
}

/// Read all NTC + MCU temperatures from the ADC DMA buffer.
///
/// Returns `(temp_charge, temp_servo, temp_5v, mcu_temp)` in degrees Celsius.
pub fn read_all_temperatures() -> (f32, f32, f32, f32) {
    let samples = platform::adc::adc_snapshot();
    power_task::read_thermal_temperatures(&samples)
}

/// Inputs collected by the RTIC telemetry wrapper for one system-info tick.
///
/// Keeping the snapshot together makes the feature boundary explicit and
/// avoids a fragile positional argument list at the RTIC boundary.
#[derive(Clone, Copy, Debug)]
pub struct SystemInfoInputs {
    pub device_id: u16,
    pub uid: u32,
    pub imu_id: u8,
    pub uptime_s: u32,
    pub free_heap_kb: u16,
    pub i2c_errors: u16,
    pub spi_errors: u16,
    pub uart_errors: u16,
    pub usb_errors: u16,
    pub frames_sent: u32,
    pub pd_voltage: u16,
    pub pd_current: u16,
    pub temp_servo: f32,
    pub temp_5v: f32,
    pub mcu_temp: f32,
    pub temp_charge: f32,
    pub temp_battery: i16,
}

/// Assemble SystemInfo from one collected telemetry snapshot.
pub fn assemble_system_info(input: SystemInfoInputs) -> SystemInfo {
    let mut info = SystemInfo {
        device_id: input.device_id,
        uid: input.uid,
        imu_id: input.imu_id,
        uptime_s: input.uptime_s,
        cpu_usage_percent: 0,
        free_heap_kb: input.free_heap_kb,
        stack_watermark_min_kb: check_stack_watermark(),
        i2c_error_count: input.i2c_errors,
        spi_error_count: input.spi_errors,
        uart_error_count: input.uart_errors,
        usb_error_count: input.usb_errors,
        frames_sent_total: input.frames_sent,
        pd_request_voltage_mv: input.pd_voltage,
        pd_request_current_ma: input.pd_current,
        ..SystemInfo::default()
    };
    fill_temperatures(
        &mut info,
        input.temp_servo,
        input.temp_5v,
        input.mcu_temp,
        input.temp_charge,
        input.temp_battery,
    );
    info
}

/// Protocol event fields computed during one system telemetry tick.
#[derive(Clone, Copy, Debug)]
pub struct SystemEventUpdate {
    pub protection_flags: servo_robot_protocol::event::ProtectionFlags,
    pub error_flags: ErrorFlags,
    pub fan_enabled: bool,
    pub charger_connected: bool,
}

/// Convert local protection state and counters to the board event projection.
pub fn build_system_event_update(
    protection: crate::features::power::protection::ProtectionFlags,
    fan_enabled: bool,
    charger_connected: bool,
    i2c_errors: u16,
    spi_errors: u16,
    uart_errors: u16,
    usb_errors: u16,
) -> SystemEventUpdate {
    SystemEventUpdate {
        protection_flags: servo_robot_protocol::event::ProtectionFlags::from_bits(
            protection.to_u16(),
        )
        .unwrap_or(servo_robot_protocol::event::ProtectionFlags::empty()),
        error_flags: compute_error_flags(i2c_errors, spi_errors, uart_errors, usb_errors),
        fan_enabled,
        charger_connected,
    }
}

/// Choose the LED color used to display charge-circuit temperature.
pub fn charge_temperature_indicator(temp_charge: f32) -> crate::platform::ws2812::Color {
    super::indicators::battery_temp_color(temp_charge)
}
