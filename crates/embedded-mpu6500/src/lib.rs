#![no_std]

//! MPU6500 6-axis IMU SPI driver
//!
//! Provides configuration and data reading for the InvenSense MPU6500
//! accelerometer and gyroscope over SPI.
//!
//! # Features
//!
//! - Accelerometer: ±2g, ±4g, ±8g, ±16g
//! - Gyroscope: ±250°/s, ±500°/s, ±1000°/s, ±2000°/s
//! - Temperature sensor
//! - Digital low-pass filter (DLPF) configuration
//! - FIFO buffer support
//! - Interrupt configuration (raw data ready, wake-on-motion, FIFO overflow)
//! - Sleep and low-power modes
//! - Gyro and accelerometer offset calibration

mod config;
mod registers;
mod types;

pub use config::*;
pub use types::*;

use embedded_hal::spi::{Operation, SpiDevice};

/// MPU6500 SPI driver.
///
/// # Type Parameters
///
/// - `SPI`: SPI device implementing `embedded_hal::spi::SpiDevice<u8>`
pub struct Mpu6500<SPI> {
    spi: SPI,
    gyro_sensitivity: f32,
    accel_sensitivity: f32,
}

impl<SPI, E> Mpu6500<SPI>
where
    SPI: SpiDevice<u8, Error = E>,
{
    /// Creates a new MPU6500 driver instance.
    ///
    /// The sensor is not initialized by this call — call [`init`](Self::init)
    /// to perform the full reset and configuration sequence.
    pub fn new(spi: SPI) -> Self {
        Self {
            spi,
            gyro_sensitivity: GyroRange::Dps250.sensitivity(),
            accel_sensitivity: AccelRange::G2.sensitivity(),
        }
    }

    /// Consume the driver and return the SPI device.
    pub fn destroy(self) -> SPI {
        self.spi
    }

    /// Performs a full hardware reset and basic initialization.
    ///
    /// This follows the datasheet-recommended SPI init sequence:
    /// 1. Device reset via PWR_MGMT_1
    /// 2. Wait 100ms
    /// 3. Signal path reset (gyro, accel, temp)
    /// 4. Wait 100ms
    /// 5. Configure clock source (PLL with X-axis gyro)
    /// 6. Set sample rate to 1kHz, DLPF to 250Hz bandwidth
    /// 7. Set gyro ±250°/s, accel ±2g
    ///
    /// A delay implementation is required for the 100ms waits.
    pub fn init<D: embedded_hal::delay::DelayNs>(&mut self, delay: &mut D) -> Result<(), Error<E>> {
        // Step 1: Device reset
        self.write_reg(registers::PWR_MGMT_1, 0x80)?;

        // Step 2: Wait 100ms for reset to complete
        delay.delay_ms(100u32);

        // Step 3: Signal path reset (gyro + accel + temp)
        self.write_reg(registers::SIGNAL_PATH_RESET, 0x07)?;

        // Step 4: Wait 100ms
        delay.delay_ms(100u32);

        // Step 5: Clock source = PLL with X-axis gyro reference
        self.write_reg(registers::PWR_MGMT_1, ClockSource::PllWithXGyro as u8)?;

        // Step 6: Sample rate = 1kHz (SMPLRT_DIV = 0)
        self.write_reg(registers::SMPLRT_DIV, 0x00)?;

        // Step 7: DLPF = 250Hz bandwidth, FIFO_MODE = overwrite
        self.write_reg(registers::CONFIG, DlpfConfig::Dlpf250 as u8)?;

        // Step 8: Gyro ±250°/s, FCHOICE_B = 00 (use DLPF)
        self.write_reg(registers::GYRO_CONFIG, GyroRange::Dps250 as u8)?;

        // Step 9: Accel ±2g
        self.write_reg(registers::ACCEL_CONFIG, AccelRange::G2 as u8)?;

        let found = self.who_am_i()?;
        if found != 0x70 {
            return Err(Error::InvalidDeviceId(found));
        }
        Ok(())
    }

    /// Performs a minimal reset without delays (for use when delays are not available).
    ///
    /// Warning: Does not wait for reset completion. Only suitable if you can
    /// guarantee sufficient time passes before the next SPI transaction.
    pub fn reset_no_delay(&mut self) -> Result<(), Error<E>> {
        self.write_reg(registers::PWR_MGMT_1, 0x80)?;
        self.write_reg(registers::SIGNAL_PATH_RESET, 0x07)?;
        self.write_reg(registers::PWR_MGMT_1, ClockSource::PllWithXGyro as u8)?;
        Ok(())
    }

    /// SPI write to a single register.
    ///
    /// Chip-select assertion and cleanup are delegated to `SpiDevice`.
    fn write_reg(&mut self, reg: u8, val: u8) -> Result<(), Error<E>> {
        let bytes = [reg & 0x7F, val];
        let mut operations = [Operation::Write(&bytes)];
        self.spi.transaction(&mut operations).map_err(Error::Spi)
    }

    /// SPI read from multiple consecutive registers.
    ///
    /// Chip-select assertion and cleanup are delegated to `SpiDevice`.
    fn read_regs(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), Error<E>> {
        let command = [reg | 0x80];
        let mut operations = [Operation::Write(&command), Operation::Read(buf)];
        self.spi.transaction(&mut operations).map_err(Error::Spi)
    }

    /// Reads the WHO_AM_I register. Should return 0x70 for MPU6500.
    pub fn who_am_i(&mut self) -> Result<u8, Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::WHO_AM_I, &mut buf)?;
        Ok(buf[0])
    }

    /// Verifies device identity by reading WHO_AM_I.
    ///
    /// Returns `Ok(true)` if the device responds with 0x70.
    pub fn verify_id(&mut self) -> Result<bool, Error<E>> {
        Ok(self.who_am_i()? == 0x70)
    }

    // --- Configuration ---

    /// Sets the gyroscope full-scale range.
    pub fn set_gyro_range(&mut self, range: GyroRange) -> Result<(), Error<E>> {
        self.write_reg(registers::GYRO_CONFIG, range as u8)?;
        self.gyro_sensitivity = range.sensitivity();
        Ok(())
    }

    /// Sets the accelerometer full-scale range.
    pub fn set_accel_range(&mut self, range: AccelRange) -> Result<(), Error<E>> {
        self.write_reg(registers::ACCEL_CONFIG, range as u8)?;
        self.accel_sensitivity = range.sensitivity();
        Ok(())
    }

    /// Sets the sample rate divider.
    ///
    /// `SAMPLE_RATE = 1kHz / (1 + div)` when DLPF is active.
    ///
    /// | div | Rate  |
    /// |-----|-------|
    /// | 0   | 1000 Hz |
    /// | 4   | 200 Hz |
    /// | 9   | 100 Hz |
    /// | 19  | 50 Hz  |
    /// | 99  | 10 Hz  |
    pub fn set_sample_rate_div(&mut self, div: u8) -> Result<(), Error<E>> {
        self.write_reg(registers::SMPLRT_DIV, div)
    }

    /// Sets the gyroscope and temperature digital low-pass filter.
    ///
    /// Only effective when FCHOICE_B = 00 in GYRO_CONFIG.
    pub fn set_dlpf(&mut self, dlpf: DlpfConfig) -> Result<(), Error<E>> {
        // CONFIG[2:0] = DLPF_CFG, CONFIG[6] = FIFO_MODE (preserve)
        let mut buf = [0u8; 1];
        self.read_regs(registers::CONFIG, &mut buf)?;
        let val = (buf[0] & 0xF8) | (dlpf as u8 & 0x07);
        self.write_reg(registers::CONFIG, val)
    }

    /// Sets the FIFO mode.
    ///
    /// - `Overwrite`: FIFO overflow overwrites oldest data (default)
    /// - `Reject`: FIFO overflow rejects new writes
    pub fn set_fifo_mode(&mut self, mode: FifoMode) -> Result<(), Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::CONFIG, &mut buf)?;
        let val = (buf[0] & 0xBF) | ((mode as u8) << 6);
        self.write_reg(registers::CONFIG, val)
    }

    /// Configures the INT pin behavior.
    pub fn configure_int_pin(
        &mut self,
        level: IntLevel,
        drive: IntDriveMode,
        latch: IntLatch,
        clear_on_any_read: bool,
    ) -> Result<(), Error<E>> {
        let mut val: u8 = 0;
        if level == IntLevel::ActiveLow {
            val |= 1 << 7;
        }
        if drive == IntDriveMode::OpenDrain {
            val |= 1 << 6;
        }
        if latch == IntLatch::Latched {
            val |= 1 << 5;
        }
        if clear_on_any_read {
            val |= 1 << 4;
        }
        self.write_reg(registers::INT_PIN_CFG, val)
    }

    /// Enables or disables I2C bypass mode.
    ///
    /// When enabled, the I2C master pins (ES_CL/ES_DA) are in pass-through
    /// mode, allowing direct I2C access from the host to any devices on the
    /// auxiliary I2C bus.
    pub fn set_i2c_bypass(&mut self, enable: bool) -> Result<(), Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::INT_PIN_CFG, &mut buf)?;
        let val = if enable {
            buf[0] | (1 << 1)
        } else {
            buf[0] & !(1 << 1)
        };
        self.write_reg(registers::INT_PIN_CFG, val)
    }

    /// Enables or disables specific interrupts.
    pub fn configure_interrupts(
        &mut self,
        raw_rdy: bool,
        fifo_overflow: bool,
        fsync: bool,
        wom: bool,
    ) -> Result<(), Error<E>> {
        let mut val: u8 = 0;
        if raw_rdy {
            val |= 1 << 0;
        }
        if fsync {
            val |= 1 << 3;
        }
        if fifo_overflow {
            val |= 1 << 4;
        }
        if wom {
            val |= 1 << 6;
        }
        self.write_reg(registers::INT_ENABLE, val)
    }

    /// Reads and clears the interrupt status register.
    pub fn read_int_status(&mut self) -> Result<IntStatus, Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::INT_STATUS, &mut buf)?;
        Ok(IntStatus {
            raw_data_rdy: buf[0] & 0x01 != 0,
            dmp: buf[0] & 0x02 != 0,
            fsync: buf[0] & 0x08 != 0,
            fifo_overflow: buf[0] & 0x10 != 0,
            wom: buf[0] & 0x40 != 0,
        })
    }

    /// Waits until raw data ready interrupt flag is set, then clears it.
    ///
    /// Polls INT_STATUS in a loop. Use with a timeout to avoid infinite blocking.
    pub fn wait_data_ready(&mut self) -> Result<bool, Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::INT_STATUS, &mut buf)?;
        Ok(buf[0] & 0x01 != 0)
    }

    // --- Power Management ---

    /// Enters sleep mode. All sensor measurements are suspended.
    pub fn sleep(&mut self) -> Result<(), Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::PWR_MGMT_1, &mut buf)?;
        let val = buf[0] | (1 << 6);
        self.write_reg(registers::PWR_MGMT_1, val)
    }

    /// Exits sleep mode.
    pub fn wakeup(&mut self) -> Result<(), Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::PWR_MGMT_1, &mut buf)?;
        let val = buf[0] & !(1 << 6);
        self.write_reg(registers::PWR_MGMT_1, val)
    }

    /// Enters cycle mode (alternates between sleep and single sample).
    ///
    /// Wake-up rate is controlled by `LP_ACCEL_ODR` (MPU6500 mode) or
    /// `LP_WAKE_CTRL` in PWR_MGMT_2 (MPU-6050 compatible mode).
    pub fn set_cycle_mode(&mut self, enable: bool) -> Result<(), Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::PWR_MGMT_1, &mut buf)?;
        let val = if enable {
            buf[0] | (1 << 5)
        } else {
            buf[0] & !(1 << 5)
        };
        self.write_reg(registers::PWR_MGMT_1, val)
    }

    /// Enables or disables the temperature sensor.
    pub fn set_temp_enabled(&mut self, enable: bool) -> Result<(), Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::PWR_MGMT_1, &mut buf)?;
        let val = if enable {
            buf[0] & !(1 << 3)
        } else {
            buf[0] | (1 << 3)
        };
        self.write_reg(registers::PWR_MGMT_1, val)
    }

    /// Sets the clock source.
    pub fn set_clock_source(&mut self, clk: ClockSource) -> Result<(), Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::PWR_MGMT_1, &mut buf)?;
        let val = (buf[0] & 0xF8) | (clk as u8 & 0x07);
        self.write_reg(registers::PWR_MGMT_1, val)
    }

    /// Configures PWR_MGMT_2: enables/disables individual sensor axes.
    ///
    /// Setting a disable bit puts that axis into standby mode.
    pub fn set_sensor_enable(
        &mut self,
        accel_x: bool,
        accel_y: bool,
        accel_z: bool,
        gyro_x: bool,
        gyro_y: bool,
        gyro_z: bool,
    ) -> Result<(), Error<E>> {
        let mut val: u8 = 0;
        if !accel_x {
            val |= 1 << 5;
        }
        if !accel_y {
            val |= 1 << 4;
        }
        if !accel_z {
            val |= 1 << 3;
        }
        if !gyro_x {
            val |= 1 << 2;
        }
        if !gyro_y {
            val |= 1 << 1;
        }
        if !gyro_z {
            val |= 1 << 0;
        }
        self.write_reg(registers::PWR_MGMT_2, val)
    }

    /// Sets the low-power accelerometer wake-up frequency.
    ///
    /// Only effective in cycle mode with accel-only low-power.
    pub fn set_lp_accel_odr(&mut self, lposc_clksel: u8) -> Result<(), Error<E>> {
        self.write_reg(registers::LP_ACCEL_ODR, lposc_clksel & 0x0F)
    }

    /// Sets the wake-on-motion threshold (LSB = 4mg, range 0–1020mg).
    pub fn set_wom_threshold(&mut self, threshold_mg: u8) -> Result<(), Error<E>> {
        self.write_reg(registers::WOM_THR, threshold_mg / 4)
    }

    /// Enables or disables wake-on-motion detection.
    pub fn set_wom_enabled(&mut self, enable: bool) -> Result<(), Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::ACCEL_INTEL_CTRL, &mut buf)?;
        let val = if enable {
            buf[0] | (1 << 7)
        } else {
            buf[0] & !(1 << 7)
        };
        self.write_reg(registers::ACCEL_INTEL_CTRL, val)
    }

    // --- FIFO ---

    /// Enables or disables the FIFO buffer.
    pub fn set_fifo_enabled(&mut self, enable: bool) -> Result<(), Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::USER_CTRL, &mut buf)?;
        let val = if enable {
            buf[0] | (1 << 6)
        } else {
            buf[0] & !(1 << 6)
        };
        self.write_reg(registers::USER_CTRL, val)
    }

    /// Resets the FIFO buffer.
    pub fn reset_fifo(&mut self) -> Result<(), Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::USER_CTRL, &mut buf)?;
        let val = buf[0] | (1 << 2);
        self.write_reg(registers::USER_CTRL, val)
    }

    /// Configures which sensor data is written to the FIFO.
    pub fn set_fifo_enable(
        &mut self,
        temp: bool,
        gyro_x: bool,
        gyro_y: bool,
        gyro_z: bool,
        accel: bool,
    ) -> Result<(), Error<E>> {
        let mut val: u8 = 0;
        if temp {
            val |= 1 << 7;
        }
        if gyro_x {
            val |= 1 << 6;
        }
        if gyro_y {
            val |= 1 << 5;
        }
        if gyro_z {
            val |= 1 << 4;
        }
        if accel {
            val |= 1 << 3;
        }
        self.write_reg(registers::FIFO_EN, val)
    }

    /// Returns the number of bytes currently in the FIFO buffer.
    pub fn fifo_count(&mut self) -> Result<u16, Error<E>> {
        let mut buf = [0u8; 2];
        self.read_regs(registers::FIFO_COUNT_H, &mut buf)?;
        Ok(u16::from_be_bytes([buf[0], buf[1]]))
    }

    /// Reads one byte from the FIFO buffer.
    pub fn fifo_read_byte(&mut self) -> Result<u8, Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::FIFO_R_W, &mut buf)?;
        Ok(buf[0])
    }

    /// Reads up to `buf.len()` bytes from the FIFO into `buf`.
    ///
    /// Returns the number of bytes actually read.
    pub fn fifo_read(&mut self, buf: &mut [u8]) -> Result<usize, Error<E>> {
        let count = self.fifo_count()? as usize;
        let len = core::cmp::min(count, buf.len());
        if len == 0 {
            return Ok(0);
        }
        self.read_regs(registers::FIFO_R_W, &mut buf[..len])?;
        Ok(len)
    }

    // --- Signal Path ---

    /// Resets the gyro, accel, and temp digital signal paths.
    ///
    /// Sensor registers are not cleared by this — use `reset_signal_conditioning()`
    /// to clear them.
    pub fn reset_signal_paths(&mut self) -> Result<(), Error<E>> {
        self.write_reg(registers::SIGNAL_PATH_RESET, 0x07)
    }

    /// Resets all signal paths and clears sensor registers.
    ///
    /// This is the most complete software reset short of DEVICE_RESET.
    pub fn reset_signal_conditioning(&mut self) -> Result<(), Error<E>> {
        let mut buf = [0u8; 1];
        self.read_regs(registers::USER_CTRL, &mut buf)?;
        let val = buf[0] | (1 << 0);
        self.write_reg(registers::USER_CTRL, val)
    }

    // --- Offset Calibration ---

    /// Sets the gyroscope offset for a single axis.
    ///
    /// The offset is a 16-bit two's complement value added to the raw gyro
    /// output before entering the sensor register. Resolution depends on
    /// the current full-scale range.
    pub fn set_gyro_offset(&mut self, axis: Axis, offset: i16) -> Result<(), Error<E>> {
        let (h_reg, l_reg) = match axis {
            Axis::X => (registers::XG_OFFSET_H, registers::XG_OFFSET_L),
            Axis::Y => (registers::YG_OFFSET_H, registers::YG_OFFSET_L),
            Axis::Z => (registers::ZG_OFFSET_H, registers::ZG_OFFSET_L),
        };
        let bytes = offset.to_be_bytes();
        self.write_reg(h_reg, bytes[0])?;
        self.write_reg(l_reg, bytes[1])?;
        Ok(())
    }

    /// Sets the accelerometer offset for a single axis.
    ///
    /// The offset is a 15-bit value (bit 0 of low byte is reserved).
    /// Resolution: 0.98mg per LSB, range ±16g.
    pub fn set_accel_offset(&mut self, axis: Axis, offset: i16) -> Result<(), Error<E>> {
        if !(-16_384..=16_383).contains(&offset) {
            return Err(Error::InvalidAccelOffset(offset));
        }
        let (h_reg, l_reg) = match axis {
            Axis::X => (registers::XA_OFFSET_H, registers::XA_OFFSET_L),
            Axis::Y => (registers::YA_OFFSET_H, registers::YA_OFFSET_L),
            Axis::Z => (registers::ZA_OFFSET_H, registers::ZA_OFFSET_L),
        };
        let shifted = (offset as u16) << 1;
        let bytes = shifted.to_be_bytes();
        self.write_reg(h_reg, bytes[0])?;
        self.write_reg(l_reg, bytes[1] & 0xFE)?;
        Ok(())
    }

    // --- Data Reading ---

    /// Reads the raw accelerometer, temperature, and gyroscope data.
    ///
    /// Reads 14 bytes starting from ACCEL_XOUT_H (0x3B):
    /// - Bytes 0-5: Accel X/Y/Z (big-endian, signed 16-bit)
    /// - Bytes 6-7: Temperature (big-endian, signed 16-bit)
    /// - Bytes 8-13: Gyro X/Y/Z (big-endian, signed 16-bit)
    pub fn read(&mut self) -> Result<ImuSensorData, Error<E>> {
        let mut buf = [0u8; 14];
        self.read_regs(registers::ACCEL_XOUT_H, &mut buf)?;

        let accel = [
            i16::from_be_bytes([buf[0], buf[1]]) as f32 / self.accel_sensitivity,
            i16::from_be_bytes([buf[2], buf[3]]) as f32 / self.accel_sensitivity,
            i16::from_be_bytes([buf[4], buf[5]]) as f32 / self.accel_sensitivity,
        ];

        let temp_c = i16::from_be_bytes([buf[6], buf[7]]) as f32 / 333.87 + 21.0;

        let gyro = [
            i16::from_be_bytes([buf[8], buf[9]]) as f32 / self.gyro_sensitivity,
            i16::from_be_bytes([buf[10], buf[11]]) as f32 / self.gyro_sensitivity,
            i16::from_be_bytes([buf[12], buf[13]]) as f32 / self.gyro_sensitivity,
        ];

        Ok(ImuSensorData {
            accel,
            gyro,
            temp_c,
        })
    }

    /// Reads raw accelerometer data only (6 bytes).
    pub fn read_accel_raw(&mut self) -> Result<[i16; 3], Error<E>> {
        let mut buf = [0u8; 6];
        self.read_regs(registers::ACCEL_XOUT_H, &mut buf)?;
        Ok([
            i16::from_be_bytes([buf[0], buf[1]]),
            i16::from_be_bytes([buf[2], buf[3]]),
            i16::from_be_bytes([buf[4], buf[5]]),
        ])
    }

    /// Reads raw gyroscope data only (6 bytes).
    pub fn read_gyro_raw(&mut self) -> Result<[i16; 3], Error<E>> {
        let mut buf = [0u8; 6];
        self.read_regs(registers::GYRO_XOUT_H, &mut buf)?;
        Ok([
            i16::from_be_bytes([buf[0], buf[1]]),
            i16::from_be_bytes([buf[2], buf[3]]),
            i16::from_be_bytes([buf[4], buf[5]]),
        ])
    }

    /// Reads raw temperature data only (2 bytes).
    pub fn read_temp_raw(&mut self) -> Result<i16, Error<E>> {
        let mut buf = [0u8; 2];
        self.read_regs(registers::TEMP_OUT_H, &mut buf)?;
        Ok(i16::from_be_bytes([buf[0], buf[1]]))
    }

    /// Reads temperature in °C.
    pub fn read_temp(&mut self) -> Result<f32, Error<E>> {
        let raw = self.read_temp_raw()?;
        Ok(raw as f32 / 333.87 + 21.0)
    }

    // --- Self-Test ---

    /// Reads the gyroscope self-test output values.
    ///
    /// Returns [x, y, z] self-test data from manufacturing tests.
    /// These values should be compared against self-test results to
    /// verify sensor health (see AN-MPU-6500A-02).
    pub fn read_gyro_self_test(&mut self) -> Result<[u8; 3], Error<E>> {
        let mut buf = [0u8; 3];
        self.read_regs(registers::SELF_TEST_X_GYRO, &mut buf)?;
        Ok(buf)
    }

    /// Reads the accelerometer self-test output values.
    ///
    /// Returns [x, y, z] self-test data from manufacturing tests.
    pub fn read_accel_self_test(&mut self) -> Result<[u8; 3], Error<E>> {
        let mut buf = [0u8; 3];
        self.read_regs(registers::SELF_TEST_X_ACCEL, &mut buf)?;
        Ok(buf)
    }

    /// Performs a basic self-test check.
    ///
    /// Reads self-test registers and returns the raw values. Compare these
    /// against the factory-programmed values in the device's OTP memory
    /// to determine pass/fail. See AN-MPU-6500A-02 for the complete
    /// self-test procedure.
    pub fn self_test(&mut self) -> Result<([u8; 3], [u8; 3]), Error<E>> {
        let gyro_st = self.read_gyro_self_test()?;
        let accel_st = self.read_accel_self_test()?;
        Ok((gyro_st, accel_st))
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal::spi::{ErrorType, Operation, SpiDevice};
    use std::collections::VecDeque;
    use std::vec::Vec;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum MockError {
        Injected,
    }

    impl embedded_hal::spi::Error for MockError {
        fn kind(&self) -> embedded_hal::spi::ErrorKind {
            embedded_hal::spi::ErrorKind::Other
        }
    }

    #[derive(Debug)]
    struct ExpectedTransaction {
        writes: Vec<Vec<u8>>,
        read: Vec<u8>,
        result: Result<(), MockError>,
    }

    impl ExpectedTransaction {
        fn write(bytes: &[u8]) -> Self {
            Self {
                writes: std::vec![bytes.to_vec()],
                read: Vec::new(),
                result: Ok(()),
            }
        }

        fn write_read(write: &[u8], read: &[u8]) -> Self {
            Self {
                writes: std::vec![write.to_vec()],
                read: read.to_vec(),
                result: Ok(()),
            }
        }

        fn failing_write(bytes: &[u8]) -> Self {
            Self {
                writes: std::vec![bytes.to_vec()],
                read: Vec::new(),
                result: Err(MockError::Injected),
            }
        }
    }

    #[derive(Debug)]
    struct MockSpi {
        expected: VecDeque<ExpectedTransaction>,
    }

    impl MockSpi {
        fn new(expected: impl IntoIterator<Item = ExpectedTransaction>) -> Self {
            Self {
                expected: expected.into_iter().collect(),
            }
        }

        fn assert_complete(&self) {
            assert!(
                self.expected.is_empty(),
                "unconsumed SPI expectations: {:?}",
                self.expected
            );
        }
    }

    impl ErrorType for MockSpi {
        type Error = MockError;
    }

    impl SpiDevice<u8> for MockSpi {
        fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
            let expected = self
                .expected
                .pop_front()
                .expect("unexpected SPI transaction");
            let mut writes = Vec::new();
            let mut read_count = 0;

            for operation in operations {
                match operation {
                    Operation::Write(bytes) => writes.push(bytes.to_vec()),
                    Operation::Read(bytes) => {
                        read_count += 1;
                        assert_eq!(bytes.len(), expected.read.len());
                        bytes.copy_from_slice(&expected.read);
                    }
                    Operation::Transfer(_, _)
                    | Operation::TransferInPlace(_)
                    | Operation::DelayNs(_) => {
                        panic!("unexpected SPI operation: {operation:?}")
                    }
                }
            }

            assert_eq!(writes, expected.writes);
            assert_eq!(read_count, usize::from(!expected.read.is_empty()));
            expected.result
        }
    }

    #[test]
    fn uses_spi_device_transactions_for_register_writes_and_reads() {
        let spi = MockSpi::new([
            ExpectedTransaction::write(&[registers::GYRO_CONFIG, GyroRange::Dps1000 as u8]),
            ExpectedTransaction::write_read(
                &[registers::GYRO_XOUT_H | 0x80],
                &[0, 1, 0xFF, 0xFE, 0, 3],
            ),
        ]);
        let mut sensor = Mpu6500::new(spi);

        sensor.set_gyro_range(GyroRange::Dps1000).unwrap();
        assert_eq!(sensor.read_gyro_raw().unwrap(), [1, -2, 3]);

        sensor.destroy().assert_complete();
    }

    #[test]
    fn writes_offsets_to_the_selected_axis_registers() {
        let spi = MockSpi::new([
            ExpectedTransaction::write(&[registers::YG_OFFSET_H, 0xFF]),
            ExpectedTransaction::write(&[registers::YG_OFFSET_L, 0xFE]),
            ExpectedTransaction::write(&[registers::ZA_OFFSET_H, 0xFF]),
            ExpectedTransaction::write(&[registers::ZA_OFFSET_L, 0xFE]),
        ]);
        let mut sensor = Mpu6500::new(spi);

        sensor.set_gyro_offset(Axis::Y, -2).unwrap();
        sensor.set_accel_offset(Axis::Z, -1).unwrap();

        sensor.destroy().assert_complete();
    }

    #[test]
    fn rejects_accel_offsets_outside_the_signed_15_bit_range() {
        let spi = MockSpi::new([]);
        let mut sensor = Mpu6500::new(spi);

        assert!(matches!(
            sensor.set_accel_offset(Axis::X, 16_384),
            Err(Error::InvalidAccelOffset(16_384))
        ));
        assert!(matches!(
            sensor.set_accel_offset(Axis::X, -16_385),
            Err(Error::InvalidAccelOffset(-16_385))
        ));

        sensor.destroy().assert_complete();
    }

    #[test]
    fn maps_spi_failures_to_driver_errors() {
        let spi = MockSpi::new([ExpectedTransaction::failing_write(&[
            registers::SMPLRT_DIV,
            0x07,
        ])]);
        let mut sensor = Mpu6500::new(spi);

        let result = sensor.set_sample_rate_div(0x07);
        assert!(matches!(result, Err(Error::Spi(MockError::Injected))));

        sensor.destroy().assert_complete();
    }
}
