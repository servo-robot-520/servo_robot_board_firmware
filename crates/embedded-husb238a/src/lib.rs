#![no_std]

//! HUSB238A USB PD Sink Controller driver
//!
//! Based on HUSB238A Register Information Rev0.1
//! Implements PD/PPS/AVS/EPR protocol detection, auto-request highest voltage, fault detection.

mod config;
mod error;
mod registers;
mod types;

pub use config::*;
pub use error::*;
pub use registers::*;
pub use types::*;

#[cfg(feature = "defmt-03")]
macro_rules! log_info { ($($arg:tt)*) => { defmt::info!($($arg)*) }; }
#[cfg(not(feature = "defmt-03"))]
macro_rules! log_info {
    ($($arg:tt)*) => {};
}
use embedded_hal::i2c::I2c;

/// HUSB238A driver
pub struct Husb238a<I2C> {
    i2c: I2C,
    address: u8,
    contract: ContractInfo,
    charger_detected: bool,
    is_fault: bool,
}

impl<I2C, I2cError> Husb238a<I2C>
where
    I2C: I2c<Error = I2cError>,
{
    /// Create a new HUSB238A driver with ADDR pin connected to GND (default)
    pub fn new(i2c: I2C) -> Self {
        Self::with_address(i2c, ADDR_GND)
    }

    /// Create a new HUSB238A driver with custom address
    pub fn with_address(i2c: I2C, address: u8) -> Self {
        Self {
            i2c,
            address,
            contract: ContractInfo {
                protocol: ChargerProtocol::Unknown,
                voltage_mv: 0,
                current_ma: 0.0,
            },
            charger_detected: false,
            is_fault: false,
        }
    }

    /// Consume the driver and return the I²C bus.
    pub fn destroy(self) -> I2C {
        self.i2c
    }

    // ========================================================================
    // Low-level register access
    // ========================================================================

    /// Read a single register
    fn read_reg(&mut self, reg: u8) -> Result<u8, Error<I2cError>> {
        let mut buf = [0u8; 1];
        self.i2c
            .write_read(self.address, &[reg], &mut buf)
            .map_err(Error::I2c)?;
        Ok(buf[0])
    }

    /// Write a single register
    fn write_reg(&mut self, reg: u8, val: u8) -> Result<(), Error<I2cError>> {
        self.i2c
            .write(self.address, &[reg, val])
            .map_err(Error::I2c)
    }

    // ========================================================================
    // Interrupt handling
    // ========================================================================

    /// Clear all interrupt flags (write-1-to-clear)
    fn clear_all_interrupts(&mut self) -> Result<(), Error<I2cError>> {
        self.write_reg(REG_INTERRUPT, 0xFF)?;
        self.write_reg(REG_INTERRUPT1, 0xFF)?;
        self.write_reg(REG_INTERRUPT2, 0xFF)?;
        Ok(())
    }

    /// Read and clear interrupt status
    pub fn read_interrupts(&mut self) -> Result<InterruptStatus, Error<I2cError>> {
        let int = self.read_reg(REG_INTERRUPT)?;
        let int1 = self.read_reg(REG_INTERRUPT1)?;
        let int2 = self.read_reg(REG_INTERRUPT2)?;

        // Clear triggered interrupts
        if int != 0 {
            self.write_reg(REG_INTERRUPT, int)?;
        }
        if int1 != 0 {
            self.write_reg(REG_INTERRUPT1, int1)?;
        }
        if int2 != 0 {
            self.write_reg(REG_INTERRUPT2, int2)?;
        }

        Ok(InterruptStatus { int, int1, int2 })
    }

    // ========================================================================
    // Status queries
    // ========================================================================

    /// Check if charger is attached
    pub fn charger_attached(&mut self) -> Result<bool, Error<I2cError>> {
        let status = self.read_reg(REG_STATUS)?;
        Ok((status & STATUS_ATTACH) != 0)
    }

    /// Check if fault occurred
    pub fn is_fault(&self) -> bool {
        self.is_fault
    }

    /// Get current contract protocol
    pub fn contract_protocol(&self) -> ChargerProtocol {
        self.contract.protocol
    }

    /// Get current contract voltage in mV
    pub fn contract_voltage_mv(&self) -> u16 {
        self.contract.voltage_mv
    }

    /// Get current contract current in mA
    pub fn contract_current_ma(&self) -> f32 {
        self.contract.current_ma
    }

    /// Read VBUS voltage via internal ADC (125mV per LSB)
    pub fn read_vbus_mv(&mut self) -> Result<u16, Error<I2cError>> {
        let raw = self.read_reg(REG_VBUS_MEASUREMENT)?;
        Ok(raw as u16 * VBUS_MEAS_LSB_MV)
    }

    // ========================================================================
    // Initialization
    // ========================================================================

    /// Initialize the HUSB238A
    ///
    /// - Clear interrupt flags
    /// - Unmask global interrupts
    /// - Configure MASK/MASK1/MASK2 for key interrupts only
    /// - Set PD_PRIORITY=1
    /// - Configure USER_CFG1 OUT2_SEL to fault indication mode
    /// - Enable HUSB238A and disable legacy protocol detection
    pub fn init(&mut self) -> Result<(), Error<I2cError>> {
        // Step 0: Verify chip exists by reading STATUS
        let _status = self.read_reg(REG_STATUS)?;

        // Step 1: Clear all interrupt flags
        self.clear_all_interrupts()?;

        // Step 2: Unmask global interrupts (CONTROL[0] = 0)
        let mut ctrl = self.read_reg(REG_CONTROL)?;
        ctrl &= !CONTROL_INT_MASK;
        self.write_reg(REG_CONTROL, ctrl)?;

        // Step 3: Configure MASK - enable ATTACH, DETACH, FAULT, VBUS_OV
        // Mask: FLGIN, ORIENT, VBUS_CHG, BC_LVL
        let mask = MASK_M_FLGIN | MASK_M_ORIENT | MASK_M_VBUS_CHG | MASK_M_BC_LVL;
        self.write_reg(REG_MASK, mask)?;

        // Step 4: Configure MASK1 - enable TSD, VBUS_UV, FRC_FAIL
        // Mask: DR_ROLE, SRC_ALERT, FRC_SUCC, VDM_MSG
        let mask1 = MASK1_M_DR_ROLE | MASK1_M_SRC_ALERT | MASK1_M_FRC_SUCC | MASK1_M_VDM_MSG;
        self.write_reg(REG_MASK1, mask1)?;

        // Step 5: Configure MASK2 - enable PD_HV, Go_Fail
        // Mask: Exit_EPR, EPR_MODE
        let mask2 = MASK2_M_EXIT_EPR | MASK2_M_EPR_MODE;
        self.write_reg(REG_MASK2, mask2)?;

        // Step 6: Set PD_PRIORITY=1 (USER_CFG2[2])
        let mut cfg2 = self.read_reg(REG_USER_CFG2)?;
        cfg2 |= CFG2_PD_PRIOR;
        self.write_reg(REG_USER_CFG2, cfg2)?;

        // Step 7: Configure USER_CFG1 OUT2_SEL to fault indication mode (00b)
        let mut cfg1 = self.read_reg(REG_USER_CFG1)?;
        cfg1 &= !CFG1_OUT2_SEL_MASK;
        cfg1 &= !CFG1_EN_HVDCP; // Disable legacy HVDCP
        self.write_reg(REG_USER_CFG1, cfg1)?;

        // Step 8: Enable HUSB238A + disable legacy protocol detection
        let mut ctrl1 = self.read_reg(REG_CONTROL1)?;
        ctrl1 |= CONTROL1_ENABLE;
        ctrl1 |= CONTROL1_EN_DPM_HIZ; // Disable legacy protocol
        self.write_reg(REG_CONTROL1, ctrl1)?;

        // Step 9: Clear all interrupts again
        self.clear_all_interrupts()?;

        // Step 10: Check if charger is already connected
        if self.charger_attached()? {
            self.charger_detected = true;
            self.update_contract_info()?;
        }

        log_info!("HUSB238A: Init OK (addr=0x{:02X})", self.address);
        Ok(())
    }

    /// Initialize the HUSB238A with custom protocol configuration.
    ///
    /// Same as `init()` but allows configuring which protocols are detected
    /// and advertised in Sink_Capabilities.
    pub fn init_with_config(&mut self, config: &ProtocolConfig) -> Result<(), Error<I2cError>> {
        // Step 0: Verify chip exists by reading STATUS
        let _status = self.read_reg(REG_STATUS)?;

        // Step 1: Clear all interrupt flags
        self.clear_all_interrupts()?;

        // Step 2: Unmask global interrupts (CONTROL[0] = 0)
        let mut ctrl = self.read_reg(REG_CONTROL)?;
        ctrl &= !CONTROL_INT_MASK;
        self.write_reg(REG_CONTROL, ctrl)?;

        // Step 3: Configure MASK - enable ATTACH, DETACH, FAULT, VBUS_OV
        let mask = MASK_M_FLGIN | MASK_M_ORIENT | MASK_M_VBUS_CHG | MASK_M_BC_LVL;
        self.write_reg(REG_MASK, mask)?;

        // Step 4: Configure MASK1 - enable TSD, VBUS_UV, FRC_FAIL
        let mut mask1 = MASK1_M_DR_ROLE | MASK1_M_SRC_ALERT | MASK1_M_FRC_SUCC | MASK1_M_VDM_MSG;
        if config.enable_vbus_uv_detection {
            // VBUS_UV interrupt is enabled by USER_CFG1[3], mask is already unmasked
        } else {
            mask1 |= MASK1_M_VBUS_UV;
        }
        self.write_reg(REG_MASK1, mask1)?;

        // Step 5: Configure MASK2 - enable PD_HV, Go_Fail
        let mask2 = MASK2_M_EXIT_EPR | MASK2_M_EPR_MODE;
        self.write_reg(REG_MASK2, mask2)?;

        // Step 6: Set PD_PRIORITY (USER_CFG2[2])
        let mut cfg2 = self.read_reg(REG_USER_CFG2)?;
        if config.pd_priority {
            cfg2 |= CFG2_PD_PRIOR;
        } else {
            cfg2 &= !CFG2_PD_PRIOR;
        }
        self.write_reg(REG_USER_CFG2, cfg2)?;

        // Step 7: Configure USER_CFG1 - HVDCP and VBUS UV detection
        let mut cfg1 = self.read_reg(REG_USER_CFG1)?;
        cfg1 &= !CFG1_OUT2_SEL_MASK; // OUT2_SEL = 00 (fault indication)
        if config.enable_hvdcp {
            cfg1 |= CFG1_EN_HVDCP;
        } else {
            cfg1 &= !CFG1_EN_HVDCP;
        }
        if config.enable_vbus_uv_detection {
            cfg1 |= 1 << 3; // EN_VB_UV
        } else {
            cfg1 &= !(1 << 3);
        }
        self.write_reg(REG_USER_CFG1, cfg1)?;

        // Step 8: Configure USER_CFG3 - protocol capabilities
        let mut cfg3 = self.read_reg(REG_USER_CFG3)?;
        if config.enable_pps {
            cfg3 |= 1 << 6; // PPS_CAP_SNK
        } else {
            cfg3 &= !(1 << 6);
        }
        if config.enable_avs {
            cfg3 |= 1 << 5; // AVS_CAP_SNK
        } else {
            cfg3 &= !(1 << 5);
        }
        if config.enable_modal_operation {
            cfg3 |= 1 << 4; // MODAL_OPERATION
        } else {
            cfg3 &= !(1 << 4);
        }
        if config.enable_epr_avs {
            cfg3 |= 1 << 3; // EPR_AVS_CAP_SNK
        } else {
            cfg3 &= !(1 << 3);
        }
        if config.snk_cap_min_voltage_3v3 {
            cfg3 |= 1 << 2; // SNK_CAP_MIN_VOLTAGE
        } else {
            cfg3 &= !(1 << 2);
        }
        cfg3 = (cfg3 & !0x03) | (config.snk_pdo1_current as u8 & 0x03);
        self.write_reg(REG_USER_CFG3, cfg3)?;

        // Step 9: Enable HUSB238A + configure legacy detection
        let mut ctrl1 = self.read_reg(REG_CONTROL1)?;
        ctrl1 |= CONTROL1_ENABLE;
        if config.enable_legacy_detection {
            ctrl1 &= !CONTROL1_EN_DPM_HIZ; // Keep D+/D- connected
        } else {
            ctrl1 |= CONTROL1_EN_DPM_HIZ; // Disconnect D+/D- (PD only)
        }
        self.write_reg(REG_CONTROL1, ctrl1)?;

        // Step 10: Clear all interrupts again
        self.clear_all_interrupts()?;

        // Step 11: Check if charger is already connected
        if self.charger_attached()? {
            self.charger_detected = true;
            self.update_contract_info()?;
        }

        log_info!(
            "HUSB238A: Init OK (addr=0x{:02X}, pps={}, avs={}, epr_avs={}, hvdcp={}, legacy={})",
            self.address,
            config.enable_pps,
            config.enable_avs,
            config.enable_epr_avs,
            config.enable_hvdcp,
            config.enable_legacy_detection
        );
        Ok(())
    }

    // ========================================================================
    // Contract and source-capability access
    // ========================================================================

    /// Convert CONTRACT_STATUS1 raw current to mA.
    fn contract_current_to_ma(raw: u8) -> u32 {
        if raw <= 0x7D {
            500 + raw as u32 * 20
        } else {
            3000 + (raw as u32 - 0x7D) * 40
        }
    }

    /// Convert a source-PDO current field (100 mA/LSB) to mA.
    fn src_pdo_current_to_ma(raw: u8) -> u16 {
        (raw & SRC_PDO_CURRENT_MASK) as u16 * 100
    }

    /// Refresh the contract currently negotiated by the controller.
    pub fn update_contract_info(&mut self) -> Result<ContractInfo, Error<I2cError>> {
        let contract0 = self.read_reg(REG_CONTRACT_STATUS0)?;
        let contract1 = self.read_reg(REG_CONTRACT_STATUS1)?;
        let pd_contract = (contract0 & CONTRACT_PD_MASK) >> 4;
        let dpm_contract = contract0 & CONTRACT_DPM_MASK;

        self.contract = if pd_contract != PD_CONTRACT_TYPEC_5V {
            let current_ma = if (PD_CONTRACT_PPS1..=PD_CONTRACT_PPS3).contains(&pd_contract)
                || pd_contract == PD_CONTRACT_AVS
                || pd_contract == PD_CONTRACT_EPR_AVS
            {
                contract1 as f32 * 50.0
            } else {
                Self::contract_current_to_ma(contract1) as f32
            };
            ContractInfo {
                protocol: ChargerProtocol::from_pd_contract(pd_contract),
                voltage_mv: ChargerProtocol::pd_voltage_mv(pd_contract),
                current_ma,
            }
        } else if dpm_contract != 0 {
            ContractInfo {
                protocol: ChargerProtocol::from_dpm_contract(dpm_contract),
                voltage_mv: 5000,
                current_ma: Self::contract_current_to_ma(contract1) as f32,
            }
        } else {
            ContractInfo {
                protocol: ChargerProtocol::Unknown,
                voltage_mv: 0,
                current_ma: 0.0,
            }
        };
        Ok(self.contract)
    }

    /// Read advertised source PDOs into `out` and return the number written.
    ///
    /// The caller owns selection policy. If `out` is smaller than the source's
    /// advertised capabilities, later capabilities are omitted.
    pub fn source_pdos(&mut self, out: &mut [PdoInfo]) -> Result<usize, Error<I2cError>> {
        const FIXED: &[(u8, u8, ChargerProtocol, u16)] = &[
            (REG_SRC_PDO_5V, SELECT_PDO_5V, ChargerProtocol::Pd5v, 5000),
            (REG_SRC_PDO_9V, SELECT_PDO_9V, ChargerProtocol::Pd9v, 9000),
            (
                REG_SRC_PDO_12V,
                SELECT_PDO_12V,
                ChargerProtocol::Pd12v,
                12000,
            ),
            (
                REG_SRC_PDO_15V,
                SELECT_PDO_15V,
                ChargerProtocol::Pd15v,
                15000,
            ),
            (
                REG_SRC_PDO_20V,
                SELECT_PDO_20V,
                ChargerProtocol::Pd20v,
                20000,
            ),
            (
                REG_SRC_PDO_28V,
                SELECT_PDO_28V,
                ChargerProtocol::Pd28v,
                28000,
            ),
            (
                REG_SRC_PDO_36V,
                SELECT_PDO_36V,
                ChargerProtocol::Pd36v,
                36000,
            ),
            (
                REG_SRC_PDO_48V,
                SELECT_PDO_48V,
                ChargerProtocol::Pd48v,
                48000,
            ),
        ];
        let mut written = 0;
        for &(reg, code, protocol, voltage_mv) in FIXED {
            let raw = self.read_reg(reg)?;
            if raw & SRC_PDO_DETECT != 0 && written < out.len() {
                out[written] = PdoInfo {
                    code,
                    protocol,
                    voltage_mv,
                    current_ma: Self::src_pdo_current_to_ma(raw),
                };
                written += 1;
            }
        }

        let pps_voltage = self.read_reg(REG_SRC_PPS_VOLTAGE)?;
        const PPS: &[(u8, u8, u8)] = &[
            (REG_SRC_PDO_PPS1, SELECT_PDO_PPS1, 6),
            (REG_SRC_PDO_PPS2, SELECT_PDO_PPS2, 4),
            (REG_SRC_PDO_PPS3, SELECT_PDO_PPS3, 2),
        ];
        for &(reg, code, shift) in PPS {
            let raw = self.read_reg(reg)?;
            if raw & SRC_PDO_DETECT != 0 && written < out.len() {
                out[written] = PdoInfo {
                    code,
                    protocol: ChargerProtocol::Pps,
                    voltage_mv: PpsMaxVoltage::from_raw((pps_voltage >> shift) & 0x03).max_mv(),
                    current_ma: Self::src_pdo_current_to_ma(raw),
                };
                written += 1;
            }
        }
        Ok(written)
    }

    /// Start a request for the caller-selected PDO.
    pub fn request_pdo(&mut self, pdo: PdoInfo) -> Result<(), Error<I2cError>> {
        if !self.charger_attached()? {
            return Err(Error::NotAttached);
        }
        let src_pdo = self.read_reg(REG_SRC_PDO)?;
        self.write_reg(
            REG_SRC_PDO,
            (src_pdo & !SRC_PDO_SELECT_MASK) | ((pdo.code << 3) & SRC_PDO_SELECT_MASK),
        )?;
        self.write_reg(REG_GO_COMMAND, GO_SELECT_PDO)
    }

    /// Poll a PDO request without waiting. This is suitable for async/event-loop code.
    pub fn poll_request(&mut self) -> Result<RequestStatus, Error<I2cError>> {
        let status = self.read_reg(REG_STATUS1)?;
        if status & STATUS1_AMS_SUCC != 0 {
            self.update_contract_info()?;
            return Ok(RequestStatus::Succeeded(self.contract));
        }
        let interrupts = self.read_reg(REG_INTERRUPT)?;
        if interrupts & INT_I_GO_FAIL != 0 {
            self.write_reg(REG_INTERRUPT, interrupts)?;
            return Err(Error::GoFailed);
        }
        Ok(RequestStatus::Pending)
    }

    /// Start a PDO request and wait for it to complete.
    pub fn request_pdo_blocking<D: embedded_hal::delay::DelayNs>(
        &mut self,
        pdo: PdoInfo,
        delay: &mut D,
        timeout_ms: u32,
    ) -> Result<ContractInfo, Error<I2cError>> {
        self.request_pdo(pdo)?;
        let mut elapsed_ms = 0;
        while elapsed_ms < timeout_ms {
            match self.poll_request()? {
                RequestStatus::Pending => {
                    let step_ms = core::cmp::min(10, timeout_ms - elapsed_ms);
                    delay.delay_ms(step_ms);
                    elapsed_ms += step_ms;
                }
                RequestStatus::Succeeded(contract) => return Ok(contract),
            }
        }
        Err(Error::GoTimeout)
    }

    // ========================================================================
    // EXTI interrupt callback
    // ========================================================================

    /// Handle EXTI interrupt (call from ISR or interrupt handler)
    ///
    /// Returns the interrupt status for further processing by the caller.
    pub fn handle_interrupt(&mut self) -> Result<InterruptStatus, Error<I2cError>> {
        // Step 1: Read interrupt flags (don't clear yet)
        let int = self.read_reg(REG_INTERRUPT)?;
        let int1 = self.read_reg(REG_INTERRUPT1)?;
        let int2 = self.read_reg(REG_INTERRUPT2)?;

        log_info!(
            "HUSB238A IRQ: INT=0x{:02X}, INT1=0x{:02X}, INT2=0x{:02X}",
            int,
            int1,
            int2
        );

        // Step 2: Parse event types
        let is_attach = int1 & INT1_I_ATTACH != 0;
        let is_detach = int1 & INT1_I_DETACH != 0;
        let is_pd_hv = int & INT_I_PD_HV != 0;
        let is_fault_int = int1 & INT1_I_FAULT != 0;
        let is_vbus_ov = int1 & INT1_I_VBUS_OV != 0;
        let is_vbus_uv = int2 & INT2_I_VBUS_UV != 0;
        let is_tsd = int2 & INT2_I_TSD != 0;

        // Step 3: Read connection status
        let attached = self.charger_attached()?;

        // Step 4: Handle events
        if is_detach {
            self.charger_detected = false;
            self.contract = ContractInfo {
                protocol: ChargerProtocol::Unknown,
                voltage_mv: 0,
                current_ma: 0.0,
            };
            log_info!("HUSB238A: Charger detached");
        }

        if is_fault_int || is_vbus_ov || is_vbus_uv || is_tsd {
            self.is_fault = true;
            if int != 0 {
                self.write_reg(REG_INTERRUPT, int)?;
            }
            if int1 != 0 {
                self.write_reg(REG_INTERRUPT1, int1)?;
            }
            if int2 != 0 {
                self.write_reg(REG_INTERRUPT2, int2)?;
            }
            return Ok(InterruptStatus { int, int1, int2 });
        }
        self.is_fault = false;

        // PDO selection is application policy. The driver only refreshes the
        // contract after attachment or a high-voltage contract event.
        if is_attach || is_pd_hv {
            self.charger_detected = true;
            self.update_contract_info()?;
        }

        // If connected but no event triggered, also update contract info
        if attached && !is_attach && !is_detach && !is_pd_hv {
            self.charger_detected = true;
            self.update_contract_info()?;
        }

        // Step 5: Clear all triggered interrupt flags (write-1-to-clear)
        if int != 0 {
            self.write_reg(REG_INTERRUPT, int)?;
        }
        if int1 != 0 {
            self.write_reg(REG_INTERRUPT1, int1)?;
        }
        if int2 != 0 {
            self.write_reg(REG_INTERRUPT2, int2)?;
        }

        Ok(InterruptStatus { int, int1, int2 })
    }

    // ========================================================================
    // Detailed status queries
    // ========================================================================

    /// Read TYPE register — connection type information
    pub fn read_type(&mut self) -> Result<u8, Error<I2cError>> {
        self.read_reg(REG_TYPE)
    }

    /// Check if in Attached.SNK state
    pub fn is_sink_attached(&mut self) -> Result<bool, Error<I2cError>> {
        let type_reg = self.read_reg(REG_TYPE)?;
        Ok((type_reg & TYPE_SINK) != 0)
    }

    /// Read DPDM_STATUS — legacy charger protocol detection result
    pub fn read_dpdm_status(&mut self) -> Result<u8, Error<I2cError>> {
        self.read_reg(REG_DPDM_STATUS)
    }

    /// Read SourceCap_INFO — source capability summary
    pub fn read_source_cap_info(&mut self) -> Result<u8, Error<I2cError>> {
        self.read_reg(REG_SOURCECAP_INFO)
    }

    /// Read SRC_PPS_VOLTAGE — PPS max voltage information for all 3 PPS PDOs
    ///
    /// Returns raw register value. Decode with:
    /// - bits [7:6]: PPS1 max voltage (00=5.9V, 01=11V, 10=16V, 11=21V)
    /// - bits [5:4]: PPS2 max voltage (same encoding)
    /// - bits [3:2]: PPS3 max voltage (same encoding)
    /// - bits [1:0]: PPS min voltage (00=3V, 01=3.3V, 10=5V)
    pub fn read_pps_voltage_info(&mut self) -> Result<u8, Error<I2cError>> {
        self.read_reg(REG_SRC_PPS_VOLTAGE)
    }

    /// Read SRC_PDO_AVS — AVS PDO detection and voltage range
    pub fn read_avs_info(&mut self) -> Result<u8, Error<I2cError>> {
        self.read_reg(REG_SRC_PDO_AVS)
    }

    /// Read SRC_AVS_PDP — AVS power delivery capability
    pub fn read_avs_pdp(&mut self) -> Result<u8, Error<I2cError>> {
        self.read_reg(REG_SRC_AVS_PDP)
    }

    /// Read SRC_EPR_AVS — EPR AVS PDO detection and voltage range
    pub fn read_epr_avs_info(&mut self) -> Result<u8, Error<I2cError>> {
        self.read_reg(REG_SRC_EPR_AVS)
    }

    /// Read EPR_AVS_PDP — EPR AVS power delivery capability
    pub fn read_epr_avs_pdp(&mut self) -> Result<u8, Error<I2cError>> {
        self.read_reg(REG_EPR_AVS_PDP)
    }

    /// Read VDM_HEADER — VDM message header from connected source
    pub fn read_vdm_header(&mut self) -> Result<u8, Error<I2cError>> {
        self.read_reg(REG_VDM_HEADER)
    }

    /// Read FSM state (sink and source state machines)
    ///
    /// Returns (sink_state, source_state) — 6-bit values each
    pub fn read_fsm_state(&mut self) -> Result<(u8, u8), Error<I2cError>> {
        let sink = self.read_reg(REG_SINK_STATE)?;
        let source = self.read_reg(REG_SOURCE_STATE)?;
        Ok((sink & 0x3F, source & 0x3F))
    }

    // ========================================================================
    // PPS/AVS/EPR request parameter setters
    // ========================================================================

    /// Set PPS request voltage and current for GO_COMMAND (SRC_PDO_PPS1/2/3)
    ///
    /// voltage_mv: 3000–23460 mV (20mV steps, offset 3V)
    /// current_ma: 0–6350 mA (50mA steps)
    ///
    /// After calling this, select a discovered PDO and use `request_pdo()` or `request_pdo_blocking()`.
    pub fn set_pps_request(
        &mut self,
        voltage_mv: u16,
        current_ma: u16,
    ) -> Result<(), Error<I2cError>> {
        // Voltage: 10-bit value, 20mV/LSB, offset 3V
        let volt_raw = ((voltage_mv as u32).saturating_sub(3000) / 20) as u16;
        let volt_raw = volt_raw.min(0x3FF);

        // Current: 7-bit value, 50mA/LSB
        let curr_raw = (current_ma / 50).min(0x7F) as u8;

        // Write SNK_PPS_VOLTAGE (low 8 bits)
        self.write_reg(REG_SNK_PPS_VOLTAGE, (volt_raw & 0xFF) as u8)?;

        // Write SNK_PPS_CURRENT
        self.write_reg(REG_SNK_PPS_CURRENT, curr_raw)?;

        // Write SNK_PPS_VOL_M into SRC_PDO[1:0] (high 2 bits of voltage)
        let mut src_pdo = self.read_reg(REG_SRC_PDO)?;
        src_pdo = (src_pdo & !0x03) | ((volt_raw >> 8) & 0x03) as u8;
        self.write_reg(REG_SRC_PDO, src_pdo)?;

        Ok(())
    }

    /// Set AVS request voltage and current for GO_COMMAND (SRC_PDO_AVS)
    ///
    /// voltage_mv: 0–25500 mV (100mV steps)
    /// current_ma: 0–6350 mA (50mA steps)
    pub fn set_avs_request(
        &mut self,
        voltage_mv: u16,
        current_ma: u16,
    ) -> Result<(), Error<I2cError>> {
        // Voltage: 8-bit value, 100mV/LSB
        let volt_raw = (voltage_mv / 100).min(0xFF) as u8;

        // Current: 7-bit value, 50mA/LSB
        let curr_raw = (current_ma / 50).min(0x7F) as u8;

        // Write SNK_AVS_VOLTAGE
        self.write_reg(REG_SNK_AVS_VOLTAGE, volt_raw)?;

        // Write SNK_AVS_CURRENT (bit[7] is SNK_AVS_VOL_M, bits[6:0] are current)
        self.write_reg(REG_SNK_AVS_CURRENT, curr_raw)?;

        Ok(())
    }

    /// Set EPR AVS request voltage and current for GO_COMMAND (SRC_EPR_AVS)
    ///
    /// voltage_mv: 0–51100 mV (100mV steps)
    /// current_ma: 0–6350 mA (50mA steps)
    pub fn set_epr_avs_request(
        &mut self,
        voltage_mv: u16,
        current_ma: u16,
    ) -> Result<(), Error<I2cError>> {
        // Voltage: 9-bit value, 100mV/LSB
        let volt_raw = (voltage_mv / 100).min(0x1FF);

        // Current: 7-bit value, 50mA/LSB
        let curr_raw = (current_ma / 50).min(0x7F) as u8;

        // Write EPR_AVS_VOLTAGE (low 8 bits)
        self.write_reg(REG_EPR_AVS_VOLTAGE, (volt_raw & 0xFF) as u8)?;

        // Write EPR_AVS_CURRENT (bit[7] is EPR_AVS_VOL_M, bits[6:0] are current)
        self.write_reg(
            REG_EPR_AVS_CURRENT,
            ((volt_raw >> 8) & 0x01) as u8 | curr_raw,
        )?;

        Ok(())
    }

    // ========================================================================
    // Convenience: scan all available PDOs
    // ========================================================================

    /// Scan all FPDO voltage slots and return a bitmask of detected voltages.
    ///
    /// Bit 0 = 5V, bit 1 = 9V, ..., bit 5 = 28V, bit 6 = 36V, bit 7 = 48V
    pub fn scan_fpdo_mask(&mut self) -> Result<u8, Error<I2cError>> {
        let mut mask: u8 = 0;
        let regs = [
            (REG_SRC_PDO_5V, 0x01),
            (REG_SRC_PDO_9V, 0x02),
            (REG_SRC_PDO_12V, 0x04),
            (REG_SRC_PDO_15V, 0x08),
            (REG_SRC_PDO_20V, 0x10),
            (REG_SRC_PDO_28V, 0x20),
            (REG_SRC_PDO_36V, 0x40),
            (REG_SRC_PDO_48V, 0x80),
        ];
        for (reg, bit) in regs {
            if let Ok(val) = self.read_reg(reg)
                && val & SRC_PDO_DETECT != 0
            {
                mask |= bit;
            }
        }
        Ok(mask)
    }

    /// Scan all PPS PDOs and return a bitmask of detected PPS slots.
    ///
    /// Bit 0 = PPS1, bit 1 = PPS2, bit 2 = PPS3
    pub fn scan_pps_mask(&mut self) -> Result<u8, Error<I2cError>> {
        let mut mask: u8 = 0;
        let regs = [
            (REG_SRC_PDO_PPS1, 0x01),
            (REG_SRC_PDO_PPS2, 0x02),
            (REG_SRC_PDO_PPS3, 0x04),
        ];
        for (reg, bit) in regs {
            if let Ok(val) = self.read_reg(reg)
                && val & SRC_PDO_DETECT != 0
            {
                mask |= bit;
            }
        }
        Ok(mask)
    }

    /// Check if AVS or EPR AVS PDO is available
    pub fn scan_avs_available(&mut self) -> Result<(bool, bool), Error<I2cError>> {
        let avs = self
            .read_reg(REG_SRC_PDO_AVS)
            .map(|v| v & SRC_PDO_DETECT != 0)
            .unwrap_or(false);
        let epr_avs = self
            .read_reg(REG_SRC_EPR_AVS)
            .map(|v| v & (1 << 7) != 0)
            .unwrap_or(false);
        Ok((avs, epr_avs))
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
        address: u8,
        write: Vec<u8>,
        read: Vec<u8>,
        result: Result<(), MockError>,
    }

    impl ExpectedTransaction {
        fn write(bytes: &[u8]) -> Self {
            Self {
                address: ADDR_GND,
                write: bytes.to_vec(),
                read: Vec::new(),
                result: Ok(()),
            }
        }

        fn write_read(write: &[u8], read: &[u8]) -> Self {
            Self {
                address: ADDR_GND,
                write: write.to_vec(),
                read: read.to_vec(),
                result: Ok(()),
            }
        }

        fn failing_write(bytes: &[u8]) -> Self {
            Self {
                address: ADDR_GND,
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
            let expected = self
                .expected
                .pop_front()
                .expect("unexpected I2C transaction");
            assert_eq!(address, expected.address);

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

    fn pdo_9v() -> PdoInfo {
        PdoInfo {
            code: SELECT_PDO_9V,
            protocol: ChargerProtocol::Pd9v,
            voltage_mv: 9000,
            current_ma: 3000,
        }
    }

    #[test]
    fn request_pdo_checks_attachment_and_preserves_unrelated_selection_bits() {
        let i2c = MockI2c::new([
            ExpectedTransaction::write_read(&[REG_STATUS], &[STATUS_ATTACH]),
            ExpectedTransaction::write_read(&[REG_SRC_PDO], &[0xA7]),
            ExpectedTransaction::write(&[REG_SRC_PDO, 0x17]),
            ExpectedTransaction::write(&[REG_GO_COMMAND, GO_SELECT_PDO]),
        ]);
        let mut controller = Husb238a::new(i2c);

        controller.request_pdo(pdo_9v()).unwrap();

        controller.destroy().assert_complete();
    }

    #[test]
    fn blocking_request_polls_after_a_real_delay_until_contract_succeeds() {
        let i2c = MockI2c::new([
            ExpectedTransaction::write_read(&[REG_STATUS], &[STATUS_ATTACH]),
            ExpectedTransaction::write_read(&[REG_SRC_PDO], &[0]),
            ExpectedTransaction::write(&[REG_SRC_PDO, SELECT_PDO_9V << 3]),
            ExpectedTransaction::write(&[REG_GO_COMMAND, GO_SELECT_PDO]),
            ExpectedTransaction::write_read(&[REG_STATUS1], &[0]),
            ExpectedTransaction::write_read(&[REG_INTERRUPT], &[0]),
            ExpectedTransaction::write_read(&[REG_STATUS1], &[STATUS1_AMS_SUCC]),
            ExpectedTransaction::write_read(&[REG_CONTRACT_STATUS0], &[PD_CONTRACT_9V << 4]),
            ExpectedTransaction::write_read(&[REG_CONTRACT_STATUS1], &[0]),
        ]);
        let mut controller = Husb238a::new(i2c);
        let mut delay = RecordingDelay::default();

        let contract = controller
            .request_pdo_blocking(pdo_9v(), &mut delay, 25)
            .unwrap();

        assert_eq!(contract.protocol, ChargerProtocol::Pd9v);
        assert_eq!(contract.voltage_mv, 9000);
        assert_eq!(contract.current_ma, 500.0);
        assert_eq!(delay.calls_ns, std::vec![10_000_000]);
        controller.destroy().assert_complete();
    }

    #[test]
    fn poll_request_acknowledges_go_failure_before_returning_the_error() {
        let i2c = MockI2c::new([
            ExpectedTransaction::write_read(&[REG_STATUS1], &[0]),
            ExpectedTransaction::write_read(&[REG_INTERRUPT], &[INT_I_GO_FAIL | 0x01]),
            ExpectedTransaction::write(&[REG_INTERRUPT, INT_I_GO_FAIL | 0x01]),
        ]);
        let mut controller = Husb238a::new(i2c);

        assert!(matches!(controller.poll_request(), Err(Error::GoFailed)));

        controller.destroy().assert_complete();
    }

    #[test]
    fn interrupt_acknowledgement_failures_are_propagated() {
        let i2c = MockI2c::new([
            ExpectedTransaction::write_read(&[REG_INTERRUPT], &[0x80]),
            ExpectedTransaction::write_read(&[REG_INTERRUPT1], &[0]),
            ExpectedTransaction::write_read(&[REG_INTERRUPT2], &[0]),
            ExpectedTransaction::failing_write(&[REG_INTERRUPT, 0x80]),
        ]);
        let mut controller = Husb238a::new(i2c);

        assert!(matches!(
            controller.read_interrupts(),
            Err(Error::I2c(MockError::Injected))
        ));

        controller.destroy().assert_complete();
    }
}
