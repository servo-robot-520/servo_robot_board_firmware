#![no_std]
#![no_main]

//! Servo Robot Bootloader
//!
//! Bootloader 占据 Flash Sector 0 (16KB), 地址 0x0800_0000
//!
//! 启动流程:
//! 1. 检查 OTA 标志 (User Data 扇区)
//! 2. 如果 OTA_PENDING: 从 OTA Temp 拷贝到 App → 清除标志
//! 3. 跳转到 App (0x0800_4000)

use cortex_m_rt::entry;
use panic_halt as _;

use stm32f4::stm32f411;

/// App 固件起始地址
const APP_ADDR: u32 = 0x0800_4000;
/// OTA 标志地址 (User Data 扇区)
const OTA_FLAG_ADDR: u32 = 0x0806_0000;
/// OTA Temp 起始地址
const OTA_TEMP_ADDR: u32 = 0x0804_0000;
/// App 最大大小 (240KB, Sectors 1-5)
const APP_MAX_SIZE: u32 = 240 * 1024;
/// OTA 标志值
const OTA_PENDING: u32 = 0x5441_4F54;
const OTA_DONE: u32 = 0x444F_4E45;

fn read_u32(addr: u32) -> u32 {
    unsafe { core::ptr::read_volatile(addr as *const u32) }
}

fn wait_flash(flash: &stm32f411::FLASH) {
    while flash.sr().read().bsy().bit_is_set() {}
}

fn unlock_flash(flash: &stm32f411::FLASH) {
    flash.keyr().write(|w| unsafe { w.bits(0x4567_0123) });
    flash.keyr().write(|w| unsafe { w.bits(0xCDEF_89AB) });
}

fn lock_flash(flash: &stm32f411::FLASH) {
    flash.cr().modify(|_, w| w.lock().set_bit());
}

fn erase_sector(flash: &stm32f411::FLASH, sector: u8) {
    wait_flash(flash);
    flash.cr().modify(|_, w| {
        w.ser().set_bit();
        unsafe { w.snb().bits(sector) }
    });
    flash.cr().modify(|_, w| w.strt().set_bit());
    wait_flash(flash);
    flash.cr().modify(|_, w| w.ser().clear_bit());
}

fn write_word(flash: &stm32f411::FLASH, addr: u32, word: u32) {
    wait_flash(flash);
    flash.cr().modify(|_, w| {
        w.pg().set_bit();
        unsafe { w.psize().bits(0b10) }
    });
    unsafe {
        core::ptr::write_volatile(addr as *mut u32, word);
    }
    wait_flash(flash);
    flash.cr().modify(|_, w| w.pg().clear_bit());
}

fn copy_ota_to_app(flash: &stm32f411::FLASH, size: u32) {
    erase_sector(flash, 1);
    erase_sector(flash, 2);
    erase_sector(flash, 3);

    let mut offset = 0u32;
    while offset < size {
        let word = read_u32(OTA_TEMP_ADDR + offset);
        write_word(flash, APP_ADDR + offset, word);
        offset += 4;
    }
}

fn clear_ota_flag(flash: &stm32f411::FLASH) {
    erase_sector(flash, 7);
    write_word(flash, OTA_FLAG_ADDR, OTA_DONE);
}

#[entry]
fn main() -> ! {
    let dp = stm32f411::Peripherals::take().unwrap();
    let flash = &dp.FLASH;

    let ota_flag = read_u32(OTA_FLAG_ADDR);

    if ota_flag == OTA_PENDING {
        let firmware_size = read_u32(OTA_TEMP_ADDR);
        if firmware_size > 0 && firmware_size <= APP_MAX_SIZE {
            unlock_flash(flash);
            copy_ota_to_app(flash, firmware_size);
            clear_ota_flag(flash);
            lock_flash(flash);
        }
    }

    // 跳转到 App
    unsafe {
        cortex_m::interrupt::disable();
        let sp = read_u32(APP_ADDR);
        let reset_vector = read_u32(APP_ADDR + 4);

        if sp < 0x2000_0000 || sp > 0x2002_0000 {
            loop {
                cortex_m::asm::wfi();
            }
        }

        // 设置 MSP 并跳转
        cortex_m::asm::bootstrap(sp as *const u32, reset_vector as *const u32);
    }
}
