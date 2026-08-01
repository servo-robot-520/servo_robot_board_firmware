#![no_std]

//! BQ24725 SMBus Battery Charge Controller driver
//!
//! Based on BQ24725 datasheet (TI SLUS702A).
//! Implements charge option configuration, charge current/voltage setting,
//! and input current limit with proper unit conversion.
//!
//! ## Register Summary
//!
//! | Address | Name             | R/W | POR     | Description               |
//! |---------|------------------|-----|---------|---------------------------|
//! | 0x12    | ChargeOption()   | R/W | 0x7904  | Charger options control    |
//! | 0x14    | ChargeCurrent()  | R/W | 0x0000  | 7-bit charge current       |
//! | 0x15    | ChargeVoltage()  | R/W | 0x0000  | 11-bit charge voltage      |
//! | 0x3F    | InputCurrent()   | R/W | 0x1000  | 6-bit input current        |
//! | 0xFE    | ManufacturerID() | R   | 0x0040  | Manufacturer ID            |
//! | 0xFF    | DeviceID()       | R   | 0x0008  | Device ID                  |

mod types;

pub use types::*;

use embedded_hal::i2c::I2c;

// ============================================================================
// Driver
// ============================================================================

/// BQ24725 driver
pub struct Bq24725<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C, I2cError> Bq24725<I2C>
where
    I2C: I2c<Error = I2cError>,
{
    /// Create a new BQ24725 driver with default address (0x09)
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            address: BQ24725_ADDR,
        }
    }

    /// Create a new BQ24725 driver with custom address
    pub fn with_address(i2c: I2C, address: u8) -> Self {
        Self { i2c, address }
    }

    /// Consume the driver and return the I²C bus.
    pub fn destroy(self) -> I2C {
        self.i2c
    }

    // ========================================================================
    // Low-level SMBus word access (16-bit, little-endian)
    // ========================================================================

    /// Read a 16-bit register (SMBus word, little-endian)
    fn read_word(&mut self, reg: u8) -> Result<u16, Error<I2cError>> {
        let mut buf = [0u8; 2];
        self.i2c
            .write_read(self.address, &[reg], &mut buf)
            .map_err(Error::I2c)?;
        Ok(u16::from_le_bytes(buf))
    }

    /// Write a 16-bit register (SMBus word, little-endian)
    fn write_word(&mut self, reg: u8, data: u16) -> Result<(), Error<I2cError>> {
        let bytes = data.to_le_bytes();
        self.i2c
            .write(self.address, &[reg, bytes[0], bytes[1]])
            .map_err(Error::I2c)
    }

    // ========================================================================
    // Identification
    // ========================================================================

    /// Get device ID (always returns 0x0008)
    pub fn device_id(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(REG_DEVICE_ID)
    }

    /// Get manufacture ID (always returns 0x0040)
    pub fn manufacture_id(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(REG_MANUFACTURE_ID)
    }

    /// Verify this is a BQ24725 by checking ManufacturerID and DeviceID
    pub fn verify_id(&mut self) -> Result<bool, Error<I2cError>> {
        let mfg = self.manufacture_id()?;
        let dev = self.device_id()?;
        Ok(mfg == 0x0040 && dev == 0x0008)
    }

    // ========================================================================
    // Charge Options
    // ========================================================================

    /// Get charge option register raw value
    pub fn charge_option_raw(&mut self) -> Result<u16, Error<I2cError>> {
        self.read_word(REG_CHARGE_OPTION)
    }

    /// Get charge option as structured data
    pub fn charge_option(&mut self) -> Result<ChargeOptions, Error<I2cError>> {
        let raw = self.read_word(REG_CHARGE_OPTION)?;
        Ok(ChargeOptions::from_u16(raw))
    }

    /// Set charge option from structured data
    pub fn set_charge_option(&mut self, opts: &ChargeOptions) -> Result<(), Error<I2cError>> {
        self.write_word(REG_CHARGE_OPTION, opts.to_u16())
    }

    // ========================================================================
    // Charge Current (0x14, 7-bit, 64mA/LSB, range 128–8128mA)
    // ========================================================================

    /// Get charge current in mA. A register value of zero disables charging.
    pub fn charge_current_ma(&mut self) -> Result<u16, Error<I2cError>> {
        Ok(decode_charge_current_ma(
            self.read_word(REG_CHARGE_CURRENT)?,
        ))
    }

    /// Set charge current in mA.
    ///
    /// `0` disables charging. Nonzero values must be in the documented
    /// 128–8128 mA range and be a multiple of 64 mA.
    pub fn set_charge_current_ma(&mut self, ma: u16) -> Result<(), Error<I2cError>> {
        let reg_val = encode_charge_current_ma(ma).map_err(|_| Error::ChargeCurrentOutOfRange)?;
        self.write_word(REG_CHARGE_CURRENT, reg_val)
    }

    // ========================================================================
    // Charge Voltage (0x15, 11-bit, 16mV/LSB, range 1024–19200mV)
    // ========================================================================

    /// Get charge voltage in mV. A register value of zero disables charging.
    pub fn charge_voltage_mv(&mut self) -> Result<u16, Error<I2cError>> {
        Ok(decode_charge_voltage_mv(
            self.read_word(REG_CHARGE_VOLTAGE)?,
        ))
    }

    /// Set charge voltage in mV.
    ///
    /// `0` disables voltage regulation. Nonzero values must be in the
    /// documented 1024–19200 mV range and be a multiple of 16 mV.
    pub fn set_charge_voltage_mv(&mut self, mv: u16) -> Result<(), Error<I2cError>> {
        let reg_val = encode_charge_voltage_mv(mv).map_err(|_| Error::ChargeVoltageOutOfRange)?;
        self.write_word(REG_CHARGE_VOLTAGE, reg_val)
    }

    // ========================================================================
    // Input Current (0x3F, 6-bit, 128mA/LSB, range 128–8064mA)
    // ========================================================================

    /// Get the input-current limit in mA. A register value of zero disables it.
    pub fn input_current_ma(&mut self) -> Result<u16, Error<I2cError>> {
        Ok(decode_input_current_ma(self.read_word(REG_INPUT_CURRENT)?))
    }

    /// Set the input-current limit in mA.
    ///
    /// `0` disables the limit. Nonzero values must be in the documented
    /// 128–8064 mA range and be a multiple of 128 mA.
    pub fn set_input_current_ma(&mut self, ma: u16) -> Result<(), Error<I2cError>> {
        let reg_val = encode_input_current_ma(ma).map_err(|_| Error::InputCurrentOutOfRange)?;
        self.write_word(REG_INPUT_CURRENT, reg_val)
    }

    // ========================================================================
    // Convenience: Charging control
    // ========================================================================

    /// Query if charging is enabled (charge_inhibit == ChargeEnable).
    ///
    /// Requires an I2C read, so takes `&mut self`.
    pub fn query_charging_enabled(&mut self) -> Result<bool, Error<I2cError>> {
        let opts = self.charge_option()?;
        Ok(opts.charge_inhibit == ChargeInhibit::ChargeEnable)
    }

    /// Enable or disable charging via ChargeOption() bit[0]
    pub fn set_charging_enabled(&mut self, enabled: bool) -> Result<(), Error<I2cError>> {
        let mut opts = self.charge_option()?;
        opts.charge_inhibit = if enabled {
            ChargeInhibit::ChargeEnable
        } else {
            ChargeInhibit::ChargeInhibit
        };
        self.set_charge_option(&opts)
    }

    // ========================================================================
    // Convenience: Watchdog
    // ========================================================================

    /// Refresh the watchdog timer by writing the current charge current value.
    ///
    /// Must be called within the watchdog timeout period (44s/88s/175s)
    /// if the watchdog is enabled, otherwise charging is suspended.
    pub fn refresh_watchdog(&mut self) -> Result<(), Error<I2cError>> {
        let raw = self.read_word(REG_CHARGE_CURRENT)?;
        self.write_word(REG_CHARGE_CURRENT, raw)
    }

    /// Set watchdog timer mode
    pub fn set_watchdog(&mut self, timer: WatchdogTimer) -> Result<(), Error<I2cError>> {
        let mut opts = self.charge_option()?;
        opts.watchdog_timer = timer;
        self.set_charge_option(&opts)
    }

    // ========================================================================
    // Convenience: IOUT pin
    // ========================================================================

    /// Select what the IOUT pin monitors
    pub fn set_iout_selection(&mut self, iout: Iout) -> Result<(), Error<I2cError>> {
        let mut opts = self.charge_option()?;
        opts.iout = iout;
        self.set_charge_option(&opts)
    }

    // ========================================================================
    // Convenience: LEARN cycle
    // ========================================================================

    /// Start a battery LEARN cycle
    ///
    /// During LEARN, the IC turns off ACFET and turns on BATFET to
    /// discharge the battery. The cycle completes automatically when
    /// battery voltage hits the depletion threshold.
    pub fn start_learn_cycle(&mut self) -> Result<(), Error<I2cError>> {
        let mut opts = self.charge_option()?;
        opts.learn_en = LearnEn::Enabled;
        self.set_charge_option(&opts)
    }

    /// Query if LEARN cycle is active.
    ///
    /// Requires an I2C read, so takes `&mut self`.
    pub fn query_learn_active(&mut self) -> Result<bool, Error<I2cError>> {
        let opts = self.charge_option()?;
        Ok(opts.learn_en == LearnEn::Enabled)
    }

    // ========================================================================
    // Convenience: Full configuration
    // ========================================================================

    /// Apply a complete charge configuration in one I2C transaction sequence.
    ///
    /// Sets charge voltage, charge current, input current, and charge options.
    /// This is the recommended way to configure the charger at startup.
    pub fn configure(
        &mut self,
        charge_voltage_mv: u16,
        charge_current_ma: u16,
        input_current_ma: u16,
        options: &ChargeOptions,
    ) -> Result<(), Error<I2cError>> {
        self.set_charge_option(options)?;
        self.set_charge_voltage_mv(charge_voltage_mv)?;
        self.set_charge_current_ma(charge_current_ma)?;
        self.set_input_current_ma(input_current_ma)?;
        Ok(())
    }

    // ========================================================================
    // Debug: Read all registers
    // ========================================================================

    /// Read all registers and return raw values for debugging.
    ///
    /// Returns `(charge_option, charge_current, charge_voltage, input_current,
    /// manufacturer_id, device_id)`.
    #[allow(clippy::type_complexity)]
    pub fn read_all_raw(&mut self) -> Result<(u16, u16, u16, u16, u16, u16), Error<I2cError>> {
        let opts = self.read_word(REG_CHARGE_OPTION)?;
        let cc = self.read_word(REG_CHARGE_CURRENT)?;
        let cv = self.read_word(REG_CHARGE_VOLTAGE)?;
        let ic = self.read_word(REG_INPUT_CURRENT)?;
        let mfg = self.read_word(REG_MANUFACTURE_ID)?;
        let dev = self.read_word(REG_DEVICE_ID)?;
        Ok((opts, cc, cv, ic, mfg, dev))
    }
}
