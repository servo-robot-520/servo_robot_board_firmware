> ## API migration note (2026-08)
> This driver is `no_std` and uses `embedded-hal` 1.0. `defmt` support is opt-in via `defmt-03`, and `destroy()` recovers the I²C bus. PDO choice is now caller policy: use `source_pdos`, select a `PdoInfo`, then call `request_pdo`/`poll_request` or `request_pdo_blocking`.
>

# embedded-husb238a

A `#![no_std]` Rust driver for the [Hynetek HUSB238A](https://www.hynetek.com/) USB PD Sink Controller.

Based on **HUSB238A Register Information Rev0.1** (Hynetek Semiconductor, 05/2023).

## Features

- PD contract negotiation with automatic highest-voltage PDO selection
- PPS (Programmable Power Supply) support (3 PPS PDO slots)
- AVS / EPR AVS support ( Adjustable Voltage Supply, up to 48V+)
- FPDO scanning across 5V–48V (8 fixed voltage slots)
- VBUS voltage measurement via internal ADC (125mV/LSB)
- Interrupt handling with write-1-to-clear semantics
- Fault detection (over-voltage, under-voltage, thermal shutdown)
- Legacy protocol detection status (BC1.2, QC2, Divider-3, HVDCP)
- Vendor Defined Message (VDM) header access
- Internal FSM state machine readback

## Usage

### Basic (PD-only)

```rust
use embedded_hal::i2c::I2c;
use embedded_husb238a::{Husb238a, Error};

fn setup_husb238a<I2C: I2c>(i2c: I2C) -> Result<(), Error<I2C::Error>> {
    let mut husb = Husb238a::new(i2c);

    // Initialize: enable chip, configure interrupts, scan attached charger
    husb.init()?;

    if husb.charger_attached()? {
        // Driver automatically finds the best PDO (28V > 20V > PPS)
        if let Some(pdo) = husb.best_pdo() {
            defmt::info!("Best PDO: {}mV, {}mA", pdo.voltage_mv, pdo.current_ma);
            husb.request_charge()?;
        }
    }

    // Read VBUS voltage
    let vbus_mv = husb.read_vbus_mv()?;
    defmt::info!("VBUS: {}mV", vbus_mv);

    // Read current contract (includes all PD and legacy protocols)
    let proto = husb.contract_protocol();
    let volt = husb.contract_voltage_mv();
    let curr = husb.contract_current_ma();
    defmt::info!("Contract: {:?}, {}mV, {}mA", proto, volt, curr);

    Ok(())
}
```

### With PPS/AVS Support

```rust
use embedded_husb238a::{Husb238a, ProtocolConfig};

fn setup_with_pps<I2C: I2c>(i2c: I2C) -> Result<(), Error<I2C::Error>> {
    let mut husb = Husb238a::new(i2c);

    // Enable PPS + AVS in Sink_Capabilities
    let config = ProtocolConfig::with_pps_avs();
    husb.init_with_config(&config)?;

    if husb.charger_attached()? {
        if let Some(pdo) = husb.best_pdo() {
            husb.request_charge()?;
        }
    }

    Ok(())
}
```

### Custom Protocol Config

```rust
use embedded_husb238a::{ProtocolConfig, SinkPdo1Current};

let config = ProtocolConfig {
    enable_pps: true,
    enable_avs: true,
    enable_epr_avs: true,
    enable_hvdcp: true,
    enable_legacy_detection: true,
    snk_pdo1_current: SinkPdo1Current::Amps2_4,
    pd_priority: true,
    ..ProtocolConfig::default()
};
husb.init_with_config(&config)?;
```

### Interrupt-Driven Usage

```rust
use embedded_husb238a::{Husb238a, InterruptStatus};

fn on_husb238a_irq<I2C: I2c>(husb: &mut Husb238a<I2C>) {
    let status = husb.handle_interrupt().unwrap();

    // status.int, status.int1, status.int2 contain the raw flags
    // Driver handles attach/detach/fault/charge automatically
}
```

### PPS Request

```rust
// Request 12V @ 3A via PPS
husb.set_pps_request(12000, 3000)?;
husb.request_charge()?;

// Request 20V @ 5A via AVS
husb.set_avs_request(20000, 5000)?;
husb.request_charge()?;
```

### PDO Scanning

```rust
// Check which fixed voltages are available
let fpdo_mask = husb.scan_fpdo_mask()?;
// bit 0 = 5V, bit 1 = 9V, ..., bit 5 = 28V, bit 6 = 36V, bit 7 = 48V

// Check PPS availability
let pps_mask = husb.scan_pps_mask()?;
// bit 0 = PPS1, bit 1 = PPS2, bit 2 = PPS3

// Check AVS/EPR AVS
let (avs, epr_avs) = husb.scan_avs_available()?;
```

## I2C Address

| ADDR Pin | 7-bit Address | Constant |
|----------|---------------|----------|
| GND (default) | `0x42` | `ADDR_GND` |
| VDD | `0x62` | `ADDR_VDD` |

## API Reference

### Construction

| Method | Description |
|--------|-------------|
| `Husb238a::new(i2c)` | Create driver with default address (0x42) |
| `Husb238a::with_address(i2c, addr)` | Create driver with custom address |

### Initialization

| Method | Description |
|--------|-------------|
| `init()` | Full initialization with default config (PD-only, no PPS/AVS) |
| `init_with_config(config)` | Full initialization with custom protocol configuration |

### Status Queries

| Method | Returns | Description |
|--------|---------|-------------|
| `charger_attached()` | `Result<bool>` | Check if charger is connected (STATUS[0]) |
| `is_sink_attached()` | `Result<bool>` | Check Attached.SNK state (TYPE[4]) |
| `is_fault()` | `bool` | Check if a fault was detected (cached) |
| `contract_protocol()` | `ChargerProtocol` | Current PD/PPS/AVS protocol |
| `contract_voltage_mv()` | `u16` | Current contract voltage in mV |
| `contract_current_ma()` | `f32` | Current contract current in mA |
| `best_pdo()` | `Option<&PdoInfo>` | Best PDO found during scanning |
| `read_vbus_mv()` | `Result<u16>` | Read VBUS via internal ADC (125mV/LSB) |

### Detailed Status

| Method | Returns | Description |
|--------|---------|-------------|
| `read_type()` | `Result<u8>` | Raw TYPE register |
| `read_dpdm_status()` | `Result<u8>` | Legacy charger detection (BC1.2/QC2/Divider-3) |
| `read_source_cap_info()` | `Result<u8>` | SourceCap summary (USB suspend, DRP, DRD, EPR) |
| `read_pps_voltage_info()` | `Result<u8>` | PPS max voltage ranges for all 3 PPS PDOs |
| `read_avs_info()` | `Result<u8>` | AVS PDO detection and voltage range |
| `read_avs_pdp()` | `Result<u8>` | AVS power delivery capability (1W/LSB) |
| `read_epr_avs_info()` | `Result<u8>` | EPR AVS PDO detection and voltage range |
| `read_epr_avs_pdp()` | `Result<u8>` | EPR AVS power delivery capability (1W/LSB) |
| `read_vdm_header()` | `Result<u8>` | VDM message header |
| `read_fsm_state()` | `Result<(u8, u8)>` | (Sink FSM, Source FSM) 6-bit state values |

### Charge Control

| Method | Description |
|--------|-------------|
| `request_charge()` | Request voltage using the best PDO found during `init()` |
| `set_pps_request(voltage_mv, current_ma)` | Set PPS request parameters (3–23.46V, 50mA steps) |
| `set_avs_request(voltage_mv, current_ma)` | Set AVS request parameters (0–25.5V, 100mV steps) |
| `set_epr_avs_request(voltage_mv, current_ma)` | Set EPR AVS request parameters (0–51.1V, 100mV steps) |

### PDO Scanning

| Method | Returns | Description |
|--------|---------|-------------|
| `scan_fpdo_mask()` | `Result<u8>` | Bitmask of detected FPDOs (bit0=5V ... bit7=48V) |
| `scan_pps_mask()` | `Result<u8>` | Bitmask of detected PPS PDOs (bit0=PPS1 .. bit2=PPS3) |
| `scan_avs_available()` | `Result<(bool, bool)>` | (AVS available, EPR AVS available) |

### Interrupt Handling

| Method | Description |
|--------|-------------|
| `read_interrupts()` | Read and clear all interrupt flags |
| `handle_interrupt()` | Full interrupt handler: parse events, manage attach/detach/fault, auto-request charge |

## Types

### `ChargerProtocol`

```rust
pub enum ChargerProtocol {
    // PD contracts
    Unknown,      // No contract established
    TypeC5v,      // 5V Type-C (no PD negotiation)
    Pd5v,         // 5V PD contract
    Pd9v,         // 9V PD contract
    Pd12v,        // 12V PD contract
    Pd15v,        // 15V PD contract
    Pd20v,        // 20V PD contract
    Pd28v,        // 28V EPR PD contract
    Pd36v,        // 36V EPR PD contract
    Pd48v,        // 48V EPR PD contract
    Pps,          // Programmable Power Supply (PPS1/PPS2/PPS3)
    Avs,          // Adjustable Voltage Supply
    EprAvs,       // EPR Adjustable Voltage Supply
    // Legacy DPM contracts
    Default5v,    // 5V Default (BC1.2 DCP/SDP/CDP)
    Divider3,     // 5V Divider-3
    Sdp,          // 5V Standard Downstream Port
    Cdp,          // 5V Charging Downstream Port
    Dcp,          // 5V Dedicated Charging Port
    Hvdcp,        // 5V High Voltage DCP
    Qc2_9v,       // QC2 9V
    Qc2_12v,      // QC2 12V
}
```

### `PdoInfo`

```rust
pub struct PdoInfo {
    pub code: u8,               // PDO selection code for SRC_PDO register
    pub protocol: ChargerProtocol,
    pub voltage_mv: u16,        // Voltage in mV (0 for PPS/AVS)
    pub current_ma: u16,        // Max current in mA
}
```

### `ContractInfo`

```rust
pub struct ContractInfo {
    pub protocol: ChargerProtocol,
    pub voltage_mv: u16,
    pub current_ma: f32,
}
```

### `PpsMaxVoltage`

```rust
pub enum PpsMaxVoltage {
    V5_9,  // 0V–7V
    V11,   // 7.02V–12V
    V16,   // 12.02V–17V
    V21,   // >17.02V
}
```

### `Error`

```rust
pub enum Error<I2cError> {
    I2c(I2cError),      // I2C bus error
    GoTimeout,           // GO_COMMAND did not complete within 500ms
    NoSuitablePdo,       // No suitable PDO found in Source_Capabilities
    NotAttached,         // No charger connected
}
```

## PDO Selection Priority

The driver selects the best PDO in this order:

1. **28V FPDO** (EPR fixed PDO) — highest priority
2. **20V FPDO** — second priority
3. **PPS** (any of PPS1/PPS2/PPS3) with max voltage in 16V–28V range

If no suitable PDO is found, `best_pdo()` returns `None`.

## Protocol Configuration

Use `ProtocolConfig` to control which protocols are detected and advertised in Sink_Capabilities. See the Usage section above for examples.

### `ProtocolConfig` Fields

| Field | Register | Description | Default |
|-------|----------|-------------|---------|
| `enable_hvdcp` | USER_CFG1[6] | QC2/QC3 high-voltage detection | `false` |
| `enable_vbus_uv_detection` | USER_CFG1[3] | VBUS under-voltage fault interrupt | `false` |
| `enable_pps` | USER_CFG3[6] | PPS Sink Capability | `false` |
| `enable_avs` | USER_CFG3[5] | AVS Sink Capability | `false` |
| `enable_epr_avs` | USER_CFG3[3] | EPR AVS Sink Capability | `false` |
| `enable_modal_operation` | USER_CFG3[4] | ACK to SOP Discover SVIDs/Modes | `false` |
| `snk_cap_min_voltage_3v3` | USER_CFG3[2] | Min voltage in Sink_Cap PDO2 (false=5V, true=3.3V) | `false` |
| `snk_pdo1_current` | USER_CFG3[1:0] | PDO1 advertised current | `3A` |
| `enable_legacy_detection` | CONTROL1[5] | Keep D+/D- connected for BC1.2/QC | `false` |
| `pd_priority` | USER_CFG2[2] | Run PD PE immediately (no 3s delay) | `true` |

### Predefined Configs

| Constructor | Description |
|-------------|-------------|
| `ProtocolConfig::default()` / `pd_only()` | PD only, no PPS/AVS, no legacy |
| `ProtocolConfig::with_pps()` | PD + PPS |
| `ProtocolConfig::with_pps_avs()` | PD + PPS + AVS |
| `ProtocolConfig::full()` | All protocols enabled |

## Supported PDO Codes

| Code | PDO | Voltage Range |
|------|-----|---------------|
| `0x01` | 5V FPDO | 5V |
| `0x02` | 9V FPDO | 8V–10V |
| `0x03` | 12V FPDO | 11V–13V |
| `0x04` | 15V FPDO | 14V–18V |
| `0x05` | 20V FPDO | 19V–21V |
| `0x06`–`0x08` | PPS1–PPS3 | 3V–23.46V (20mV steps) |
| `0x09` | AVS | 0V–25.5V (100mV steps) |
| `0x18` | 28V FPDO (EPR) | 22V–28V |
| `0x1A` | 36V FPDO (EPR) | 29V–36V |
| `0x1C` | 48V FPDO (EPR) | 37V–48V |
| `0x1E` | EPR AVS | 0V–51.1V (100mV steps) |

## Register Documentation

See [REGISTERS.md](docs/husb238a-registers.md) for a complete register-level reference derived from the datasheet.

## Dependencies

- `embedded-hal` — I2C trait
- `defmt` — Logging (for `#[derive(Format)]` and log macros)

## License

GPL-3.0
