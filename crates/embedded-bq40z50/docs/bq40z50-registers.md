# BQ40Z50 寄存器参考

基于 TI BQ40Z50-R1 手册 Chapter 12 (SLUUA43A — December 2013 — Revised May 2015)。

---

## 1. 标准 SBS 命令

通过 SMBus Word/Block 读写，直接访问 `0x00`-`0x3F` 地址。

### 1.1 ManufacturerAccess (0x00) / ManufacturerBlockAccess (0x44)

MAC 子命令通过 `ManufacturerAccess` (0x00) 写入子命令码，然后从 `ManufacturerBlockAccess` (0x44) 或 `ManufacturerData` (0x23) 读取结果。

> 注意：向 0x00 写入时以 SMBus Word Write 发送（小端），读取时以 SMBus Block Read 从 0x44 读取。

### 1.2 标准 SBS 命令表

| 地址 | 命令名 | 类型 | 单位 | 说明 |
|------|--------|------|------|------|
| 0x00 | ManufacturerAccess | R/W | — | 写入 MAC 子命令 |
| 0x01 | RemainingCapacityAlarm | R/W | mAh/10mWh | 低容量报警阈值 |
| 0x02 | RemainingTimeAlarm | R/W | min | 低剩余时间报警阈值 |
| 0x03 | BatteryMode | R/W | — | 电池操作模式选项 |
| 0x04 | AtRate | R/W | mA/10mW | 设置 AtRate 计算值 |
| 0x05 | AtRateTimeToFull | R | min | 按 AtRate 充满时间 (65535=未充电) |
| 0x06 | AtRateTimeToEmpty | R | min | 按 AtRate 放空时间 (65535=未放电) |
| 0x07 | AtRateOK | R | bool | 电池能否以 AtRate 持续 10s |
| 0x08 | Temperature | R | 0.1°K | 综合温度 |
| 0x09 | Voltage | R | mV | 所有电芯电压总和 |
| 0x0A | Current | R | mA (signed) | 库仑计电流 |
| 0x0B | AverageCurrent | R | mA (signed) | 平均电流 |
| 0x0C | MaxError | R | % | SOC 计算最大误差 (1~100) |
| 0x0D | RelativeStateOfCharge | R | % | 相对 SOC |
| 0x0E | AbsoluteStateOfCharge | R | % | 绝对 SOC |
| 0x0F | RemainingCapacity | R | mAh/10mWh | 剩余容量 |
| 0x10 | FullChargeCapacity | R | mAh/10mWh | 满充容量 |
| 0x11 | RunTimeToEmpty | R | min | 当前放电率下放空时间 |
| 0x12 | AverageTimeToEmpty | R | min | 平均放电率下放空时间 |
| 0x13 | AverageTimeToFull | R | min | 平均充电率下充满时间 |
| 0x14 | ChargingCurrent | R | mA | 推荐充电电流 |
| 0x15 | ChargingVoltage | R | mV | 推荐充电电压 |
| 0x16 | BatteryStatus | R | — | 电池状态标志 |
| 0x17 | CycleCount | R/W | cycles | 放电循环次数 |
| 0x18 | DesignCapacity | R/W | mAh/10mWh | 设计容量 |
| 0x19 | DesignVoltage | R/W | mV | 设计电压 |
| 0x1A | SpecificationInfo | R/W | — | SBS 规格版本信息 |
| 0x1B | ManufacturerDate | R/W | — | 制造日期 (Day+Month\*32+(Year-1980)\*256) |
| 0x1C | SerialNumber | R/W | — | 电池包序列号 |
| 0x20 | ManufacturerName | R | Block | 制造商名称 ("Texas Inst.") |
| 0x21 | DeviceName | R | Block | 设备名称 ("bq40z50") |
| 0x22 | DeviceChemistry | R | Block | 电池化学类型 ("LION"/"LIPO"/"LIFE") |
| 0x23 | ManufacturerData | R | Block | MAC 响应数据 / ManufacturerInfo |
| 0x2F | Authenticate | R/W | Block | SHA-1 认证挑战/响应 |
| 0x3C | CellVoltage4 | R | mV | 电芯 4 电压 |
| 0x3D | CellVoltage3 | R | mV | 电芯 3 电压 |
| 0x3E | CellVoltage2 | R | mV | 电芯 2 电压 |
| 0x3F | CellVoltage1 | R | mV | 电芯 1 电压 |

### 1.3 BatteryStatus (0x16) 位定义

```
Bit 15: OCA  — 过充报警 (Overcharged Alarm)
Bit 14: TCA  — 终止充电报警 (Terminate Charge Alarm)
Bit 13: RSVD — 保留
Bit 12: OTA  — 过温报警 (Overtemperature Alarm)
Bit 11: TDA  — 终止放电报警 (Terminate Discharge Alarm)
Bit 10: RSVD — 保留
Bit 9:  RCA  — 剩余容量报警 (Remaining Capacity Alarm)
Bit 8:  RTA  — 剩余时间报警 (Remaining Time Alarm)
Bit 7:  INIT — 初始化完成标志 (1=完成, 0=进行中)
Bit 6:  DSG  — 放电/休眠 (1=放电或休眠, 0=充电)
Bit 5:  FC   — 完全充满 (Fully Charged)
Bit 4:  FD   — 完全放电 (Fully Discharged)
Bit 3-0: EC3~EC0 — 错误码
              0x0 = OK
              0x1 = Busy
              0x2 = Reserved Command
              0x3 = Unsupported Command
              0x4 = AccessDenied
              0x5 = Overflow/Underflow
              0x6 = BadSize
              0x7 = UnknownError
```

### 1.4 BatteryMode (0x03) 位定义

```
Bit 15: CAPM — 容量模式 (0=mA/mAh, 1=10mW/10mWh)
Bit 14: CHGM — 充电器模式 (0=广播充电电压/电流, 1=禁用广播)
Bit 13: AM   — 报警模式 (0=广播报警, 1=禁用广播)
Bit 9:  PB   — 主电池 (0=副电池, 1=主电池)
Bit 8:  CC   — 充电控制器使能 (0=禁用, 1=使能)
Bit 7:  CF   — 条件标志 (R, 0=正常, 1=需要条件循环)
Bit 1:  PBS  — 主电池支持 (R)
Bit 0:  ICC  — 内部充电控制器 (R)
```

---

## 2. MAC 子命令 (ManufacturerAccess)

通过向 `ManufacturerAccess` (0x00) 写入 16 位子命令码触发，结果从 `ManufacturerBlockAccess` (0x44) 读取。

### 2.1 信息查询类

| 子命令 | 名称 | 读取格式 | 说明 |
|--------|------|----------|------|
| 0x0001 | DeviceType | Block | IC 型号 |
| 0x0002 | FirmwareVersion | Block | 固件版本 (ddDDwVVvvBBTTzzZZRREE) |
| 0x0003 | HardwareVersion | Block | 硬件版本 |
| 0x0004 | IFChecksum | Block | 指令 Flash 校验和 |
| 0x0005 | StaticDFSignature | Block | 静态 DF 签名 |
| 0x0006 | ChemID | Block | OCV 表化学 ID |
| 0x0008 | StaticChemDFSignature | Block | 静态化学 DF 签名 |
| 0x0009 | AllDFSignature | Block | 全部 DF 参数签名 |

### 2.2 安全/状态类 (Block Read, 32 位)

| 子命令 | 名称 | 说明 |
|--------|------|------|
| 0x0050 | SafetyAlert | 安全告警标志 (锁存) |
| 0x0051 | SafetyStatus | 安全状态标志 (活动) |
| 0x0052 | PFAlert | 永久故障告警 |
| 0x0053 | PFStatus | 永久故障状态 |
| 0x0054 | OperationStatus | 运行状态标志 |
| 0x0055 | ChargingStatus | 充电状态标志 |
| 0x0056 | GaugingStatus | 计量状态标志 |
| 0x0057 | ManufacturingStatus | 制造状态标志 |

### 2.3 SafetyAlert (0x0050) / SafetyStatus (0x0051) 位定义

```
Bit 27: UTD  — 放电欠温
Bit 26: UTC  — 充电欠温
Bit 25: PCHGC — 预充电过流
Bit 24: CHGV — 充电过压
Bit 23: CHGC — 充电过流
Bit 22: OC   — 过充
Bit 21: CTOS — 充电超时挂起 (仅 SafetyAlert)
Bit 20: CTO  — 充电超时
Bit 19: PTOS — 预充超时挂起 (仅 SafetyAlert)
Bit 18: PTO  — 预充超时
Bit 16: OTF  — FET 过温
Bit 14: CUVC — 电芯欠压补偿
Bit 13: OTD  — 放电过温
Bit 12: OTC  — 充电过温
Bit 11: ASCDL — 放电短路锁存 (SafetyAlert)
Bit 10: ASCD  — 放电短路 (SafetyStatus)
Bit 9:  ASCCL — 充电短路锁存 (SafetyAlert)
Bit 8:  ASCC  — 充电短路 (SafetyStatus)
Bit 7:  AOLDL — 放电过载锁存 (SafetyAlert)
Bit 6:  AOLD  — 放电过载 (SafetyStatus)
Bit 5:  OCD2  — 放电过流 2
Bit 4:  OCD1  — 放电过流 1
Bit 3:  OCC2  — 充电过流 2 (SafetyStatus) / RSVD (SafetyAlert)
Bit 2:  OCC1  — 充电过流 1
Bit 1:  COV   — 电芯过压
Bit 0:  CUV   — 电芯欠压
```

### 2.4 OperationStatus (0x0054) 位定义

```
Bit 29: EMSHUT — 紧急关断
Bit 28: CB     — 电芯均衡状态
Bit 27: SLPCC  — SLEEP 模式 CC 测量
Bit 26: SLPAD  — SLEEP 模式 ADC 测量
Bit 25: SMBLCAL — 自动 CC 校准
Bit 24: INIT   — 全复位后初始化
Bit 23: SLEEPM — 命令触发的 SLEEP 模式
Bit 22: XL     — 400kHz SMBus 模式
Bit 21: CAL_OFFSET — 校准输出 (CC 偏移)
Bit 20: CAL    — 校准输出 (ADC+CC)
Bit 19: AUTOCALM — 自动 CC 偏移校准
Bit 18: AUTH   — 认证进行中
Bit 17: LED    — LED 显示
Bit 16: SDM    — 命令触发关断
Bit 15: SLEEP  — SLEEP 条件满足
Bit 14: XCHG   — 充电禁用
Bit 13: XDSG   — 放电禁用
Bit 12: PF     — 永久故障模式
Bit 11: SS     — 安全模式
Bit 10: SDV    — 低电压关断
Bit 9-8: SEC1,SEC0 — 安全模式
              00=保留, 01=Full Access, 10=Unsealed, 11=Sealed
Bit 7:  BTP_INT — 电池断点中断
Bit 5:  FUSE   — 保险丝状态
Bit 3:  PCHG   — 预充 FET 状态
Bit 2:  CHG    — 充电 FET 状态
Bit 1:  DSG    — 放电 FET 状态
Bit 0:  PRES   — 系统在位
```

### 2.5 ChargingStatus (0x0055) 位定义

```
Bit 17: CCC  — 充电损耗补偿
Bit 16: CVR  — 充电电压变化率
Bit 15: CCR  — 充电电流变化率
Bit 14: VCT  — 充电终止
Bit 13: MCHG — 维护充电
Bit 12: IN   — 充电禁止
Bit 11: HV   — 高压区域
Bit 10: MV   — 中压区域
Bit 9:  LV   — 低压区域
Bit 8:  PV   — 预充电压区域
Bit 6:  OT   — 过温区域
Bit 5:  HT   — 高温区域
Bit 4:  STH  — 标准高温区域
Bit 3:  RT   — 推荐温度区域
Bit 2:  STL  — 标准低温区域
Bit 1:  LT   — 低温区域
Bit 0:  UT   — 欠温区域
```

### 2.6 GaugingStatus (0x0056) 位定义

```
Bit 20: OCVFR — OCV 平坦区域 (RELAX 期间)
Bit 19: LDMD  — 负载模式 (1=恒功率, 0=恒流)
Bit 18: RX    — 电阻更新标志 (每次更新翻转)
Bit 17: QMax  — QMax 更新标志 (每次更新翻转)
Bit 16: VDQ   — 放电合格学习 (R_DIS 取反)
Bit 15: NSFM  — 负缩放因子模式
Bit 13: SLPQMax — SLEEP 模式 OCV 更新
Bit 12: QEN   — 阻抗跟踪计量使能
Bit 11: VOK   — 电压 OK (QMax 更新就绪)
Bit 10: R_DIS — 电阻更新禁用
Bit 8:  REST  — OCV 读取完成
Bit 7:  CF    — 条件标志 (MaxError > 限制)
Bit 6:  DSG   — 放电/休眠
Bit 5:  EDV   — 放电终止电压到达
Bit 4:  BAL_EN — 电芯均衡可能
Bit 3:  TC    — 终止充电
Bit 2:  TD    — 终止放电
Bit 1:  FC    — 完全充满
Bit 0:  FD    — 完全放电
```

### 2.7 数据块读取类

| 子命令 | 名称 | 字节数 | 说明 |
|--------|------|--------|------|
| 0x0060 | LifetimeDataBlock1 | 32 | 电芯极值电压、电流、温度 |
| 0x0061 | LifetimeDataBlock2 | 16 | 关机/复位次数、均衡时间 |
| 0x0062 | LifetimeDataBlock3 | 16 | 各温度区域运行时间 |
| 0x0063 | LifetimeDataBlock4 | 32 | COV/CUV/OCD/OCC/AOLD/ASCD 事件计数 |
| 0x0064 | LifetimeDataBlock5 | 32 | ASCC/OTC/OTD/OTF 事件、Qmax/Ra 更新计数 |
| 0x0070 | ManufacturerInfo | 32 | 制造商信息 |
| 0x0071 | DAStatus1 | 32 | 电芯电压、PACK 电压、电流、功率 |
| 0x0072 | DAStatus2 | 14 | 温度详情 (7 × 0.1°K) |
| 0x0073 | GaugeStatus1 | 32 | IT 计量详情 (剩余容量/Qmax/Ra 等) |
| 0x0074 | GaugeStatus2 | 32 | Grid 点、DOD、状态时间 |
| 0x0075 | GaugeStatus3 | 24 | QMax 值、DOD0、热模型参数 |
| 0x0076 | CBStatus | 8 | 电芯均衡时间 (4 × Cell) |
| 0x0077 | StateOfHealth | 4 | SOH FCC (mAh) + 能量 (cWh) |
| 0x0078 | FilteredCapacity | 8 | 滤波后剩余/满充容量 |

### 2.8 DAStatus2 (0x0072) 详细格式

向 `ManufacturerAccess` (0x00) 写入 `0x0072`，然后从 `ManufacturerBlockAccess` (0x44) 读取 **14 字节**：

| 偏移 | 长度 | 值 | 单位 |
|------|------|-----|------|
| 0-1 | 2 | Int Temperature | 0.1°K |
| 2-3 | 2 | TS1 Temperature | 0.1°K |
| 4-5 | 2 | TS2 Temperature | 0.1°K |
| 6-7 | 2 | TS3 Temperature | 0.1°K |
| 8-9 | 2 | TS4 Temperature | 0.1°K |
| 10-11 | 2 | Cell Temperature | 0.1°K |
| 12-13 | 2 | FET Temperature | 0.1°K |

### 2.9 控制/写入类子命令

| 子命令 | 名称 | 说明 |
|--------|------|------|
| 0x0010 | ShutdownMode | 进入关断模式 (SHIP) |
| 0x0011 | SleepMode | 进入 SLEEP 模式 |
| 0x0013 | AutoCCOffset | 手动自动 CC 偏移校准 (~16s) |
| 0x001D | FuseToggle | 激活/去激活 FUSE 输出 |
| 0x001E | PCHG_FET_Toggle | 预充 FET 开关 (测试) |
| 0x001F | CHG_FET_Toggle | 充电 FET 开关 (测试) |
| 0x0020 | DSG_FET_Toggle | 放电 FET 开关 (测试) |
| 0x0021 | Gauging | 使能/禁用计量功能 |
| 0x0022 | FETControl | 使能/禁用固件 FET 控制 |
| 0x0023 | LifetimeDataCollection | 使能/禁用寿命数据采集 |
| 0x0024 | PermanentFailure | 使能/禁用永久故障保护 |
| 0x0025 | BlackBoxRecorder | 使能/禁用黑盒记录 |
| 0x0026 | Fuse | 使能/禁用固件保险丝 |
| 0x0028 | LifetimeDataReset | 重置寿命数据 |
| 0x0029 | PermanentFailureDataReset | 重置 PF 数据 |
| 0x002A | BlackBoxRecorderReset | 重置黑盒数据 |
| 0x002D | CalibrationMode | 使能/禁用校准模式 |
| 0x002E | LifetimeDataFlush | 刷写 RAM 寿命数据到 Flash |
| 0x0030 | SealDevice | 密封设备 (禁用部分命令) |
| 0x0035 | SecurityKeys | 读写 UNSEAL/FULL ACCESS 密钥 |
| 0x0041 | DeviceReset | 设备复位 |

### 2.10 安全模式说明

| SEC1 | SEC0 | 模式 | 说明 |
|------|------|------|------|
| 0 | 0 | Reserved | — |
| 0 | 1 | Full Access | 完全访问，可修改配置 |
| 1 | 0 | Unsealed | 解封，可访问大部分命令 |
| 1 | 1 | Sealed | 密封，仅标准 SBS 命令 |

默认 UNSEAL 密钥: `0x0414`, `0x3672`
默认 FULL ACCESS 密钥: `0xFFFF`, `0xFFFF`

---

## 3. 数据 Flash 访问 (0x4000~0x5FFF)

通过 `ManufacturerBlockAccess` (0x44) 访问物理地址。写入格式：`[2字节起始地址] + [DF数据块]`。读取：先写入起始地址，再从 0x44 读取 32 字节。支持地址自动递增。


