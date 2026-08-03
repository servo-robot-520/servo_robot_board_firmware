//! Board-specific passive-buzzer driver.
//!
//! The board routes the buzzer to PB3 (TIM2_CH2, AF1). This module owns the
//! fixed pin/timer setup and exposes only basic tones; product-specific melody
//! composition belongs to a feature module.

use stm32f4xx_hal::pac::TIM2;

/// TIM2 counter clock on this board: APB1 is 48 MHz and timer clocks are
/// doubled because APB1 is prescaled.
const TIM_TICK_HZ: u32 = 96_000_000;

/// Passive buzzer connected to PB3 / TIM2 CH2.
pub struct Buzzer {
    tim2: TIM2,
}

impl Buzzer {
    /// Configure PB3 as TIM2_CH2 and initialize TIM2 PWM channel 2.
    ///
    /// This consumes the unique TIM2 PAC token. GPIOB and TIM2 clocks are
    /// enabled here so the hardware setup remains owned by the platform.
    pub fn new(tim2: TIM2) -> Self {
        let rcc = unsafe { &*stm32f4xx_hal::pac::RCC::ptr() };
        rcc.ahb1enr().modify(|_, w| w.gpioben().set_bit());
        rcc.apb1enr().modify(|_, w| w.tim2en().set_bit());

        let gpiob = unsafe { &*stm32f4xx_hal::pac::GPIOB::ptr() };
        gpiob.afrl().modify(|_, w| unsafe { w.afrl3().bits(1) });
        gpiob.moder().modify(|_, w| w.moder3().alternate());
        gpiob.otyper().modify(|_, w| w.ot3().push_pull());
        gpiob.ospeedr().modify(|_, w| w.ospeedr3().high_speed());

        tim2.cr1().modify(|_, w| w.cen().clear_bit());
        tim2.psc().write(|w| unsafe { w.psc().bits(0) });
        tim2.arr().write(|w| unsafe { w.arr().bits(0) });
        tim2.ccr2().write(|w| unsafe { w.ccr().bits(0) });
        tim2.ccmr1_output()
            .modify(|_, w| unsafe { w.oc2m().bits(6).oc2pe().clear_bit() });
        tim2.ccer().modify(|_, w| w.cc2e().set_bit());
        tim2.egr().write(|w| w.ug().set_bit());

        Self { tim2 }
    }

    /// Play a 50%-duty-cycle tone for `duration_ms`.
    ///
    /// Passing `freq_hz = 0` produces a silent rest of the requested duration.
    pub fn tone(&mut self, freq_hz: u32, duration_ms: u32) {
        if duration_ms == 0 {
            return;
        }
        if freq_hz == 0 {
            self.stop();
            delay_ms(duration_ms);
            return;
        }

        // TIM2 is 32-bit, so low frequencies do not need to be quantized to a
        // 16-bit auto-reload value.
        let period_ticks = (TIM_TICK_HZ / freq_hz).max(2);
        let arr = period_ticks - 1;
        self.tim2.arr().write(|w| unsafe { w.arr().bits(arr) });
        self.tim2
            .ccr2()
            .write(|w| unsafe { w.ccr().bits(period_ticks / 2) });
        self.tim2.egr().write(|w| w.ug().set_bit());
        self.tim2.cr1().modify(|_, w| w.cen().set_bit());

        delay_ms(duration_ms);
        self.stop();
    }

    /// Stop PWM output and drive the timer compare value low.
    pub fn stop(&mut self) {
        self.tim2.cr1().modify(|_, w| w.cen().clear_bit());
        self.tim2.ccr2().write(|w| unsafe { w.ccr().bits(0) });
    }
}

fn delay_ms(duration_ms: u32) {
    cortex_m::asm::delay(duration_ms.saturating_mul(TIM_TICK_HZ / 1_000));
}
