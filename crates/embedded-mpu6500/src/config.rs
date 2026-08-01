/// Digital low-pass filter configuration for gyroscope and temperature.
///
/// Only effective when FCHOICE_B = 00 in GYRO_CONFIG.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum DlpfConfig {
    /// 250Hz bandwidth, 8kHz internal Fs
    Dlpf250 = 0x00,
    /// 184Hz bandwidth, 1kHz internal Fs
    Dlpf184 = 0x01,
    /// 92Hz bandwidth, 1kHz internal Fs
    Dlpf92 = 0x02,
    /// 41Hz bandwidth, 1kHz internal Fs
    Dlpf41 = 0x03,
    /// 20Hz bandwidth, 1kHz internal Fs
    Dlpf20 = 0x04,
    /// 10Hz bandwidth, 1kHz internal Fs
    Dlpf10 = 0x05,
    /// 5Hz bandwidth, 1kHz internal Fs
    Dlpf5 = 0x06,
    /// 3600Hz bandwidth, 8kHz internal Fs (same as FCHOICE_B bypass)
    Dlpf3600 = 0x07,
}

impl DlpfConfig {
    /// Gyro bandwidth in Hz
    pub fn gyro_bandwidth_hz(&self) -> u32 {
        match self {
            DlpfConfig::Dlpf250 => 250,
            DlpfConfig::Dlpf184 => 184,
            DlpfConfig::Dlpf92 => 92,
            DlpfConfig::Dlpf41 => 41,
            DlpfConfig::Dlpf20 => 20,
            DlpfConfig::Dlpf10 => 10,
            DlpfConfig::Dlpf5 => 5,
            DlpfConfig::Dlpf3600 => 3600,
        }
    }

    /// Temperature bandwidth in Hz
    pub fn temp_bandwidth_hz(&self) -> u32 {
        match self {
            DlpfConfig::Dlpf250 => 250,
            DlpfConfig::Dlpf184 => 188,
            DlpfConfig::Dlpf92 => 98,
            DlpfConfig::Dlpf41 => 42,
            DlpfConfig::Dlpf20 => 20,
            DlpfConfig::Dlpf10 => 10,
            DlpfConfig::Dlpf5 => 5,
            DlpfConfig::Dlpf3600 => 4000,
        }
    }
}

/// Accelerometer digital low-pass filter configuration.
///
/// Only effective when ACCEL_FCHOICE_B = 0 in ACCEL_CONFIG_2.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AccelDlpfConfig {
    /// 460Hz bandwidth, 1kHz output rate
    Dlpf460 = 0x00,
    /// 184Hz bandwidth
    Dlpf184 = 0x01,
    /// 92Hz bandwidth
    Dlpf92 = 0x02,
    /// 41Hz bandwidth
    Dlpf41 = 0x03,
    /// 20Hz bandwidth
    Dlpf20 = 0x04,
    /// 10Hz bandwidth
    Dlpf10 = 0x05,
    /// 5Hz bandwidth
    Dlpf5 = 0x06,
}

/// Gyroscope full-scale range.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum GyroRange {
    /// ±250°/s
    Dps250 = 0x00,
    /// ±500°/s
    Dps500 = 0x08,
    /// ±1000°/s
    Dps1000 = 0x10,
    /// ±2000°/s
    Dps2000 = 0x18,
}

impl GyroRange {
    /// Sensitivity in LSB/(°/s)
    pub fn sensitivity(&self) -> f32 {
        match self {
            GyroRange::Dps250 => 131.0,
            GyroRange::Dps500 => 65.5,
            GyroRange::Dps1000 => 32.8,
            GyroRange::Dps2000 => 16.4,
        }
    }

    /// Full-scale range in °/s
    pub fn full_scale(&self) -> f32 {
        match self {
            GyroRange::Dps250 => 250.0,
            GyroRange::Dps500 => 500.0,
            GyroRange::Dps1000 => 1000.0,
            GyroRange::Dps2000 => 2000.0,
        }
    }
}

/// Accelerometer full-scale range.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AccelRange {
    /// ±2g
    G2 = 0x00,
    /// ±4g
    G4 = 0x08,
    /// ±8g
    G8 = 0x10,
    /// ±16g
    G16 = 0x18,
}

impl AccelRange {
    /// Sensitivity in LSB/g
    pub fn sensitivity(&self) -> f32 {
        match self {
            AccelRange::G2 => 16384.0,
            AccelRange::G4 => 8192.0,
            AccelRange::G8 => 4096.0,
            AccelRange::G16 => 2048.0,
        }
    }

    /// Full-scale range in g
    pub fn full_scale(&self) -> f32 {
        match self {
            AccelRange::G2 => 2.0,
            AccelRange::G4 => 4.0,
            AccelRange::G8 => 8.0,
            AccelRange::G16 => 16.0,
        }
    }
}

/// Clock source selection for PWR_MGMT_1 CLKSEL bits.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum ClockSource {
    /// Internal 20MHz oscillator
    InternalOsc = 0x00,
    /// PLL with X-axis gyro reference
    PllWithXGyro = 0x01,
    /// PLL with Y-axis gyro reference
    PllWithYGyro = 0x02,
    /// PLL with Z-axis gyro reference
    PllWithZGyro = 0x03,
    /// PLL with external 32.768kHz reference
    PllWithExt32k = 0x04,
    /// PLL with external 19.2MHz reference
    PllWithExt19m = 0x05,
}

/// Interrupt active level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IntLevel {
    /// Active high (default)
    ActiveHigh = 0,
    /// Active low
    ActiveLow = 1,
}

/// INT pin drive mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IntDriveMode {
    /// Push-pull (default)
    PushPull = 0,
    /// Open drain
    OpenDrain = 1,
}

/// INT pin latch mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IntLatch {
    /// 50μs pulse (default)
    Pulse = 0,
    /// Latched until status is cleared
    Latched = 1,
}

/// FIFO mode for CONFIG register bit 6.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum FifoMode {
    /// When FIFO is full, new data overwrites oldest
    Overwrite = 0,
    /// When FIFO is full, new data is rejected
    Reject = 1,
}
