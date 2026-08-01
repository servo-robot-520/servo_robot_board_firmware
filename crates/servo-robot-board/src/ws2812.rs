//! WS2812 TIM1 CH2 DMA 驱动
//!
//! 使用 TIM1 CH2 (PA9) + DMA2 Stream5 Channel6
//! 通过 PWM 占空比编码 WS2812 数据:
//! - 0 bit: 高电平 350ns, 低电平 800ns
//! - 1 bit: 高电平 700ns, 低电平 600ns
//!
//! WS2812 数据格式: GRB 顺序, 每色 8bit, 共 24bit/LED

/// WS2812 LED 数量
pub const LED_COUNT: usize = 3;

/// PWM 频率: 800kHz → 周期 1.25μs
/// 在 96MHz APB2 时钟下, 预分频 0, 周期 = 120 个计数
const PWM_PERIOD: u16 = 120;
/// 0 bit: 高电平 ~350ns → 42 计数 (0.35µs * 96 + 10 余量)
const PWM_ZERO: u16 = 42;
/// 1 bit: 高电平 ~700ns → 78 计数 (0.7µs * 96 + 10 余量)
const PWM_ONE: u16 = 78;
/// Reset: 低电平 >50μs → 50 个 PWM 周期 (50 × 1.25µs = 62.5µs)

/// DMA 缓冲区大小: 24 bits × LED_COUNT + reset (50 周期)
pub const DMA_BUF_SIZE: usize = 24 * LED_COUNT + 50;

/// GRB 颜色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub g: u8,
    pub r: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { g, r, b }
    }

    pub const BLACK: Self = Self::new(0, 0, 0);
    pub const RED: Self = Self::new(255, 0, 0);
    pub const GREEN: Self = Self::new(0, 255, 0);
    pub const BLUE: Self = Self::new(0, 0, 255);
    pub const YELLOW: Self = Self::new(255, 255, 0);
    pub const ORANGE: Self = Self::new(255, 128, 0);
    pub const PURPLE: Self = Self::new(128, 0, 255);
    pub const PINK: Self = Self::new(255, 0, 128);
    pub const WHITE: Self = Self::new(255, 255, 255);
}

/// 将颜色编码为 DMA PWM 缓冲区
///
/// `colors`: LED 颜色数组 (GRB 顺序)
/// `buf`: DMA 缓冲区, 大小至少 `DMA_BUF_SIZE` (24 * LED_COUNT + 50)
pub fn encode_colors(colors: &[Color], buf: &mut [u16]) {
    let led_count = colors.len().min(LED_COUNT);
    let mut pos = 0;

    for color in colors.iter().take(led_count) {
        // GRB 顺序
        let grb = ((color.g as u32) << 16) | ((color.r as u32) << 8) | (color.b as u32);
        for bit in (0..24).rev() {
            buf[pos] = if (grb >> bit) & 1 != 0 {
                PWM_ONE
            } else {
                PWM_ZERO
            };
            pos += 1;
        }
    }

    // Reset (低电平)
    while pos < DMA_BUF_SIZE {
        buf[pos] = 0;
        pos += 1;
    }
}

/// 根据电池温度返回 LED 颜色
///
/// - <5°C: 蓝色
/// - 5~25°C: 绿色
/// - 25~40°C: 黄色
/// - 40~55°C: 橙色
/// - >55°C: 红色
pub fn battery_temp_color(temp_c: f32) -> Color {
    if temp_c < 5.0 {
        Color::BLUE
    } else if temp_c < 25.0 {
        Color::GREEN
    } else if temp_c < 40.0 {
        Color::YELLOW
    } else if temp_c < 55.0 {
        Color::ORANGE
    } else {
        Color::RED
    }
}

/// 根据电池 SOC 返回 LED 颜色
///
/// - <2%: 灭
/// - 2~10%: 红色
/// - 10~30%: 橙色
/// - 30~80%: 蓝色
/// - 80~100%: 绿色
pub fn battery_soc_color(soc: u8) -> Color {
    if soc < 2 {
        Color::BLACK
    } else if soc < 10 {
        Color::RED
    } else if soc < 30 {
        Color::ORANGE
    } else if soc < 80 {
        Color::BLUE
    } else {
        Color::GREEN
    }
}
