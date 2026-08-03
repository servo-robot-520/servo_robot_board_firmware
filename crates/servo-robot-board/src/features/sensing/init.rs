//! Sensing-feature initialization.

use embedded_mpu6500::Mpu6500;

/// Initialize the MPU6500 and return its identity plus readiness state.
///
/// SPI construction and pin ownership stay with the board runtime, while the
/// sensor-specific initialization sequence stays with the sensing feature.
pub fn initialize_mpu<SPI, E, D>(mpu: &mut Mpu6500<SPI>, delay: &mut D) -> (u8, bool)
where
    SPI: embedded_hal::spi::SpiDevice<u8, Error = E>,
    D: embedded_hal::delay::DelayNs,
{
    match mpu.init(delay) {
        Ok(()) => {
            let who = mpu.who_am_i().unwrap_or(0);
            defmt::info!("MPU6500 WHO_AM_I: 0x{:02X}", who);
            (who, true)
        }
        Err(_) => {
            defmt::error!("MPU6500 init failed");
            (0, false)
        }
    }
}
