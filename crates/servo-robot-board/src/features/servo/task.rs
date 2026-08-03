//! Servo UART1 task helpers.

#[cfg(feature = "servo")]
use super::{uart, with_usart1};

#[cfg(feature = "servo")]
fn set_tx_enabled(enabled: bool) {
    let gpiob = unsafe { &*stm32f4xx_hal::pac::GPIOB::ptr() };
    gpiob.bsrr().write(|w| {
        if enabled {
            w.bs12().set_bit()
        } else {
            w.br12().set_bit()
        }
    });
}

/// Handle one USART1 interrupt and report whether it received data.
///
/// This is called only by the `servo` feature's manual interrupt vector.
#[cfg(feature = "servo")]
pub fn handle_usart1_irq() -> bool {
    with_usart1(|usart1| {
        let had_rx = usart1.sr().read().rxne().bit_is_set();
        uart::handle_usart1_irq(usart1);
        if !uart::uart1_tx_pending() {
            set_tx_enabled(false);
        }
        had_rx
    })
    .unwrap_or(false)
}

/// Return buffered UART1 data for forwarding to the host.
pub fn servo_rx_check() -> Option<servo_robot_protocol::servo::ServoCmdWrapper> {
    #[cfg(feature = "servo")]
    {
        let mut buf = [0u8; 512];
        let count = uart::uart1_read_rx(&mut buf);
        if count == 0 {
            return None;
        }

        defmt::info!("Servo RX: {} bytes forwarded to host", count);
        Some(servo_robot_protocol::servo::ServoCmdWrapper::new(
            alloc::vec::Vec::from(&buf[..count]),
        ))
    }

    #[cfg(not(feature = "servo"))]
    {
        None
    }
}

/// Queue a host command for UART1 and start transmission.
///
/// In a build without the feature this is intentionally a no-op; the inbound
/// command is rejected before this task is spawned.
pub fn servo_forward(data: &[u8]) {
    #[cfg(feature = "servo")]
    {
        let queued = with_usart1(|usart1| {
            set_tx_enabled(true);
            uart::uart1_enqueue_tx(data);
            uart::uart1_trigger_tx(usart1);
        })
        .is_some();

        if queued {
            defmt::info!("ServoForward: {} bytes sent to UART1", data.len());
        } else {
            defmt::warn!("ServoForward ignored: UART1 is not initialized");
        }
    }

    #[cfg(not(feature = "servo"))]
    {
        let _ = data;
    }
}
