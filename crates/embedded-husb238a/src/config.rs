//! Protocol detection configuration for the HUSB238A.

/// Sink PDO1 current capability (USER_CFG3 bits [1:0])
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub enum SinkPdo1Current {
    /// 3A (default)
    Amps3 = 0b00,
    /// 2.4A
    Amps2_4 = 0b01,
    /// 2.1A
    Amps2_1 = 0b10,
    /// 1.5A
    Amps1_5 = 0b11,
}

/// Protocol detection configuration for USER_CFG1/USER_CFG3/CONTROL1 registers.
///
/// Controls which charging protocols the HUSB238A will detect and advertise
/// in its Sink_Capabilities message to the connected Source.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct ProtocolConfig {
    /// Enable HVDCP detection after BC1.2 (USER_CFG1[6]).
    /// When enabled, the chip will detect Qualcomm QC2/QC3 chargers.
    /// Default: false (only BC1.2 CDP/SDP/DCP).
    pub enable_hvdcp: bool,

    /// Enable VBUS under-voltage detection (USER_CFG1[3]).
    /// When enabled, VBUS_UV fault interrupt will trigger on brownout.
    /// Default: false.
    pub enable_vbus_uv_detection: bool,

    /// Enable PPS Sink Capability (USER_CFG3[6]).
    /// Must be true for PPS PDOs to appear in Sink_Capabilities.
    /// Default: false.
    pub enable_pps: bool,

    /// Enable AVS Sink Capability (USER_CFG3[5]).
    /// Must be true for AVS PDOs to appear in Sink_Capabilities.
    /// Default: false.
    pub enable_avs: bool,

    /// Enable EPR AVS Sink Capability (USER_CFG3[3]).
    /// Must be true for EPR AVS PDOs (>28V) to appear in Sink_Capabilities.
    /// Default: false.
    pub enable_epr_avs: bool,

    /// Enable modal operation for SOP Discover SVIDs/Modes/Enter/Exit (USER_CFG3[4]).
    /// When enabled, the chip responds with ACK instead of NAK to these commands.
    /// Default: false.
    pub enable_modal_operation: bool,

    /// Min Voltage in Sink_Capabilities PDO2 (USER_CFG3[2]).
    /// false = 5V, true = 3.3V.
    /// Default: false (5V).
    pub snk_cap_min_voltage_3v3: bool,

    /// PDO1 current in Sink_Capabilities (USER_CFG3[1:0]).
    /// Advertises the maximum current the sink can accept on the 5V fixed PDO.
    /// Default: 3A.
    pub snk_pdo1_current: SinkPdo1Current,

    /// Enable legacy protocol detection via D+/D- (CONTROL1[5] EN_DPM_HIZ inverted).
    /// When true, D+/D- remain connected for BC1.2/QC detection.
    /// When false (default), D+/D- are disconnected (EN_DPM_HIZ=1) — PD-only mode.
    pub enable_legacy_detection: bool,

    /// PD protocol priority (USER_CFG2[2]).
    /// true = PD PE runs immediately after connection (no 3s delay).
    /// Default: true.
    pub pd_priority: bool,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            enable_hvdcp: false,
            enable_vbus_uv_detection: false,
            enable_pps: false,
            enable_avs: false,
            enable_epr_avs: false,
            enable_modal_operation: false,
            snk_cap_min_voltage_3v3: false,
            snk_pdo1_current: SinkPdo1Current::Amps3,
            enable_legacy_detection: false,
            pd_priority: true,
        }
    }
}

impl ProtocolConfig {
    /// Create a PD-only config (no legacy protocols, no PPS/AVS).
    pub fn pd_only() -> Self {
        Self::default()
    }

    /// Create a config with PPS support enabled.
    pub fn with_pps() -> Self {
        Self {
            enable_pps: true,
            ..Self::default()
        }
    }

    /// Create a config with PPS + AVS support enabled.
    pub fn with_pps_avs() -> Self {
        Self {
            enable_pps: true,
            enable_avs: true,
            ..Self::default()
        }
    }

    /// Create a full-featured config enabling all protocols.
    pub fn full() -> Self {
        Self {
            enable_hvdcp: true,
            enable_vbus_uv_detection: true,
            enable_pps: true,
            enable_avs: true,
            enable_epr_avs: true,
            enable_legacy_detection: true,
            ..Self::default()
        }
    }
}
