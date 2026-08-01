//! IMU 数据采集辅助函数
//!
//! 使用 embedded-mpu6500 crate 读取 MPU6500 传感器数据。

use embedded_mpu6500::ImuSensorData;
use servo_robot_protocol::imu::ImuData;

/// 将 MPU6500 原始数据转换为协议 ImuData
pub fn sensor_data_to_imu(data: &ImuSensorData) -> ImuData {
    ImuData {
        accel: data.accel,
        gyro: data.gyro,
        quaternion: [1.0, 0.0, 0.0, 0.0],
        timestamp_ms: 0,
        roll: 0.0,
        pitch: 0.0,
        yaw: 0.0,
    }
}

/// 读取 MPU6500 数据并转换为协议格式
pub fn read_imu_data<SPI, E>(
    mpu: &mut embedded_mpu6500::Mpu6500<SPI>,
) -> Result<ImuData, embedded_mpu6500::Error<E>>
where
    SPI: embedded_hal::spi::SpiDevice<u8, Error = E>,
{
    let data = mpu.read()?;
    Ok(sensor_data_to_imu(&data))
}

/// 读取 MPU6500 WHO_AM_I 寄存器
pub fn read_imu_id<SPI, E>(mpu: &mut embedded_mpu6500::Mpu6500<SPI>) -> u8
where
    SPI: embedded_hal::spi::SpiDevice<u8, Error = E>,
{
    mpu.who_am_i().unwrap_or(0)
}
