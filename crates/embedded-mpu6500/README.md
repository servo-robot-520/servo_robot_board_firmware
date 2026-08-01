> ## API migration note (2026-08)
> This driver is `no_std` and uses `embedded-hal` 1.0. `defmt` support is opt-in via `defmt-03`. Construct it with one `SpiDevice` (not `SpiBus` plus CS), pass `DelayNs` to `init`, and use `Axis::{X,Y,Z}` for offsets.
>

# embedded-mpu6500

A `no_std` Rust driver for the InvenSense MPU6500 6-axis inertial measurement unit (IMU) over SPI.

## Features

- **Accelerometer**: ±2g, ±4g, ±8g, ±16g full-scale ranges
- **Gyroscope**: ±250°/s, ±500°/s, ±1000°/s, ±2000°/s full-scale ranges
- **Temperature sensor**: on-chip temperature measurement
- **Digital low-pass filter (DLPF)**: configurable bandwidth for gyro, accel, and temperature
- **FIFO buffer**: up to 512 bytes with configurable data selection
- **Interrupts**: raw data ready, wake-on-motion, FIFO overflow, FSYNC
- **Power management**: sleep mode, cycle mode, per-axis enable/disable
- **Offset calibration**: gyro and accelerometer offset registers
- **Self-test**: register readout for factory self-test verification

## Usage

### Basic initialization

```rust
use embedded_mpu6500::Mpu6500;

// Create driver instance
let mut mpu = Mpu6500::new(spi_bus, cs_pin);

// Full init with 100ms delay support (recommended)
mpu.init(&mut delay)?;

// Verify device identity
assert!(mpu.verify_id()?); // Returns true if WHO_AM_I == 0x70
```

### Reading sensor data

```rust
// Read all sensors at once (14-byte burst read)
let data = mpu.read()?;
defmt::info!("Accel: {:.2} {:.2} {:.2} g", data.accel[0], data.accel[1], data.accel[2]);
defmt::info!("Gyro:  {:.2} {:.2} {:.2} °/s", data.gyro[0], data.gyro[1], data.gyro[2]);
defmt::info!("Temp:  {:.1} °C", data.temp_c);

// Read individual sensors
let accel = mpu.read_accel_raw()?;
let gyro = mpu.read_gyro_raw()?;
let temp = mpu.read_temp()?;
```

### Configuring ranges and filters

```rust
use embedded_mpu6500::{GyroRange, AccelRange, DlpfConfig};

mpu.set_gyro_range(GyroRange::Dps500)?;
mpu.set_accel_range(AccelRange::G8)?;
mpu.set_dlpf(DlpfConfig::Dlpf41)?; // 41Hz bandwidth
mpu.set_sample_rate_div(9)?;        // 100Hz sample rate (1kHz / 10)
```

### Interrupt configuration

```rust
use embedded_mpu6500::{IntLevel, IntDriveMode, IntLatch};

// Configure INT pin: active-low, push-pull, latched
mpu.configure_int_pin(IntLevel::ActiveLow, IntDriveMode::PushPull, IntLatch::Latched, false)?;

// Enable raw data ready interrupt
mpu.configure_interrupts(true, false, false, false)?;

// Poll for data ready
if mpu.wait_data_ready()? {
    let data = mpu.read()?;
    // ...
}
```

### Power management

```rust
// Enter sleep mode
mpu.sleep()?;

// Exit sleep
mpu.wakeup()?;

// Enable only accelerometer axes (for low-power mode)
mpu.set_sensor_enable(true, true, true, false, false, false)?;
mpu.set_cycle_mode(true)?;
```

### FIFO usage

```rust
// Enable FIFO with gyro X/Y/Z and accel
mpu.set_fifo_enable(false, true, true, true, true)?;
mpu.set_fifo_enabled(true)?;

// Read FIFO count and drain
let count = mpu.fifo_count()?;
let mut buf = [0u8; 256];
let n = mpu.fifo_read(&mut buf)?;
```

### Offset calibration

```rust
// Set gyro offsets (16-bit two's complement)
mpu.set_gyro_offset(0, 100)?;  // X-axis
mpu.set_gyro_offset(1, -50)?;  // Y-axis
mpu.set_gyro_offset(2, 0)?;    // Z-axis

// Set accel offsets (15-bit, 0.98mg/LSB)
mpu.set_accel_offset(0, 200)?;  // X-axis
```

## Architecture

```
Mpu6500<SPI, CS>
├── SPI bus (SpiBus<u8>)
├── CS pin (OutputPin)
├── gyro_sensitivity: f32
└── accel_sensitivity: f32
```

The driver is generic over the SPI bus and chip-select pin types, making it compatible with any `embedded_hal` 1.0 implementation.

## Reference

- [MPU-6500 Register Map and Descriptions (RM-MPU-6500A-00, Rev 2.1)](docs/MPU-6500-Register-Map2.pdf)
- [MPU-6500 Product Specification (PS-MPU-6500A-00)](https://invensense.tdk.com/wp-content/uploads/2015/09/PS-MPU-6500A-00.pdf)
- [Register reference](docs/mpu6500-registers.md)

## License

GPL-3.0
