//! BQ40Z50 command addresses and MAC sub-command codes.

// ============================================================================
// I2C Address (7-bit)
// ============================================================================

/// BQ40Z50 7-bit I2C address
pub const BQ40Z50_ADDR: u8 = 0x0B;

// ============================================================================
// SMBus Standard SBS Command Codes
// ============================================================================

/// Low capacity alarm threshold (R/W, mAh or 10mWh)
pub const CMD_REMAINING_CAPACITY_ALARM: u8 = 0x01;
/// Low remaining time alarm threshold (R/W, min)
pub const CMD_REMAINING_TIME_ALARM: u8 = 0x02;
/// Battery mode options (R/W)
pub const CMD_BATTERY_MODE: u8 = 0x03;
/// AtRate value for time-to-full/empty calculation (R/W, mA or 10mW)
pub const CMD_AT_RATE: u8 = 0x04;
/// Time to full at AtRate (R, min, 65535=not charging)
pub const CMD_AT_RATE_TIME_TO_FULL: u8 = 0x05;
/// Time to empty at AtRate (R, min, 65535=not discharging)
pub const CMD_AT_RATE_TIME_TO_EMPTY: u8 = 0x06;
/// Battery can deliver AtRate for 10s (R, bool)
pub const CMD_AT_RATE_OK: u8 = 0x07;
/// Temperature (0.1K units)
pub const CMD_TEMPERATURE: u8 = 0x08;
/// Total voltage of all cells (mV)
pub const CMD_VOLTAGE: u8 = 0x09;
/// Coulomb counter current (mA, signed)
pub const CMD_CURRENT: u8 = 0x0A;
/// Average current (mA, signed)
pub const CMD_AVERAGE_CURRENT: u8 = 0x0B;
/// Max error in SOC calculation (%)
pub const CMD_MAX_ERROR: u8 = 0x0C;
/// Relative SOC (%)
pub const CMD_RELATIVE_SOC: u8 = 0x0D;
/// Absolute SOC (%)
pub const CMD_ABSOLUTE_SOC: u8 = 0x0E;
/// Remaining capacity (mAh or 10mWh)
pub const CMD_REMAINING_CAPACITY: u8 = 0x0F;
/// Full charge capacity (mAh or 10mWh)
pub const CMD_FULL_CHARGE_CAPACITY: u8 = 0x10;
/// Runtime to empty at current rate (min)
pub const CMD_RUNTIME_TO_EMPTY: u8 = 0x11;
/// Average time to empty (min)
pub const CMD_AVG_TIME_TO_EMPTY: u8 = 0x12;
/// Average time to full (min)
pub const CMD_AVG_TIME_TO_FULL: u8 = 0x13;
/// Charging current recommendation (mA)
pub const CMD_CHARGING_CURRENT: u8 = 0x14;
/// Charging voltage recommendation (mV)
pub const CMD_CHARGING_VOLTAGE: u8 = 0x15;
/// Battery status flags
pub const CMD_BATTERY_STATUS: u8 = 0x16;
/// Cycle count
pub const CMD_CYCLE_COUNT: u8 = 0x17;
/// Design capacity (mAh or 10mWh)
pub const CMD_DESIGN_CAPACITY: u8 = 0x18;
/// Design voltage (mV)
pub const CMD_DESIGN_VOLTAGE: u8 = 0x19;
/// Specification info (SBS version)
pub const CMD_SPECIFICATION_INFO: u8 = 0x1A;
/// Manufacturer date (Day + Month*32 + (Year-1980)*256)
pub const CMD_MANUFACTURER_DATE: u8 = 0x1B;
/// Battery pack serial number
pub const CMD_SERIAL: u8 = 0x1C;
/// Manufacturer name (Block read)
pub const CMD_MANUFACTURER_NAME: u8 = 0x20;
/// Device name (Block read)
pub const CMD_DEVICE_NAME: u8 = 0x21;
/// Device chemistry string (Block read)
pub const CMD_DEVICE_CHEMISTRY: u8 = 0x22;
/// Manufacturer data / MAC response (Block read)
pub const CMD_MANUFACTURER_DATA: u8 = 0x23;
/// Cell 4 voltage (mV)
pub const CMD_CELL_VOLTAGE_4: u8 = 0x3C;
/// Cell 3 voltage (mV)
pub const CMD_CELL_VOLTAGE_3: u8 = 0x3D;
/// Cell 2 voltage (mV)
pub const CMD_CELL_VOLTAGE_2: u8 = 0x3E;
/// Cell 1 voltage (mV)
pub const CMD_CELL_VOLTAGE_1: u8 = 0x3F;

// ============================================================================
// Manufacturer Access
// ============================================================================

/// ManufacturerAccess — write MAC sub-command here
pub const CMD_MANUFACTURER_ACCESS: u8 = 0x00;
/// ManufacturerBlockAccess — read block data here
pub const CMD_MANUFACTURER_BLOCK_ACCESS: u8 = 0x44;

// MAC sub-commands (write to CMD_MANUFACTURER_ACCESS, read from CMD_MANUFACTURER_BLOCK_ACCESS)

// --- Information query ---
/// DeviceType (IC part number)
pub const SUBCMD_DEVICE_TYPE: u16 = 0x0001;
/// FirmwareVersion
pub const SUBCMD_FIRMWARE_VERSION: u16 = 0x0002;
/// HardwareVersion
pub const SUBCMD_HARDWARE_VERSION: u16 = 0x0003;
/// ChemicalID (OCV table ID)
pub const SUBCMD_CHEM_ID: u16 = 0x0006;

// --- Safety / Status (Block read, 32-bit) ---
/// SafetyAlert — latched safety alarm flags
pub const SUBCMD_SAFETY_ALERT: u16 = 0x0050;
/// SafetyStatus — active safety status flags
pub const SUBCMD_SAFETY_STATUS: u16 = 0x0051;
/// PFAlert — permanent failure alarm flags
pub const SUBCMD_PF_ALERT: u16 = 0x0052;
/// PFStatus — permanent failure status flags
pub const SUBCMD_PF_STATUS: u16 = 0x0053;
/// OperationStatus — device operation status flags
pub const SUBCMD_OPERATION_STATUS: u16 = 0x0054;
/// ChargingStatus — charging status flags
pub const SUBCMD_CHARGING_STATUS: u16 = 0x0055;
/// GaugingStatus — gauging status flags
pub const SUBCMD_GAUGING_STATUS: u16 = 0x0056;
/// ManufacturingStatus — manufacturing test status
pub const SUBCMD_MANUFACTURING_STATUS: u16 = 0x0057;

// --- Data block reads ---
/// DAStatus1 — cell voltages, pack voltage, currents, powers
pub const SUBCMD_DA_STATUS1: u16 = 0x0071;
/// DAStatus2 — internal temp, TS1~TS4, cell temp, FET temp (14 bytes)
pub const SUBCMD_DA_STATUS2: u16 = 0x0072;
/// GaugeStatus1 — IT gauging detail (True Rem Q/E, FCC, RaScale, CompRes)
pub const SUBCMD_GAUGE_STATUS1: u16 = 0x0073;
/// GaugeStatus2 — grid points, DOD, state time
pub const SUBCMD_GAUGE_STATUS2: u16 = 0x0074;
/// GaugeStatus3 — QMax values, DOD0, thermal model
pub const SUBCMD_GAUGE_STATUS3: u16 = 0x0075;
/// CBStatus — cell balancing time
pub const SUBCMD_CB_STATUS: u16 = 0x0076;
/// StateOfHealth — SOH FCC (mAh) + energy (cWh)
pub const SUBCMD_STATE_OF_HEALTH: u16 = 0x0077;

// --- Control commands ---
/// DeviceReset — reset the device
pub const SUBCMD_DEVICE_RESET: u16 = 0x0041;
/// ShutdownMode — enter SHIP mode
pub const SUBCMD_SHUTDOWN_MODE: u16 = 0x0010;
/// SleepMode — enter SLEEP mode
pub const SUBCMD_SLEEP_MODE: u16 = 0x0011;
/// Gauging — enable/disable gauging
pub const SUBCMD_GAUGING: u16 = 0x0021;
/// FETControl — enable/disable firmware FET control
pub const SUBCMD_FET_CONTROL: u16 = 0x0022;
