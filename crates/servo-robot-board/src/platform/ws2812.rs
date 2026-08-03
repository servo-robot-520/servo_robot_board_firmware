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
/// (常量保留供文档参考，实际配置在本模块)
// const PWM_PERIOD: u16 = 120;
/// 0 bit: 高电平 ~350ns → 42 计数 (0.35µs * 96 + 10 余量)
const PWM_ZERO: u16 = 42;
/// 1 bit: 高电平 ~700ns → 78 计数 (0.7µs * 96 + 10 余量)
const PWM_ONE: u16 = 78;
/// Reset: 低电平 >50μs → 50 个 PWM 周期 (50 × 1.25µs = 62.5µs)
///
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

use stm32f4xx_hal::pac::{DMA2, TIM1};

/// Static DMA buffer (initialized to 0 = reset condition).
/// Address must remain stable for the duration of the DMA transfer.
static mut WS2812_DMA_BUF: [u16; DMA_BUF_SIZE] = [0; DMA_BUF_SIZE];

/// Initialize TIM1 CH2 for WS2812 PWM output and DMA2 Stream5
/// for automatic data transfer.
///
/// After this call, TIM1 and DMA are configured but stopped.
/// Use [`send_colors`] to encode LED data and start a transfer.
///
/// # Prerequisites
/// - GPIOA clock must be enabled
/// - TIM1 clock must be enabled (RCC APB2ENR.TIM1EN)
/// - DMA2 clock must be enabled (RCC AHB1ENR.DMA2EN)
///
/// # Safety Note
/// 此函数直接操作 GPIOA 寄存器配置 PA9 为 AF1，绕过 HAL 的所有权模型。
/// 调用后 PA9 不应再通过 HAL 使用（如 `gpioa.pa9.into_alternate()`），
/// 否则 HAL 可能 panic 或行为异常。
pub fn init_tim1_dma(tim1: &TIM1) {
    // --- Configure PA9 as AF1 (TIM1 CH2) ---
    // 注意: 直接操作寄存器，绕过 HAL，PA9 后续不可再通过 HAL 使用
    {
        let gpioa = unsafe { &*stm32f4xx_hal::pac::GPIOA::ptr() };
        // AFRH[11:8] = AF1 for PA9
        gpioa.afrh().modify(|_, w| unsafe { w.afrh9().bits(1) });
        // MODER[19:18] = 10 (alternate function)
        gpioa.moder().modify(|_, w| w.moder9().alternate());
        // OTYPER[9] = 0 (push-pull)
        gpioa.otyper().modify(|_, w| w.ot9().push_pull());
        // OSPEEDR[19:18] = 11 (very high speed for clean WS2812 edges)
        gpioa
            .ospeedr()
            .modify(|_, w| w.ospeedr9().very_high_speed());
    }

    // --- TIM1 Configuration ---
    // Disable timer before reconfiguration
    tim1.cr1().modify(|_, w| w.cen().clear_bit());

    // Prescaler = 0 → timer clock = APB2 = 96MHz
    tim1.psc().write(|w| unsafe { w.psc().bits(0) });
    // Auto-reload = 119 → period = 120 cycles = 1.25µs (800kHz)
    tim1.arr().write(|w| unsafe { w.arr().bits(119) });
    // Initial duty = 0 (output low)
    tim1.ccr2().write(|w| unsafe { w.ccr().bits(0) });

    // CCMR1 (output mode): CH2 → PWM mode 1, preload enable
    //   OC2M[14:12] = 110 (PWM mode 1)
    //   OC2PE[11] = 1 (preload enable, CCR2 latched at update event)
    tim1.ccmr1_output()
        .modify(|_, w| unsafe { w.oc2m().bits(6).oc2pe().set_bit() });

    // CCER: enable CH2 output
    //   CC2E[4] = 1
    tim1.ccer().modify(|_, w| w.cc2e().set_bit());

    // BDTR: main output enable (required for advanced timer TIM1)
    //   MOE[15] = 1
    tim1.bdtr().modify(|_, w| w.moe().set_bit());

    // DIER: update DMA request enable
    //   UDE[8] = 1 → DMA request on each update (overflow) event
    tim1.dier().modify(|_, w| w.ude().set_bit());

    // CR1: auto-reload preload enable
    //   ARPE[7] = 1
    tim1.cr1().modify(|_, w| w.arpe().set_bit());

    // --- DMA2 Stream5 Channel6 Configuration ---
    let dma2 = unsafe { &*DMA2::ptr() };
    let stream = dma2.st(5);

    // Disable stream before reconfiguration
    stream.cr().modify(|_, w| w.en().clear_bit());
    while stream.cr().read().en().bit_is_set() {}

    // Clear all stream 5 status flags
    dma2.hifcr().write(|w| {
        w.cfeif5()
            .set_bit()
            .cdmeif5()
            .set_bit()
            .cteif5()
            .set_bit()
            .chtif5()
            .set_bit()
            .ctcif5()
            .set_bit()
    });

    // Peripheral address: TIM1 CCR2 register
    let ccr2_addr = &tim1.ccr2() as *const _ as u32;
    stream.par().write(|w| unsafe { w.pa().bits(ccr2_addr) });

    // Stream control register:
    //   CHSEL[27:25] = 110 (channel 6 = TIM1_UP)
    //   DIR[7:6]     = 01  (memory → peripheral)
    //   MSIZE[14:13] = 01  (16-bit memory)
    //   PSIZE[11:10] = 01  (16-bit peripheral)
    //   MINC[10]     = 1   (memory address increment)
    //   PINC[9]      = 0   (peripheral address fixed)
    //   CIRC[8]      = 0   (one-shot mode)
    stream.cr().write(|w| unsafe {
        w.chsel()
            .bits(6) // Channel 6 (TIM1_UP)
            .dir()
            .bits(1) // Memory-to-peripheral
            .msize()
            .bits(1) // 16-bit
            .psize()
            .bits(1) // 16-bit
            .minc()
            .set_bit() // Memory address increment
    });
    // Note: stream and timer are NOT enabled here.
    // send_colors() will configure the buffer address/length and start.
}

/// Start a DMA transfer to update the WS2812 LEDs.
///
/// Encodes `colors` into the static DMA buffer using [`encode_colors`],
/// then starts a one-shot DMA transfer from memory to TIM1 CCR2.
/// The timer clocks out one PWM period per DMA word, and the DMA stream
/// auto-disables when the transfer count reaches zero.
///
/// If a previous transfer is still in progress, this call is a no-op.
/// Since the main update source (`bat_task`) runs at 10 Hz, any dropped
/// frame is corrected on the next cycle.
pub fn send_colors(colors: &[Color]) {
    let dma2 = unsafe { &*DMA2::ptr() };
    let tim1 = unsafe { &*TIM1::ptr() };
    let stream = dma2.st(5);

    // If DMA stream is still running from a previous transfer, skip
    if stream.cr().read().en().bit_is_set() {
        return;
    }

    // Stop timer so the data line idles low
    tim1.cr1().modify(|_, w| w.cen().clear_bit());

    // Clear any pending stream 5 flags (TC, HT, TE)
    dma2.hifcr()
        .write(|w| w.ctcif5().set_bit().chtif5().set_bit().cteif5().set_bit());

    // Encode GRB color data into the static DMA buffer
    let buf = unsafe { &mut *core::ptr::addr_of_mut!(WS2812_DMA_BUF) };
    encode_colors(colors, buf);

    // Point DMA at the buffer and set the transfer length
    let buf_ptr = unsafe { (*core::ptr::addr_of!(WS2812_DMA_BUF)).as_ptr() } as u32;
    stream.m0ar().write(|w| unsafe { w.m0a().bits(buf_ptr) });
    stream
        .ndtr()
        .write(|w| unsafe { w.ndt().bits(DMA_BUF_SIZE as u16) });

    // Enable DMA stream (will wait for TIM1 update requests)
    stream.cr().modify(|_, w| w.en().set_bit());

    // Reset counter and start the timer
    tim1.cnt().write(|w| unsafe { w.cnt().bits(0) });
    tim1.cr1().modify(|_, w| w.cen().set_bit());
}
