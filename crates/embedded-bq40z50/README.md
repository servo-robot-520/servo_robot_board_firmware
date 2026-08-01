> ## API migration note (2026-08)
> This driver is `no_std` and uses `embedded-hal` 1.0. `defmt` support is opt-in via `defmt-03`, and `destroy()` recovers the I²C bus. SMBus names and MAC data now use caller-provided buffers and validated count bytes.
>

# embedded-bq40z50

A `no_std` Rust driver for the Texas Instruments [BQ40Z50-R1](https://www.ti.com/product/BQ40Z50-R1) Smart Battery Gauge, communicating over SMBus/I2C.

## Features

- Standard SBS (Smart Battery Specification) command support
- Manufacturer Access (MAC) sub-command interface
- Cell voltage monitoring (4 cells)
- Detailed temperature readings (DAStatus2: internal, TS1–TS4, cell, FET)
- Safety, operation, charging, and gauging status flag parsing
- Device information queries (firmware, hardware, chemistry, SOH)
- `embedded-hal` 1.x I2C trait based — works with any HAL
- `defmt` formatting support for structured logging

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
embedded-bq40z50 = { path = "../embedded-bq40z50" }
```

### Basic example

```rust
use embedded_bq40z50::Bq40z50;

fn read_battery<I2C, E>(i2c: I2C)
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    let mut gauge = Bq40z50::new(i2c);

    if !gauge.is_connected() {
        defmt::warn!("BQ40Z50 not found on I2C bus");
        return;
    }

    let voltage = gauge.voltage_mv().unwrap_or(0);
    let current = gauge.current_ma().unwrap_or(0);
    let soc = gauge.relative_soc().unwrap_or(0);
    let temp = gauge.temperature_c().unwrap_or(250);

    defmt::info!(
        "Battery: {}mV, {}mA, SOC={}%, temp={}°C",
        voltage, current, soc, temp
    );
}
```

### Reading safety status

```rust
let safety = gauge.safety_status().unwrap();
if safety.has_any() {
    defmt::error!("Safety fault! flags={:#x}", safety.0);
    if safety.cov() { defmt::error!("  Cell overvoltage"); }
    if safety.cuv() { defmt::error!("  Cell undervoltage"); }
    if safety.otd() { defmt::error!("  Overtemperature during discharge"); }
}
```

### Checking operation status

```rust
let op = gauge.operation_status().unwrap();
defmt::info!("Security mode: {:?}", op.security_mode());
defmt::info!("CHG FET: {}, DSG FET: {}", op.charge_fet(), op.discharge_fet());
```

### Reading all cell voltages

```rust
let cells = gauge.cell_voltages_mv().unwrap();
defmt::info!(
    "Cell voltages: {}mV, {}mV, {}mV, {}mV",
    cells[0], cells[1], cells[2], cells[3]
);
```

## API Reference

### Construction

| Method | Description |
|--------|-------------|
| `Bq40z50::new(i2c)` | Create driver with default address `0x0B` |
| `Bq40z50::with_address(i2c, addr)` | Create driver with custom 7-bit I2C address |

### Standard SBS Commands

| Method | Return | Description |
|--------|--------|-------------|
| `is_connected()` | `bool` | Probe device by reading voltage |
| `voltage_mv()` | `u16` | Total pack voltage (mV) |
| `current_ma()` | `i16` | Coulomb counter current (mA, +discharge / −charge) |
| `average_current_ma()` | `i16` | Average current (mA) |
| `temperature_c()` | `i16` | Temperature (°C, value/10 for decimal) |
| `relative_soc()` | `u8` | Relative state of charge (%) |
| `absolute_soc()` | `u8` | Absolute state of charge (%) |
| `remaining_capacity_mah()` | `u16` | Remaining capacity (mAh) |
| `full_charge_capacity_mah()` | `u16` | Full charge capacity (mAh) |
| `design_capacity_mah()` | `u16` | Design capacity (mAh) |
| `design_voltage_mv()` | `u16` | Design voltage (mV) |
| `runtime_to_empty_min()` | `u16` | Time to empty at current rate (min) |
| `avg_time_to_empty_min()` | `u16` | Average time to empty (min) |
| `avg_time_to_full_min()` | `u16` | Average time to full (min) |
| `charging_current_ma()` | `u16` | Recommended charging current (mA) |
| `charging_voltage_mv()` | `u16` | Recommended charging voltage (mV) |
| `max_error()` | `u8` | SOC calculation max error (%) |
| `cycle_count()` | `u16` | Discharge cycle count |
| `serial_number()` | `u16` | Battery pack serial number |
| `manufacturer_date()` | `u16` | Manufacture date (encoded) |
| `battery_status()` | `u16` | Raw battery status flags |
| `battery_mode()` | `u16` | Battery mode flags |

### Cell Voltages

| Method | Return | Description |
|--------|--------|-------------|
| `cell_voltage_1_mv()` – `cell_voltage_4_mv()` | `u16` | Individual cell voltage (mV) |
| `cell_voltages_mv()` | `[u16; 4]` | All 4 cell voltages at once |

### AtRate

| Method | Return | Description |
|--------|--------|-------------|
| `set_at_rate(value)` | `()` | Set AtRate value (write) |
| `at_rate_time_to_full_min()` | `u16` | Time to full at AtRate (min) |
| `at_rate_time_to_empty_min()` | `u16` | Time to empty at AtRate (min) |

### Device Information (MAC)

| Method | Return | Description |
|--------|--------|-------------|
| `device_type()` | `u16` | IC part number |
| `firmware_version()` | `[u8; 4]` | Firmware version bytes |
| `hardware_version()` | `u16` | Hardware version |
| `chem_id()` | `u16` | Chemical ID (OCV table) |
| `device_chemistry_u16()` | `u16` | Chemistry packed ASCII (`"LO"`=LiOn, `"LP"`=LiPo) |
| `device_name_bytes()` | `[u8; 8]` | Device name (`"bq40z50"`) |
| `manufacturer_name_bytes()` | `[u8; 11]` | Manufacturer name |
| `state_of_health()` | `StateOfHealth` | SOH FCC (mAh) + energy (cWh) |

### Safety / Status (MAC Block Reads)

| Method | Return | Description |
|--------|--------|-------------|
| `safety_alert()` | `SafetyFlags` | Latched safety alarm flags |
| `safety_status()` | `SafetyFlags` | Active safety status flags |
| `operation_status()` | `OperationStatus` | Device operation status |
| `charging_status()` | `ChargingStatus` | Charging status flags |
| `gauging_status()` | `GaugingStatus` | Gauging status flags |

### Temperature Detail (MAC)

| Method | Return | Description |
|--------|--------|-------------|
| `da_status_2()` | `TempDetail` | Internal, TS1–TS4, cell, FET temperatures (°C) |

### Control (MAC)

| Method | Return | Description |
|--------|--------|-------------|
| `device_reset()` | `()` | Reset the gauge |
| `toggle_gauging()` | `()` | Toggle gauging enable/disable |
| `toggle_fet_control()` | `()` | Toggle firmware FET control |

## Status Flag Types

### `SafetyFlags`

Wrapper around `u32` with named accessor methods:

`utd()`, `utc()`, `pchgc()`, `chgv()`, `chgc()`, `oc()`, `cto()`, `pto()`, `otf()`, `cuvc()`, `otd()`, `otc()`, `ascd()`, `ascc()`, `aold()`, `ocd2()`, `ocd1()`, `occ2()`, `occ1()`, `cov()`, `cuv()`, `has_any()`

### `OperationStatus`

Wrapper around `u32` with named accessor methods:

`emergency_shutdown()`, `cell_balancing()`, `initializing()`, `sleep_mode()`, `charging_disabled()`, `discharging_disabled()`, `permanent_failure()`, `safety_mode()`, `fuse_active()`, `precharge_fet()`, `charge_fet()`, `discharge_fet()`, `system_present()`, `security_mode()` → `SecurityMode` enum

### `ChargingStatus`

Wrapper around `u32` with named accessor methods:

`charge_terminated()`, `maintenance_charge()`, `charge_inhibit()`, `high_voltage_region()`, `mid_voltage_region()`, `low_voltage_region()`, `precharge_region()`, `overtemp_region()`, `high_temp_region()`, `recommended_temp_region()`, `low_temp_region()`, `under_temp_region()`

### `GaugingStatus`

Wrapper around `u32` with named accessor methods:

`it_enabled()`, `vok()`, `resistance_updates_disabled()`, `ocv_reading_taken()`, `condition_flag()`, `discharging()`, `edv_reached()`, `cell_balancing_possible()`, `terminate_charge()`, `terminate_discharge()`, `fully_charged()`, `fully_discharged()`, `discharge_qualified()`, `constant_power_load()`

## Constants

All SBS command codes, MAC sub-command codes, and flag bitmasks are exported as public constants. See [docs/bq40z50-registers.md](../../docs/bq40z50-registers.md) for the full register map.

## Register Documentation

See [docs/bq40z50-registers.md](../../docs/bq40z50-registers.md) for a complete register reference based on the BQ40Z50-R1 datasheet (SLUUA43A).

## Dependencies

- `embedded-hal` — I2C trait
- `defmt` — structured logging

## License

MIT OR Apache-2.0
