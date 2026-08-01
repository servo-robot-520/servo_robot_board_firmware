# HUSB238A Register Reference

Based on HUSB238ARegisterInformationRev0.1 (Hynetek Semiconductor, 05/2023).

## Overview

The HUSB238A is a USB PD Sink Controller with I2C interface (7-bit address: 0x42 with ADDR=GND, 0x62 with ADDR=VDD).
Registers are organized into two groups:

- **User Configuration Registers** (0x01–0x22): Control, mask, interrupt, PDO selection, request parameters
- **Status Registers** (0x63–0x91): Read-only status, source capability info, VDM data, manufacturer info

## User Configuration Registers

### CONTROL (0x01) — Global Interrupt Control

| Bit | Field | Description |
|-----|-------|-------------|
| 7:1 | Reserved | — |
| 0 | INT_MASK | Global interrupt mask. 1=mask all, 0=controlled by MASK/MASK1 registers. Default: 1 |

### CONTROL1 (0x02) — Mode Control

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | Reserved | — |
| 5 | EN_DPM_HIZ | D+/D- disconnect when DCP not detected. 0=keep connected, 1=disconnect. Default: 0 |
| 4 | VDM_RESPOND | VDM SOP message response. 0=NAK, 1=ACK. Default: 0 |
| 3 | ENABLE | HUSB238A enable in I2C mode. 1=enable, 0=disable (push to I2CDisable State). Default: 0 |
| 2:0 | TCCDEB | Attach debounce time. 000=120ms, 001=130ms, ..., 110=180ms, 111=Reserved. Default: 011 (150ms) |

### MANUAL (0x03) — Force State Control

| Bit | Field | Description |
|-----|-------|-------------|
| 7:6 | Reserved | — |
| 5 | FORCE_DPM_HIZ | Disconnect D+/D- from internal circuitry (R/W) |
| 4 | Reserved | — |
| 3 | UNATT_SNK | Jump to Unattached.SNK state (WC — write-1-to-clear) |
| 2 | Reserved | — |
| 1 | DISABLED | Jump to Disabled state (R/W). 0b: exit Disabled and enter ErrorRecovery |
| 0 | ERROR_REC | Jump to ErrorRecovery state (WC) |

**Note:** Write-1-to-clear (WC) bits self-clear after execution. Bit[1] (DISABLED) has highest priority among WC bits. ERROR_REC > UNATT_SNK > FORCE_DPM_HIZ priority order.

### RESET (0x04) — Chip Reset

| Bit | Field | Description |
|-----|-------|-------------|
| 7:1 | Reserved | — |
| 0 | SW_RES | Software reset. Write 1b to jump to initialization state (WC). Read returns 0b. |

### MASK (0x05) — Interrupt Mask for INT_N Pin (Group 1)

Each bit: 1=mask (DO NOT assert INT_N), 0=unmask (assert INT_N on event). Default: all 0.

| Bit | Field | Source Interrupt |
|-----|-------|-----------------|
| 7 | M_FLGIN | I_FLGIN — FLGIN in STATUS1 changed from 0b to 1b |
| 6 | M_ORIENT | I_ORIENT — ORIENT in STATUS changed from 00b to 01b/10b |
| 5 | M_FAULT | I_FAULT — FAULT1 or FAULT2 is set |
| 4 | M_VBUS_CHG | I_VBUS_CHG — VBUS_OK transitions 0→1 or 1→0 |
| 3 | M_VBUS_OV | I_VBUS_OV — VBUS_OV fault is set |
| 2 | M_BC_LVL | I_BC_LVL — BC_LVL in STATUS changed |
| 1 | M_DETACH | I_DETACH — Exit from Attached.SNK/DebugAccessory.SNK |
| 0 | M_ATTACH | I_ATTACH — Entry into Attached.SNK/DebugAccessory.SNK |

### MASK1 (0x06) — Interrupt Mask for INT_N Pin (Group 2)

| Bit | Field | Source Interrupt |
|-----|-------|-----------------|
| 7 | M_TSD | I_TSD — Thermal shutdown |
| 6 | M_VBUS_UV | I_VBUS_UV — VBUS under-voltage |
| 5 | M_DR_ROLE | I_DR_ROLE — Data Role changed |
| 4 | Reserved | — |
| 3 | M_SRC_ALERT | I_SRC_ALERT — Alert Message from connected source |
| 2 | M_FRC_FAIL | I_FRC_FAIL — FORCE_SNK has failed |
| 1 | M_FRC_SUCC | I_FRC_SUCC — FORCE_SNK has been done |
| 0 | M_VDM_MSG | I_VDM_MSG — VDM message received |

### MASK2 (0x07) — Interrupt Mask for INT_N Pin (Group 3)

**Note:** PDF page 5 labels this as "MASK1 (Address: 0x07)" which is a documentation error — this is the third mask register.

| Bit | Field | Source Interrupt |
|-----|-------|-----------------|
| 7:4 | Reserved | — |
| 3 | M_Exit_EPR | I_Exit_EPR — Exit EPR interruption |
| 2 | M_Go_Fail | I_Go_Fail — Go fail interruption |
| 1 | M_EPR_MODE | I_EPR_MODE — VDM_MODE interruption (PDF typo: "M_EPR_MDOE") |
| 0 | M_PD_HV | I_PD_HV — PD High Voltage Request done |

### INTERRUPT (0x09) — Interrupt Status (Group 3, Write-1-to-Clear)

| Bit | Field | Description |
|-----|-------|-------------|
| 7:4 | Reserved | — |
| 3 | I_Exit_EPR | Exit EPR interruption occurred |
| 2 | I_Go_Fail | Go fail interruption occurred |
| 1 | I_EPR_MODE | EPR mode has been entered |
| 0 | I_PD_HV | PD High Voltage Request is done |

### INTERRUPT1 (0x0A) — Interrupt Status (Group 1, Write-1-to-Clear)

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | I_FLGIN | FLGIN in STATUS1 changed from 0b to 1b |
| 6 | I_ORIENT | ORIENT in STATUS changed from 00b to 01b/10b |
| 5 | I_FAULT | FAULT1 or FAULT2 is set |
| 4 | I_VBUS_CHG | VBUS_OK transitions 0→1 or 1→0 |
| 3 | I_VBUS_OV | VBUS_OV fault is set |
| 2 | I_BC_LVL | BC_LVL in STATUS changed |
| 1 | I_DETACH | Exit from Attached.SNK/DebugAccessory.SNK |
| 0 | I_ATTACH | Entry into Attached.SNK/DebugAccessory.SNK |

### INTERRUPT2 (0x0B) — Interrupt Status (Group 2, Write-1-to-Clear)

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | I_TSD | TSD is set |
| 6 | I_VBUS_UV | Valid VBUS_UV fault is set |
| 5 | I_DR_ROLE | Data Role changed |
| 4 | Reserved | — |
| 3 | I_SRC_ALERT | Alert Message received from connected source |
| 2 | I_FRC_FAIL | FORCE_SNK has failed |
| 1 | I_FRC_SUCC | FORCE_SNK has been done |
| 0 | I_VDM_MSG | VDM message received |

**Key behavior:** Interrupt bits are latched until cleared by writing 1b. Even if masked in MASK/MASK1, the interrupt bit still sets in the INTERRUPT register — only the INT_N pin assertion is suppressed.

### USER_CFG0 (0x0C)

| Bit | Field | Description |
|-----|-------|-------------|
| 7:6 | TSNKDSCNT | Debounce: Attached.SNK → Unattached.SNK. 00=0ms, 01=5ms, 10=15ms, 11=30ms. Default: 10 |
| 5 | CC_DSCNTEN | CC disconnect monitoring. 0=disabled, 1=enabled |
| 4 | TFAULT | Fault debounce. 0=10µs, 1=1ms. Default: 0 |
| 3:2 | TVDSGTIMEOUT | VBUS_DSG max conduction time. 00=disable always, 01=50ms, 10=100ms, 11=200ms |
| 1:0 | TBC_LEVEL | BC_LVL debounce. 00=3ms, 01=12ms, 10=15ms, 11=18ms |

### USER_CFG1 (0x0D)

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | Reserved | — |
| 6 | EN_HVDCP | HVDCP detection. 0=only BC1.2, 1=HVDCP after BC1.2. Default: EN_HVDCP/OUT1 |
| 5:4 | Reserved | — |
| 3 | EN_VB_UV | VBUS UV detection. 0=disable, 1=enable. Default: 0 |
| 2 | EN_SVID3 | 3rd SVID in Discover SVIDs ACK. 0=not respond, 1=respond. Default: 0 |
| 1:0 | OUT2_SEL | FAULT/OUT2 pin function. 00=Fault Indication, 01=ID Indication, 10=Controlled by EN_OUT2, 11=Reserved |

### USER_CFG2 (0x0E)

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | EN_OUT2 | OUT2 output when OUT2_SEL=10b. 0=drive low, 1=drive high. Default: 0 |
| 6 | FLG_POLARITY | FAULT/OUT2 polarity. 0=normal, 1=inverted. Default: 0 |
| 5 | EN_FAULTIN | FLGGIN input action. 0=not turn off GATE, 1=turn off GATE (Hi-Z) immediately. Default: 0 |
| 4 | EN_OUT1 | OUT1 output. 0=drive low, 1=drive high. Default: 0 |
| 3 | Reserved | — |
| 2 | PD_PRIOR | PD priority. 0=low (delay 3s), 1=high (run PD PE after connection). Default: 0 |
| 1 | EN_DRS | DR_Swap response. 0=reject, 1=accept. Default: 1 |
| 0 | Reserved | — |

### USER_CFG3 (0x0F)

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | Reserved | — |
| 6 | PPS_CAP_SNK | PPS Sink Capability. 0=not supported, 1=supported. Default: 0 |
| 5 | AVS_CAP_SNK | AVS Sink Capability. 0=not supported, 1=supported. Default: 0 |
| 4 | MODAL_OPERATION | SOP Discover Identity response. 0=NAK, 1=ACK for SVIDs/Modes/Enter/Exit. Default: 0 |
| 3 | EPR_AVS_CAP_SNK | EPR AVS Capability. 0=not supported, 1=supported. Default: 0 |
| 2 | SNK_CAP_MIN_VOLTAGE | Min Voltage in Sink_Capabilities PDO2. 0=5V, 1=3.3V. Default: 0 |
| 1:0 | SNK_PDO1_CURRENT | PDO1 current in Sink_Capabilities. 00=3A, 01=2.4A, 10=2.1A, 11=1.5A. Default: 00 |

### SVID/MODE Registers (0x10–0x17)

| Address | Field | Description |
|---------|-------|-------------|
| 0x10 | SVID0_MSB | Discover SVIDs VDO1 [31:24] |
| 0x11 | SVID0_LSB | Discover SVIDs VDO1 [23:16] |
| 0x12 | SVID1_MSB | Discover SVIDs VDO1 [16:8] |
| 0x13 | SVID1_LSB | Discover SVIDs VDO1 [7:0] |
| 0x14 | MODE0_MSB | Discover Modes VDO1 [23:16] |
| 0x15 | MODE0_LSB | Discover Modes VDO1 [7:0] |
| 0x16 | MODE1_MSB | Discover Modes VDO1 [23:16] |
| 0x17 | MODE1_LSB | Discover Modes VDO1 [7:0] |

### GO_COMMAND (0x18) — PDO Selection and Commands

| Bit | Field | Description |
|-----|-------|-------------|
| 7:5 | Reserved | — |
| 4:0 | GO | Command code (write-only). See command table below. |

**GO Command Codes:**

| Code | Command |
|------|---------|
| 00000b | None |
| 00001b | Set PDO_SELECT + GO to select target PDO (ignores VSET and SNK_PDO2) |
| 00010b | BIST data mode test |
| 00011b | BIST carrier mode test |
| 00100b | Get_SRC_Cap |
| 00101b | DR_Swap |
| 00110b | Get_PPS_Status |
| 00111b | Get_Manufacturer_Info |
| 01000b | Discover Identity |
| 01001b | Discover SVIDs |
| 01010b | Discover Modes (SVID0) |
| 01011b | Discover Modes (SVID1) |
| 01100b | Enter Mode (SVID0 & MODE0) |
| 01101b | Enter Mode (SVID1 & MODE1) |
| 01110b | Exit Mode (SVID0 & MODE0) |
| 01111b | Exit Mode (SVID1 & MODE1) |
| 10000b | EPR_Get_Source_Cap |
| 10001b | EPR_Mode Enter |
| 10010b | EPR_Mode Exit |
| 11101b | Soft Reset |
| 11110b | Hard Reset |

### SRC_PDO (0x19) — Source PDO Selection

| Bit | Field | Description |
|-----|-------|-------------|
| 7:3 | PDO_SELECT | Target SRC_PDO as RDO. 00000=not selected, 00001=5V, 00010=9V, ..., 11110=EPR_AVS |
| 2 | Reserved | — |
| 1:0 | SNK_PPS_VOL_M | High 2 bits of PPS request voltage (combined with SNK_PPS_VOL_L) |

**PDO_SELECT Codes:**

| Code | PDO |
|------|-----|
| 00000b | Not selected |
| 00001b | SRC_PDO_5V |
| 00010b | SRC_PDO_9V |
| 00011b | SRC_PDO_12V |
| 00100b | SRC_PDO_15V |
| 00101b | SRC_PDO_20V |
| 00110b | SRC_PDO_PPS1 |
| 00111b | SRC_PDO_PPS2 |
| 01000b | SRC_PDO_PPS3 |
| 01001b | SRC_PDO_AVS |
| 10000b | QC2_5V |
| 10001b | QC2_9V |
| 10010b | QC2_12V |
| 11000b | SRC_PDO_28V |
| 11010b | SRC_PDO_36V |
| 11100b | SRC_PDO_48V |
| 11110b | SRC_EPR_AVS |

### SNK_PPS_VOLTAGE (0x1A) — PPS Request Voltage

| Bit | Field | Description |
|-----|-------|-------------|
| 7:0 | SNK_PPS_VOL_L | Low 8 bits of PPS request voltage. 20mV/LSB, offset 3V. Range: 3.00V–23.46V |

Combined with SNK_PPS_VOL_M (SRC_PDO[1:0]) for a 10-bit voltage value.

### SNK_PPS_CURRENT (0x1B) — PPS Request Current

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | Reserved | — |
| 6:0 | SNK_PPS_CURRENT | Request operating current. 50mA/LSB. Range: 0A–6.35A |

### SNK_AVS_VOLTAGE (0x1C) — AVS Request Voltage

| Bit | Field | Description |
|-----|-------|-------------|
| 7:0 | SNK_AVS_VOL_L | Request output voltage. 100mV/LSB. Range: 0V–25.5V |

### SNK_AVS_CURRENT (0x1D) — AVS Request Current

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | SNK_AVS_VOL_M | High bit of AVS request voltage (combined with SNK_AVS_VOL_L) |
| 6:0 | SNK_AVS_CURRENT | Request operating current. 50mA/LSB. Range: 0A–6.35A |

### EPR_AVS_VOLTAGE (0x1E) — EPR AVS Request Voltage

| Bit | Field | Description |
|-----|-------|-------------|
| 7:0 | EPR_AVS_VOL_L | Low 8 bits. 100mV/LSB. Range: 0V–51.1V |

### EPR_AVS_CURRENT (0x20) — EPR AVS Request Current

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | EPR_AVS_VOL_M | High bit of EPR AVS voltage |
| 6:0 | EPR_AVS_CURRENT | 50mA/LSB. Range: 0A–6.35A |

### SNK_PDP (0x21) / EPR_PDP (0x22)

| Address | Field | Description |
|---------|-------|-------------|
| 0x21 | SNK_PDP [6:0] | Sink PDP value. 1W/LSB |
| 0x22 | SNK_EPR_PDP [7:0] | Sink EPR PDP value. 1W/LSB |

## Status Registers (Read-Only)

### STATUS (0x63)

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | AMS_PROCESS | 1=HUSB238A in AMS process |
| 6 | PD_EPR_SNK | 1=EPR Mode entered successfully |
| 5:4 | Reserved | — |
| 3 | TSD | 1=Thermal shutdown set |
| 2:1 | BC_LVL | CC line voltage level (2-bit field). See BC_LVL table below |
| 0 | ATTACH | 1=Attached.SNK/DebugAccessory.SNK |

**BC_LVL Values:**

| BC_LVL | DEF_COMP | 1P5_COMP | 3P0_COMP | State |
|--------|----------|----------|----------|-------|
| 00b | 0 | 0 | 0 | In Attached.SNK |
| 00b | X | X | X | Not in Attached.SNK |
| 01b | 1 | 0 | 0 | In Attached.SNK |
| 10b | 1 | 1 | 0 | In Attached.SNK |
| 11b | 1 | 1 | 1 | In Attached.SNK |

### STATUS1 (0x64)

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | FLGIN | 1=FLGIN is High, 0=Low |
| 6 | Reserved | — |
| 5 | PD_HV | 1=non-PDO1 PD contract established, 0=PDO1/no PD |
| 4 | PD_COMM | 1=PD communication detected |
| 3 | SRC_ALERT | 1=Alert message from source received |
| 2 | AMS_SUCC | 1=GO_COMMAND executed successfully, 0=not executed (in AMS) |
| 1 | FAULT | 1=FAULT1_COMP or FAULT2_COMP set |
| 0 | DATA_ROLE | 1=DFP, 0=UFP |

### TYPE (0x65)

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | CC_RX_ACTIVE | 1=CC line NOT in PD-idle |
| 6 | Reserved | — |
| 5 | DEBUGSNK | 1=DebugAccessory.SNK state |
| 4 | SINK | 1=Attached.SNK state |
| 3:0 | Reserved | — |

### DPDM_STATUS (0x66)

| Bit | Field | Description |
|-----|-------|-------------|
| 7:3 | DPDM_STATUS | Legacy charger protocol. 0x00=Unattached, 0x01=Unattached, 0x02=Divider-3, 0x03=BC1.2, 0x04=Reserved, 0x05=QC2, 0x06=Hi-Z, 0x07–0x7F=Reserved |
| 2 | CDP_FLAG | 1=CDP mode supported |
| 1 | SDP_FLAG | 1=SDP mode supported |
| 0 | DIVIDER3_FLAG | 1=DIVIDER3 mode supported |

### CONTRACT_STATUS0 (0x67)

| Bit | Field | Description |
|-----|-------|-------------|
| 7:4 | PD_CONTRACT | Current PD contract. 0000=5V type-C, 0001=5V, 0010=9V, ..., 1011=48V (EPR PDO3), 1101=EPR_AVS |
| 3:0 | DPM_CONTRACT | Current DPM contract. 0000=5V Default, 0001=Divider3, 0010=SDP, 0011=CDP, 0100=DCP, 0101=HVDCP, 0110=QC2 9V, 0111=QC2 12V |

### CONTRACT_STATUS1 (0x68) — Operating Current

**For PD contracts (FPDO, checked by PD_CONTRACT):**

| Range | Resolution | Offset |
|-------|-----------|--------|
| 0x00–0x7D | 20mA/LSB | 500mA |
| 0x7E–0xFF | 40mA/LSB | 500mA (continues from 3000mA) |

**For APDO contracts (PPS/AVS, checked by PD_CONTRACT):**

| Range | Resolution | Offset |
|-------|-----------|--------|
| 0x00–0xFF | 50mA/LSB | 0mA |

### SourceCap_INFO (0x69)

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | Reserved | — |
| 6 | VDM_MODE | 1=VDM Mode activated |
| 5 | Power Limit | PPS PDO [27] of received Source_Capabilities. 1=DRP supported |
| 4 | Dual-Role Power | PDO1 [29]. 1=DRP supported |
| 3 | USB Suspend Supported | PDO1 [28]. 1=USB Suspend supported |
| 2 | USB Communications Capable | PDO1 [26]. 1=USB Comm supported |
| 1 | Dual-Role Data | PDO1 [25]. 1=DRD supported |
| 0 | EPR Mode Capable | PDO1 [23]. 1=EPR Mode supported |

### SRC_PDO_5V–48V (0x6A–0x71) — Source PDO Detection

Each register has the same format:

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | DETECT | 0=not detected, 1=PDO detected in received Source_Capabilities |
| 6:0 | CURRENT | Max current. 100mA/LSB. Range: 0A–8.0A |

**Voltage ranges per register:**

| Register | Voltage Range |
|----------|--------------|
| SRC_PDO_5V (0x6A) | PDO1 |
| SRC_PDO_9V (0x6B) | 8V–10V |
| SRC_PDO_12V (0x6C) | 11V–13V |
| SRC_PDO_15V (0x6D) | 14V–18V |
| SRC_PDO_20V (0x6E) | 19V–21V |
| SRC_PDO_28V (0x6F) | 22V–28V (EPR) |
| SRC_PDO_36V (0x70) | 29V–36V (EPR) |
| SRC_PDO_48V (0x71) | 37V–48V (EPR) |

### SRC_PDO_PPS1–PPS3 (0x72–0x74) — PPS Detection

Same format as FPDO registers (DETECT + CURRENT at 100mA/LSB).

### SRC_PPS_VOLTAGE (0x75) — PPS Voltage Info

| Bit | Field | Description |
|-----|-------|-------------|
| 7:6 | PPS1_MAX_VOLTAGE | PPS1 max voltage. 00=5.9V(0–7V), 01=11V(7.02–12V), 10=16V(12.02–17V), 11=21V(>17.02V) |
| 5:4 | PPS2_MAX_VOLTAGE | PPS2 max voltage (same encoding) |
| 3:2 | PPS3_MAX_VOLTAGE | PPS3 max voltage (same encoding) |
| 1:0 | PPS_MIN_VOLTAGE | Max of all PPS min voltages. 00=3V(0–3.14V), 01=3.3V(3.16–3.46V), 10=5V(>3.46V), 11=Reserved |

### SRC_PDO_AVS (0x76) — AVS Detection

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | AVS_DETECT | 1=AVS PDO detected (PDO[31:28]=1110b) |
| 6:3 | AVS_MAX_VOLTAGE | 1V/LSB, 5V offset. 0000=5V(0–5V), ..., 1111=20V(>19.1V) |
| 2 | Reserved | — |
| 1:0 | AVS_MIN_VOLTAGE | 00=5V(0–6V), 01=9V(6.1–9V), 10=15V(>9.1V), 11=Reserved |

### SRC_AVS_PDP (0x77) / EPR_AVS_PDP (0x78)

| Address | Field | Description |
|---------|-------|-------------|
| 0x77 | SRC_AVS_PDP [7:0] | AVS PDP. 1W/LSB. Range: 0–255W |
| 0x78 | EPR_AVS_PDP [7:0] | EPR AVS PDP. 1W/LSB. Range: 0–255W |

### SRC_EPR_AVS (0x79) — EPR AVS Detection

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | EPR_AVS_DETECT | 1=EPR AVS PDO detected (PDO[31:28]=1101b) |
| 6:2 | EPR_AVS_MAX_VOLTAGE | 1V/LSB, 20V offset. 00000=20V(0–20V), ..., 11111=52V(>51.1V) |
| 1:0 | EPR_AVS_MIN_VOLTAGE | Same encoding as SRC_PDO_AVS MIN_VOLTAGE |

### VDM Registers (0x7A–0x86)

| Address | Field | Description |
|---------|-------|-------------|
| 0x7A | VDM_HEADER | [7:5]=Object Position, [4:3]=Command Type (00=REQ, 01=ACK, 10=NAK, 11=BUSY), [2:0]=VDM_TYPE |
| 0x7B–0x7E | VDM_VDO1_0–3 | VDO1 bytes [7:0], [15:8], [23:16], [31:24] |
| 0x7F–0x82 | VDM_VDO2_0–3 | VDO2 bytes |
| 0x83–0x86 | VDM_VDO3_0–3 | VDO3 bytes |

### VBUS_MEASUREMENT (0x87)

| Bit | Field | Description |
|-----|-------|-------------|
| 7:0 | VBUS_MEA | Sample VBUS voltage. 125mV/LSB. Range: 0V–31.875V |

### SRC_ALERT (0x88) — Alert Data Object

| Bit | Field | Description |
|-----|-------|-------------|
| 7 | EXTENDED | Alert Data Object [31] |
| 6 | OVP_EVENT | Alert Data Object [30] |
| 5 | SRC_INPUT | Alert Data Object [29] |
| 4 | OP_CHANGE | Alert Data Object [28] |
| 3 | OTP_EVENT | Alert Data Object [27] |
| 2 | OCP_EVENT | Alert Data Object [26] |
| 1 | BATTERY_STATUS | Alert Data Object [25] |
| 0 | Reserved | Alert Data Object [24] |

### SRC_PPS_STATUS (0x89–0x8B)

| Address | Field | Description |
|---------|-------|-------------|
| 0x89 | SRC_PPS_VOL_L [7:0] | Low 8 bits of source output voltage. 20mV/LSB, offset 3V |
| 0x8A | SRC_PPS_CURRENT [6:0] | Source output current. 50mA/LSB. Range: 0–6.35A |
| 0x8B | SRC_PPS_STATUS_FLAG | [7:6]=VOL_M (high 2 bits), [3]=OMF, [2:1]=PTF |

### Manufacturer_Info (0x8C–0x8F)

| Address | Field | Description |
|---------|-------|-------------|
| 0x8C | MNF_OFST_0 | Manufacturer_Info offset 0: Vendor ID |
| 0x8D | MNF_OFST_1 | Manufacturer_Info offset 1: Vendor ID |
| 0x8E | MNF_OFST_2 | Manufacturer_Info offset 2: Product ID |
| 0x8F | MNF_OFST_3 | Manufacturer_Info offset 3: Product ID |

### FSM State Registers (0x90–0x91)

| Address | Field | Description |
|---------|-------|-------------|
| 0x90 | Sink state [5:0] | Current state of SINK FSM |
| 0x91 | Source state [5:0] | Current state of SOURCE FSM |
