#![no_std]

//! BQ40Z50 Smart Battery Gauge driver
//!
//! Based on BQ40Z50-R1 SMBus protocol.
//! Reference: https://www.ti.com/product/BQ40Z50-R1

mod cmd;
mod flags;
mod types;

// Re-export everything so downstream users can `use embedded_bq40z50::*`
pub use cmd::*;
pub use flags::*;
pub use types::*;

use embedded_hal::i2c::I2c;

// ============================================================================
// Driver
// ============================================================================

/// BQ40Z50 driver
pub struct Bq40z50<I2C> {
    i2c: I2C,
    address: u8,
}

/// Convert 0.1K to Celsius, Actual temperature = return value / 10
fn kelvin_to_celsius(temp_0_1k: u16) -> i16 {
    (temp_0_1k as i32 - 2732) as i16
}

impl<I2C, I2cError> Bq40z50<I2C>
where
    I2C: I2c<Error = I2cError>,
{
    /// Create a new BQ40Z50 driver
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            address: BQ40Z50_ADDR,
        }
    }

    /// Create with custom address
    pub fn with_address(i2c: I2C, address: u8) -> Self {
        Self { i2c, address }
    }

    /// Consume the driver and return the I²C bus.
    pub fn destroy(self) -> I2C {
        self.i2c
    }

    // ========================================================================
    // Low-level SMBus access
    // ========================================================================

    /// Read a 16-bit word (SMBus word, little-endian)
    fn read_word(&mut self, cmd: u8) -> Result<u16, Error<I2cError>> {
        let mut buf = [0u8; 2];
        self.i2c
            .write_read(self.address, &[cmd], &mut buf)
            .map_err(Error::I2c)?;
        Ok(u16::from_le_bytes(buf))
    }

    /// Read a single byte
    fn read_byte(&mut self, cmd: u8) -> Result<u8, Error<I2cError>> {
        let mut buf = [0u8; 1];
        self.i2c
            .write_read(self.address, &[cmd], &mut buf)
            .map_err(Error::I2c)?;
        Ok(buf[0])
    }

    /// Write a 16-bit word (for ManufacturerAccess sub-commands)
    fn write_word(&mut self, cmd: u8, data: u16) -> Result<(), Error<I2cError>> {
        let bytes = data.to_le_bytes();
        self.i2c
            .write(self.address, &[cmd, bytes[0], bytes[1]])
            .map_err(Error::I2c)
    }

    /// Read an SMBus block into `buf` and return the number of data bytes.
    ///
    /// SMBus block reads start with a count byte. This method validates that
    /// the count fits in the caller buffer before copying any data.
    pub fn read_block(&mut self, cmd: u8, buf: &mut [u8]) -> Result<usize, Error<I2cError>> {
        let mut response = [0u8; 33];
        self.i2c
            .write_read(self.address, &[cmd], &mut response)
            .map_err(Error::I2c)?;
        let reported = response[0] as usize;
        if reported > 32 || reported > buf.len() {
            return Err(Error::InvalidBlockLength {
                reported: response[0],
                capacity: buf.len() as u8,
            });
        }
        buf[..reported].copy_from_slice(&response[1..=reported]);
        Ok(reported)
    }

    // ========================================================================
    // Public API — Standard SBS commands
    // ========================================================================

    /// Check if device is connected (try reading voltage)
    pub fn is_connected(&mut self) -> bool {
        self.read_word(CMD_VOLTAGE).is_ok()
    }

    /// Get serial number
    pub fn serial_number(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_SERIAL)
    }

    /// Get temperature in Celsius
    pub fn temperature_c(&mut self) -> Result<i16, Error<I2cError>> {
        let raw = self.read_word(CMD_TEMPERATURE)?;
        Ok(kelvin_to_celsius(raw))
    }

    /// Get battery voltage (mV)
    pub fn voltage_mv(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_VOLTAGE)
    }

    /// Get current (mA, signed: positive=discharge, negative=charge)
    pub fn current_ma(&mut self) -> Result<i16, Error<I2cError>> {
        let raw = self.read_word(CMD_CURRENT)?;
        Ok(raw as i16)
    }

    /// Get average current (mA, signed)
    pub fn average_current_ma(&mut self) -> Result<i16, Error<I2cError>> {
        let raw = self.read_word(CMD_AVERAGE_CURRENT)?;
        Ok(raw as i16)
    }

    /// Get max error (%)
    pub fn max_error(&mut self) -> Result<u8, Error<I2cError>> {
        self.read_byte(CMD_MAX_ERROR)
    }

    /// Get relative SOC (%)
    pub fn relative_soc(&mut self) -> Result<u8, Error<I2cError>> {
        self.read_byte(CMD_RELATIVE_SOC)
    }

    /// Get absolute SOC (%)
    pub fn absolute_soc(&mut self) -> Result<u8, Error<I2cError>> {
        self.read_byte(CMD_ABSOLUTE_SOC)
    }

    /// Get remaining capacity (mAh)
    pub fn remaining_capacity_mah(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_REMAINING_CAPACITY)
    }

    /// Get full charge capacity (mAh)
    pub fn full_charge_capacity_mah(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_FULL_CHARGE_CAPACITY)
    }

    /// Get runtime to empty (min)
    pub fn runtime_to_empty_min(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_RUNTIME_TO_EMPTY)
    }

    /// Get average time to empty (min)
    pub fn avg_time_to_empty_min(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_AVG_TIME_TO_EMPTY)
    }

    /// Get average time to full (min)
    pub fn avg_time_to_full_min(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_AVG_TIME_TO_FULL)
    }

    /// Get recommended charging current (mA)
    pub fn charging_current_ma(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_CHARGING_CURRENT)
    }

    /// Get recommended charging voltage (mV)
    pub fn charging_voltage_mv(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_CHARGING_VOLTAGE)
    }

    /// Get battery status flags
    pub fn battery_status(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_BATTERY_STATUS)
    }

    /// Get cycle count
    pub fn cycle_count(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_CYCLE_COUNT)
    }

    /// Get design capacity (mAh)
    pub fn design_capacity_mah(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_DESIGN_CAPACITY)
    }

    /// Get battery mode flags
    pub fn battery_mode(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_BATTERY_MODE)
    }

    /// Read device chemistry as packed ASCII (first 2 bytes of block read)
    ///
    /// Common values: 0x4F4C="LO"(LiOn), 0x504C="LP"(LiPo), 0x464C="LF"(LiFe)
    pub fn device_chemistry_u16(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_DEVICE_CHEMISTRY)
    }

    // ========================================================================
    // Cell voltages
    // ========================================================================

    /// Get cell 1 voltage (mV)
    pub fn cell_voltage_1_mv(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_CELL_VOLTAGE_1)
    }

    /// Get cell 2 voltage (mV)
    pub fn cell_voltage_2_mv(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_CELL_VOLTAGE_2)
    }

    /// Get cell 3 voltage (mV)
    pub fn cell_voltage_3_mv(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_CELL_VOLTAGE_3)
    }

    /// Get cell 4 voltage (mV)
    pub fn cell_voltage_4_mv(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_CELL_VOLTAGE_4)
    }

    /// Get all 4 cell voltages (mV)
    pub fn cell_voltages_mv(&mut self) -> Result<[u16; 4], Error<I2cError>> {
        Ok([
            self.cell_voltage_1_mv()?,
            self.cell_voltage_2_mv()?,
            self.cell_voltage_3_mv()?,
            self.cell_voltage_4_mv()?,
        ])
    }

    // ========================================================================
    // Standard SBS — additional commands
    // ========================================================================

    /// Get design voltage (mV)
    pub fn design_voltage_mv(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_DESIGN_VOLTAGE)
    }

    /// Get manufacturer date
    ///
    /// Encoded as: Day + Month*32 + (Year-1980)*256
    pub fn manufacturer_date(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_MANUFACTURER_DATE)
    }

    /// Get AtRateTimeToFull (min). 65535 = not charging.
    pub fn at_rate_time_to_full_min(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_AT_RATE_TIME_TO_FULL)
    }

    /// Get AtRateTimeToEmpty (min). 65535 = not discharging.
    pub fn at_rate_time_to_empty_min(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(CMD_AT_RATE_TIME_TO_EMPTY)
    }

    /// Set AtRate value (mA or 10mW depending on BatteryMode[CAPM])
    pub fn set_at_rate(&mut self, value: u16) -> Result<(), Error<I2cError>> {
        self.write_word(CMD_AT_RATE, value)
    }

    /// Read the device name into a caller-provided UTF-8/ASCII byte buffer.
    pub fn device_name(&mut self, buf: &mut [u8]) -> Result<usize, Error<I2cError>> {
        self.read_block(CMD_DEVICE_NAME, buf)
    }

    /// Read the manufacturer name into a caller-provided UTF-8/ASCII byte buffer.
    pub fn manufacturer_name(&mut self, buf: &mut [u8]) -> Result<usize, Error<I2cError>> {
        self.read_block(CMD_MANUFACTURER_NAME, buf)
    }

    // ========================================================================
    // MAC sub-command helpers
    // ========================================================================

    /// Start a MAC command. Use [`read_mac_block`](Self::read_mac_block) after
    /// the gauge-specific command-response delay has elapsed.
    pub fn start_mac_command(&mut self, subcmd: u16) -> Result<(), Error<I2cError>> {
        self.write_word(CMD_MANUFACTURER_ACCESS, subcmd)
    }

    /// Read the response to a previously started MAC command.
    pub fn read_mac_block(&mut self, buf: &mut [u8]) -> Result<usize, Error<I2cError>> {
        self.read_block(CMD_MANUFACTURER_BLOCK_ACCESS, buf)
    }

    /// Execute a MAC read after a caller-provided delay.
    pub fn mac_read_block_after<D: embedded_hal::delay::DelayNs>(
        &mut self,
        subcmd: u16,
        delay: &mut D,
        wait_ms: u32,
        buf: &mut [u8],
    ) -> Result<usize, Error<I2cError>> {
        self.start_mac_command(subcmd)?;
        delay.delay_ms(wait_ms);
        self.read_mac_block(buf)
    }

    /// Execute an immediate MAC read when the caller knows the response is ready.
    fn mac_read_block(&mut self, subcmd: u16, buf: &mut [u8]) -> Result<(), Error<I2cError>> {
        self.start_mac_command(subcmd)?;
        let reported = self.read_mac_block(buf)?;
        if reported != buf.len() {
            return Err(Error::UnexpectedBlockLength {
                expected: buf.len() as u8,
                reported: reported as u8,
            });
        }
        Ok(())
    }

    // ========================================================================
    // Safety / Status (MAC block reads)
    // ========================================================================

    /// Read SafetyAlert flags (latched alarm flags, write-1-to-clear)
    pub fn safety_alert(&mut self) -> Result<SafetyFlags, Error<I2cError>> {
        let mut buf = [0u8; 4];
        self.mac_read_block(SUBCMD_SAFETY_ALERT, &mut buf)?;
        Ok(SafetyFlags(u32::from_le_bytes(buf)))
    }

    /// Read SafetyStatus flags (active safety status)
    pub fn safety_status(&mut self) -> Result<SafetyFlags, Error<I2cError>> {
        let mut buf = [0u8; 4];
        self.mac_read_block(SUBCMD_SAFETY_STATUS, &mut buf)?;
        Ok(SafetyFlags(u32::from_le_bytes(buf)))
    }

    /// Read OperationStatus flags
    pub fn operation_status(&mut self) -> Result<OperationStatus, Error<I2cError>> {
        let mut buf = [0u8; 4];
        self.mac_read_block(SUBCMD_OPERATION_STATUS, &mut buf)?;
        Ok(OperationStatus(u32::from_le_bytes(buf)))
    }

    /// Read ChargingStatus flags
    pub fn charging_status(&mut self) -> Result<ChargingStatus, Error<I2cError>> {
        let mut buf = [0u8; 4];
        self.mac_read_block(SUBCMD_CHARGING_STATUS, &mut buf)?;
        Ok(ChargingStatus(u32::from_le_bytes(buf)))
    }

    /// Read GaugingStatus flags
    pub fn gauging_status(&mut self) -> Result<GaugingStatus, Error<I2cError>> {
        let mut buf = [0u8; 4];
        self.mac_read_block(SUBCMD_GAUGING_STATUS, &mut buf)?;
        Ok(GaugingStatus(u32::from_le_bytes(buf)))
    }

    // ========================================================================
    // Device information (MAC block reads)
    // ========================================================================

    /// Read DeviceType (IC part number) via MAC
    pub fn device_type(&mut self) -> Result<u16, Error<I2cError>> {
        let mut buf = [0u8; 4];
        self.mac_read_block(SUBCMD_DEVICE_TYPE, &mut buf)?;
        Ok(u16::from_le_bytes([buf[0], buf[1]]))
    }

    /// Read FirmwareVersion via MAC (returns raw 4 bytes: device_number, version, build, type)
    pub fn firmware_version(&mut self) -> Result<[u8; 4], Error<I2cError>> {
        let mut buf = [0u8; 4];
        self.mac_read_block(SUBCMD_FIRMWARE_VERSION, &mut buf)?;
        Ok([buf[0], buf[1], buf[2], buf[3]])
    }

    /// Read HardwareVersion via MAC
    pub fn hardware_version(&mut self) -> Result<u16, Error<I2cError>> {
        let mut buf = [0u8; 4];
        self.mac_read_block(SUBCMD_HARDWARE_VERSION, &mut buf)?;
        Ok(u16::from_le_bytes([buf[0], buf[1]]))
    }

    /// Read ChemicalID via MAC (OCV table ID)
    pub fn chem_id(&mut self) -> Result<u16, Error<I2cError>> {
        let mut buf = [0u8; 4];
        self.mac_read_block(SUBCMD_CHEM_ID, &mut buf)?;
        Ok(u16::from_le_bytes([buf[0], buf[1]]))
    }

    /// Read StateOfHealth via MAC (SOH FCC + energy)
    pub fn state_of_health(&mut self) -> Result<StateOfHealth, Error<I2cError>> {
        let mut buf = [0u8; 4];
        self.mac_read_block(SUBCMD_STATE_OF_HEALTH, &mut buf)?;
        Ok(StateOfHealth {
            fcc_mah: u16::from_le_bytes([buf[0], buf[1]]),
            energy_cwh: u16::from_le_bytes([buf[2], buf[3]]),
        })
    }

    // ========================================================================
    // DAStatus2 (ManufacturerBlockAccess)
    // ========================================================================

    /// Read DAStatus2: detailed temperature readings (7 × 0.1K values = 14 bytes).
    ///
    /// Returns internal temp, TS1~TS4, cell temp, and FET temp.
    pub fn da_status_2(&mut self) -> Result<TempDetail, Error<I2cError>> {
        let mut buf = [0u8; 14];
        self.mac_read_block(SUBCMD_DA_STATUS2, &mut buf)?;

        Ok(TempDetail {
            int_temp: kelvin_to_celsius(u16::from_le_bytes([buf[0], buf[1]])),
            ts1_temp: kelvin_to_celsius(u16::from_le_bytes([buf[2], buf[3]])),
            ts2_temp: kelvin_to_celsius(u16::from_le_bytes([buf[4], buf[5]])),
            ts3_temp: kelvin_to_celsius(u16::from_le_bytes([buf[6], buf[7]])),
            ts4_temp: kelvin_to_celsius(u16::from_le_bytes([buf[8], buf[9]])),
            cell_temp: kelvin_to_celsius(u16::from_le_bytes([buf[10], buf[11]])),
            fet_temp: kelvin_to_celsius(u16::from_le_bytes([buf[12], buf[13]])),
        })
    }

    // ========================================================================
    // Control commands
    // ========================================================================

    /// Reset the device (sends MAC DeviceReset 0x0041).
    ///
    /// WARNING: This will reset the gauge. Use with caution.
    pub fn device_reset(&mut self) -> Result<(), Error<I2cError>> {
        self.write_word(CMD_MANUFACTURER_ACCESS, SUBCMD_DEVICE_RESET)
    }

    /// Enable or disable gauging (MAC 0x0021).
    ///
    /// When called, toggles the current state (GAUGING command is a toggle).
    /// Check `gauging_status().it_enabled()` afterwards.
    pub fn toggle_gauging(&mut self) -> Result<(), Error<I2cError>> {
        self.write_word(CMD_MANUFACTURER_ACCESS, SUBCMD_GAUGING)
    }

    /// Enable or disable firmware FET control (MAC 0x0022).
    ///
    /// Toggles the FET_EN flag. Check `operation_status()` afterwards.
    pub fn toggle_fet_control(&mut self) -> Result<(), Error<I2cError>> {
        self.write_word(CMD_MANUFACTURER_ACCESS, SUBCMD_FET_CONTROL)
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal::delay::DelayNs;
    use embedded_hal::i2c::{ErrorType, I2c, Operation};
    use std::collections::VecDeque;
    use std::vec::Vec;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum MockError {
        Injected,
    }

    impl embedded_hal::i2c::Error for MockError {
        fn kind(&self) -> embedded_hal::i2c::ErrorKind {
            embedded_hal::i2c::ErrorKind::Other
        }
    }

    #[derive(Debug)]
    struct ExpectedTransaction {
        write: Vec<u8>,
        read: Vec<u8>,
        result: Result<(), MockError>,
    }

    impl ExpectedTransaction {
        fn write(bytes: &[u8]) -> Self {
            Self {
                write: bytes.to_vec(),
                read: Vec::new(),
                result: Ok(()),
            }
        }

        fn write_read(write: &[u8], read: &[u8]) -> Self {
            Self {
                write: write.to_vec(),
                read: read.to_vec(),
                result: Ok(()),
            }
        }

        fn failing_write(bytes: &[u8]) -> Self {
            Self {
                write: bytes.to_vec(),
                read: Vec::new(),
                result: Err(MockError::Injected),
            }
        }
    }

    #[derive(Debug)]
    struct MockI2c {
        expected: VecDeque<ExpectedTransaction>,
    }

    impl MockI2c {
        fn new(expected: impl IntoIterator<Item = ExpectedTransaction>) -> Self {
            Self {
                expected: expected.into_iter().collect(),
            }
        }

        fn assert_complete(&self) {
            assert!(
                self.expected.is_empty(),
                "unconsumed I2C expectations: {:?}",
                self.expected
            );
        }
    }

    impl ErrorType for MockI2c {
        type Error = MockError;
    }

    impl I2c for MockI2c {
        fn transaction(
            &mut self,
            address: u8,
            operations: &mut [Operation<'_>],
        ) -> Result<(), Self::Error> {
            assert_eq!(address, BQ40Z50_ADDR);
            let expected = self
                .expected
                .pop_front()
                .expect("unexpected I2C transaction");
            let mut write = None;
            let mut read_count = 0;

            for operation in operations {
                match operation {
                    Operation::Write(bytes) => {
                        assert!(write.replace(bytes.to_vec()).is_none(), "multiple writes")
                    }
                    Operation::Read(bytes) => {
                        read_count += 1;
                        assert_eq!(bytes.len(), expected.read.len());
                        bytes.copy_from_slice(&expected.read);
                    }
                }
            }

            assert_eq!(write.unwrap_or_default(), expected.write);
            assert_eq!(read_count, usize::from(!expected.read.is_empty()));
            expected.result
        }
    }

    #[derive(Default)]
    struct RecordingDelay {
        calls_ns: Vec<u32>,
    }

    impl DelayNs for RecordingDelay {
        fn delay_ns(&mut self, ns: u32) {
            self.calls_ns.push(ns);
        }
    }

    fn smbus_block(data: &[u8]) -> [u8; 33] {
        assert!(data.len() <= 32);
        let mut response = [0u8; 33];
        response[0] = data.len() as u8;
        response[1..=data.len()].copy_from_slice(data);
        response
    }

    #[test]
    fn reads_smbus_blocks_using_the_count_byte() {
        let response = smbus_block(b"BQ40");
        let i2c = MockI2c::new([ExpectedTransaction::write_read(
            &[CMD_DEVICE_NAME],
            &response,
        )]);
        let mut gauge = Bq40z50::new(i2c);
        let mut name = [0u8; 8];

        let len = gauge.device_name(&mut name).unwrap();

        assert_eq!(len, 4);
        assert_eq!(&name[..len], b"BQ40");
        gauge.destroy().assert_complete();
    }

    #[test]
    fn rejects_oversized_smbus_blocks_without_mutating_the_output_buffer() {
        let response = smbus_block(b"12345");
        let i2c = MockI2c::new([ExpectedTransaction::write_read(
            &[CMD_MANUFACTURER_NAME],
            &response,
        )]);
        let mut gauge = Bq40z50::new(i2c);
        let mut name = [0xA5; 4];

        assert!(matches!(
            gauge.manufacturer_name(&mut name),
            Err(Error::InvalidBlockLength {
                reported: 5,
                capacity: 4
            })
        ));
        assert_eq!(name, [0xA5; 4]);
        gauge.destroy().assert_complete();
    }

    #[test]
    fn mac_read_block_after_writes_little_endian_subcommand_then_waits() {
        let response = smbus_block(&[0x50, 0x40]);
        let i2c = MockI2c::new([
            ExpectedTransaction::write(&[CMD_MANUFACTURER_ACCESS, 0x01, 0x00]),
            ExpectedTransaction::write_read(&[CMD_MANUFACTURER_BLOCK_ACCESS], &response),
        ]);
        let mut gauge = Bq40z50::new(i2c);
        let mut delay = RecordingDelay::default();
        let mut data = [0u8; 4];

        let len = gauge
            .mac_read_block_after(SUBCMD_DEVICE_TYPE, &mut delay, 15, &mut data)
            .unwrap();

        assert_eq!(len, 2);
        assert_eq!(&data[..len], &[0x50, 0x40]);
        assert_eq!(delay.calls_ns, std::vec![15_000_000]);
        gauge.destroy().assert_complete();
    }

    #[test]
    fn mac_command_write_errors_are_mapped_to_driver_errors() {
        let i2c = MockI2c::new([ExpectedTransaction::failing_write(&[
            CMD_MANUFACTURER_ACCESS,
            0x01,
            0x00,
        ])]);
        let mut gauge = Bq40z50::new(i2c);

        assert!(matches!(
            gauge.start_mac_command(SUBCMD_DEVICE_TYPE),
            Err(Error::I2c(MockError::Injected))
        ));

        gauge.destroy().assert_complete();
    }
}
