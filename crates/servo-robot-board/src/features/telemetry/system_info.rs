//! 系统信息辅助函数

/// 获取 STM32 设备 ID
pub fn get_device_id() -> u16 {
    let dp = unsafe { stm32f4xx_hal::pac::Peripherals::steal() };
    dp.DBGMCU.idcode().read().dev_id().bits()
}

/// 获取 STM32 唯一 ID
pub fn get_uid() -> u32 {
    let uid = stm32f4xx_hal::signature::Uid::get();
    ((uid.x() as u32) << 16) | (uid.y() as u32)
}
