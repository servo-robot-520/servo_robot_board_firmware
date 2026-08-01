# MPU6500 Register Reference

Based on InvenSense MPU-6500 Register Map and Descriptions, Revision 2.1 (RM-MPU-6500A-00).

## Register Map Overview

| Address | Name | R/W | Description |
|---------|------|-----|-------------|
| 0x00 | SELF_TEST_X_GYRO | R/W | X Gyro self-test output |
| 0x01 | SELF_TEST_Y_GYRO | R/W | Y Gyro self-test output |
| 0x02 | SELF_TEST_Z_GYRO | R/W | Z Gyro self-test output |
| 0x0D | SELF_TEST_X_ACCEL | R/W | X Accel self-test output |
| 0x0E | SELF_TEST_Y_ACCEL | R/W | Y Accel self-test output |
| 0x0F | SELF_TEST_Z_ACCEL | R/W | Z Accel self-test output |
| 0x13 | XG_OFFSET_H | R/W | X Gyro offset [15:8] |
| 0x14 | XG_OFFSET_L | R/W | X Gyro offset [7:0] |
| 0x15 | YG_OFFSET_H | R/W | Y Gyro offset [15:8] |
| 0x16 | YG_OFFSET_L | R/W | Y Gyro offset [7:0] |
| 0x17 | ZG_OFFSET_H | R/W | Z Gyro offset [15:8] |
| 0x18 | ZG_OFFSET_L | R/W | Z Gyro offset [7:0] |
| 0x19 | SMPLRT_DIV | R/W | Sample rate divider |
| 0x1A | CONFIG | R/W | Configuration |
| 0x1B | GYRO_CONFIG | R/W | Gyroscope configuration |
| 0x1C | ACCEL_CONFIG | R/W | Accelerometer configuration |
| 0x1D | ACCEL_CONFIG_2 | R/W | Accelerometer configuration 2 |
| 0x1E | LP_ACCEL_ODR | R/W | Low-power accelerometer ODR control |
| 0x1F | WOM_THR | R/W | Wake-on motion threshold |
| 0x23 | FIFO_EN | R/W | FIFO enable |
| 0x24 | I2C_MST_CTRL | R/W | I2C master control |
| 0x25-0x2D | I2C_SLV0-2 | R/W | I2C slave 0-2 control |
| 0x2E-0x34 | I2C_SLV3-4 | R/W | I2C slave 3-4 control |
| 0x37 | INT_PIN_CFG | R/W | INT pin / bypass enable config |
| 0x38 | INT_ENABLE | R/W | Interrupt enable |
| 0x3A | INT_STATUS | R | Interrupt status |
| 0x3B-0x40 | ACCEL_XYZ_OUT | R | Accelerometer measurements (6 bytes) |
| 0x41-0x42 | TEMP_OUT | R | Temperature measurement (2 bytes) |
| 0x43-0x48 | GYRO_XYZ_OUT | R | Gyroscope measurements (6 bytes) |
| 0x49-0x60 | EXT_SENS_DATA | R | External sensor data (24 bytes) |
| 0x63-0x66 | I2C_SLV0-3_DO | R/W | I2C slave 0-3 data out |
| 0x67 | I2C_MST_DELAY_CTRL | R/W | I2C master delay control |
| 0x68 | SIGNAL_PATH_RESET | R/W | Signal path reset |
| 0x69 | ACCEL_INTEL_CTRL | R/W | Accelerometer interrupt control |
| 0x6A | USER_CTRL | R/W | User control |
| 0x6B | PWR_MGMT_1 | R/W | Power management 1 |
| 0x6C | PWR_MGMT_2 | R/W | Power management 2 |
| 0x72-0x73 | FIFO_COUNT | R/W | FIFO count (2 bytes) |
| 0x74 | FIFO_R_W | R/W | FIFO read/write |
| 0x75 | WHO_AM_I | R | Device identity (0x70) |
| 0x77,0x78 | XA_OFFSET | R/W | X Accel offset |
| 0x7A,0x7B | YA_OFFSET | R/W | Y Accel offset |
| 0x7D,0x7E | ZA_OFFSET | R/W | Z Accel offset |

## Key Registers Detail

### PWR_MGMT_1 (0x6B) - Power Management 1

Reset value: 0x01

| Bit | Name | Description |
|-----|------|-------------|
| 7 | DEVICE_RESET | Write 1 to reset all registers to defaults (auto-clears) |
| 6 | SLEEP | 1 = chip in sleep mode |
| 5 | CYCLE | Cycle between sleep and wake (when SLEEP=0, STANDBY=0) |
| 4 | GYRO_STANDBY | Gyro drive/PLL on, sense paths disabled (low-power ready) |
| 3 | TEMP_DIS | 1 = disable temperature sensor |
| 2:0 | CLKSEL | Clock source select |

**CLKSEL values:**

| Code | Clock Source |
|------|-------------|
| 0 | Internal 20MHz oscillator |
| 1 | PLL with X-axis gyro reference |
| 2 | PLL with Y-axis gyro reference |
| 3 | PLL with Z-axis gyro reference |
| 4 | PLL with external 32.768kHz reference |
| 5 | PLL with external 19.2MHz reference |
| 6 | Reserved (stops clock) |
| 7 | Reserved |

### PWR_MGMT_2 (0x6C) - Power Management 2

Reset value: 0x00

| Bit | Name | Description |
|-----|------|-------------|
| 7:6 | LP_WAKE_CTRL | Wake-up frequency in low-power accel-only mode |
| 5 | DISABLE_XA | 1 = disable X accel |
| 4 | DISABLE_YA | 1 = disable Y accel |
| 3 | DISABLE_ZA | 1 = disable Z accel |
| 2 | DISABLE_XG | 1 = disable X gyro |
| 1 | DISABLE_YG | 1 = disable Y gyro |
| 0 | DISABLE_ZG | 1 = disable Z gyro |

### CONFIG (0x1A) - Configuration

Reset value: 0x00

| Bit | Name | Description |
|-----|------|-------------|
| 6 | FIFO_MODE | 0 = FIFO overflow overwrites oldest; 1 = new writes rejected |
| 5:3 | EXT_SYNC_SET | FSYNC pin data sampling enable |
| 2:0 | DLPF_CFG | Digital low pass filter configuration |

**DLPF_CFG for Gyro + Temp:**

| DLPF_CFG | Gyro Bandwidth (Hz) | Gyro Fs (kHz) | Temp Bandwidth (Hz) |
|----------|---------------------|---------------|---------------------|
| 0 | 250 | 8 | 250 |
| 1 | 184 | 1 | 188 |
| 2 | 92 | 1 | 98 |
| 3 | 41 | 1 | 42 |
| 4 | 20 | 1 | 20 |
| 5 | 10 | 1 | 10 |
| 6 | 5 | 1 | 5 |
| 7 | 3600 | 8 | 4000 |

Note: DLPF_CFG only effective when FCHOICE_B = 2'b00.

### GYRO_CONFIG (0x1B) - Gyroscope Configuration

Reset value: 0x00

| Bit | Name | Description |
|-----|------|-------------|
| 7 | XG_ST | X Gyro self-test |
| 6 | YG_ST | Y Gyro self-test |
| 5 | ZG_ST | Z Gyro self-test |
| 4:3 | GYRO_FS_SEL | Full scale select |
| 2 | Reserved | |
| 1:0 | FCHOICE_B | DLPF bypass (00 = use DLPF) |

**GYRO_FS_SEL:**

| Value | Full Scale | Sensitivity (LSB/(°/s)) |
|-------|-----------|------------------------|
| 00 | ±250°/s | 131.0 |
| 01 | ±500°/s | 65.5 |
| 10 | ±1000°/s | 32.8 |
| 11 | ±2000°/s | 16.4 |

### ACCEL_CONFIG (0x1C) - Accelerometer Configuration

Reset value: 0x00

| Bit | Name | Description |
|-----|------|-------------|
| 7 | XA_ST | X Accel self-test |
| 6 | YA_ST | Y Accel self-test |
| 5 | ZA_ST | Z Accel self-test |
| 4:3 | ACCEL_FS_SEL | Full scale select |
| 2:0 | Reserved | |

**ACCEL_FS_SEL:**

| Value | Full Scale | Sensitivity (LSB/g) |
|-------|-----------|---------------------|
| 00 | ±2g | 16384 |
| 01 | ±4g | 8192 |
| 10 | ±8g | 4096 |
| 11 | ±16g | 2048 |

### ACCEL_CONFIG_2 (0x1D) - Accelerometer Configuration 2

Reset value: 0x00

| Bit | Name | Description |
|-----|------|-------------|
| 3 | ACCEL_FCHOICE_B | 0 = use DLPF, 1 = bypass DLPF (1.13kHz output) |
| 2:0 | A_DLPF_CFG | Accel DLPF configuration |

### SMPLRT_DIV (0x19) - Sample Rate Divider

Reset value: 0x00

```
SAMPLE_RATE = INTERNAL_SAMPLE_RATE / (1 + SMPLRT_DIV)
```

Where INTERNAL_SAMPLE_RATE = 1kHz (when DLPF_CFG is active).

Only effective when FCHOICE_B = 2'b00 and 0 < DLPF_CFG < 7.

### INT_PIN_CFG (0x37) - INT Pin / Bypass Enable Configuration

Reset value: 0x00

| Bit | Name | Description |
|-----|------|-------------|
| 7 | ACTL | INT pin active level: 0=active high, 1=active low |
| 6 | OPEN | INT pin config: 0=push-pull, 1=open drain |
| 5 | LATCH_INT_EN | 0=50μs pulse, 1=latched until cleared |
| 4 | INT_ANYRD_2CLEAR | 1=clear on any read, 0=clear on INT_STATUS read |
| 3 | ACTL_FSYNC | FSYNC pin active level |
| 2 | FSYNC_INT_MODE_EN | 1=enable FSYNC as interrupt |
| 1 | BYPASS_EN | 1=I2C master in bypass mode (pass-through) |

### INT_ENABLE (0x38) - Interrupt Enable

Reset value: 0x00

| Bit | Name | Description |
|-----|------|-------------|
| 6 | WOM_EN | Wake-on-motion interrupt enable |
| 4 | FIFO_OVERFLOW_EN | FIFO overflow interrupt enable |
| 3 | FSYNC_INT_EN | FSYNC interrupt enable |
| 0 | RAW_RDY_EN | Raw data ready interrupt enable |

### INT_STATUS (0x3A) - Interrupt Status (Read-only)

| Bit | Name | Description |
|-----|------|-------------|
| 6 | WOM_INT | Wake-on-motion interrupt occurred |
| 4 | FIFO_OVERFLOW_INT | FIFO overflow occurred |
| 3 | FSYNC_INT | FSYNC interrupt occurred |
| 1 | DMP_INT | DMP interrupt generated |
| 0 | RAW_DATA_RDY_INT | Sensor data ready to be read |

### SIGNAL_PATH_RESET (0x68)

Reset value: 0x00

| Bit | Name | Description |
|-----|------|-------------|
| 2 | GYRO_RST | Reset gyro digital signal path |
| 1 | ACCEL_RST | Reset accel digital signal path |
| 0 | TEMP_RST | Reset temp digital signal path |

### USER_CTRL (0x6A) - User Control

Reset value: 0x00

| Bit | Name | Description |
|-----|------|-------------|
| 7 | DMP_EN | Enable DMP features |
| 6 | FIFO_EN | Enable FIFO |
| 5 | I2C_MST_EN | Enable I2C master interface |
| 4 | I2C_IF_DIS | Disable I2C slave, put serial interface in SPI mode |
| 3 | DMP_RST | Reset DMP (auto-clears) |
| 2 | FIFO_RST | Reset FIFO (auto-clears) |
| 1 | I2C_MST_RST | Reset I2C master (auto-clears) |
| 0 | SIG_COND_RST | Reset all signal paths and sensor registers |

### FIFO_EN (0x23) - FIFO Enable

| Bit | Name | Description |
|-----|------|-------------|
| 7 | TEMP_OUT | Enable temp to FIFO |
| 6 | GYRO_XOUT | Enable gyro X to FIFO |
| 5 | GYRO_YOUT | Enable gyro Y to FIFO |
| 4 | GYRO_ZOUT | Enable gyro Z to FIFO |
| 3 | ACCEL | Enable all accel axes to FIFO |
| 2 | SLV2 | Enable I2C slave 2 ext sensor data to FIFO |
| 1 | SLV1 | Enable I2C slave 1 ext sensor data to FIFO |
| 0 | SLV0 | Enable I2C slave 0 ext sensor data to FIFO |

### SMPLRT_DIV (0x19)

Formula: `SAMPLE_RATE = 1kHz / (1 + SMPLRT_DIV)` (when DLPF active)

| SMPLRT_DIV | Sample Rate |
|-----------|-------------|
| 0 | 1000 Hz |
| 4 | 200 Hz |
| 9 | 100 Hz |
| 19 | 50 Hz |
| 99 | 10 Hz |

### WOM_THR (0x1F) - Wake-on Motion Threshold

LSB = 4mg. Range: 0mg to 1020mg.

### ACCEL_INTEL_CTRL (0x69)

| Bit | Name | Description |
|-----|------|-------------|
| 7 | ACCEL_INTEL_EN | Enable wake-on-motion detection |
| 6 | ACCEL_INTEL_MODE | 1=compare current with previous sample |

## SPI Protocol

- **Write**: CS low → send `[addr & 0x7F]` → send `[data]` → CS high
- **Read**: CS low → send `[addr | 0x80]` → clock out N bytes → CS high
- Max SPI clock: 1MHz

## Init Sequence (SPI, per datasheet page 41)

1. Write 0x80 to PWR_MGMT_1 (DEVICE_RESET)
2. Wait 100ms
3. Write 0x07 to SIGNAL_PATH_RESET (reset gyro/accel/temp paths)
4. Wait 100ms
5. Write desired config to PWR_MGMT_1 (clock source, sleep, etc.)
6. Configure SMPLRT_DIV, CONFIG, GYRO_CONFIG, ACCEL_CONFIG as needed

## Temperature Conversion

```
TEMP_degC = (TEMP_OUT / 333.87) + 21.0
```

Where TEMP_OUT is the signed 16-bit raw value from registers 0x41-0x42.

## Accelerometer Offset Registers

| Register | Address | Description |
|----------|---------|-------------|
| XA_OFFSET_H | 0x77 | X accel offset [14:7] |
| XA_OFFSET_L | 0x78 | X accel offset [6:0] (bit 0 reserved) |
| YA_OFFSET_H | 0x7A | Y accel offset [14:7] |
| YA_OFFSET_L | 0x7B | Y accel offset [6:0] (bit 0 reserved) |
| ZA_OFFSET_H | 0x7D | Z accel offset [14:7] |
| ZA_OFFSET_L | 0x7E | Z accel offset [6:0] (bit 0 reserved) |

Offset resolution: 0.98mg per LSB, range ±16g.
