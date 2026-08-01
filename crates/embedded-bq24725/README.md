> ## API migration note (2026-08)
> These drivers are `no_std` and use `embedded-hal` 1.0. `defmt` support is now opt-in via the `defmt-03` feature, and each driver exposes `destroy()` to recover its bus/device. See the crate API documentation for the current, breaking API surface.
>

# embedded-bq24725

`#![no_std]` driver for the [TI BQ24725](http://www.ti.com/lit/ds/symlink/bq24725.pdf) SMBus Battery Charge Controller.

## Features

- Charge current setting with automatic mA ↔ register conversion (64mA/LSB, 128–8128mA)
- Charge voltage setting with automatic mV ↔ register conversion (16mV/LSB, 1024–19200mV)
- Input current limit with automatic mA ↔ register conversion (128mA/LSB, 128–8064mA)
- Full ChargeOption register bit-field abstraction (watchdog, LEARN, ACOC, IOUT, etc.)
- Watchdog refresh, charging enable/disable, LEARN cycle control
- Device identification and verification
- Built on [`embedded-hal`](https://docs.rs/embedded-hal) I2C traits — works with any HAL
- [`defmt`](https://docs.rs/defmt) formatting support for embedded logging

## Usage

```rust
use embedded_bq24725::{Bq24725, ChargeOptions, WatchdogTimer};

// `i2c` is any `embedded_hal::i2c::I2c` implementation
let mut charger = Bq24725::new(i2c);

// Verify chip identity
assert!(charger.verify_id()?);

// Configure for 4S LiPo: 16.8V charge voltage, 2A charge current, 3A input limit
let mut opts = ChargeOptions::por_default();
opts.watchdog_timer = WatchdogTimer::T44s;

charger.configure(
    16_800,  // 16.8V charge voltage
    2_000,   // 2A charge current
    3_000,   // 3A input current limit
    &opts,
)?;
```

## API Overview

### Identification

| Method | Description |
|--------|-------------|
| `device_id()` | Read Device ID register (always `0x0008`) |
| `manufacture_id()` | Read Manufacturer ID register (always `0x0040`) |
| `verify_id()` | Check both IDs match expected BQ24725 values |

### Charge Configuration

| Method | Description |
|--------|-------------|
| `set_charge_current_ma(ma)` | Set charge current (128–8128 mA) |
| `charge_current_ma()` | Read current charge current setting (mA) |
| `set_charge_voltage_mv(mv)` | Set charge voltage (1024–19200 mV) |
| `charge_voltage_mv()` | Read current charge voltage setting (mV) |
| `set_input_current_ma(ma)` | Set input current limit (128–8064 mA) |
| `input_current_ma()` | Read current input current limit (mA) |
| `configure(v, c, i, opts)` | Apply all settings in one call |

### Charge Options

| Method | Description |
|--------|-------------|
| `charge_option()` | Read options as `ChargeOptions` struct |
| `set_charge_option(opts)` | Write `ChargeOptions` struct |
| `charge_option_raw()` | Read raw 16-bit register value |

### Control

| Method | Description |
|--------|-------------|
| `set_charging_enabled(bool)` | Enable/disable charging |
| `query_charging_enabled()` | Query current charging state |
| `set_watchdog(timer)` | Set watchdog timeout |
| `refresh_watchdog()` | Kick the watchdog timer |
| `set_iout_selection(iout)` | Select IOUT pin monitor target |
| `start_learn_cycle()` | Start battery LEARN discharge cycle |
| `query_learn_active()` | Query if LEARN cycle is active |

### Debug

| Method | Description |
|--------|-------------|
| `read_all_raw()` | Read all 6 registers as raw `u16` values |

## Register Map

| Address | Name | R/W | POR | Bits | Resolution | Range |
|---------|------|-----|-----|------|------------|-------|
| `0x12` | ChargeOption() | R/W | `0x7904` | 16 | — | — |
| `0x14` | ChargeCurrent() | R/W | `0x0000` | 7 (bits 12:6) | 64 mA | 128–8128 mA |
| `0x15` | ChargeVoltage() | R/W | `0x0000` | 11 (bits 14:4) | 16 mV | 1024–19200 mV |
| `0x3F` | InputCurrent() | R/W | `0x1000` | 6 (bits 12:7) | 128 mA | 128–8064 mA |
| `0xFE` | ManufacturerID() | R | `0x0040` | — | — | — |
| `0xFF` | DeviceID() | R | `0x0008` | — | — | — |

### Conversion Formulas

**Charge Current** (0x14):
```text
Encode:  reg = ((mA - 128) / 64) << 6
Decode:  mA = (reg >> 6) × 64 + 128
```

**Charge Voltage** (0x15):
```text
Encode:  reg = ((mV - 1024) / 16) << 4
Decode:  mV = (reg >> 4) × 16 + 1024
```

**Input Current** (0x3F):
```text
Encode:  reg = ((mA - 128) / 128) << 7
Decode:  mA = (reg >> 7) × 128 + 128
```

## ChargeOption Bit Fields

| Bits | Field | Default | Values |
|------|-------|---------|--------|
| [15] | ACOK Deglitch | 150ms | `0` = 150ms, `1` = 1.3s |
| [14:13] | Watchdog Timer | 44s | `00` = off, `01` = 44s, `10` = 88s, `11` = 175s |
| [12:11] | BAT Depletion | 62.65% | `00` = 59.19%, `01` = 62.65%, `10` = 66.55%, `11` = 70.97% |
| [10] | EMI Freq Adj Dir | Dec 18% | `0` = reduce, `1` = increase |
| [9] | EMI Freq Adj En | Off | `0` = disabled, `1` = enabled |
| [8:7] | IFAULT_HI | 700mV | `00` = 300mV, `01` = 500mV, `10` = 700mV, `11` = 900mV |
| [6] | LEARN Enable | Off | `0` = disabled, `1` = enabled (auto-resets) |
| [5] | IOUT Select | Adapter | `0` = adapter current, `1` = charge current |
| [2:1] | ACOC Threshold | 1.66x | `00` = off, `01` = 1.33x, `10` = 1.66x, `11` = 2.22x |
| [0] | Charge Inhibit | Enable | `0` = charge enabled, `1` = charge inhibited |

## Dependencies

- `embedded-hal` — I2C traits
- `defmt` — `Format` derive for debug logging

## License

GPL-3.0
