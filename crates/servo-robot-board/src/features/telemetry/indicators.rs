//! 全局错误统计
//!
//! 使用原子计数器记录各类错误，可在系统信息上报中读取。

use core::sync::atomic::{AtomicUsize, Ordering};

/// 全局错误统计
pub static ERROR_STATS: ErrorStats = ErrorStats::new();

pub struct ErrorStats {
    pub i2c_errors: AtomicUsize,
    pub spi_errors: AtomicUsize,
    pub charge_errors: AtomicUsize,
}

impl ErrorStats {
    pub const fn new() -> Self {
        Self {
            i2c_errors: AtomicUsize::new(0),
            spi_errors: AtomicUsize::new(0),
            charge_errors: AtomicUsize::new(0),
        }
    }

    pub fn inc_i2c(&self) {
        self.i2c_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_spi(&self) {
        self.spi_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_charge(&self) {
        self.charge_errors.fetch_add(1, Ordering::Relaxed);
    }
}

/// Choose a status color for the battery-temperature indicator.
pub fn battery_temp_color(temp_c: f32) -> crate::platform::ws2812::Color {
    use crate::platform::ws2812::Color;

    if temp_c < 5.0 {
        Color::BLUE
    } else if temp_c < 25.0 {
        Color::GREEN
    } else if temp_c < 40.0 {
        Color::YELLOW
    } else if temp_c < 55.0 {
        Color::ORANGE
    } else {
        Color::RED
    }
}

/// Choose a status color for the battery state-of-charge indicator.
pub fn battery_soc_color(soc: u8) -> crate::platform::ws2812::Color {
    use crate::platform::ws2812::Color;

    if soc < 2 {
        Color::BLACK
    } else if soc < 10 {
        Color::RED
    } else if soc < 30 {
        Color::ORANGE
    } else if soc < 80 {
        Color::BLUE
    } else {
        Color::GREEN
    }
}
