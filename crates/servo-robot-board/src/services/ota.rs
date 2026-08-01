//! OTA 固件更新服务
//!
//! 处理固件数据块的接收、验证和写入 OTA Temp Flash。

use crate::hal;
use stm32f4xx_hal::pac::FLASH;

/// OTA 写入状态
pub struct OtaWriter {
    /// 当前写入偏移
    offset: u32,
    /// 是否已擦除 OTA Temp 区域
    erased: bool,
}

/// OTA 写入结果
pub enum OtaWriteResult {
    /// 写入成功，返回新偏移
    Success { new_offset: u32 },
    /// 固件过大
    TooLarge,
    /// 擦除失败
    EraseFailed,
    /// 写入失败
    WriteFailed,
}

impl OtaWriter {
    /// 创建新的 OTA 写入器
    pub fn new() -> Self {
        Self {
            offset: 0,
            erased: false,
        }
    }

    /// 重置写入器（新的 OTA 会话）
    pub fn reset(&mut self) {
        self.offset = 0;
        self.erased = false;
    }

    /// 获取当前偏移
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// 写入固件数据块
    ///
    /// 首次写入时自动擦除 OTA Temp 区域。
    /// 数据必须 4 字节对齐（调用者负责对齐）。
    pub fn write_block(&mut self, flash: &FLASH, data: &[u8]) -> OtaWriteResult {
        // 边界检查
        if self.offset + data.len() as u32 > hal::flash::OTA_TEMP_MAX_SIZE {
            defmt::error!("Firmware too large, exceeds OTA temp region");
            return OtaWriteResult::TooLarge;
        }

        // 首次写入时擦除 OTA Temp 区域
        if !self.erased {
            defmt::info!("Erasing OTA temp region...");
            if hal::flash::erase_ota_temp(flash).is_err() {
                defmt::error!("Failed to erase OTA temp");
                return OtaWriteResult::EraseFailed;
            }
            self.erased = true;
        }

        // 写入 Flash（需要 4 字节对齐）
        let write_addr = hal::flash::OTA_TEMP_ADDR + self.offset;
        let aligned_len = data.len() & !3;

        if aligned_len > 0 {
            if hal::flash::program_flash(flash, write_addr, &data[..aligned_len]).is_ok() {
                self.offset += aligned_len as u32;
                defmt::info!(
                    "Firmware: {} bytes written at offset {}",
                    aligned_len,
                    self.offset - aligned_len as u32
                );
                OtaWriteResult::Success {
                    new_offset: self.offset,
                }
            } else {
                defmt::error!("Flash write failed at offset {}", self.offset);
                OtaWriteResult::WriteFailed
            }
        } else {
            // 数据不足 4 字节，忽略
            OtaWriteResult::Success {
                new_offset: self.offset,
            }
        }
    }
}

/// 启动 OTA 更新（写入 OTA 标志并复位）
pub fn start_ota(flash: &FLASH) -> bool {
    defmt::info!("OTA requested, writing flag and resetting");
    if hal::flash::write_ota_flag(flash, hal::flash::OtaFlag::Pending).is_ok() {
        true
    } else {
        defmt::error!("Failed to write OTA flag");
        false
    }
}
