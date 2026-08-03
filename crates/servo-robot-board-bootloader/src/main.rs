#![no_std]
#![no_main]

//! Servo Robot Bootloader
//!
//! Bootloader 占据 Flash Sector 0 (16KB), 地址 0x0800_0000
//!
//! 启动流程:
//! 1. 检查 OTA 标志 (User Data 扇区)
//! 2. 如果 OTA_PENDING: 解析 OtaImageHeader (magic="OTAI"),
//!    从 OTA Temp (跳过 16 字节头) 拷贝固件到 App → 清除标志
//! 3. 跳转到 App (0x0800_4000)

use cortex_m_rt::entry;
use panic_halt as _;

use stm32f4::stm32f411;

#[derive(Debug, PartialEq)]
enum FlashError {
    Timeout,
    ProgramError,
}

/// App 固件起始地址
const APP_ADDR: u32 = 0x0800_4000;
/// OTA 标志地址 (User Data 扇区)
const OTA_FLAG_ADDR: u32 = 0x0806_0000;
/// OTA Temp 起始地址
const OTA_TEMP_ADDR: u32 = 0x0804_0000;
/// OTA 镜像头大小 (16 bytes)
const OTA_IMAGE_HEADER_SIZE: u32 = 16;
/// OTA 镜像头魔数 "OTAI"
const OTA_IMAGE_MAGIC: u32 = 0x4F54_4149;
/// App 最大大小 (240KB, Sectors 1-5)
const APP_MAX_SIZE: u32 = 240 * 1024;
/// OTA image format version
const OTA_IMAGE_FORMAT_VERSION: u8 = 1;
/// OTA target MCU identifier for STM32F411
const OTA_TARGET_MCU_F411: u8 = 0x41;
/// OTA 标志值
const OTA_PENDING: u32 = 0x5441_4F54;
const OTA_DONE: u32 = 0x444F_4E45;

fn read_u32(addr: u32) -> u32 {
    unsafe { core::ptr::read_volatile(addr as *const u32) }
}

/// Compute CRC32 (IEEE 802.3 / zlib polynomial) over a memory region.
fn crc32_compute(base: u32, size: u32) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    let mut offset = 0u32;
    while offset < size {
        let word = read_u32(base + offset);
        let bytes = word.to_le_bytes();
        let remaining = size - offset;
        let bytes_to_process = if remaining >= 4 {
            4
        } else {
            remaining as usize
        };
        for i in 0..bytes_to_process {
            crc ^= bytes[i] as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB8_8320;
                } else {
                    crc >>= 1;
                }
            }
        }
        offset += 4;
    }
    !crc
}

fn wait_flash(flash: &stm32f411::FLASH) -> Result<(), FlashError> {
    let mut timeout = 1_000_000u32;
    while flash.sr().read().bsy().bit_is_set() {
        timeout -= 1;
        if timeout == 0 {
            return Err(FlashError::Timeout);
        }
    }
    // Check error flags
    let sr = flash.sr().read();
    if sr.pgperr().bit_is_set() || sr.pgaerr().bit_is_set() || sr.wrperr().bit_is_set() {
        // Clear error flags
        flash
            .sr()
            .write(|w| unsafe { w.bits(1 << 6 | 1 << 5 | 1 << 4) });
        return Err(FlashError::ProgramError);
    }
    Ok(())
}

fn unlock_flash(flash: &stm32f411::FLASH) {
    flash.keyr().write(|w| unsafe { w.bits(0x4567_0123) });
    flash.keyr().write(|w| unsafe { w.bits(0xCDEF_89AB) });
}

fn lock_flash(flash: &stm32f411::FLASH) {
    flash.cr().modify(|_, w| w.lock().set_bit());
}

fn erase_sector(flash: &stm32f411::FLASH, sector: u8) -> Result<(), FlashError> {
    wait_flash(flash)?;
    flash.cr().modify(|_, w| {
        w.ser().set_bit();
        unsafe { w.snb().bits(sector) }
    });
    flash.cr().modify(|_, w| w.strt().set_bit());
    let result = wait_flash(flash);
    flash.cr().modify(|_, w| w.ser().clear_bit());
    result
}

fn write_word(flash: &stm32f411::FLASH, addr: u32, word: u32) -> Result<(), FlashError> {
    wait_flash(flash)?;
    flash.cr().modify(|_, w| {
        w.pg().set_bit();
        unsafe { w.psize().bits(0b10) }
    });
    unsafe {
        core::ptr::write_volatile(addr as *mut u32, word);
    }
    let result = wait_flash(flash);
    flash.cr().modify(|_, w| w.pg().clear_bit());
    result
}

fn copy_ota_to_app(flash: &stm32f411::FLASH, size: u32) -> Result<(), FlashError> {
    // 擦除 App 区域: Sectors 1-5 (240KB)
    erase_sector(flash, 1)?;
    erase_sector(flash, 2)?;
    erase_sector(flash, 3)?;
    erase_sector(flash, 4)?;
    erase_sector(flash, 5)?;

    // 跳过 OtaImageHeader, 从固件数据开始拷贝
    let src_base = OTA_TEMP_ADDR + OTA_IMAGE_HEADER_SIZE;
    let mut offset = 0u32;

    // 按 4 字节对齐拷贝（Flash 最小写入单位）
    // 对于末尾不足 4 字节的部分，读取完整 word 但只写入有效字节
    let aligned_size = size & !3; // 向下对齐到 4 字节
    let tail_bytes = size - aligned_size; // 末尾剩余字节 (0-3)

    // 拷贝完整的 4 字节 word
    while offset < aligned_size {
        let word = read_u32(src_base + offset);
        write_word(flash, APP_ADDR + offset, word)?;
        offset += 4;
    }

    // 处理末尾不足 4 字节（如果有的话）
    // 保留低 tail_bytes 字节，高位设为 0xFF（擦除状态）
    if tail_bytes > 0 {
        let word = read_u32(src_base + aligned_size);
        // 构造掩码：低 tail_bytes 字节保留，高位设为 0xFF
        // 例如 tail_bytes=1: padding_mask = 0xFFFFFF00, 保留低 1 字节
        // 例如 tail_bytes=2: padding_mask = 0xFFFF0000, 保留低 2 字节
        let padding_mask = !((1u32 << (tail_bytes * 8)) - 1);
        let padded_word = word | padding_mask;
        write_word(flash, APP_ADDR + aligned_size, padded_word)?;
    }

    Ok(())
}

fn clear_ota_flag(flash: &stm32f411::FLASH) -> Result<(), FlashError> {
    // Preserve sector contents by reading before erase
    let mut sector_buf = [0u8; 256];
    for i in 0..64u32 {
        let word = read_u32(OTA_FLAG_ADDR + i * 4);
        let bytes = word.to_le_bytes();
        let offset = (i * 4) as usize;
        sector_buf[offset..offset + 4].copy_from_slice(&bytes);
    }

    // Modify OTA flag in buffer
    sector_buf[0..4].copy_from_slice(&OTA_DONE.to_le_bytes());

    // Erase sector
    erase_sector(flash, 7)?;

    // Write back preserved contents
    for i in 0..64u32 {
        let offset = (i * 4) as usize;
        let word = u32::from_le_bytes([
            sector_buf[offset],
            sector_buf[offset + 1],
            sector_buf[offset + 2],
            sector_buf[offset + 3],
        ]);
        write_word(flash, OTA_FLAG_ADDR + i * 4, word)?;
    }

    Ok(())
}

#[entry]
fn main() -> ! {
    let dp = stm32f411::Peripherals::take().unwrap();
    let flash = &dp.FLASH;

    let ota_flag = read_u32(OTA_FLAG_ADDR);

    if ota_flag == OTA_PENDING {
        // 解析 OtaImageHeader
        let header_word0 = read_u32(OTA_TEMP_ADDR); // magic
        let header_word1 = read_u32(OTA_TEMP_ADDR + 4); // format_version(8) | target_mcu(8) | _reserved(16)
        let image_size = read_u32(OTA_TEMP_ADDR + 8);
        let image_crc32 = read_u32(OTA_TEMP_ADDR + 12);

        let format_version = (header_word1 & 0xFF) as u8;
        let target_mcu = ((header_word1 >> 8) & 0xFF) as u8;

        if header_word0 == OTA_IMAGE_MAGIC
            && format_version == OTA_IMAGE_FORMAT_VERSION
            && target_mcu == OTA_TARGET_MCU_F411
            && image_size > 0
            && image_size <= APP_MAX_SIZE
        {
            // 验证源镜像 CRC（在擦除 App 之前）
            let src_data_addr = OTA_TEMP_ADDR + OTA_IMAGE_HEADER_SIZE;
            let source_crc = crc32_compute(src_data_addr, image_size);
            if source_crc != image_crc32 {
                // 源镜像 CRC 不匹配：不擦除 App；清除 Pending 标志后启动现有固件。
                unlock_flash(flash);
                let _ = clear_ota_flag(flash);
                lock_flash(flash);
            } else {
                unlock_flash(flash);
                if copy_ota_to_app(flash, image_size).is_ok() {
                    // Verify CRC32 of the copied image
                    let computed_crc = crc32_compute(APP_ADDR, image_size);
                    if computed_crc == image_crc32 {
                        if clear_ota_flag(flash).is_err() {
                            // 无法清除 OTA 标志，设备将重试 OTA。
                            // App 已写入有效固件，下次启动可恢复。
                        }
                    } else {
                        // CRC mismatch — invalidate: erase app so stale image cannot boot
                        let _ = erase_sector(flash, 1);
                        let _ = erase_sector(flash, 2);
                        let _ = erase_sector(flash, 3);
                        let _ = erase_sector(flash, 4);
                        let _ = erase_sector(flash, 5);
                    }
                }
                lock_flash(flash);
            }
        }
    }

    // 跳转到 App
    unsafe {
        cortex_m::interrupt::disable();
        let sp = read_u32(APP_ADDR);
        let reset_vector = read_u32(APP_ADDR + 4);

        // 验证 MSP 在 SRAM 范围
        if sp < 0x2000_0000 || sp > 0x2002_0000 {
            loop {
                cortex_m::asm::wfi();
            }
        }

        // 验证 reset vector 在 App Flash 范围且是 Thumb 地址
        if reset_vector < APP_ADDR
            || reset_vector > (APP_ADDR + APP_MAX_SIZE)
            || (reset_vector & 1) == 0
        {
            loop {
                cortex_m::asm::wfi();
            }
        }

        // 设置 MSP 并跳转
        cortex_m::asm::bootstrap(sp as *const u32, reset_vector as *const u32);
    }
}
