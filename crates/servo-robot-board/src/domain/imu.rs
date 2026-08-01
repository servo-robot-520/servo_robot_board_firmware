//! IMU 数据采集与传感器融合
//!
//! 使用 embedded-mpu6500 crate 读取 MPU6500 传感器数据，
//! 并通过 Mahony AHRS 滤波器进行姿态估计。

use embedded_mpu6500::ImuSensorData;
use servo_robot_protocol::imu::ImuData;

/// Degrees to radians conversion factor.
const DEG_TO_RAD: f32 = core::f32::consts::PI / 180.0;

/// Mahony AHRS filter for attitude estimation.
///
/// Fuses accelerometer and gyroscope data into a quaternion orientation.
/// Suitable for embedded systems: no matrix operations, minimal CPU.
///
/// Reference: Mahony, R., Hamel, T., & Pflimlin, J.-M. (2008).
/// "Nonlinear Complementary Filters on the Special Orthogonal Group".
pub struct MahonyFilter {
    /// Quaternion state [w, x, y, z]
    q0: f32,
    q1: f32,
    q2: f32,
    q3: f32,
    /// Integral error feedback
    integral_fb_x: f32,
    integral_fb_y: f32,
    integral_fb_z: f32,
    /// 2 * proportional gain
    two_kp: f32,
    /// 2 * integral gain
    two_ki: f32,
}

impl MahonyFilter {
    /// Create a new filter with default gains.
    ///
    /// - Kp = 5.0 (proportional gain for accelerometer correction)
    /// - Ki = 0.1 (integral gain for gyro drift compensation)
    pub fn new() -> Self {
        Self {
            q0: 1.0,
            q1: 0.0,
            q2: 0.0,
            q3: 0.0,
            integral_fb_x: 0.0,
            integral_fb_y: 0.0,
            integral_fb_z: 0.0,
            two_kp: 2.0 * 5.0,
            two_ki: 2.0 * 0.1,
        }
    }

    /// Update filter with accelerometer (g) and gyroscope (deg/s) readings.
    ///
    /// `dt` is the sample period in seconds (e.g. 0.01 for 100 Hz).
    pub fn update(&mut self, ax: f32, ay: f32, az: f32, gx: f32, gy: f32, gz: f32, dt: f32) {
        // Convert gyro from deg/s to rad/s
        let mut gx = gx * DEG_TO_RAD;
        let mut gy = gy * DEG_TO_RAD;
        let mut gz = gz * DEG_TO_RAD;

        // Compute feedback only if accelerometer measurement is valid
        let accel_norm_sq = ax * ax + ay * ay + az * az;
        if accel_norm_sq > 0.0 {
            // Normalize accelerometer measurement
            let recip_norm = recip_sqrt(accel_norm_sq);
            let ax = ax * recip_norm;
            let ay = ay * recip_norm;
            let az = az * recip_norm;

            // Estimated direction of gravity (from quaternion)
            let halfvx = self.q1 * self.q3 - self.q0 * self.q2;
            let halfvy = self.q0 * self.q1 + self.q2 * self.q3;
            let halfvz = self.q0 * self.q0 - 0.5 + self.q3 * self.q3;

            // Error = cross product of estimated and measured gravity direction
            let halfex = ay * halfvz - az * halfvy;
            let halfey = az * halfvx - ax * halfvz;
            let halfez = ax * halfvy - ay * halfvx;

            // Apply integral feedback (gyro bias drift compensation)
            if self.two_ki > 0.0 {
                self.integral_fb_x += self.two_ki * halfex * dt;
                self.integral_fb_y += self.two_ki * halfey * dt;
                self.integral_fb_z += self.two_ki * halfez * dt;
                gx += self.integral_fb_x;
                gy += self.integral_fb_y;
                gz += self.integral_fb_z;
            } else {
                self.integral_fb_x = 0.0;
                self.integral_fb_y = 0.0;
                self.integral_fb_z = 0.0;
            }

            // Apply proportional feedback
            gx += self.two_kp * halfex;
            gy += self.two_kp * halfey;
            gz += self.two_kp * halfez;
        }

        // Integrate rate of change of quaternion
        gx *= 0.5 * dt;
        gy *= 0.5 * dt;
        gz *= 0.5 * dt;

        let qa = self.q0;
        let qb = self.q1;
        let qc = self.q2;

        self.q0 += -qb * gx - qc * gy - self.q3 * gz;
        self.q1 += qa * gx + qc * gz - self.q3 * gy;
        self.q2 += qa * gy - qb * gz + self.q3 * gx;
        self.q3 += qa * gz + qb * gy - qc * gx;

        // Normalize quaternion
        let recip_norm = recip_sqrt(
            self.q0 * self.q0 + self.q1 * self.q1 + self.q2 * self.q2 + self.q3 * self.q3,
        );
        self.q0 *= recip_norm;
        self.q1 *= recip_norm;
        self.q2 *= recip_norm;
        self.q3 *= recip_norm;
    }

    /// Get quaternion as [w, x, y, z].
    pub fn quaternion(&self) -> [f32; 4] {
        [self.q0, self.q1, self.q2, self.q3]
    }

    /// Get Euler angles as [roll, pitch, yaw] in degrees.
    ///
    /// - Roll: rotation about X axis (-180..+180)
    /// - Pitch: rotation about Y axis (-90..+90)
    /// - Yaw: rotation about Z axis (-180..+180)
    pub fn euler(&self) -> [f32; 3] {
        let roll = libm::atan2f(
            2.0 * (self.q0 * self.q1 + self.q2 * self.q3),
            1.0 - 2.0 * (self.q1 * self.q1 + self.q2 * self.q2),
        );

        // Clamp sinp to [-1, 1] to avoid NaN from asin at gimbal lock
        let sinp = 2.0 * (self.q0 * self.q2 - self.q3 * self.q1);
        let pitch = if sinp.abs() >= 1.0 {
            core::f32::consts::FRAC_PI_2.copysign(sinp)
        } else {
            libm::asinf(sinp)
        };

        let yaw = libm::atan2f(
            2.0 * (self.q0 * self.q3 + self.q1 * self.q2),
            1.0 - 2.0 * (self.q2 * self.q2 + self.q3 * self.q3),
        );

        [roll.to_degrees(), pitch.to_degrees(), yaw.to_degrees()]
    }
}

/// Reciprocal square root (1/sqrt(x)).
/// 编译为 VSQRT.F32 + VDIV.F32 两条指令。
#[inline(always)]
fn recip_sqrt(x: f32) -> f32 {
    1.0 / libm::sqrtf(x)
}

/// 将 MPU6500 原始数据转换为协议 ImuData，同时更新 Mahony 滤波器。
pub fn sensor_data_to_imu(data: &ImuSensorData, filter: &mut MahonyFilter, dt: f32) -> ImuData {
    // Update filter with accel (g) and gyro (deg/s)
    filter.update(
        data.accel[0],
        data.accel[1],
        data.accel[2],
        data.gyro[0],
        data.gyro[1],
        data.gyro[2],
        dt,
    );

    let q = filter.quaternion();
    let euler = filter.euler();

    ImuData {
        accel: data.accel,
        gyro: data.gyro,
        quaternion: q,
        timestamp_ms: 0,
        roll: euler[0],
        pitch: euler[1],
        yaw: euler[2],
    }
}

/// 读取 MPU6500 数据并转换为协议格式（含传感器融合）。
///
/// `filter` is the persistent Mahony AHRS state. `dt` is the sample
/// period in seconds (0.01 for 100 Hz).
pub fn read_imu_data<SPI, E>(
    mpu: &mut embedded_mpu6500::Mpu6500<SPI>,
    filter: &mut MahonyFilter,
    dt: f32,
) -> Result<ImuData, embedded_mpu6500::Error<E>>
where
    SPI: embedded_hal::spi::SpiDevice<u8, Error = E>,
{
    let data = mpu.read()?;
    Ok(sensor_data_to_imu(&data, filter, dt))
}

/// 读取 MPU6500 WHO_AM_I 寄存器
pub fn read_imu_id<SPI, E>(mpu: &mut embedded_mpu6500::Mpu6500<SPI>) -> u8
where
    SPI: embedded_hal::spi::SpiDevice<u8, Error = E>,
{
    mpu.who_am_i().unwrap_or(0)
}
