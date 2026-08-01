/// A physical axis used by the offset calibration registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub enum Axis {
    /// X axis.
    X,
    /// Y axis.
    Y,
    /// Z axis.
    Z,
}

/// MPU6500 driver error.
#[derive(Debug)]
pub enum Error<E> {
    /// The SPI device operation failed.
    Spi(E),
    /// The device identity did not match the MPU6500 value `0x70`.
    InvalidDeviceId(u8),
    /// An accelerometer offset did not fit the signed 15-bit register format.
    InvalidAccelOffset(i16),
}

/// IMU sensor data (accelerometer, gyroscope, temperature).
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct ImuSensorData {
    /// Acceleration in g: [x, y, z]
    pub accel: [f32; 3],
    /// Angular velocity in °/s: [x, y, z]
    pub gyro: [f32; 3],
    /// Temperature in °C
    pub temp_c: f32,
}

/// Interrupt status flags read from INT_STATUS (0x3A).
#[derive(Debug, Clone, Copy, Default)]
pub struct IntStatus {
    /// Wake-on-motion interrupt occurred
    pub wom: bool,
    /// FIFO overflow interrupt occurred
    pub fifo_overflow: bool,
    /// FSYNC interrupt occurred
    pub fsync: bool,
    /// DMP interrupt generated
    pub dmp: bool,
    /// Raw sensor data ready to be read
    pub raw_data_rdy: bool,
}
