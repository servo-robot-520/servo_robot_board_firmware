//! UART 驱动
//!
//! UART2 (PA2=TX, PA3=RX): 与 Linux 上位机通讯
//! UART1 (PA15=TX, PA10=RX): 与串口舵机通讯，PB12=SERVO_TX_EN
//!
//! TX: 中断驱动，从 ring buffer 取字节发送
//! RX: 中断驱动读取

use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use stm32f4xx_hal::pac::{USART1, USART2};

/// UART2 TX ring buffer 大小 (必须是 2 的幂)
pub const UART_TX_BUF_SIZE: usize = 2048;

/// UART2 接收缓冲区大小
pub const UART_RX_BUF_SIZE: usize = 512;

/// 全局 TX ring buffer
pub static UART_TX_BUF: UartRingBuffer<2048> = UartRingBuffer::new();

/// 全局 RX ring buffer
pub static UART2_RX_BUF: UartRingBuffer<512> = UartRingBuffer::new();

/// UART 环形缓冲区 (const generic 大小，必须是 2 的幂)
pub struct UartRingBuffer<const N: usize> {
    buf: [u8; N],
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl<const N: usize> UartRingBuffer<N> {
    const MASK: usize = N - 1;

    pub const fn new() -> Self {
        Self {
            buf: [0; N],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// 写入数据，返回实际写入字节数
    pub fn write(&self, data: &[u8]) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        let available = N - head.wrapping_sub(tail);
        let to_write = data.len().min(available);
        if to_write == 0 {
            return 0;
        }
        let pos = head & Self::MASK;
        let first = (N - pos).min(to_write);
        unsafe {
            core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.buf.as_ptr().add(pos) as *mut u8,
                first,
            );
        }
        if to_write > first {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    data.as_ptr().add(first),
                    self.buf.as_ptr() as *mut u8,
                    to_write - first,
                );
            }
        }
        self.head
            .store(head.wrapping_add(to_write), Ordering::Release);
        to_write
    }

    /// 读取一个字节（从 tail 位置）
    pub fn read_byte(&self) -> Option<u8> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail == head {
            return None;
        }
        let pos = tail & Self::MASK;
        let byte = unsafe { *self.buf.as_ptr().add(pos) };
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(byte)
    }

    /// 可读数据长度
    pub fn available(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Relaxed);
        head.wrapping_sub(tail)
    }
}

/// 初始化 USART2
///
/// PA2 = USART2_TX (AF7), PA3 = USART2_RX (AF7), 115200 8N1
///
/// 注意: GPIO AF 配置和时钟使能在 main.rs init() 中通过直接寄存器操作完成。
/// 此函数仅用于非 RTIC 场景，RTIC app 中请直接在 init() 中配置。

/// 发送单个字节（阻塞等待 TXE）
pub fn send_byte(usart2: &USART2, byte: u8) {
    while !usart2.sr().read().txe().bit_is_set() {}
    usart2.dr().write(|w| unsafe { w.dr().bits(byte as u16) });
}

/// 从 ring buffer 发送一个字节（如果有）
/// 返回 true 表示发送了数据
pub fn try_send_from_queue(usart2: &USART2) -> bool {
    if let Some(byte) = UART_TX_BUF.read_byte() {
        while !usart2.sr().read().txe().bit_is_set() {}
        usart2.dr().write(|w| unsafe { w.dr().bits(byte as u16) });
        true
    } else {
        false
    }
}

/// 将数据写入 UART TX 队列
pub fn enqueue_tx(data: &[u8]) -> usize {
    UART_TX_BUF.write(data)
}

/// 处理 USART2 中断
///
/// - TXE: TX 寄存器空，从 ring buffer 取下一个字节
/// - RXNE: RX 寄存器非空，读取数据存入 RX ring buffer
pub fn handle_usart2_irq(usart2: &USART2) {
    let sr = usart2.sr().read();

    // TX 中断: TXE 且 TXEIE 使能
    if sr.txe().bit_is_set() {
        if let Some(byte) = UART_TX_BUF.read_byte() {
            usart2.dr().write(|w| unsafe { w.dr().bits(byte as u16) });
        } else {
            // 队列空，关闭 TXE 中断
            usart2.cr1().modify(|_, w| w.txeie().clear_bit());
        }
    }

    // RX 中断: RXNE
    if sr.rxne().bit_is_set() {
        let data = usart2.dr().read().dr().bits() as u8;
        UART2_RX_BUF.write(&[data]);
    }
}

/// 从 UART2 RX 缓冲区尝试解码一帧
///
/// 流式解析: 搜索帧头 0xAA, 解码失败时消费字节继续搜索
/// 返回解码成功的 RawFrame，或 None
pub fn uart2_try_decode_frame() -> Option<servo_robot_protocol::frame::RawFrame> {
    use servo_robot_protocol::frame::RawFrame;

    loop {
        let available = UART2_RX_BUF.available();
        if available < 4 {
            return None;
        }

        // 读取数据到临时缓冲区
        let mut buf = [0u8; 512];
        let to_read = available.min(512);
        let rx_mask: usize = UART_RX_BUF_SIZE - 1;
        let head = UART2_RX_BUF.head.load(Ordering::Acquire);
        let tail = UART2_RX_BUF.tail.load(Ordering::Relaxed);
        for i in 0..to_read {
            let pos = (tail + i) & rx_mask;
            buf[i] = unsafe { *UART2_RX_BUF.buf.as_ptr().add(pos) };
        }

        // 搜索帧头 0xAA
        let header_pos = buf[..to_read].iter().position(|&b| b == 0xAA);
        match header_pos {
            None => {
                // 没有找到帧头，丢弃所有数据
                for _ in 0..to_read {
                    UART2_RX_BUF.read_byte();
                }
                continue;
            }
            Some(pos) if pos > 0 => {
                // 丢弃帧头之前的数据
                for _ in 0..pos {
                    UART2_RX_BUF.read_byte();
                }
                continue;
            }
            Some(_) => {
                // 帧头在位置 0，尝试解码
                match RawFrame::decode(&buf[..to_read]) {
                    Ok((frame, consumed)) => {
                        for _ in 0..consumed {
                            UART2_RX_BUF.read_byte();
                        }
                        return Some(frame);
                    }
                    Err(_) => {
                        // 解码失败，丢弃帧头字节，继续搜索
                        UART2_RX_BUF.read_byte();
                        continue;
                    }
                }
            }
        }
    }
}

/// 触发 UART2 TX 中断发送
///
/// 使能 TXE 中断，中断 handler 会从 ring buffer 发送数据
pub fn trigger_tx(usart2: &USART2) {
    if UART_TX_BUF.available() > 0 {
        usart2.cr1().modify(|_, w| w.txeie().set_bit());
    }
}

/// 编码帧并写入 UART2 TX 队列，返回写入字节数
pub fn enqueue_frame(frame: &servo_robot_protocol::frame::RawFrame) -> usize {
    let mut buf = [0u8; 512];
    let len = crate::domain::comm::encode_frame(frame, &mut buf);
    if len > 0 { enqueue_tx(&buf[..len]) } else { 0 }
}

// ============================================================================
// UART1 驱动 (PA15=TX, PA10=RX, PB12=SERVO_TX_EN)
//
// 与串口舵机通讯。
// TX: 中断驱动，发送前拉高 PB12，发送完成后拉低 PB12
// RX: 中断驱动，收到数据后包装为 ServoCmdWrapper 返回给上位机
// ============================================================================

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
