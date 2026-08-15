//! ADC 通道定义 + ADC1/DMA2 初始化
//!
//! ADC1 通道: IN0(充电温度), IN1(舵机电源温度), IN4(5V电源温度),
//!           IN8(BC_IOUT), IN9(CV_ADC), TempSens(MCU内部温度)
//!
//! DMA2 Stream0 Channel0 循环传输, 结果自动写入 adc_buf

use core::cell::UnsafeCell;

use stm32f4xx_hal::pac::{ADC1, DMA2};

/// ADC 通道索引
pub const CH_TEMP_CHARGE: usize = 0; // IN0 - PA0 - 充电电路温度
pub const CH_TEMP_SERVO: usize = 1; // IN1 - PA1 - 舵机电源温度
pub const CH_TEMP_5V: usize = 2; // IN4 - PA4 - 5V电源温度
pub const CH_BC_IOUT: usize = 3; // IN8 - PB0 - 充电电流
pub const CH_CV_ADC: usize = 4; // IN9 - PB1 - PD输入电压
pub const CH_MCU_TEMP: usize = 5; // TempSens - MCU内部温度

/// ADC 通道数
pub const ADC_CHANNEL_COUNT: usize = 6;

/// ADC 参考电压 (mV)
pub const VREF_MV: f32 = 3300.0;
/// ADC 分辨率 (12-bit)
pub const ADC_MAX: f32 = 4095.0;

/// ADC 原始值转电压 (mV)
pub fn adc_to_mv(adc_val: u16) -> f32 {
    adc_val as f32 * VREF_MV / ADC_MAX
}

/// ADC 通道号列表（对应 adc_buf 索引顺序）
const ADC_CHANNELS: [u8; ADC_CHANNEL_COUNT] = [0, 1, 4, 8, 9, 18];
// 通道 18 是内部温度传感器

/// DMA 持续写入的 ADC 采样存储。
///
/// `UnsafeCell` 明确表达此内存可在普通 Rust 借用之外被 DMA 修改。
/// 对外只提供复制快照，避免暴露一个指向 DMA 持续写入内存的共享引用。
#[repr(transparent)]
struct AdcDmaBuffer {
    samples: UnsafeCell<[u16; ADC_CHANNEL_COUNT]>,
}

// DMA 是外部写入者；CPU 侧仅通过 `snapshot` 作 volatile 读取，并且不会
// 向外泄漏对内部存储的引用。
unsafe impl Sync for AdcDmaBuffer {}

impl AdcDmaBuffer {
    const fn new() -> Self {
        Self {
            samples: UnsafeCell::new([0; ADC_CHANNEL_COUNT]),
        }
    }

    fn as_mut_ptr(&self) -> *mut u16 {
        self.samples.get().cast::<u16>()
    }

    fn snapshot(&self) -> [u16; ADC_CHANNEL_COUNT] {
        let samples = self.samples.get().cast::<u16>();
        core::array::from_fn(|index| {
            // SAFETY: `samples` points at the statically allocated DMA buffer.
            // Volatile loads ensure every element is read from memory even while
            // DMA2 is updating the circular buffer. A snapshot can span two DMA
            // scan cycles, which is acceptable for these monitoring channels.
            unsafe { core::ptr::read_volatile(samples.add(index)) }
        })
    }
}

/// DMA 缓冲区（由 DMA2 直接写入，地址必须稳定）。
static ADC_DMA_BUF: AdcDmaBuffer = AdcDmaBuffer::new();

/// Copy the latest ADC DMA samples.
///
/// The returned array is independent of the DMA storage. Values from one call
/// may originate from adjacent scan cycles because DMA continuously updates the
/// channels; temperature and voltage monitoring tolerates that skew.
pub fn adc_snapshot() -> [u16; ADC_CHANNEL_COUNT] {
    ADC_DMA_BUF.snapshot()
}

/// 初始化 ADC1 + DMA2 循环采集
///
/// - ADC1 扫描模式，6 通道
/// - DMA2 Stream0 Channel0 循环传输
/// - 结果自动写入 ADC_DMA_BUF
///
/// 调用前需要：
/// 1. RCC 时钟已使能 (ADC1, DMA2, GPIOA, GPIOB)
/// 2. ADC 引脚已配置为模拟输入 (PA0, PA1, PA4, PB0, PB1)
pub fn init_adc_dma(adc1: &ADC1, dma2: &DMA2) {
    let adc_buf_ptr = ADC_DMA_BUF.as_mut_ptr();

    // === DMA2 Stream0 Channel0 配置 ===
    let stream = dma2.st(0);

    // 关闭 DMA Stream（先关闭再配置）
    stream.cr().modify(|_, w| w.en().clear_bit());
    while stream.cr().read().en().bit_is_set() {}

    // 外设地址: ADC1->DR
    let adc_dr_addr = &adc1.dr() as *const _ as u32;
    stream.par().write(|w| unsafe { w.pa().bits(adc_dr_addr) });

    // 内存地址: ADC_DMA_BUF
    stream
        .m0ar()
        .write(|w| unsafe { w.m0a().bits(adc_buf_ptr as u32) });

    // 传输数量
    stream
        .ndtr()
        .write(|w| unsafe { w.ndt().bits(ADC_CHANNEL_COUNT as u16) });

    // 配置 CR:
    // - Channel 0 (CHSEL = 000)
    // - 优先级高 (PL = 11)
    // - 内存地址递增 (MINC)
    // - 循环模式 (CIRC)
    // - 外设到内存 (DIR = 00)
    // - 16 位数据宽度 (PSIZE = 01, MSIZE = 01)
    stream.cr().write(|w| unsafe {
        w.chsel()
            .bits(0) // Channel 0
            .pl()
            .bits(3) // 高优先级
            .msize()
            .bits(1) // 16 位
            .psize()
            .bits(1) // 16 位
            .minc()
            .set_bit() // 内存地址递增
            .circ()
            .set_bit() // 循环模式
            .dir()
            .bits(0) // 外设→内存
            .en()
            .set_bit() // 使能
    });

    // === ADC1 配置 ===

    // CR1: 扫描模式使能, 12 位分辨率
    adc1.cr1().write(|w| w.scan().set_bit());

    // CR2: DMA 使能, 连续转换, 右对齐
    adc1.cr2()
        .write(|w| w.dma().set_bit().cont().set_bit().align().clear_bit());

    // 采样时间: 所有通道使用 480 cycles (SMP = 7)
    for &ch in &ADC_CHANNELS {
        if ch < 10 {
            adc1.smpr2().modify(|_, w| unsafe { w.smp(ch).bits(7) });
        } else {
            adc1.smpr1().modify(|_, w| unsafe { w.smp(ch).bits(7) });
        }
    }

    // 序列长度: 6 通道 (L = 5, 即 N-1)
    adc1.sqr1()
        .modify(|_, w| unsafe { w.l().bits((ADC_CHANNEL_COUNT - 1) as u8) });

    // 序列: 按 ADC_CHANNELS 顺序转换
    // SQR3: 序列 1-5 (索引 0-4)
    adc1.sqr3().modify(|_, w| unsafe {
        w.sq(0)
            .bits(ADC_CHANNELS[0])
            .sq(1)
            .bits(ADC_CHANNELS[1])
            .sq(2)
            .bits(ADC_CHANNELS[2])
            .sq(3)
            .bits(ADC_CHANNELS[3])
            .sq(4)
            .bits(ADC_CHANNELS[4])
    });
    // SQR2: 序列 6 (索引 5)
    adc1.sqr2()
        .modify(|_, w| unsafe { w.sq(5).bits(ADC_CHANNELS[5]) });

    // 温度传感器和 VREFINT 使能 (CR2 bit 23 = TSVREFE)
    adc1.cr2()
        .modify(|r, w| unsafe { w.bits(r.bits() | (1 << 23)) });

    // ADC 使能
    adc1.cr2().modify(|_, w| w.adon().set_bit());
    // 等待 ADC 稳定
    cortex_m::asm::delay(1000);

    // 复位校准
    adc1.cr2()
        .modify(|r, w| unsafe { w.bits(r.bits() | (1 << 2)) }); // RSTCAL
    while adc1.cr2().read().bits() & (1 << 2) != 0 {} // 等待 RSTCAL 清除

    // 启动校准
    adc1.cr2()
        .modify(|r, w| unsafe { w.bits(r.bits() | (1 << 3)) }); // CAL
    while adc1.cr2().read().bits() & (1 << 3) != 0 {} // 等待 CAL 清除

    // 启动转换
    adc1.cr2().modify(|_, w| w.swstart().set_bit());
}
