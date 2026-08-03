//! Board-specific runtime infrastructure.
//!
//! This module owns fixed STM32F411 resources and board wiring. It does not
//! contain product policy; feature modules decide how the board capabilities
//! are used.

pub mod adc;
pub mod buzzer;
pub mod flash;
pub mod interrupts;
pub mod peripherals;
pub mod startup;
pub mod uart2;
pub mod ws2812;
