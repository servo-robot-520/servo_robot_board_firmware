# 伺服机器人电源管理板固件

**中文** | [English](README_EN.md)

基于 STM32F411 的嵌入式 Rust 固件，为四足/双足伺服机器人提供一体化电源管理、传感器采集和通信方案。

## 硬件平台

| 组件 | 型号 | 接口 | 用途 |
|------|------|------|------|
| MCU | STM32F411CEU6 (Cortex-M4F, 96MHz) | — | 主控 |
| USB PD 控制器 | HUSB238A | I2C1 (0x42) | USB PD 受电 |
| 充电控制器 | BQ24725 | I2C1 (0x09) | CC/CV 充电 |
| 电池电量计 | BQ40Z50 | I2C1 (0x0B) | 4S 智能电量计 |
| 电源监控 | INA219 | I2C1 (0x40) | 舵机母线电压/电流 |
| IMU | MPU6500 | SPI1 (1MHz) | 6 轴姿态传感器 |
| LED | WS2812 × 3 | TIM1 CH2 + DMA | 状态指示 |
| 蜂鸣器 | 5025 | TIM2 CH2 | 音频反馈 |

## 仓库结构

```
├── crates/
│   ├── servo-robot-board/           # 主应用固件
│   │   ├── src/
│   │   │   ├── main.rs              # RTIC 应用入口
│   │   │   ├── features/            # 功能模块
│   │   │   │   ├── communication/   # 双通道上位机通信 (UART2 + USB CDC)
│   │   │   │   ├── sensing/         # 传感器采集 (IMU, 电池, 电源, 温度)
│   │   │   │   ├── charge/          # 充电状态机管理
│   │   │   │   ├── power/           # 电源输出控制与保护
│   │   │   │   ├── servo/           # 串口舵机命令转发
│   │   │   │   └── telemetry/       # 系统信息、日志、事件上报
│   │   │   └── platform/            # HAL 平台抽象 (ADC, Flash, UART, WS2812)
│   │   └── ...
│   ├── servo-robot-board-bootloader/ # OTA 引导程序 (16KB)
│   ├── embedded-ina219/              # INA219 电流/功率监测驱动
│   ├── embedded-mpu6500/             # MPU6500 6 轴 IMU 驱动
│   ├── embedded-husb238a/            # HUSB238A USB PD 受电驱动
│   ├── embedded-bq24725/             # BQ24725 充电控制器驱动
│   └── embedded-bq40z50/             # BQ40Z50 智能电量计驱动
├── memory.x                          # Flash/RAM 内存布局
└── .cargo/config.toml                # 编译目标配置
```

## Flash 分区

| 区域 | 起始地址 | 大小 | 用途 |
|------|----------|------|------|
| Bootloader | `0x0800_0000` | 16KB (Sector 0) | OTA 引导程序 |
| App Firmware | `0x0800_4000` | 240KB (Sectors 1-5) | 主固件 |
| OTA Temp | `0x0804_0000` | 128KB (Sector 6) | OTA 临时存储 |
| User Data | `0x0806_0000` | 128KB (Sector 7) | 配置 + OTA 标志 |

## 核心功能

### 电源管理
- **4S LiPo 充电**：BQ24725 CC/CV 充电状态机，支持温度降额保护
- **USB PD 受电**：HUSB238A PDO 发现与协商，中断事件驱动
- **多路输出控制**：舵机电源 (PC13)、5V (PC15)、电池对外输出 (PC14)、关机 (PB13)

### 监控系统
- **电池监控**：BQ40Z50 电量计 @ 10Hz — 电压、电流、SOC、温度、电芯电压
- **电源监控**：INA219 @ 20Hz — 舵机母线电压/电流 + ADC 充电参数
- **IMU 姿态**：MPU6500 @ 100Hz — 加速度、角速度、Mahony AHRS 四元数/欧拉角
- **温度监控**：3 路 NTC + MCU 内部温度 @ 1Hz

### 保护系统
- **过流保护**：舵机电流超限持续 30 秒 → 自动切断
- **过温保护**：舵机/5V 温度超限持续 30 秒 → 自动切断
- **风扇控制**：温度触发开/关，5°C 滞回防抖

### 通信协议
- **双通道**：UART2 (PA2/PA3) + USB CDC (PA11/PA12)，统一二进制帧协议
- **帧格式**：`[HEAD=0xAA][TYPE:1][LEN:2LE][PAYLOAD:N][CRC16-CCITT:2]`
- **上行数据**：IMU (100Hz)、电源 (20Hz)、电池 (10Hz)、系统信息 (1Hz)、事件/日志（按需）
- **下行命令**：配置写入、配置查询、舵机转发、固件升级、系统命令（重启/关机/OTA）

### 串口舵机转发
- **通道**：USART1 (PA15/PA10)，半双工，TX 方向控制 (PB12)
- **转发**：上位机 → `ServoForward (0x83)` → 固件 → UART1 原始字节 → 舵机
- **响应**：舵机 → UART1 → 固件 → `AckServoCmd (0xC3)` → 上位机

### OTA 固件升级
- **协议传输**：`FirmwareUpdate (0x84)` 分包写入 OTA Temp，`Command(Ota)` 触发更新
- **USB MSD 拖拽**：FAT12 虚拟 U 盘，拖入 `FIRMWARE.BIN` 即可升级
- **Bootloader**：验证镜像头 (magic, CRC32) → 复制 OTA Temp → App → 跳转

### 状态显示
- **WS2812 LED × 3**：充电温度 / 电池 SOC / 电池温度，渐变色指示
- **蜂鸣器**：R2-D2 风格启动旋律，任意频率/时长音调

## 构建

需要 Rust nightly 工具链和 `thumbv7em-none-eabihf` 目标。

```bash
# 构建主固件
cargo build -p servo-robot-board

# 构建 bootloader
cargo build -p servo-robot-board-bootloader

# 构建整个工作区
cargo build --workspace

# 启用串口舵机功能
cargo build -p servo-robot-board --features servo
```

## 代码检查

```bash
# 格式检查
cargo fmt --all -- --check

# Clippy 静态分析
cargo clippy --workspace --all-targets

# 运行宿主机单元测试
cargo test --workspace --target x86_64-unknown-linux-gnu
```

## 烧录与调试

使用 [probe-rs](https://probe.rs/) 或 ST-Link 烧录：

```bash
# 烧录主固件
probe-rs download --chip STM32F411CEU target/thumbv7em-none-eabihf/release/servo-robot-board

# RTT 日志查看
probe-rs attach --chip STM32F411CEU
```

## 驱动 crate

每个硬件驱动均为独立的 `#![no_std]` 库 crate，可单独复用：

| crate | 芯片 | 功能 |
|-------|------|------|
| `embedded-ina219` | TI INA219 | 双向电流/功率监测 |
| `embedded-mpu6500` | InvenSense MPU6500 | 6 轴 IMU |
| `embedded-husb238a` | Hynetek HUSB238A | USB PD 受电 |
| `embedded-bq24725` | TI BQ24725 | CC/CV 充电控制器 |
| `embedded-bq40z50` | TI BQ40Z50-R1 | 4S 智能电量计 |

各驱动的详细用法见对应 crate 目录下的 `README.md`。

## 许可证

[GPL-3.0](LICENSE)
