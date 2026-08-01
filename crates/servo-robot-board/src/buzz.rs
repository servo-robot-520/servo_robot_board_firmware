//! 蜂鸣器驱动
//!
//! BUZZ 引脚: PB3, TIM2 CH2
//! 基于 demo_c/buzz.c 移植
//!
//! TIM2 配置: APB1 Timer Clock = 96MHz
//!   → Tick = 96MHz / 1 = 96MHz
//!   → ARR = 96000000 / freq - 1

/// TIM2 定时器频率 (APB1 Timer Clock)
const TIM_TICK_HZ: u32 = 96_000_000;

/// 播放指定频率和时长的音调
///
/// # Arguments
/// * `tim` - TIM2 外设引用
/// * `freq_hz` - 频率 (Hz), 0 = 静音
/// * `duration_ms` - 时长 (ms)
pub fn tone(tim: &stm32f4xx_hal::pac::TIM2, freq_hz: u32, duration_ms: u32) {
    if freq_hz == 0 || duration_ms == 0 {
        return;
    }

    // 计算 ARR 和 CCR (50% 占空比)
    let arr = (TIM_TICK_HZ / freq_hz).min(0xFFFF) - 1;
    let ccr = arr / 2;

    tim.arr().write(|w| unsafe { w.arr().bits(arr) });
    tim.ccr2().write(|w| unsafe { w.ccr().bits(ccr) });

    // 启动 PWM
    tim.cr1().modify(|_, w| w.cen().set_bit());

    // 延时 (简单忙等, 适用于启动阶段)
    for _ in 0..duration_ms * 1000 {
        cortex_m::asm::nop();
    }

    // 停止 PWM
    tim.cr1().modify(|_, w| w.cen().clear_bit());
    tim.ccr2().write(|w| unsafe { w.ccr().bits(0) });
}

/// 停止蜂鸣器
pub fn stop(tim: &stm32f4xx_hal::pac::TIM2) {
    tim.cr1().modify(|_, w| w.cen().clear_bit());
    tim.ccr2().write(|w| unsafe { w.ccr().bits(0) });
}

/// R2-D2 风格启动音效
///
/// 1. 快速上升音 (低→高)
/// 2. 短促高频嘟嘟 × 3
/// 3. 下滑音
/// 4. 欢快上升音 × 2
/// 5. 结束长音
pub fn startup_sound(tim: &stm32f4xx_hal::pac::TIM2) {
    // 阶段1: 快速上升滑音
    let mut f = 800u32;
    while f <= 3000 {
        tone(tim, f, 12);
        f += 200;
    }

    // 阶段2: 短促高频嘟嘟 × 3
    tone(tim, 3500, 30);
    tone(tim, 0, 15);
    tone(tim, 4000, 30);
    tone(tim, 0, 15);
    tone(tim, 3200, 30);
    tone(tim, 0, 20);

    // 阶段3: 下滑音
    let mut f = 2500u32;
    while f >= 600 {
        tone(tim, f, 10);
        f = f.saturating_sub(150);
    }

    // 阶段4: 欢快上升音 × 2
    tone(tim, 0, 30);
    let mut f = 1000u32;
    while f <= 2800 {
        tone(tim, f, 15);
        f += 300;
    }
    tone(tim, 0, 20);
    let mut f = 1200u32;
    while f <= 3500 {
        tone(tim, f, 15);
        f += 300;
    }

    // 阶段5: 结束长音
    tone(tim, 0, 20);
    tone(tim, 2500, 60);
    tone(tim, 3000, 80);

    stop(tim);
}
