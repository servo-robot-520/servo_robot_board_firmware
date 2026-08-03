//! 硬件抽象层
//!
//! 封装 ADC、Flash、USB 等外设的具体操作。

pub mod adc;
pub mod flash;
pub mod uart;
#[cfg(feature = "servo")]
pub mod uart1;
pub mod usb;
pub mod ws2812;
