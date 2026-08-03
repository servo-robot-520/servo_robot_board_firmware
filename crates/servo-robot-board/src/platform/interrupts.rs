//! EXTI / NVIC and UART register-level initialization.
//!
//! These functions contain bare-register writes that bypass the HAL for
//! configuration that must happen before the HAL takes ownership of the
//! peripherals.
//!
//! UART1 (servo) initialization is in `features::servo::init::init_uart1`.

/// Configure EXTI lines for PB5 (BC_ACOK) and PB14 (HUSB238A INT_N).
///
/// - PB5: both-edge trigger (charger connect/disconnect)
/// - PB14: falling-edge trigger (active-low interrupt)
///
/// Unmasks `EXTI9_5` and `EXTI15_10` in the NVIC.
pub fn configure_exti() {
    let syscfg = unsafe { &*stm32f4xx_hal::pac::SYSCFG::ptr() };
    // EXTICR2: EXTI5 = PB (1) -- PB5 = BC_ACOK
    syscfg.exticr2().modify(|_, w| w.exti5().pb());
    // EXTICR4: EXTI14 = PB (1) -- PB14 = HUSB238A INT_N
    syscfg.exticr4().modify(|_, w| w.exti14().pb());

    let exti = unsafe { &*stm32f4xx_hal::pac::EXTI::ptr() };
    // PB5 both edges (rising + falling), PB14 falling edge
    exti.rtsr().modify(|_, w| w.tr5().set_bit());
    exti.ftsr()
        .modify(|_, w| w.tr5().set_bit().tr14().set_bit());
    // Enable interrupt lines
    exti.imr().modify(|_, w| w.mr5().set_bit().mr14().set_bit());

    // Enable NVIC interrupts
    unsafe {
        cortex_m::peripheral::NVIC::unmask(stm32f4xx_hal::pac::Interrupt::EXTI9_5);
        cortex_m::peripheral::NVIC::unmask(stm32f4xx_hal::pac::Interrupt::EXTI15_10);
    }
    defmt::info!("EXTI configured: PB5 (BC_ACOK both edges), PB14 (HUSB238A INT_N falling)");
}

/// Configure USART2 GPIO (PA2=TX, PA3=RX, AF7) and register settings
/// (115200 baud, 8N1) on the APB1 48 MHz bus. Unmasks the USART2 NVIC interrupt.
pub fn init_usart2(usart2: &stm32f4xx_hal::pac::USART2) {
    // GPIO AF config: PA2=AF7, PA3=AF7
    let gpioa = unsafe { &*stm32f4xx_hal::pac::GPIOA::ptr() };
    gpioa.afrl().modify(|_, w| unsafe {
        w.afrl2()
            .bits(7) // PA2 -> AF7
            .afrl3()
            .bits(7) // PA3 -> AF7
    });
    gpioa
        .moder()
        .modify(|_, w| w.moder2().alternate().moder3().alternate());
    gpioa
        .otyper()
        .modify(|_, w| w.ot2().push_pull().ot3().push_pull());
    gpioa
        .ospeedr()
        .modify(|_, w| w.ospeedr2().high_speed().ospeedr3().high_speed());

    // USART2: 115200 baud, APB1 = 48 MHz, BRR = 48000000 / 115200 = 416
    usart2.brr().write(|w| unsafe { w.bits(416) });
    usart2
        .cr1()
        .write(|w| w.te().set_bit().re().set_bit().rxneie().set_bit());
    usart2.cr1().modify(|_, w| w.ue().set_bit());
    unsafe {
        cortex_m::peripheral::NVIC::unmask(stm32f4xx_hal::pac::Interrupt::USART2);
    }
    defmt::info!("UART2 initialized (115200 baud)");
}
