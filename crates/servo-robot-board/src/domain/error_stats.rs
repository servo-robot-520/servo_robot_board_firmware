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
