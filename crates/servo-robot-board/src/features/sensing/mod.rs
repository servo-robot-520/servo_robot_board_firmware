//! Sensing feature: IMU and battery data acquisition
//!
//! Contains:
//! - IMU data acquisition and Mahony AHRS sensor fusion
//! - Battery state reading from BQ40Z50 gauge

pub mod battery;
pub mod imu;
pub mod init;
pub mod task;
