//! Optional UART1 serial-servo forwarding.
//!
//! `main.rs` always exposes the RTIC software tasks so source-level tools can
//! navigate the firmware. The optional UART1 peripheral and its vector are
//! owned here and compiled only when the `servo` feature is enabled.

#[cfg(feature = "servo")]
pub mod init;
pub mod task;
#[cfg(feature = "servo")]
pub mod uart;

#[cfg(feature = "servo")]
use core::cell::RefCell;
#[cfg(feature = "servo")]
use cortex_m::interrupt::Mutex;
#[cfg(feature = "servo")]
use stm32f4xx_hal::pac;

/// Feature-private ownership of the USART1 PAC token.
///
/// The UART is accessed from the manually declared USART1 ISR and RTIC
/// software tasks, so accesses are short and protected by a Cortex-M critical
/// section. It is initialized before USART1 is unmasked.
#[cfg(feature = "servo")]
static USART1: Mutex<RefCell<Option<pac::USART1>>> = Mutex::new(RefCell::new(None));

/// Initialize optional UART1 serial-servo forwarding.
///
/// With the feature disabled this consumes the PAC token without enabling the
/// peripheral clock, configuring GPIO, or touching the USART1 NVIC line.
pub fn initialize(usart1: stm32f4xx_hal::pac::USART1, nvic: &mut cortex_m::peripheral::NVIC) {
    #[cfg(feature = "servo")]
    {
        init::configure_uart1(&usart1);
        cortex_m::interrupt::free(|cs| {
            let mut slot = USART1.borrow(cs).borrow_mut();
            debug_assert!(slot.is_none(), "USART1 must be initialized once");
            *slot = Some(usart1);
        });
        init::enable_interrupt(nvic);
    }

    #[cfg(not(feature = "servo"))]
    {
        let _ = (usart1, nvic);
    }
}

/// Access the initialized UART1 peripheral while interrupts are masked.
#[cfg(feature = "servo")]
fn with_usart1<R>(f: impl FnOnce(&mut pac::USART1) -> R) -> Option<R> {
    cortex_m::interrupt::free(|cs| {
        let mut slot = USART1.borrow(cs).borrow_mut();
        slot.as_mut().map(f)
    })
}

/// Whether this build enables UART1 serial-servo forwarding.
pub const fn is_enabled() -> bool {
    cfg!(feature = "servo")
}

/// Record that a servo command was ignored by a build without UART1 support.
pub fn log_forwarding_disabled() {
    defmt::warn!("ServoForward ignored: build without `servo` feature");
}
