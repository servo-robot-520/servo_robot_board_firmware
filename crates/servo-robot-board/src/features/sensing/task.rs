//! Sensing task helper functions
//!
//! Wraps IMU and battery data acquisition for use by RTIC tasks in main.rs.

use super::{battery, imu};
use embedded_bq40z50::Bq40z50;
use embedded_mpu6500::Mpu6500;
use servo_robot_protocol::battery_state::BatteryState;
use servo_robot_protocol::imu::ImuData;

/// Read IMU data with sensor fusion (100 Hz sample period).
///
/// Delegates to `imu::read_imu_data` with the given Mahony filter.
pub fn read_imu<SPI, E>(
    mpu: &mut Mpu6500<SPI>,
    filter: &mut imu::MahonyFilter,
    dt: f32,
) -> Result<ImuData, embedded_mpu6500::Error<E>>
where
    SPI: embedded_hal::spi::SpiDevice<u8, Error = E>,
{
    imu::read_imu_data(mpu, filter, dt)
}

/// Read full battery state from BQ40Z50 gauge.
///
/// Delegates to `battery::read_bq40z50_data`.
pub fn read_battery<I2C, E>(gauge: &mut Bq40z50<I2C>) -> BatteryState
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    battery::read_bq40z50_data(gauge)
}
