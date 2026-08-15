# Servo Robot Power Management Board Firmware

Embedded Rust firmware for STM32F411, providing integrated power management, sensor acquisition, and communication for quadruped/biped servo robots.

[中文](README.md) | **English**

## Hardware Platform

| Component | Model | Interface | Purpose |
|-----------|-------|-----------|---------|
| MCU | STM32F411CEU6 (Cortex-M4F, 96MHz) | — | Main controller |
| USB PD Controller | HUSB238A | I2C1 (0x42) | USB PD sink |
| Charge Controller | BQ24725 | I2C1 (0x09) | CC/CV charging |
| Battery Gauge | BQ40Z50 | I2C1 (0x0B) | 4S smart fuel gauge |
| Power Monitor | INA219 | I2C1 (0x40) | Servo bus voltage/current |
| IMU | MPU6500 | SPI1 (1MHz) | 6-axis motion sensor |
| LEDs | WS2812 × 3 | TIM1 CH2 + DMA | Status indicators |
| Buzzer | Passive buzzer | TIM2 CH2 | Audio feedback |

## Repository Structure

```
├── crates/
│   ├── servo-robot-board/           # Main application firmware
│   │   ├── src/
│   │   │   ├── main.rs              # RTIC application entry
│   │   │   ├── features/            # Feature modules
│   │   │   │   ├── communication/   # Dual-channel host comms (UART2 + USB CDC)
│   │   │   │   ├── sensing/         # Sensor acquisition (IMU, battery, power, temp)
│   │   │   │   ├── charge/          # Charging state machine
│   │   │   │   ├── power/           # Power output control & protection
│   │   │   │   ├── servo/           # Serial servo command forwarding
│   │   │   │   └── telemetry/       # System info, logging, event reporting
│   │   │   └── platform/            # HAL platform abstractions (ADC, Flash, UART, WS2812)
│   │   └── ...
│   ├── servo-robot-board-bootloader/ # OTA bootloader (16KB)
│   ├── embedded-ina219/              # INA219 current/power monitor driver
│   ├── embedded-mpu6500/             # MPU6500 6-axis IMU driver
│   ├── embedded-husb238a/            # HUSB238A USB PD sink driver
│   ├── embedded-bq24725/             # BQ24725 charge controller driver
│   └── embedded-bq40z50/             # BQ40Z50 smart battery gauge driver
├── memory.x                          # Flash/RAM memory layout
└── .cargo/config.toml                # Build target configuration
```

## Flash Partition Map

| Region | Start Address | Size | Purpose |
|--------|---------------|------|---------|
| Bootloader | `0x0800_0000` | 16KB (Sector 0) | OTA bootstrap |
| App Firmware | `0x0800_4000` | 240KB (Sectors 1-5) | Main firmware |
| OTA Temp | `0x0804_0000` | 128KB (Sector 6) | OTA staging area |
| User Data | `0x0806_0000` | 128KB (Sector 7) | Config + OTA flags |

## Core Features

### Power Management
- **4S LiPo Charging**: BQ24725 CC/CV state machine with thermal derating
- **USB PD Sink**: HUSB238A PDO discovery & negotiation, interrupt-driven
- **Multi-channel Output Control**: Servo power (PC13), 5V (PC15), battery external output (PC14), shutdown (PB13)

### Monitoring
- **Battery**: BQ40Z50 gauge @ 10Hz — voltage, current, SOC, temperature, cell voltages
- **Power**: INA219 @ 20Hz — servo bus voltage/current + ADC charge parameters
- **IMU**: MPU6500 @ 100Hz — acceleration, angular velocity, Mahony AHRS quaternion/euler angles
- **Temperature**: 3x NTC + MCU internal sensor @ 1Hz

### Protection
- **Overcurrent**: Servo current exceeds limit for 30s → auto cutoff
- **Overtemperature**: Servo/5V temperature exceeds limit for 30s → auto cutoff
- **Fan Control**: Temperature-triggered on/off with 5°C hysteresis

### Communication Protocol
- **Dual Channel**: UART2 (PA2/PA3) + USB CDC (PA11/PA12), unified binary frame protocol
- **Frame Format**: `[HEAD=0xAA][TYPE:1][LEN:2LE][PAYLOAD:N][CRC16-CCITT:2]`
- **Upstream**: IMU (100Hz), power (20Hz), battery (10Hz), system info (1Hz), events/logs (on-demand)
- **Downstream**: Config write/query, servo forwarding, firmware update, system commands (reset/shutdown/OTA)

### Serial Servo Forwarding
- **Channel**: USART1 (PA15/PA10), half-duplex with TX direction control (PB12)
- **Forward**: Host → `ServoForward (0x83)` → firmware → UART1 raw bytes → servo
- **Response**: Servo → UART1 → firmware → `AckServoCmd (0xC3)` → host

### OTA Firmware Update
- **Protocol Transfer**: `FirmwareUpdate (0x84)` chunked write to OTA Temp, `Command(Ota)` triggers update
- **USB MSD Drag & Drop**: FAT12 virtual USB drive, drop `FIRMWARE.BIN` to upgrade
- **Bootloader**: Verify image header (magic, CRC32) → copy OTA Temp → App → jump

### Status Display
- **WS2812 LED × 3**: Charge temperature / battery SOC / battery temperature, gradient color indicators
- **Buzzer**: R2-D2 style startup melody, arbitrary frequency/duration tones

## Building

Requires Rust nightly toolchain and `thumbv7em-none-eabihf` target.

```bash
# Build main firmware
cargo build -p servo-robot-board

# Build bootloader
cargo build -p servo-robot-board-bootloader

# Build entire workspace
cargo build --workspace

# Build with serial servo feature
cargo build -p servo-robot-board --features servo
```

## Code Quality

```bash
# Format check
cargo fmt --all -- --check

# Clippy static analysis
cargo clippy --workspace --all-targets

# Run host-side unit tests
cargo test --workspace --target x86_64-unknown-linux-gnu
```

## Flashing & Debugging

Using [probe-rs](https://probe.rs/) or ST-Link:

```bash
# Flash main firmware
probe-rs download --chip STM32F411CEU target/thumbv7em-none-eabihf/release/servo-robot-board

# RTT log viewer
probe-rs attach --chip STM32F411CEU
```

## Driver Crates

Each hardware driver is an independent `#![no_std]` library crate, reusable standalone:

| Crate | Chip | Function |
|-------|------|----------|
| `embedded-ina219` | TI INA219 | Bidirectional current/power monitor |
| `embedded-mpu6500` | InvenSense MPU6500 | 6-axis IMU |
| `embedded-husb238a` | Hynetek HUSB238A | USB PD sink |
| `embedded-bq24725` | TI BQ24725 | CC/CV charge controller |
| `embedded-bq40z50` | TI BQ40Z50-R1 | 4S smart battery gauge |

See each crate's `README.md` for detailed usage.

## License

[GPL-3.0](LICENSE)
