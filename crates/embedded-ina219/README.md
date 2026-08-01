> ## API migration note (2026-08)
> This driver is `no_std` and uses `embedded-hal` 1.0. `defmt` support is opt-in via `defmt-03`, and `destroy()` recovers the I²C bus. Replace float `calibrate(shunt, current)` calls with `calibrate(Calibration::new(shunt_micro_ohms, max_current_microamps)?)`.
>

# embedded-ina219

A `#![no_std]` Rust driver for the [TI INA219](http://www.ti.com/lit/ds/symlink/ina219.pdf) bidirectional current/power monitor with I²C interface.

Based on INA219 datasheet SBOS448G (August 2008 — Revised December 2015).

## Features

- Full register access: Configuration, Shunt Voltage, Bus Voltage, Current, Power, Calibration
- Type-safe configuration via enums (`BusVoltageRange`, `PgaGain`, `AdcResolution`, `OperatingMode`)
- Auto-calibration from shunt resistor value and max expected current
- Hardware-accelerated current and power readings (via INA219 internal multiplier)
- Software current/power calculation as fallback (no calibration required)
- Bus voltage status flags (conversion ready, overflow)
- Software reset support
- Compatible with `embedded-hal` 1.x `I2c` trait

## Quick Start

```rust
use embedded_ina219::{Ina219, BusVoltageRange, PgaGain, AdcResolution, OperatingMode};

// Create driver with default address (0x40)
let mut ina = Ina219::new(i2c);
ina.set_shunt_resistor(2.0); // 2mΩ shunt

// Configure: 32V bus range, PGA ×1 (±40mV), 12-bit, continuous mode
ina.configure(
    BusVoltageRange::Range32V,
    PgaGain::Gain1,
    AdcResolution::Bits12,
    AdcResolution::Bits12,
    OperatingMode::ShuntAndBusContinuous,
)?;

// Calibrate for 2mΩ shunt, 15A max current
ina.calibrate(2.0, 15.0)?;

// Read measurements
let v_bus = ina.read_bus_voltage()?;       // V
let v_shunt = ina.read_shunt_voltage_uv()?; // µV
let i_ma = ina.read_current_ma()?;          // mA (software calculated)
let p_mw = ina.read_power_ma()?;            // mW (software calculated)
```

## Calibration

The INA219 requires writing a calibration value to the Calibration Register (0x05) before the hardware Current (0x04) and Power (0x03) registers produce valid data. Without calibration, these registers read zero.

### Using `calibrate()` (recommended)

```rust
// Arguments: shunt resistance (mΩ), max expected current (A)
ina.calibrate(2.0, 15.0)?;
```

This internally computes:

```text
Current_LSB = Max_Expected_Current / 2^15
Cal = trunc(0.04096 / (Current_LSB × R_SHUNT))
```

### Using `calculate_calibration()` helper

```rust
let (cal, current_lsb, power_lsb) = embedded_ina219::calculate_calibration(2.0, 15.0);
ina.write_calibration(cal)?;
```

This is useful when you need the `current_lsb` and `power_lsb` values for hardware reading functions.

### Hardware vs Software Current/Power Reading

| Method | Requires Calibration | Function |
|--------|---------------------|----------|
| Software | No | `read_current_ma()`, `read_power_ma()` |
| Hardware | Yes | `read_current_hardware(lsb)`, `read_power_hardware(lsb)` |

Software reading computes from the shunt voltage register and resistor value. Hardware reading uses the INA219's internal multiplier for potentially better accuracy.

## Configuration Reference

### Bus Voltage Range (`BusVoltageRange`)

| Variant | Range | BRNG Bit |
|---------|-------|----------|
| `Range16V` | 0–16V | 0 |
| `Range32V` | 0–32V | 1 (default) |

### PGA Gain (`PgaGain`)

| Variant | Gain | Shunt Voltage Range |
|---------|------|-------------------|
| `Gain1` | ×1 | ±40 mV |
| `Gain2` | /2 | ±80 mV |
| `Gain4` | /4 | ±160 mV |
| `Gain8` | /8 | ±320 mV (default) |

### ADC Resolution (`AdcResolution`)

| Variant | Resolution | Samples | Conversion Time |
|---------|-----------|---------|----------------|
| `Bits9` | 9-bit | 1 | 84 µs |
| `Bits10` | 10-bit | 1 | 148 µs |
| `Bits11` | 11-bit | 1 | 276 µs |
| `Bits12` | 12-bit | 1 | 532 µs (default) |
| `Samples2` | 12-bit | 2 | 1.06 ms |
| `Samples4` | 12-bit | 4 | 2.13 ms |
| `Samples8` | 12-bit | 8 | 4.26 ms |
| `Samples16` | 12-bit | 16 | 8.51 ms |
| `Samples32` | 12-bit | 32 | 17.02 ms |
| `Samples64` | 12-bit | 64 | 34.05 ms |
| `Samples128` | 12-bit | 128 | 68.10 ms |

### Operating Mode (`OperatingMode`)

| Variant | Mode |
|---------|------|
| `PowerDown` | Shutdown, minimal current |
| `ShuntVoltageTriggered` | Single shunt conversion on trigger |
| `BusVoltageTriggered` | Single bus conversion on trigger |
| `ShuntAndBusTriggered` | Single both conversion on trigger |
| `AdcOff` | ADC disabled |
| `ShuntVoltageContinuous` | Continuous shunt measurement |
| `BusVoltageContinuous` | Continuous bus measurement |
| `ShuntAndBusContinuous` | Continuous both (default) |

## API Reference

### Construction

| Method | Description |
|--------|-------------|
| `Ina219::new(i2c)` | Create with default address (0x40) and 2mΩ shunt |
| `Ina219::with_address(i2c, addr)` | Create with custom I²C address |
| `set_shunt_resistor(mohm)` | Set shunt resistance for software calculations |

### Configuration

| Method | Description |
|--------|-------------|
| `configure(bus_range, pga, bus_adc, shunt_adc, mode)` | Write full configuration |
| `read_config()` | Read current configuration register |
| `reset()` | Software reset (same as power-on reset) |

### Calibration

| Method | Description |
|--------|-------------|
| `calibrate(shunt_mohm, max_current_a)` | Auto-calculate and write calibration |
| `write_calibration(cal)` | Write raw calibration value |

### Measurement

| Method | Returns | Description |
|--------|---------|-------------|
| `read_bus_voltage()` | `f32` (V) | Bus voltage, 4mV LSB |
| `read_bus_voltage_raw()` | `u16` | Raw register value |
| `read_bus_voltage_status()` | `BusVoltageStatus` | CNVR and OVF flags |
| `read_shunt_voltage_uv()` | `f32` (µV) | Shunt voltage, 10µV LSB |
| `read_shunt_voltage_mv()` | `f32` (mV) | Shunt voltage in mV |
| `read_shunt_voltage_raw()` | `i16` | Raw register value |
| `read_current_ma()` | `f32` (mA) | Current via software calculation |
| `read_current_hardware(lsb)` | `f32` (mA) | Current via hardware register |
| `read_current_raw()` | `i16` | Raw register value |
| `read_power_ma()` | `f32` (mW) | Power via software calculation |
| `read_power_hardware(lsb)` | `f32` (mW) | Power via hardware register |
| `read_power_raw()` | `u16` | Raw register value |
| `read_all()` | `PowerMeasurement` | All readings in one call |

### Standalone Functions

| Function | Description |
|----------|-------------|
| `calculate_calibration(shunt_mohm, max_current_a)` | Returns `(cal, current_lsb, power_lsb)` |

## I²C Address

The INA219 supports 16 addresses via A0 and A1 pins:

| A1 | A0 | Address |
|----|-----|---------|
| GND | GND | 0x40 (default) |
| GND | Vs+ | 0x41 |
| GND | SDA | 0x44 |
| GND | SCL | 0x45 |
| Vs+ | GND | 0x48 |
| Vs+ | Vs+ | 0x49 |
| Vs+ | SDA | 0x4C |
| Vs+ | SCL | 0x4D |
| SDA | GND | 0x50 |
| SDA | Vs+ | 0x51 |
| SDA | SDA | 0x54 |
| SDA | SCL | 0x55 |
| SCL | GND | 0x58 |
| SCL | Vs+ | 0x59 |
| SCL | SDA | 0x5C |
| SCL | SCL | 0x5D |

## Dependencies

- `embedded-hal` 1.x (I²C trait)
- `defmt` (formatting for `defmt::Format` derive)

## License

MIT OR Apache-2.0
