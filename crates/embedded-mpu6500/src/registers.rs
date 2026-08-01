//! MPU6500 register addresses.
//!
//! Reference: MPU-6500 Register Map and Descriptions, Revision 2.1 (RM-MPU-6500A-00).
//! See also [bq24725-registers.md](../bq24725-registers.md) for a full register reference.

#![allow(dead_code)]

/// Self-test registers
pub const SELF_TEST_X_GYRO: u8 = 0x00;
pub const SELF_TEST_Y_GYRO: u8 = 0x01;
pub const SELF_TEST_Z_GYRO: u8 = 0x02;
pub const SELF_TEST_X_ACCEL: u8 = 0x0D;
pub const SELF_TEST_Y_ACCEL: u8 = 0x0E;
pub const SELF_TEST_Z_ACCEL: u8 = 0x0F;

/// Gyro offset registers (16-bit, two's complement)
pub const XG_OFFSET_H: u8 = 0x13;
pub const XG_OFFSET_L: u8 = 0x14;
pub const YG_OFFSET_H: u8 = 0x15;
pub const YG_OFFSET_L: u8 = 0x16;
pub const ZG_OFFSET_H: u8 = 0x17;
pub const ZG_OFFSET_L: u8 = 0x18;

/// Sample rate divider: SAMPLE_RATE = 1kHz / (1 + SMPLRT_DIV)
pub const SMPLRT_DIV: u8 = 0x19;

/// Configuration register (DLPF, EXT_SYNC, FIFO_MODE)
pub const CONFIG: u8 = 0x1A;

/// Gyroscope configuration (full-scale, self-test, FCHOICE_B)
pub const GYRO_CONFIG: u8 = 0x1B;

/// Accelerometer configuration (full-scale, self-test)
pub const ACCEL_CONFIG: u8 = 0x1C;

/// Accelerometer configuration 2 (DLPF, FCHOICE_B)
pub const ACCEL_CONFIG_2: u8 = 0x1D;

/// Low-power accelerometer ODR control
pub const LP_ACCEL_ODR: u8 = 0x1E;

/// Wake-on motion threshold (LSB = 4mg)
pub const WOM_THR: u8 = 0x1F;

/// FIFO enable
pub const FIFO_EN: u8 = 0x23;

/// I2C master control
pub const I2C_MST_CTRL: u8 = 0x24;

/// INT pin / bypass enable configuration
pub const INT_PIN_CFG: u8 = 0x37;

/// Interrupt enable
pub const INT_ENABLE: u8 = 0x38;

/// Interrupt status (read-only)
pub const INT_STATUS: u8 = 0x3A;

/// Accelerometer measurements, 6 bytes starting here (X_H, X_L, Y_H, Y_L, Z_H, Z_L)
pub const ACCEL_XOUT_H: u8 = 0x3B;

/// Temperature measurement, 2 bytes starting here (H, L)
pub const TEMP_OUT_H: u8 = 0x41;

/// Gyroscope measurements, 6 bytes starting here (X_H, X_L, Y_H, Y_L, Z_H, Z_L)
pub const GYRO_XOUT_H: u8 = 0x43;

/// External sensor data, 24 bytes (0x49..0x60)
pub const EXT_SENS_DATA_00: u8 = 0x49;

/// I2C master delay control
pub const I2C_MST_DELAY_CTRL: u8 = 0x67;

/// Signal path reset
pub const SIGNAL_PATH_RESET: u8 = 0x68;

/// Accelerometer interrupt control (wake-on-motion)
pub const ACCEL_INTEL_CTRL: u8 = 0x69;

/// User control (FIFO, DMP, I2C master, signal path reset)
pub const USER_CTRL: u8 = 0x6A;

/// Power management 1 (reset, sleep, cycle, clock source)
pub const PWR_MGMT_1: u8 = 0x6B;

/// Power management 2 (sensor disable, low-power wake freq)
pub const PWR_MGMT_2: u8 = 0x6C;

/// FIFO count high byte (bits [12:8])
pub const FIFO_COUNT_H: u8 = 0x72;

/// FIFO count low byte
pub const FIFO_COUNT_L: u8 = 0x73;

/// FIFO read/write
pub const FIFO_R_W: u8 = 0x74;

/// Device identity register (reset value = 0x70)
pub const WHO_AM_I: u8 = 0x75;

/// Accelerometer offset registers (15-bit, 0.98mg/LSB)
pub const XA_OFFSET_H: u8 = 0x77;
pub const XA_OFFSET_L: u8 = 0x78;
pub const YA_OFFSET_H: u8 = 0x7A;
pub const YA_OFFSET_L: u8 = 0x7B;
pub const ZA_OFFSET_H: u8 = 0x7D;
pub const ZA_OFFSET_L: u8 = 0x7E;
