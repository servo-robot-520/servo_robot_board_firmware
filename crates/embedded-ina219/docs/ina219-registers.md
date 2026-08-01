# INA219 寄存器详解

基于 TI INA219 datasheet (SBOS448G, August 2008 — Revised December 2015) 整理。

## 寄存器总览

| 地址 | 名称 | 类型 | 上电默认值 | 说明 |
|------|------|------|-----------|------|
| 0x00 | Configuration | R/W | 0x39F | 配置寄存器：PGA增益、ADC分辨率、总线电压范围、工作模式 |
| 0x01 | Shunt Voltage | R | — | 分流电压测量值，10µV/LSB，有符号 |
| 0x02 | Bus Voltage | R | — | 总线电压测量值，4mV/LSB，含 CNVR 和 OVF 标志位 |
| 0x03 | Power | R | 0x0000 | 功率测量值（需先校准） |
| 0x04 | Current | R | 0x0000 | 电流测量值（需先校准） |
| 0x05 | Calibration | R/W | 0x0000 | 校准值，决定 Current 和 Power 寄存器的缩放 |

**重要：** Power 和 Current 寄存器在 Calibration 寄存器写入有效值之前始终返回 0。

---

## 0x00 — Configuration Register

16 位 R/W 寄存器，控制 INA219 的所有测量参数。

```
Bit:  15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
      RST  —  BRNG PG1 PG0 BADC4 BADC3 BADC2 BADC1 SADC4 SADC3 SADC2 SADC1 MODE3 MODE2 MODE1
      R/W  R/W  R/W R/W R/W  R/W  R/W  R/W  R/W   R/W   R/W   R/W   R/W   R/W   R/W   R/W
```

上电默认值：0x39F = `0011 1001 1111` → RST=0, BRNG=1(32V), PG=11(/8), BADC=1111(12bit), SADC=1111(12bit), MODE=111(连续)

### RST (Bit 15) — 复位
- 写 1 触发系统复位，等同于上电复位
- 该位自清除

### BRNG (Bit 13) — 总线电压量程
| 值 | 量程 | 满量程 |
|----|------|--------|
| 0 | 16V | 4000 decimal |
| 1 | 32V (默认) | 8000 decimal |

### PG (Bits 12:11) — PGA 增益（分流电压）
| PG1 | PG0 | 增益 | 量程 |
|-----|-----|------|------|
| 0 | 0 | ×1 | ±40mV |
| 0 | 1 | /2 | ±80mV |
| 1 | 0 | /4 | ±160mV |
| 1 | 1 | /8 (默认) | ±320mV |

### BADC (Bits 10:7) — 总线 ADC 分辨率/平均
| ADC4 | ADC3 | ADC2 | ADC1 | 模式 | 转换时间 |
|------|------|------|------|------|---------|
| 0 | X | 0 | 0 | 9 bit | 84µs |
| 0 | X | 0 | 1 | 10 bit | 148µs |
| 0 | X | 1 | 0 | 11 bit | 276µs |
| 0 | X | 1 | 1 | 12 bit (默认) | 532µs |
| 1 | 0 | 0 | 0 | 12 bit | 532µs |
| 1 | 0 | 0 | 1 | 2 samples | 1.06ms |
| 1 | 0 | 1 | 0 | 4 samples | 2.13ms |
| 1 | 0 | 1 | 1 | 8 samples | 4.26ms |
| 1 | 1 | 0 | 0 | 16 samples | 8.51ms |
| 1 | 1 | 0 | 1 | 32 samples | 17.02ms |
| 1 | 1 | 1 | 0 | 64 samples | 34.05ms |
| 1 | 1 | 1 | 1 | 128 samples | 68.10ms |

### SADC (Bits 6:3) — 分流 ADC 分辨率/平均
与 BADC 编码相同。

### MODE (Bits 2:0) — 工作模式
| MODE3 | MODE2 | MODE1 | 模式 |
|-------|-------|-------|------|
| 0 | 0 | 0 | Power-down |
| 0 | 0 | 1 | Shunt voltage, triggered |
| 0 | 1 | 0 | Bus voltage, triggered |
| 0 | 1 | 1 | Shunt and bus, triggered |
| 1 | 0 | 0 | ADC off (disabled) |
| 1 | 0 | 1 | Shunt voltage, continuous |
| 1 | 1 | 0 | Bus voltage, continuous |
| 1 | 1 | 1 | Shunt and bus, continuous (默认) |

---

## 0x01 — Shunt Voltage Register

只读，16 位有符号值 (2's complement)。

- **LSB = 10µV**
- 根据 PGA 设置左对齐（高位符号扩展）
- 范围：PGA=/8 时 ±320mV (±32000 decimal)

```
PGA=/8: [SIGN SD14 SD13 SD12 SD11 SD10 SD9 SD8 SD7 SD6 SD5 SD4 SD3 SD2 SD1 SD0]
PGA=/4: [SIGN SIGN SD13 SD12 SD11 SD10 SD9 SD8 SD7 SD6 SD5 SD4 SD3 SD2 SD1 SD0]
PGA=/2: [SIGN SIGN SIGN SD12 SD11 SD10 SD9 SD8 SD7 SD6 SD5 SD4 SD3 SD2 SD1 SD0]
PGA=/1: [SIGN SIGN SIGN SIGN SD11 SD10 SD9 SD8 SD7 SD6 SD5 SD4 SD3 SD2 SD1 SD0]
```

---

## 0x02 — Bus Voltage Register

只读，16 位。

```
Bit:  15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
      BD12 BD11 BD10 BD9 BD8 BD7 BD6 BD5 BD4 BD3 BD2 BD1 BD0  —  CNVR OVF
```

- **数据位：** Bits 15:3 = BD12-BD0（13 位有效数据，但实际是 12 位数据右对齐在高 12 位）
- **LSB = 4mV**
- **CNVR (Bit 1)：** 转换就绪标志，所有转换、平均和乘法完成后置 1
  - 写入 Configuration 寄存器（除 Power-Down/Disable 模式外）清除此位
  - 读取 Bus Voltage 寄存器清除此位
- **OVF (Bit 0)：** 溢出标志，Power 或 Current 计算超出范围时置 1

**读取总线电压：** `(raw >> 3) & 0x1FFF`，然后乘以 4mV 得到电压值。

---

## 0x03 — Power Register

只读，16 位无符号值。

- 需要先写入 Calibration 寄存器才有有效值
- Power_LSB = 20 × Current_LSB
- 内部计算：`Power = (Current_Register × Bus_Voltage_Register) / 5000`

---

## 0x04 — Current Register

只读，16 位有符号值 (2's complement)。

- 需要先写入 Calibration 寄存器才有有效值
- 内部计算：`Current = (Shunt_Voltage_Register × Calibration_Register) / 4096`
- Current_LSB 取决于 Calibration 值

---

## 0x05 — Calibration Register

16 位 R/W 寄存器。

```
Bit:  15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
      FS15 FS14 FS13 FS12 FS11 FS10 FS9 FS8 FS7 FS6 FS5 FS4 FS3 FS2 FS1 FS0
      R/W  R/W  R/W  R/W  R/W  R/W  R/W R/W R/W R/W R/W R/W R/W R/W R/W  R-0
```

- **FS0 (Bit 0) 是无效位，永远为 0**，无法写入 1
- CALIBRATION 值存储在 FS15:FS1

### 校准公式

```
Cal = trunc(0.04096 / (Current_LSB × R_SHUNT))

其中：
- 0.04096 是内部固定值
- Current_LSB = 最大预期电流 / 2^15
- Power_LSB = 20 × Current_LSB
```

### 校准示例（使用 2mΩ 分流电阻，最大 15A）

```
Current_LSB = 15 / 32768 ≈ 0.00045776 A/bit ≈ 0.458 mA/bit
Cal = trunc(0.04096 / (0.00045776 × 0.002)) = trunc(0.04096 / 0.00000091553) = 44741
```

---

## I2C 通信

- 支持 16 个可编程地址（通过 A0、A1 引脚）
- 支持标准模式 (100kHz)、快速模式 (400kHz)、高速模式 (最高 2.56MHz)
- SMBus 超时：28ms
- 寄存器内容在写命令完成后 4µs 更新

---

## 关键公式汇总

| 公式 | 说明 |
|------|------|
| `V_SHUNT = Register_01h × 10µV` | 分流电压 |
| `V_BUS = (Register_02h >> 3) × 4mV` | 总线电压 |
| `I = V_SHUNT / R_SHUNT` | 电流（手动计算） |
| `I = Register_04h × Current_LSB` | 电流（硬件计算，需校准） |
| `P = Register_03h × Power_LSB` | 功率（硬件计算，需校准） |
| `Cal = trunc(0.04096 / (Current_LSB × R_SHUNT))` | 校准值 |
| `Current_LSB = Max_Expected_Current / 2^15` | 电流 LSB |
| `Power_LSB = 20 × Current_LSB` | 功率 LSB |
