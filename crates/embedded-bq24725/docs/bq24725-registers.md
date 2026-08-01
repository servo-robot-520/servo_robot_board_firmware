# BQ24725 Register Reference

Based on TI SLUS702A (July 2010, Revised November 2010).

## Overview

The BQ24725 operates as an SMBus slave at address **0x09** (7-bit). It uses SMBus Read-Word and Write-Word protocols (16-bit, little-endian). Two identification registers (DeviceID 0xFF, ManufacturerID 0xFE) are read-only.

**Write-Word format:** `[START] [Addr+W] [ACK] [RegCmd] [ACK] [DataLow] [ACK] [DataHigh] [ACK] [STOP]`

**Read-Word format:** `[START] [Addr+W] [ACK] [RegCmd] [ACK] [Sr] [Addr+R] [ACK] [DataLow] [ACK] [DataHigh] [NACK] [STOP]`

---

## Register Map

| Address | Name | R/W | POR State | Description |
|---------|------|-----|-----------|-------------|
| 0x12 | ChargeOption() | R/W | 0x7904 | Charger options control |
| 0x14 | ChargeCurrent() | R/W | 0x0000 | 7-bit charge current setting |
| 0x15 | ChargeVoltage() | R/W | 0x0000 | 11-bit charge voltage setting |
| 0x3F | InputCurrent() | R/W | 0x1000 | 6-bit input current setting |
| 0xFE | ManufacturerID() | R | 0x0040 | Manufacturer ID |
| 0xFF | DeviceID() | R | 0x0008 | Device ID |

---

## 0x12 — ChargeOption() Register

POR: **0x7904**

```
Bit 15:    ACOK Deglitch Time Adjust
           0 = 150ms (default)
           1 = 1.3s

Bits [14:13]: WATCHDOG Timer Adjust
           00 = Disable (default after POR reset, but POR=0x7904 → 01 = 44s)
           01 = 44s
           10 = 88s
           11 = 175s

Bits [12:11]: BAT Depletion Comparator Threshold Adjust
           00 = 59.19% of regulation limit (~2.486V/cell)
           01 = 62.65% (~2.631V/cell) ← POR default
           10 = 66.55% (~2.795V/cell)
           11 = 70.97% (~2.981V/cell)

Bit [10]:  EMI Switching Frequency Adjust
           0 = Reduce PWM by 18% (default)
           1 = Increase PWM by 18%

Bit [9]:   EMI Switching Frequency Enable
           0 = Disable adjust (default)
           1 = Enable adjust

Bits [8:7]: IFAULT_HI Comparator Threshold Adjust
           00 = 300mV
           01 = 500mV
           10 = 700mV (default)
           11 = 900mV

Bit [6]:   LEARN Enable
           0 = Disable LEARN cycle (default)
           1 = Enable LEARN cycle
           (Auto-reset to 0 after LEARN cycle completes)

Bit [5]:   IOUT Selection
           0 = 20x adapter current amplifier output (default)
           1 = 20x charge current amplifier output

Bit [4]:   Not in Use (always 0 at POR)

Bit [3]:   Not in Use (always 0 at POR)

Bits [2:1]: ACOC Threshold Adjust
           00 = Disable ACOC
           01 = 1.33x input current regulation limit
           10 = 1.66x (default)
           11 = 2.22x input current regulation limit

Bit [0]:   Charge Inhibit
           0 = Enable Charge (default)
           1 = Inhibit Charge
```

### POR Value Decoding (0x7904)

```
Bit 15     = 0  → ACOK deglitch = 150ms
Bits 14:13 = 01 → Watchdog = 44s
Bits 12:11 = 00 → BAT depletion = 59.19%
Bit 10     = 0  → EMI freq adj = reduce 18%
Bit 9      = 0  → EMI freq adj = disabled
Bits 8:7   = 10 → IFAULT_HI = 700mV
Bit 6      = 0  → LEARN = disabled
Bit 5      = 0  → IOUT = adapter current
Bit 4      = 0  → reserved
Bit 3      = 0  → reserved
Bits 2:1   = 00 → ACOC = disabled
Bit 0      = 0  → Charge enabled
```

---

## 0x14 — ChargeCurrent() Register

POR: **0x0000** (0mA, charging disabled)

Using 10mΩ sense resistor: range **128mA – 8128mA**, resolution **64mA/LSB**.

```
Bits [15:13]: Not used (always 0)
Bit 12:      DACICHG 6  (4096mA)
Bit 11:      DACICHG 5  (2048mA)
Bit 10:      DACICHG 4  (1024mA)
Bit 9:       DACICHG 3  (512mA)
Bit 8:       DACICHG 2  (256mA)
Bit 7:       DACICHG 1  (128mA)
Bit 6:       DACICHG 0  (64mA)
Bits [5:0]:  Not used (always 0)
```

**Register mask: 0x1FC0** (bits 12:6)

### Conversion

```
Register value = ((mA - 128) / 64) << 6    (for mA >= 128)
mA value = (register_value >> 6) * 64 + 128
```

- Sending < 128mA or > 8128mA clears the register and terminates charging.
- Minimum programmable current = 128mA (0x0040).

---

## 0x15 — ChargeVoltage() Register

POR: **0x0000** (0V, charging disabled)

Range **1.024V – 19.200V**, resolution **16mV/LSB**.

```
Bit 15:      Not used
Bit 14:      DACV 10  (16384mV)
Bit 13:      DACV 9   (8192mV)
Bit 12:      DACV 8   (4096mV)
Bit 11:      DACV 7   (2048mV)
Bit 10:      DACV 6   (1024mV)
Bit 9:       DACV 5   (512mV)
Bit 8:       DACV 4   (256mV)
Bit 7:       DACV 3   (128mV)
Bit 6:       DACV 2   (64mV)
Bit 5:       DACV 1   (32mV)
Bit 4:       DACV 0   (16mV)
Bits [3:0]:  Not used
```

**Register mask: 0x7FF0** (bits 14:4)

### Conversion

```
Register value = ((mV - 1024) / 16) << 4    (for mV >= 1024)
mV value = (register_value >> 4) * 16 + 1024
```

- Sending < 1.024V or > 19.2V clears the register and terminates charging.
- For 4S LiPo: 16.8V charge voltage → register = ((16800 - 1024) / 16) << 4 = 0x3C00.

---

## 0x3F — InputCurrent() Register

POR: **0x1000** (4096mA)

Using 10mΩ sense resistor: range **128mA – 8064mA**, resolution **128mA/LSB**.

```
Bits [15:13]: Not used
Bit 12:      DACIIN 5  (4096mA)
Bit 11:      DACIIN 4  (2048mA)
Bit 10:      DACIIN 3  (1024mA)
Bit 9:       DACIIN 2  (512mA)
Bit 8:       DACIIN 1  (256mA)
Bit 7:       DACIIN 0  (128mA)
Bits [6:0]:  Not used
```

**Register mask: 0x1F80** (bits 12:7)

### Conversion

```
Register value = ((mA - 128) / 128) << 7    (for mA >= 128)
mA value = (register_value >> 7) * 128 + 128
```

- Sending < 128mA or > 8064mA clears the register and terminates charging.
- If input current exceeds 108% of set point, charger shuts down immediately.
- Suggested minimum: 512mA.

---

## 0xFE — ManufacturerID() (Read-Only)

Always returns **0x0040**.

## 0xFF — DeviceID() (Read-Only)

Always returns **0x0008**.

---

## Key Operating Notes

### Watchdog Timer
- Must periodically write ChargeVoltage() or ChargeCurrent() to refresh.
- If timeout occurs: all registers kept, charging suspended.
- Write either register to resume charging.
- After watchdog timeout, writing ChargeOption() bit[14:13]=00 disables watchdog AND resumes charging.

### Charging Conditions (all must be met)
1. ChargeOption() bit[0] = 0 (charge enabled)
2. ILIM pin > 105mV
3. All three DACs (charge current, charge voltage, input current) have valid values
4. ACOK is valid (ACDET between 2.4V and 3.15V, VCC > UVLO, VCC-VSRN > 275mV)
5. ACFET on, RBFET on, gate voltage high enough
6. VSRN does not exceed BATOVP threshold
7. IC temperature < TSHUT (155°C)
8. Not in ACOC condition

### Charge Suspension Triggers
- ChargeOption() bit[0] = 1 (inhibit)
- ILIM pin < 75mV
- Any DAC set to 0 or out of range
- ACOK pulled low
- ACFET turns off
- VSRN exceeds BATOVP
- TSHUT reached
- ACOC detected
- Short circuit detected
- Watchdog timeout (if enabled)

### IOUT Pin
- Analog output: 20x amplified current through sense resistor.
- Bit[5]=0: adapter current (ACP-ACN); bit[5]=1: charge current (SRP-SRN).
- 100pF decoupling capacitor recommended.

### LEARN Cycle
- Set bit[6]=1 to start. IC turns off ACFET, turns on BATFET to discharge battery.
- When battery voltage hits depletion threshold (bits[12:11]), BATFET off, ACFET on.
- Bit[6] auto-resets to 0 after cycle completes.
