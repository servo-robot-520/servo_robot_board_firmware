//! UART1 驱动 (PA15=TX, PA10=RX, PB12=SERVO_TX_EN) - 仅 servo 特性
//!
//! 与串口舵机通讯。
//! TX: 中断驱动，发送前拉高 PB12，发送完成后拉低 PB12
//! RX: 中断驱动，收到数据后包装为 ServoCmdWrapper 返回给上位机

use core::sync::atomic::{AtomicU8, Ordering};
use stm32f4xx_hal::pac::USART1;

use super::uart::UartRingBuffer;

/// UART1 TX ring buffer 大小
pub const UART1_TX_BUF_SIZE: usize = 1024;

/// UART1 RX ring buffer 大小
pub const UART1_RX_BUF_SIZE: usize = 512;

/// UART1 TX ring buffer
pub static UART1_TX_BUF: UartRingBuffer<1024> = UartRingBuffer::new();

/// UART1 RX ring buffer
pub static UART1_RX_BUF: UartRingBuffer<512> = UartRingBuffer::new();

/// UART1 是否正在发送（用于 TX 完成后拉低 PB12）
static UART1_TX_ACTIVE: AtomicU8 = AtomicU8::new(0);

/// 写入 UART1 TX 队列
pub fn uart1_enqueue_tx(data: &[u8]) -> usize {
    UART1_TX_BUF.write(data)
}

/// 编码帧并写入 UART1 TX 队列
pub fn uart1_enqueue_frame(frame: &servo_robot_protocol::frame::RawFrame) -> usize {
    let mut buf = [0u8; 512];
    let len = crate::domain::comm::encode_frame(frame, &mut buf);
    if len > 0 {
        uart1_enqueue_tx(&buf[..len])
    } else {
        0
    }
}

/// 触发 UART1 TX 中断发送（需要先拉高 SERVO_TX_EN）
pub fn uart1_trigger_tx(usart1: &USART1) {
    if UART1_TX_BUF.available() > 0 {
        UART1_TX_ACTIVE.store(1, Ordering::Relaxed);
        usart1.cr1().modify(|_, w| w.txeie().set_bit());
    }
}

/// 检查 UART1 是否还有数据待发送
pub fn uart1_tx_pending() -> bool {
    UART1_TX_BUF.available() > 0 || UART1_TX_ACTIVE.load(Ordering::Relaxed) != 0
}

/// 读取 UART1 RX 缓冲区中的所有数据
///
/// 返回读取的字节数，数据写入 `buf`
pub fn uart1_read_rx(buf: &mut [u8]) -> usize {
    let mut count = 0;
    for byte in buf.iter_mut() {
        match UART1_RX_BUF.read_byte() {
            Some(b) => {
                *byte = b;
                count += 1;
            }
            None => break,
        }
    }
    count
}

/// 处理 USART1 中断
///
/// - TXE: TX 寄存器空，从 ring buffer 取下一个字节
/// - RXNE: RX 寄存器非空，读取数据存入 RX ring buffer
pub fn handle_usart1_irq(usart1: &USART1) {
    let sr = usart1.sr().read();

    // TX 中断: TXE 且 TXEIE 使能
    if sr.txe().bit_is_set() {
        if let Some(byte) = UART1_TX_BUF.read_byte() {
            usart1.dr().write(|w| unsafe { w.dr().bits(byte as u16) });
        } else {
            // 队列空，关闭 TXE 中断，标记发送完成
            usart1.cr1().modify(|_, w| w.txeie().clear_bit());
            UART1_TX_ACTIVE.store(0, Ordering::Relaxed);
        }
    }

    // RX 中断: RXNE
    if sr.rxne().bit_is_set() {
        let data = usart1.dr().read().dr().bits() as u8;
        UART1_RX_BUF.write(&[data]);
    }
}
