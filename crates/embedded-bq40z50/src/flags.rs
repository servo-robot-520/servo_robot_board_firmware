//! BQ40Z50 status flag bit constants.

// ============================================================================
// BatteryStatus (0x16) flag bits
// ============================================================================

/// Bit 15: OCA — Overcharged Alarm
pub const STATUS_OVER_CHARGED_ALARM: u16 = 1 << 15;
/// Deprecated: use `STATUS_OVER_CHARGED_ALARM` instead
pub const STATUS_OVER_CHARGE_ALARM: u16 = STATUS_OVER_CHARGED_ALARM;
/// Bit 14: TCA — Terminate Charge Alarm
pub const STATUS_TERM_CHARGE_ALARM: u16 = 1 << 14;
/// Bit 12: OTA — Overtemperature Alarm
pub const STATUS_OVER_TEMP_ALARM: u16 = 1 << 12;
/// Bit 11: TDA — Terminate Discharge Alarm
pub const STATUS_TERM_DISCHARGE_ALARM: u16 = 1 << 11;
/// Bit 9: RCA — Remaining Capacity Alarm
pub const STATUS_REMAINING_CAP_ALARM: u16 = 1 << 9;
/// Bit 8: RTA — Remaining Time Alarm
pub const STATUS_REMAINING_TIME_ALARM: u16 = 1 << 8;
/// Bit 7: INIT — Gauge initialization complete
pub const STATUS_INITIALIZATION: u16 = 1 << 7;
/// Bit 6: DSG — Discharging or Relax (1) / Charging (0)
pub const STATUS_DISCHARGING: u16 = 1 << 6;
/// Bit 5: FC — Fully Charged
pub const STATUS_FULLY_CHARGED: u16 = 1 << 5;
/// Bit 4: FD — Fully Discharged
pub const STATUS_FULLY_DISCHARGED: u16 = 1 << 4;

/// Error code mask (Bits 3:0)
pub const STATUS_ERROR_CODE_MASK: u16 = 0x000F;
/// Error code: OK
pub const ERROR_OK: u16 = 0x0;
/// Error code: Busy
pub const ERROR_BUSY: u16 = 0x1;
/// Error code: Reserved Command
pub const ERROR_RESERVED_CMD: u16 = 0x2;
/// Error code: Unsupported Command
pub const ERROR_UNSUPPORTED_CMD: u16 = 0x3;
/// Error code: Access Denied
pub const ERROR_ACCESS_DENIED: u16 = 0x4;
/// Error code: Overflow/Underflow
pub const ERROR_OVERFLOW: u16 = 0x5;
/// Error code: Bad Size
pub const ERROR_BAD_SIZE: u16 = 0x6;
/// Error code: Unknown Error
pub const ERROR_UNKNOWN: u16 = 0x7;

// ============================================================================
// SafetyAlert / SafetyStatus (0x0050 / 0x0051) flag bits (32-bit)
// ============================================================================

/// Bit 27: UTD — Undervoltage During Discharge
pub const SAFETY_UTD: u32 = 1 << 27;
/// Bit 26: UTC — Undervoltage During Charge
pub const SAFETY_UTC: u32 = 1 << 26;
/// Bit 25: PCHGC — Over-Precharge Current
pub const SAFETY_PCHGC: u32 = 1 << 25;
/// Bit 24: CHGV — Overcharging Voltage
pub const SAFETY_CHGV: u32 = 1 << 24;
/// Bit 23: CHGC — Overcharging Current
pub const SAFETY_CHGC: u32 = 1 << 23;
/// Bit 22: OC — Overcharge
pub const SAFETY_OC: u32 = 1 << 22;
/// Bit 20: CTO — Charge Timeout
pub const SAFETY_CTO: u32 = 1 << 20;
/// Bit 18: PTO — Precharge Timeout
pub const SAFETY_PTO: u32 = 1 << 18;
/// Bit 16: OTF — Overtemperature FET
pub const SAFETY_OTF: u32 = 1 << 16;
/// Bit 14: CUVC — Cell Undervoltage Compensated
pub const SAFETY_CUVC: u32 = 1 << 14;
/// Bit 13: OTD — Overtemperature During Discharge
pub const SAFETY_OTD: u32 = 1 << 13;
/// Bit 12: OTC — Overtemperature During Charge
pub const SAFETY_OTC: u32 = 1 << 12;
/// Bit 11: ASCDL — Short-Circuit During Discharge Latch
pub const SAFETY_ASCDL: u32 = 1 << 11;
/// Bit 10: ASCD — Short-Circuit During Discharge
pub const SAFETY_ASCD: u32 = 1 << 10;
/// Bit 9: ASCCL — Short-Circuit During Charge Latch
pub const SAFETY_ASCCL: u32 = 1 << 9;
/// Bit 8: ASCC — Short-Circuit During Charge
pub const SAFETY_ASCC: u32 = 1 << 8;
/// Bit 7: AOLDL — Overload During Discharge Latch
pub const SAFETY_AOLDL: u32 = 1 << 7;
/// Bit 6: AOLD — Overload During Discharge
pub const SAFETY_AOLD: u32 = 1 << 6;
/// Bit 5: OCD2 — Overcurrent During Discharge 2
pub const SAFETY_OCD2: u32 = 1 << 5;
/// Bit 4: OCD1 — Overcurrent During Discharge 1
pub const SAFETY_OCD1: u32 = 1 << 4;
/// Bit 3: OCC2 — Overcurrent During Charge 2
pub const SAFETY_OCC2: u32 = 1 << 3;
/// Bit 2: OCC1 — Overcurrent During Charge 1
pub const SAFETY_OCC1: u32 = 1 << 2;
/// Bit 1: COV — Cell Overvoltage
pub const SAFETY_COV: u32 = 1 << 1;
/// Bit 0: CUV — Cell Undervoltage
pub const SAFETY_CUV: u32 = 1 << 0;

// ============================================================================
// OperationStatus (0x0054) flag bits (32-bit)
// ============================================================================

/// Bit 29: EMSHUT — Emergency Shutdown
pub const OP_STATUS_EMSHUT: u32 = 1 << 29;
/// Bit 28: CB — Cell Balancing active
pub const OP_STATUS_CB: u32 = 1 << 28;
/// Bit 24: INIT — Initialization after full reset
pub const OP_STATUS_INIT: u32 = 1 << 24;
/// Bit 23: SLEEPM — SLEEP mode triggered via command
pub const OP_STATUS_SLEEPM: u32 = 1 << 23;
/// Bit 22: XL — 400-kHz SMBus mode
pub const OP_STATUS_XL: u32 = 1 << 22;
/// Bit 18: AUTH — Authentication in progress
pub const OP_STATUS_AUTH: u32 = 1 << 18;
/// Bit 17: LED — LED Display on
pub const OP_STATUS_LED: u32 = 1 << 17;
/// Bit 16: SDM — Shutdown triggered via command
pub const OP_STATUS_SDM: u32 = 1 << 16;
/// Bit 15: SLEEP — SLEEP mode conditions met
pub const OP_STATUS_SLEEP: u32 = 1 << 15;
/// Bit 14: XCHG — Charging disabled
pub const OP_STATUS_XCHG: u32 = 1 << 14;
/// Bit 13: XDSG — Discharging disabled
pub const OP_STATUS_XDSG: u32 = 1 << 13;
/// Bit 12: PF — PERMANENT FAILURE mode status
pub const OP_STATUS_PF: u32 = 1 << 12;
/// Bit 11: SS — SAFETY mode status
pub const OP_STATUS_SS: u32 = 1 << 11;
/// Bit 10: SDV — Shutdown triggered via low pack voltage
pub const OP_STATUS_SDV: u32 = 1 << 10;
/// Bits 9:8: SEC1,SEC0 — Security mode (11=Sealed, 10=Unsealed, 01=Full Access)
pub const OP_STATUS_SEC_MASK: u32 = 0x0300;
/// Bit 5: FUSE — Fuse status
pub const OP_STATUS_FUSE: u32 = 1 << 5;
/// Bit 3: PCHG — Precharge FET status
pub const OP_STATUS_PCHG: u32 = 1 << 3;
/// Bit 2: CHG — CHG FET status
pub const OP_STATUS_CHG: u32 = 1 << 2;
/// Bit 1: DSG — DSG FET status
pub const OP_STATUS_DSG: u32 = 1 << 1;
/// Bit 0: PRES — System present low
pub const OP_STATUS_PRES: u32 = 1 << 0;

// ============================================================================
// ChargingStatus (0x0055) flag bits (32-bit)
// ============================================================================

/// Bit 17: CCC — Charging Loss Compensation
pub const CHG_STATUS_CCC: u32 = 1 << 17;
/// Bit 16: CVR — Charging Voltage Rate of Change
pub const CHG_STATUS_CVR: u32 = 1 << 16;
/// Bit 15: CCR — Charging Current Rate of Change
pub const CHG_STATUS_CCR: u32 = 1 << 15;
/// Bit 14: VCT — Charge Termination
pub const CHG_STATUS_VCT: u32 = 1 << 14;
/// Bit 13: MCHG — Maintenance Charge
pub const CHG_STATUS_MCHG: u32 = 1 << 13;
/// Bit 12: IN — Charge Inhibit
pub const CHG_STATUS_IN: u32 = 1 << 12;
/// Bit 11: HV — High Voltage Region
pub const CHG_STATUS_HV: u32 = 1 << 11;
/// Bit 10: MV — Mid Voltage Region
pub const CHG_STATUS_MV: u32 = 1 << 10;
/// Bit 9: LV — Low Voltage Region
pub const CHG_STATUS_LV: u32 = 1 << 9;
/// Bit 8: PV — Precharge Voltage Region
pub const CHG_STATUS_PV: u32 = 1 << 8;
/// Bit 6: OT — Overtemperature Region
pub const CHG_STATUS_OT: u32 = 1 << 6;
/// Bit 5: HT — High Temperature Region
pub const CHG_STATUS_HT: u32 = 1 << 5;
/// Bit 4: STH — Standard Temperature High Region
pub const CHG_STATUS_STH: u32 = 1 << 4;
/// Bit 3: RT — Recommended Temperature Region
pub const CHG_STATUS_RT: u32 = 1 << 3;
/// Bit 2: STL — Standard Temperature Low Region
pub const CHG_STATUS_STL: u32 = 1 << 2;
/// Bit 1: LT — Low Temperature Region
pub const CHG_STATUS_LT: u32 = 1 << 1;
/// Bit 0: UT — Undertemperature Region
pub const CHG_STATUS_UT: u32 = 1 << 0;

// ============================================================================
// GaugingStatus (0x0056) flag bits (32-bit)
// ============================================================================

/// Bit 20: OCVFR — Open Circuit Voltage in Flat Region
pub const GAUGE_OCVFR: u32 = 1 << 20;
/// Bit 19: LDMD — LOAD mode (1=Constant Power, 0=Constant Current)
pub const GAUGE_LDMD: u32 = 1 << 19;
/// Bit 18: RX — Resistance Update toggle
pub const GAUGE_RX: u32 = 1 << 18;
/// Bit 17: QMax — QMax Update toggle
pub const GAUGE_QMAX: u32 = 1 << 17;
/// Bit 16: VDQ — Discharge Qualified for Learning
pub const GAUGE_VDQ: u32 = 1 << 16;
/// Bit 15: NSFM — Negative Scale Factor Mode
pub const GAUGE_NSFM: u32 = 1 << 15;
/// Bit 12: QEN — Impedance Track Gauging enabled
pub const GAUGE_QEN: u32 = 1 << 12;
/// Bit 11: VOK — Voltages OK for QMax update
pub const GAUGE_VOK: u32 = 1 << 11;
/// Bit 10: R_DIS — Resistance Updates disabled
pub const GAUGE_R_DIS: u32 = 1 << 10;
/// Bit 8: REST — OCV Reading Taken
pub const GAUGE_REST: u32 = 1 << 8;
/// Bit 7: CF — Condition Flag (MaxError > limit)
pub const GAUGE_CF: u32 = 1 << 7;
/// Bit 6: DSG — Discharge/Relax
pub const GAUGE_DSG: u32 = 1 << 6;
/// Bit 5: EDV — End-of-Discharge Termination Voltage
pub const GAUGE_EDV: u32 = 1 << 5;
/// Bit 4: BAL_EN — Cell Balancing possible
pub const GAUGE_BAL_EN: u32 = 1 << 4;
/// Bit 3: TC — Terminate Charge
pub const GAUGE_TC: u32 = 1 << 3;
/// Bit 2: TD — Terminate Discharge
pub const GAUGE_TD: u32 = 1 << 2;
/// Bit 1: FC — Fully Charged
pub const GAUGE_FC: u32 = 1 << 1;
/// Bit 0: FD — Fully Discharged
pub const GAUGE_FD: u32 = 1 << 0;
