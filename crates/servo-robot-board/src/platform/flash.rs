//! STM32F411 内部 Flash 擦除/编程驱动 + 配置持久化存储
//!
//! Flash 布局:
//!   0x0800_0000 - 0x0800_3FFF: Bootloader (16KB, Sector 0)
//!   0x0800_4000 - 0x0803_FFFF: App Firmware (240KB, Sectors 1-5)
//!   0x0804_0000 - 0x0805_FFFF: OTA Temp (128KB, Sector 6)
//!   0x0806_0000 - 0x0807_FFFF: User Data (128KB, Sector 7)
//!
//! STM32F411 Flash 扇区:
//!   Sector 0: 16KB  @ 0x0800_0000
//!   Sector 1: 16KB  @ 0x0800_4000
//!   Sector 2: 16KB  @ 0x0800_8000
//!   Sector 3: 16KB  @ 0x0800_C000
//!   Sector 4: 64KB  @ 0x0801_0000
//!   Sector 5: 128KB @ 0x0802_0000
//!   Sector 6: 128KB @ 0x0804_0000
//!   Sector 7: 128KB @ 0x0806_0000
//!
//! 配置存储 (Sector 7):
//!   OTA 标志占用前 4 字节, 配置数据从偏移 16 开始.
//!   0x0806_0000 + 0:  OTA 标志 (4 bytes)
//!   0x0806_0000 + 4:  保留 (12 bytes)
//!   0x0806_0010:      配置数据起始
//!   0x0806_0010 + 0:  魔数 0x434F_4E46 "CONF" (4 bytes)
//!   0x0806_0010 + 4:  版本号 (1 byte)
//!   0x0806_0010 + 5:  校验和 (1 byte)
//!   0x0806_0010 + 6:  保留 (2 bytes)
//!   0x0806_0014:      配置字段 (可变长度)

use servo_robot_protocol::config::BoardConfigSnapshot;
use stm32f4xx_hal::pac::FLASH;

// ============================================================================
// Flash 基础操作
// ============================================================================

/// Flash 错误
#[derive(Debug, defmt::Format)]
pub enum FlashError {
    /// 编程失败
    ProgramError,
    /// 地址未对齐
    NotAligned,
    /// 地址超出范围
    OutOfRange,
    /// 忙碌
    Busy,
}

/// OTA 标志位
#[repr(u32)]
pub enum OtaFlag {
    /// 无操作
    None = 0x0000_0000,
    /// OTA 待处理 (从 OTA Temp 拷贝到 App)
    Pending = 0x5441_4F54, // "OTAT"
    /// OTA 完成
    Done = 0x444F_4E45, // "DONE"
}

/// OTA 镜像头魔数 "OTAI"
pub const OTA_IMAGE_MAGIC: u32 = 0x4F54_4149;

/// OTA 镜像头格式版本
pub const OTA_IMAGE_FORMAT_VERSION: u8 = 1;

/// 目标 MCU 标识: STM32F411
pub const OTA_TARGET_MCU_F411: u8 = 0x41;

/// OTA 传输镜像头的固定长度（magic、版本、目标、保留字段、大小与 CRC32）。
pub const OTA_IMAGE_HEADER_SIZE: u32 = 16;

/// OTA 标志存储地址 (User Data 扇区起始)
pub const OTA_FLAG_ADDR: u32 = 0x0806_0000;

/// OTA Temp 区域起始地址
pub const OTA_TEMP_ADDR: u32 = 0x0804_0000;

/// OTA Temp 最大大小 (128KB, Sector 6)
pub const OTA_TEMP_MAX_SIZE: u32 = 128 * 1024;

/// 应用运行期使用的 Flash 扇区编号
pub const SECTOR_OTA_START: u8 = 6;
pub const SECTOR_USER_DATA: u8 = 7;

/// Flash 写入粒度 (字节)
const FLASH_WRITE_GRANULARITY: usize = 4;

/// 等待 Flash 操作完成
fn wait_flash(flash: &FLASH) -> Result<(), FlashError> {
    // 超时计数器 (约 100ms @ 96MHz)
    let mut timeout = 1_000_000u32;
    while flash.sr().read().bsy().bit_is_set() {
        timeout -= 1;
        if timeout == 0 {
            return Err(FlashError::Busy);
        }
    }
    // 检查错误标志
    let sr = flash.sr().read();
    if sr.pgperr().bit_is_set() || sr.pgaerr().bit_is_set() || sr.wrperr().bit_is_set() {
        // 清除错误标志 (write-1-to-clear)
        flash.sr().write(|w| unsafe {
            w.bits(1 << 6 | 1 << 5 | 1 << 4) // PGPERR | PGAERR | WRPERR
        });
        return Err(FlashError::ProgramError);
    }
    Ok(())
}

/// 解锁 Flash
fn unlock_flash(flash: &FLASH) {
    // 写入解锁密钥
    flash.keyr().write(|w| unsafe { w.bits(0x4567_0123) });
    flash.keyr().write(|w| unsafe { w.bits(0xCDEF_89AB) });
}

/// 锁定 Flash
fn lock_flash(flash: &FLASH) {
    flash.cr().modify(|_, w| w.lock().set_bit());
}

/// 擦除指定扇区
pub fn erase_sector(flash: &FLASH, sector: u8) -> Result<(), FlashError> {
    if sector > 7 {
        return Err(FlashError::OutOfRange);
    }

    unlock_flash(flash);
    wait_flash(flash)?;

    // 设置擦除操作
    flash.cr().modify(|_, w| {
        w.ser().set_bit();
        unsafe { w.snb().bits(sector) }
    });
    flash.cr().modify(|_, w| w.strt().set_bit());

    let result = wait_flash(flash);

    // 清除 SER 位
    flash.cr().modify(|_, w| w.ser().clear_bit());

    lock_flash(flash);
    result
}

/// 擦除 OTA Temp 区域 (Sector 6)
pub fn erase_ota_temp(flash: &FLASH) -> Result<(), FlashError> {
    erase_sector(flash, SECTOR_OTA_START)
}

/// 擦除 User Data 扇区
pub fn erase_user_data(flash: &FLASH) -> Result<(), FlashError> {
    erase_sector(flash, SECTOR_USER_DATA)
}

/// 向 Flash 写入数据 (32-bit 对齐)
///
/// # Safety
/// - 目标区域必须已擦除
/// - 地址必须 4 字节对齐
/// - 数据长度必须 4 字节对齐
pub fn program_flash(flash: &FLASH, addr: u32, data: &[u8]) -> Result<(), FlashError> {
    if addr % FLASH_WRITE_GRANULARITY as u32 != 0 {
        return Err(FlashError::NotAligned);
    }
    if data.len() % FLASH_WRITE_GRANULARITY != 0 {
        return Err(FlashError::NotAligned);
    }

    unlock_flash(flash);

    // 设置编程模式 (PSIZE = 32-bit)
    flash.cr().modify(|_, w| {
        w.pg().set_bit();
        unsafe { w.psize().bits(0b10) } // 32-bit
    });

    let mut offset = 0u32;
    let words = data.len() / FLASH_WRITE_GRANULARITY;

    for _ in 0..words {
        let word = u32::from_le_bytes([
            data[offset as usize],
            data[offset as usize + 1],
            data[offset as usize + 2],
            data[offset as usize + 3],
        ]);

        let write_addr = addr + offset;
        unsafe {
            core::ptr::write_volatile(write_addr as *mut u32, word);
        }

        let result = wait_flash(flash);
        if result.is_err() {
            flash.cr().modify(|_, w| w.pg().clear_bit());
            lock_flash(flash);
            return result;
        }

        offset += FLASH_WRITE_GRANULARITY as u32;
    }

    flash.cr().modify(|_, w| w.pg().clear_bit());
    lock_flash(flash);
    Ok(())
}

/// 从 Flash 读取数据
pub fn read_flash(addr: u32, buf: &mut [u8]) {
    let src = addr as *const u8;
    for (i, byte) in buf.iter_mut().enumerate() {
        *byte = unsafe { core::ptr::read_volatile(src.add(i)) };
    }
}

/// 读取 OTA 标志
pub fn read_ota_flag(_flash: &FLASH) -> OtaFlag {
    let mut buf = [0u8; 4];
    read_flash(OTA_FLAG_ADDR, &mut buf);
    let val = u32::from_le_bytes(buf);
    match val {
        0x5441_4F54 => OtaFlag::Pending,
        0x444F_4E45 => OtaFlag::Done,
        _ => OtaFlag::None,
    }
}

/// 写入 OTA 标志
///
/// 委托给 `save_metadata`, 单次擦除完成.
pub fn write_ota_flag(flash: &FLASH, flag: OtaFlag) -> Result<(), FlashError> {
    save_metadata(flash, flag, None)
}

/// 写入元数据（OTA 标志 + 配置），单次擦除
///
/// 读取当前扇区内容 → 修改目标字段 → 擦除 → 写回.
/// 解决 OTA 标志与配置共用 Sector 7 时的擦除冲突.
///
/// 缓冲区大小 256 字节，足够容纳 OTA 标志(4B) + 保留(12B) + 配置头(8B) + 配置数据(24B+)
pub fn save_metadata(
    flash: &FLASH,
    ota_flag: OtaFlag,
    config: Option<&BoardConfigSnapshot>,
) -> Result<(), FlashError> {
    // 1. 读取当前扇区前 256 字节到 RAM（足够容纳元数据）
    let mut sector_buf = [0u8; 256];
    read_flash(OTA_FLAG_ADDR, &mut sector_buf);

    // 2. 修改 OTA 标志
    let flag_bytes = (ota_flag as u32).to_le_bytes();
    sector_buf[0..4].copy_from_slice(&flag_bytes);

    // 3. 修改配置（如果提供）—— 使用栈缓冲区，避免堆分配
    if let Some(cfg) = config {
        let mut config_buf = [0u8; CONFIG_PAYLOAD_SIZE];
        let config_len = serialize_config_to_buf(cfg, &mut config_buf);
        let checksum = calc_checksum(&config_buf[..config_len]);
        let header = ConfigHeader {
            magic: CONFIG_MAGIC,
            version: CONFIG_VERSION,
            checksum,
            _reserved: 0,
        };
        let header_bytes = unsafe {
            core::slice::from_raw_parts(
                &header as *const ConfigHeader as *const u8,
                core::mem::size_of::<ConfigHeader>(),
            )
        };
        sector_buf[16..24].copy_from_slice(header_bytes);
        let copy_len = config_len.min(sector_buf.len() - 24);
        sector_buf[24..24 + copy_len].copy_from_slice(&config_buf[..copy_len]);
    }

    // 4. 擦除扇区
    erase_user_data(flash)?;

    // 5. 写回（按 4 字节对齐）
    let write_len = (sector_buf.len() + 3) & !3;
    program_flash(flash, OTA_FLAG_ADDR, &sector_buf[..write_len])
}

// ============================================================================
// 配置持久化存储
// ============================================================================

/// 配置存储魔数
const CONFIG_MAGIC: u32 = 0x434F_4E46; // "CONF"

/// 配置存储版本
const CONFIG_VERSION: u8 = 0x01;

/// 配置数据起始地址 (OTA 标志之后)
const CONFIG_ADDR: u32 = OTA_FLAG_ADDR + 0x10;

/// 配置存储头
#[repr(C, packed)]
struct ConfigHeader {
    magic: u32,
    version: u8,
    checksum: u8,
    _reserved: u16,
}

/// 计算校验和 (简单 XOR)
fn calc_checksum(data: &[u8]) -> u8 {
    let mut sum = 0u8;
    for &b in data {
        sum ^= b;
    }
    sum
}

/// 配置有效载荷大小 (4 bool + 2 u8 + 7 u16 + 1 u32 = 24 bytes)
const CONFIG_PAYLOAD_SIZE: usize = 24;

/// 将 BoardConfigSnapshot 序列化到栈缓冲区，避免堆分配
///
/// 返回写入的字节数 (固定为 CONFIG_PAYLOAD_SIZE)。
fn serialize_config_to_buf(config: &BoardConfigSnapshot, buf: &mut [u8]) -> usize {
    let mut o = 0;

    // === Switches (0x10~0x13) ===
    buf[o] = config.power_servo_on as u8;
    o += 1;
    buf[o] = config.power_5v_on as u8;
    o += 1;
    buf[o] = config.charge_on as u8;
    o += 1;
    buf[o] = config.bat_ext_out_on as u8;
    o += 1;

    // === Charge settings (0x20~0x21) ===
    buf[o] = config.charge_stop_percentage;
    o += 1;
    buf[o] = config.tx_log_level as u8;
    o += 1;

    // === Limits (0x30~0x37) ===
    buf[o..o + 2].copy_from_slice(&config.servo_current_limit_ma.to_le_bytes());
    o += 2;
    buf[o..o + 2].copy_from_slice(&config.servo_temp_limit.to_le_bytes());
    o += 2;
    buf[o..o + 2].copy_from_slice(&config.temp_5v_limit.to_le_bytes());
    o += 2;
    buf[o..o + 2].copy_from_slice(&config.charge_max_current_ma.to_le_bytes());
    o += 2;
    buf[o..o + 2].copy_from_slice(&config.charge_temp_derating.to_le_bytes());
    o += 2;
    buf[o..o + 2].copy_from_slice(&config.charge_temp_limit.to_le_bytes());
    o += 2;
    buf[o..o + 2].copy_from_slice(&config.charge_stop_voltage_mv.to_le_bytes());
    o += 2;
    buf[o..o + 4].copy_from_slice(&config.servo_baud_rate.to_le_bytes());
    o += 4;

    o
}

/// 从字节数组反序列化 BoardConfigSnapshot
fn deserialize_config(buf: &[u8]) -> Option<BoardConfigSnapshot> {
    BoardConfigSnapshot::from_bytes(buf).ok()
}

/// 保存配置到 Flash
///
/// 委托给 `save_metadata`, 保留当前 OTA 标志, 单次擦除完成.
pub fn save_config(flash: &FLASH, config: &BoardConfigSnapshot) -> Result<(), FlashError> {
    let ota_flag = read_ota_flag(flash);
    save_metadata(flash, ota_flag, Some(config))
}

/// 从 Flash 读取配置
///
/// 返回 Some(config) 如果配置有效, 否则返回 None (使用默认值).
pub fn load_config() -> Option<BoardConfigSnapshot> {
    // 读取头
    let mut header_buf = [0u8; 8];
    read_flash(CONFIG_ADDR, &mut header_buf);

    let magic = u32::from_le_bytes([header_buf[0], header_buf[1], header_buf[2], header_buf[3]]);
    let version = header_buf[4];
    let stored_checksum = header_buf[5];

    // 验证魔数和版本
    if magic != CONFIG_MAGIC || version != CONFIG_VERSION {
        return None;
    }

    // 读取配置数据 (BoardConfigSnapshot::PAYLOAD_SIZE = 24 bytes)
    let mut config_buf = [0u8; 24];
    read_flash(CONFIG_ADDR + 8, &mut config_buf);

    // 验证校验和
    let computed_checksum = calc_checksum(&config_buf);
    if computed_checksum != stored_checksum {
        return None;
    }

    deserialize_config(&config_buf)
}
