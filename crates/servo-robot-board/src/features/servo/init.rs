//! Servo UART1 initialization.
//!
//! Configures PA15 (TX), PA10 (RX), PB12 (half-duplex direction), and USART1
//! for the board's serial-servo forwarding link.

use cortex_m::peripheral::NVIC;
use stm32f4xx_hal::pac;

/// Configure UART1 GPIO, direction output, and USART1 registers.
///
/// This intentionally does not unmask USART1. The caller first stores the PAC
/// token in the servo feature's synchronized slot, then calls
/// [`enable_interrupt`] so an early interrupt can never observe no UART.
pub fn configure_uart1(usart1: &pac::USART1) {
    let rcc = unsafe { &*pac::RCC::ptr() };
    rcc.apb2enr().modify(|_, w| w.usart1en().set_bit());

    let gpioa = unsafe { &*pac::GPIOA::ptr() };
    gpioa.afrh().modify(|_, w| unsafe {
        w.afrh15()
            .bits(7) // PA15 -> AF7
            .afrh10()
            .bits(7) // PA10 -> AF7
    });
    gpioa
        .moder()
        .modify(|_, w| w.moder15().alternate().moder10().alternate());
    gpioa
        .otyper()
        .modify(|_, w| w.ot15().push_pull().ot10().push_pull());
    gpioa
        .ospeedr()
        .modify(|_, w| w.ospeedr15().high_speed().ospeedr10().high_speed());

    let gpiob = unsafe { &*pac::GPIOB::ptr() };
    gpiob.moder().modify(|_, w| w.moder12().output());
    gpiob.otyper().modify(|_, w| w.ot12().push_pull());
    gpiob.ospeedr().modify(|_, w| w.ospeedr12().high_speed());
    gpiob.bsrr().write(|w| w.br12().set_bit());

    // USART1: APB2 = 96 MHz, 115200 baud, 8N1.
    usart1.brr().write(|w| unsafe { w.bits(833) });
    usart1
        .cr1()
        .write(|w| w.te().set_bit().re().set_bit().rxneie().set_bit());
    usart1.cr1().modify(|_, w| w.ue().set_bit());
}

/// Set the same logical priority formerly declared by RTIC and unmask USART1.
pub fn enable_interrupt(nvic: &mut NVIC) {
    const LOGICAL_PRIORITY: u8 = 5;

    // This mirrors RTIC 2.1's `cortex_logical2hw` conversion for STM32F4.
    let hardware_priority = LOGICAL_PRIORITY << (8 - pac::NVIC_PRIO_BITS);
    unsafe {
        nvic.set_priority(pac::Interrupt::USART1, hardware_priority);
        NVIC::unmask(pac::Interrupt::USART1);
    }
    defmt::info!("UART1 initialized (115200 baud, servo)");
}
