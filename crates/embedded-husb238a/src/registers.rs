//! HUSB238A register addresses and bit definitions.
//!
//! Based on HUSB238A Register Information Rev0.1 (Hynetek Semiconductor, 05/2023).

// ============================================================================
// I2C Addresses (7-bit)
// ============================================================================

/// ADDR pin connected to VDD (900k pull-up)
pub const ADDR_VDD: u8 = 0x62;
/// ADDR pin connected to GND (900k pull-down), this is the default
pub const ADDR_GND: u8 = 0x42;

// ============================================================================
// Register Addresses
// ============================================================================

// User configuration registers
pub const REG_CONTROL: u8 = 0x01;
pub const REG_CONTROL1: u8 = 0x02;
pub const REG_MANUAL: u8 = 0x03;
pub const REG_RESET: u8 = 0x04;
pub const REG_MASK: u8 = 0x05;
pub const REG_MASK1: u8 = 0x06;
pub const REG_MASK2: u8 = 0x07;

// Interrupt registers (write-1-to-clear)
pub const REG_INTERRUPT: u8 = 0x09;
pub const REG_INTERRUPT1: u8 = 0x0A;
pub const REG_INTERRUPT2: u8 = 0x0B;

// User configuration registers
pub const REG_USER_CFG0: u8 = 0x0C;
pub const REG_USER_CFG1: u8 = 0x0D;
pub const REG_USER_CFG2: u8 = 0x0E;
pub const REG_USER_CFG3: u8 = 0x0F;

// PDO selection and GO command
pub const REG_GO_COMMAND: u8 = 0x18;
pub const REG_SRC_PDO: u8 = 0x19;

// PPS/AVS/EPR AVS request parameters
pub const REG_SNK_PPS_VOLTAGE: u8 = 0x1A;
pub const REG_SNK_PPS_CURRENT: u8 = 0x1B;
pub const REG_SNK_AVS_VOLTAGE: u8 = 0x1C;
pub const REG_SNK_AVS_CURRENT: u8 = 0x1D;
pub const REG_EPR_AVS_VOLTAGE: u8 = 0x1E;
pub const REG_EPR_AVS_CURRENT: u8 = 0x20;

// PDP registers
pub const REG_SNK_PDP: u8 = 0x21;
pub const REG_EPR_PDP: u8 = 0x22;

// Status registers (read-only)
pub const REG_STATUS: u8 = 0x63;
pub const REG_STATUS1: u8 = 0x64;
pub const REG_TYPE: u8 = 0x65;
pub const REG_DPDM_STATUS: u8 = 0x66;
pub const REG_CONTRACT_STATUS0: u8 = 0x67;
pub const REG_CONTRACT_STATUS1: u8 = 0x68;

// SourceCap_INFO
pub const REG_SOURCECAP_INFO: u8 = 0x69;

// SRC PDO detection registers (read-only)
pub const REG_SRC_PDO_5V: u8 = 0x6A;
pub const REG_SRC_PDO_9V: u8 = 0x6B;
pub const REG_SRC_PDO_12V: u8 = 0x6C;
pub const REG_SRC_PDO_15V: u8 = 0x6D;
pub const REG_SRC_PDO_20V: u8 = 0x6E;
pub const REG_SRC_PDO_28V: u8 = 0x6F;
pub const REG_SRC_PDO_36V: u8 = 0x70;
pub const REG_SRC_PDO_48V: u8 = 0x71;

// SRC PPS detection registers
pub const REG_SRC_PDO_PPS1: u8 = 0x72;
pub const REG_SRC_PDO_PPS2: u8 = 0x73;
pub const REG_SRC_PDO_PPS3: u8 = 0x74;
pub const REG_SRC_PPS_VOLTAGE: u8 = 0x75;
pub const REG_SRC_PDO_AVS: u8 = 0x76;
pub const REG_SRC_AVS_PDP: u8 = 0x77;
pub const REG_EPR_AVS_PDP: u8 = 0x78;
pub const REG_SRC_EPR_AVS: u8 = 0x79;

// VDM registers (read-only)
pub const REG_VDM_HEADER: u8 = 0x7A;

// VBUS measurement
pub const REG_VBUS_MEASUREMENT: u8 = 0x87;

// FSM state registers (read-only)
pub const REG_SINK_STATE: u8 = 0x90;
pub const REG_SOURCE_STATE: u8 = 0x91;

// ============================================================================
// Bit Definitions
// ============================================================================

// CONTROL (0x01)
pub const CONTROL_INT_MASK: u8 = 1 << 0;

// CONTROL1 (0x02)
pub const CONTROL1_ENABLE: u8 = 1 << 3;
pub const CONTROL1_EN_DPM_HIZ: u8 = 1 << 5;

// MASK (0x05)
pub const MASK_M_FLGIN: u8 = 1 << 7;
pub const MASK_M_ORIENT: u8 = 1 << 6;
pub const MASK_M_FAULT: u8 = 1 << 5;
pub const MASK_M_VBUS_CHG: u8 = 1 << 4;
pub const MASK_M_VBUS_OV: u8 = 1 << 3;
pub const MASK_M_BC_LVL: u8 = 1 << 2;
pub const MASK_M_DETACH: u8 = 1 << 1;
pub const MASK_M_ATTACH: u8 = 1 << 0;

// MASK1 (0x06)
pub const MASK1_M_TSD: u8 = 1 << 7;
pub const MASK1_M_VBUS_UV: u8 = 1 << 6;
pub const MASK1_M_DR_ROLE: u8 = 1 << 5;
pub const MASK1_M_SRC_ALERT: u8 = 1 << 3;
pub const MASK1_M_FRC_FAIL: u8 = 1 << 2;
pub const MASK1_M_FRC_SUCC: u8 = 1 << 1;
pub const MASK1_M_VDM_MSG: u8 = 1 << 0;

// MASK2 (0x07)
pub const MASK2_M_EXIT_EPR: u8 = 1 << 3;
pub const MASK2_M_GO_FAIL: u8 = 1 << 2;
pub const MASK2_M_EPR_MODE: u8 = 1 << 1;
pub const MASK2_M_PD_HV: u8 = 1 << 0;

// INTERRUPT (0x09)
pub const INT_I_EXIT_EPR: u8 = 1 << 3;
pub const INT_I_GO_FAIL: u8 = 1 << 2;
pub const INT_I_EPR_MODE: u8 = 1 << 1;
pub const INT_I_PD_HV: u8 = 1 << 0;

// INTERRUPT1 (0x0A)
pub const INT1_I_FLGIN: u8 = 1 << 7;
pub const INT1_I_ORIENT: u8 = 1 << 6;
pub const INT1_I_FAULT: u8 = 1 << 5;
pub const INT1_I_VBUS_CHG: u8 = 1 << 4;
pub const INT1_I_VBUS_OV: u8 = 1 << 3;
pub const INT1_I_BC_LVL: u8 = 1 << 2;
pub const INT1_I_DETACH: u8 = 1 << 1;
pub const INT1_I_ATTACH: u8 = 1 << 0;

// INTERRUPT2 (0x0B)
pub const INT2_I_TSD: u8 = 1 << 7;
pub const INT2_I_VBUS_UV: u8 = 1 << 6;
pub const INT2_I_DR_ROLE: u8 = 1 << 5;
pub const INT2_I_SRC_ALERT: u8 = 1 << 3;
pub const INT2_I_FRC_FAIL: u8 = 1 << 2;
pub const INT2_I_FRC_SUCC: u8 = 1 << 1;
pub const INT2_I_VDM_MSG: u8 = 1 << 0;

// USER_CFG1 (0x0D)
pub const CFG1_EN_HVDCP: u8 = 1 << 6;
pub const CFG1_OUT2_SEL_MASK: u8 = 0x03;

// USER_CFG2 (0x0E)
pub const CFG2_PD_PRIOR: u8 = 1 << 2;

// GO_COMMAND (0x18)
pub const GO_COMMAND_MASK: u8 = 0x1F;
pub const GO_SELECT_PDO: u8 = 0x01;
pub const GO_SOFT_RESET: u8 = 0x1D;
pub const GO_HARD_RESET: u8 = 0x1E;

// SRC_PDO (0x19)
pub const SRC_PDO_SELECT_MASK: u8 = 0x1F << 3;

// PDO selection codes (write to [7:3])
pub const SELECT_PDO_NONE: u8 = 0x00;
pub const SELECT_PDO_5V: u8 = 0x01;
pub const SELECT_PDO_9V: u8 = 0x02;
pub const SELECT_PDO_12V: u8 = 0x03;
pub const SELECT_PDO_15V: u8 = 0x04;
pub const SELECT_PDO_20V: u8 = 0x05;
pub const SELECT_PDO_PPS1: u8 = 0x06;
pub const SELECT_PDO_PPS2: u8 = 0x07;
pub const SELECT_PDO_PPS3: u8 = 0x08;
pub const SELECT_PDO_AVS: u8 = 0x09;
pub const SELECT_PDO_28V: u8 = 0x18;
pub const SELECT_PDO_36V: u8 = 0x1A;
pub const SELECT_PDO_48V: u8 = 0x1C;
pub const SELECT_EPR_AVS: u8 = 0x1E;

// TYPE (0x65)
pub const TYPE_SINK: u8 = 1 << 4;
pub const TYPE_DEBUGSNK: u8 = 1 << 5;
pub const TYPE_CC_RX_ACTIVE: u8 = 1 << 7;

// DPDM_STATUS (0x66)
pub const DPDM_DIVIDER3_FLAG: u8 = 1 << 0;
pub const DPDM_SDP_FLAG: u8 = 1 << 1;
pub const DPDM_CDP_FLAG: u8 = 1 << 2;
pub const DPDM_STATUS_MASK: u8 = 0x1F << 3;

// STATUS (0x63)
pub const STATUS_AMS_PROCESS: u8 = 1 << 7;
pub const STATUS_PD_EPR_SNK: u8 = 1 << 6;
pub const STATUS_TSD: u8 = 1 << 3;
pub const STATUS_BC_LVL_MASK: u8 = 0x03 << 1;
pub const STATUS_ATTACH: u8 = 1 << 0;

// STATUS1 (0x64)
pub const STATUS1_FLGIN: u8 = 1 << 7;
pub const STATUS1_PD_HV: u8 = 1 << 5;
pub const STATUS1_PD_COMM: u8 = 1 << 4;
pub const STATUS1_SRC_ALERT: u8 = 1 << 3;
pub const STATUS1_AMS_SUCC: u8 = 1 << 2;
pub const STATUS1_FAULT: u8 = 1 << 1;
pub const STATUS1_DATA_ROLE: u8 = 1 << 0;

// CONTRACT_STATUS0 (0x67)
pub const CONTRACT_PD_MASK: u8 = 0x0F << 4;
pub const CONTRACT_DPM_MASK: u8 = 0x0F;

// PD contract codes
pub const PD_CONTRACT_TYPEC_5V: u8 = 0x00;
pub const PD_CONTRACT_5V: u8 = 0x01;
pub const PD_CONTRACT_9V: u8 = 0x02;
pub const PD_CONTRACT_12V: u8 = 0x03;
pub const PD_CONTRACT_15V: u8 = 0x04;
pub const PD_CONTRACT_20V: u8 = 0x05;
pub const PD_CONTRACT_PPS1: u8 = 0x06;
pub const PD_CONTRACT_PPS2: u8 = 0x07;
pub const PD_CONTRACT_PPS3: u8 = 0x08;
pub const PD_CONTRACT_AVS: u8 = 0x09;
pub const PD_CONTRACT_28V: u8 = 0x0A;
pub const PD_CONTRACT_36V: u8 = 0x0B;
pub const PD_CONTRACT_48V: u8 = 0x0C;
pub const PD_CONTRACT_EPR_AVS: u8 = 0x0D;

// SRC_PDO register format
pub const SRC_PDO_DETECT: u8 = 1 << 7;
pub const SRC_PDO_CURRENT_MASK: u8 = 0x7F;

// VBUS measurement: 125mV per LSB
pub const VBUS_MEAS_LSB_MV: u16 = 125;

// GO_COMMAND timeout
pub const GO_COMMAND_TIMEOUT_MS: u32 = 500;
pub const SETTLE_TIME_MS: u32 = 50;
