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
    /// 擦除失败
    EraseError,
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

/// OTA 镜像头 (位于 OTA Temp 区域起始)
///
/// 布局 (16 bytes):
///   [0..4]   magic: u32           = 0x4F54_4149 "OTAI"
///   [4]      format_version: u8   = 1
///   [5]      target_mcu: u8       = 0x41 (STM32F411)
///   [6..8]   _reserved: u16
///   [8..12]  image_size: u32      = 镜像大小 (不含头)
///   [12..16] image_crc32: u32     = 镜像 CRC32
#[repr(C, packed)]
pub struct OtaImageHeader {
    pub magic: u32,
    pub format_version: u8,
    pub target_mcu: u8,
    pub _reserved: u16,
    pub image_size: u32,
    pub image_crc32: u32,
}

/// OTA 镜像头大小
pub const OTA_IMAGE_HEADER_SIZE: u32 = 16;

impl OtaImageHeader {
    /// 从字节数组解析镜像头
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < OTA_IMAGE_HEADER_SIZE as usize {
            return None;
        }
        Some(Self {
            magic: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            format_version: data[4],
            target_mcu: data[5],
            _reserved: u16::from_le_bytes([data[6], data[7]]),
            image_size: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            image_crc32: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
        })
    }
}

/// OTA 镜像验证错误
#[derive(Debug, defmt::Format)]
pub enum OtaValidationError {
    /// 镜像头读取失败
    ReadError,
    /// 魔数不匹配
    InvalidMagic,
    /// 格式版本不支持
    UnsupportedVersion,
    /// 目标 MCU 不匹配
    WrongTargetMcu,
    /// 镜像大小超出范围
    ImageTooLarge,
    /// CRC32 校验失败
    CrcMismatch,
}

/// 验证 OTA 镜像
///
/// 1. 读取 OTA Temp 起始的 OtaImageHeader
/// 2. 验证 magic == OTA_IMAGE_MAGIC
/// 3. 验证 format_version == OTA_IMAGE_FORMAT_VERSION
/// 4. 验证 target_mcu == OTA_TARGET_MCU_F411
/// 5. 验证 image_size <= APP_MAX_SIZE
/// 6. 验证 image_crc32
pub fn validate_ota_image(flash: &FLASH) -> Result<OtaImageHeader, OtaValidationError> {
    // 1. 读取镜像头
    let mut header_buf = [0u8; OTA_IMAGE_HEADER_SIZE as usize];
    read_flash(OTA_TEMP_ADDR, &mut header_buf);

    let header = OtaImageHeader::from_bytes(&header_buf)
        .ok_or(OtaValidationError::ReadError)?;

    // 读取字段到局部变量 (packed struct 不能直接引用)
    let magic = header.magic;
    let format_version = header.format_version;
    let target_mcu = header.target_mcu;
    let image_size = header.image_size;
    let image_crc32 = header.image_crc32;

    // 2. 验证魔数
    if magic != OTA_IMAGE_MAGIC {
        defmt::error!("OTA: invalid magic 0x{:08X}", magic);
        return Err(OtaValidationError::InvalidMagic);
    }

    // 3. 验证格式版本
    if format_version != OTA_IMAGE_FORMAT_VERSION {
        defmt::error!("OTA: unsupported format version {}", format_version);
        return Err(OtaValidationError::UnsupportedVersion);
    }

    // 4. 验证目标 MCU
    if target_mcu != OTA_TARGET_MCU_F411 {
        defmt::error!("OTA: wrong target MCU 0x{:02X}", target_mcu);
        return Err(OtaValidationError::WrongTargetMcu);
    }

    // 5. 验证镜像大小
    if image_size > APP_MAX_SIZE {
        defmt::error!("OTA: image too large ({} > {})", image_size, APP_MAX_SIZE);
        return Err(OtaValidationError::ImageTooLarge);
    }

    // 6. 验证 CRC32 (计算镜像数据的 CRC，跳过镜像头)
    let image_data_addr = OTA_TEMP_ADDR + OTA_IMAGE_HEADER_SIZE;
    if !verify_firmware_crc(image_data_addr, image_size + 4) {
        defmt::error!("OTA: CRC mismatch");
        return Err(OtaValidationError::CrcMismatch);
    }

    defmt::info!("OTA: image validated (size={}, crc=0x{:08X})", image_size, image_crc32);
    Ok(header)
}

/// OTA 标志存储地址 (User Data 扇区起始)
pub const OTA_FLAG_ADDR: u32 = 0x0806_0000;

/// App 固件起始地址
pub const APP_START_ADDR: u32 = 0x0800_4000;

/// OTA Temp 区域起始地址
pub const OTA_TEMP_ADDR: u32 = 0x0804_0000;

/// App 固件最大大小 (240KB, Sectors 1-5)
pub const APP_MAX_SIZE: u32 = 240 * 1024;

/// OTA Temp 最大大小 (128KB, Sector 6)
pub const OTA_TEMP_MAX_SIZE: u32 = 128 * 1024;

/// 扇区编号
pub const SECTOR_BOOTLOADER: u8 = 0;
pub const SECTOR_APP_START: u8 = 1;
pub const SECTOR_APP_END: u8 = 5;
pub const SECTOR_OTA_START: u8 = 6;
pub const SECTOR_OTA_END: u8 = 6;
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

/// 擦除 OTA Temp 区域 (Sector 4-6)
pub fn erase_ota_temp(flash: &FLASH) -> Result<(), FlashError> {
    erase_sector(flash, SECTOR_OTA_START)?;
    erase_sector(flash, 5)?;
    erase_sector(flash, SECTOR_OTA_END)
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

    for i in 0..words {
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
pub fn read_ota_flag(flash: &FLASH) -> OtaFlag {
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
pub fn save_metadata(
    flash: &FLASH,
    ota_flag: OtaFlag,
    config: Option<&BoardConfigSnapshot>,
) -> Result<(), FlashError> {
    // 1. 读取当前扇区前 48 字节到 RAM
    let mut sector_buf = [0u8; 48];
    read_flash(OTA_FLAG_ADDR, &mut sector_buf);

    // 2. 修改 OTA 标志
    let flag_bytes = (ota_flag as u32).to_le_bytes();
    sector_buf[0..4].copy_from_slice(&flag_bytes);

    // 3. 修改配置（如果提供）
    if let Some(cfg) = config {
        let config_data = serialize_config(cfg);
        let checksum = calc_checksum(&config_data);
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
        let copy_len = config_data.len().min(sector_buf.len() - 24);
        sector_buf[24..24 + copy_len].copy_from_slice(&config_data[..copy_len]);
    }

    // 4. 擦除扇区
    erase_user_data(flash)?;

    // 5. 写回
    program_flash(flash, OTA_FLAG_ADDR, &sector_buf)
}

/// 将 OTA Temp 区域拷贝到 App 区域
///
/// 这是 servo-robot-board-bootloader 的核心功能:
/// 1. 擦除 App 扇区 (1-3)
/// 2. 从 OTA Temp 读取数据并写入 App
/// 3. 清除 OTA 标志
pub fn copy_ota_to_app(flash: &FLASH, size: u32) -> Result<(), FlashError> {
    if size > APP_MAX_SIZE {
        return Err(FlashError::OutOfRange);
    }

    // 擦除 App 扇区
    for sector in SECTOR_APP_START..=SECTOR_APP_END {
        erase_sector(flash, sector)?;
    }

    // 按 4KB 块拷贝
    let mut offset = 0u32;
    let mut buf = [0u8; 4096];

    while offset < size {
        let chunk_size = (size - offset).min(4096) as usize;

        // 读取 OTA Temp
        read_flash(OTA_TEMP_ADDR + offset, &mut buf[..chunk_size]);

        // 写入 App
        program_flash(flash, APP_START_ADDR + offset, &buf[..chunk_size])?;

        offset += chunk_size as u32;
    }

    // 清除 OTA 标志
    write_ota_flag(flash, OtaFlag::Done)?;

    Ok(())
}

/// 计算 CRC32
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// 验证固件 CRC32
///
/// 固件末尾 4 字节存储 CRC32 值
pub fn verify_firmware_crc(addr: u32, size: u32) -> bool {
    if size < 8 {
        return false;
    }

    let mut buf = [0u8; 4096];
    let mut crc = 0xFFFF_FFFFu32;
    let data_size = size - 4;

    let mut offset = 0u32;
    while offset < data_size {
        let chunk_size = (data_size - offset).min(4096) as usize;
        read_flash(addr + offset, &mut buf[..chunk_size]);

        for &byte in &buf[..chunk_size] {
            crc ^= byte as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB8_8320;
                } else {
                    crc >>= 1;
                }
            }
        }

        offset += chunk_size as u32;
    }

    let computed_crc = !crc;
    let mut stored_crc_buf = [0u8; 4];
    read_flash(addr + data_size, &mut stored_crc_buf);
    let stored_crc = u32::from_le_bytes(stored_crc_buf);

    computed_crc == stored_crc
}

/// 跳转到 App 固件
///
/// # Safety
/// - App 固件必须有效 (向量表正确)
/// - 调用前必须禁用所有中断
pub unsafe fn jump_to_app(app_addr: u32) -> ! {
    let sp = unsafe { core::ptr::read_volatile(app_addr as *const u32) };
    let reset_vector = unsafe { core::ptr::read_volatile((app_addr + 4) as *const u32) };

    // 设置栈指针并跳转
    unsafe {
        cortex_m::register::msp::write(sp);
        let jump = core::mem::transmute::<u32, fn() -> !>(reset_vector);
        jump()
    }
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

/// 配置数据最大大小 (User Data 扇区 16KB - 前 16 字节)
const CONFIG_MAX_SIZE: usize = 16 * 1024 - 16;

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

/// 将 BoardConfigSnapshot 序列化为字节数组
fn serialize_config(config: &BoardConfigSnapshot) -> alloc::vec::Vec<u8> {
    config.to_bytes()
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
