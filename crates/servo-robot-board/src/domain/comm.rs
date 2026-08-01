//! 通讯链路
//!
//! 帧编解码 + TX 队列 + 配置处理。
//!
//! 协议帧格式: [HEAD:1][TYPE:1][LEN:2][PAYLOAD:N][CRC:2]
//! HEAD = 0xAA, CRC = CRC16-CCITT

use servo_robot_protocol::config::{BoardConfigSnapshot, Config, ConfigType};
use servo_robot_protocol::crc::crc16_ccitt_table;
use servo_robot_protocol::frame::{FRAME_HEAD, FrameType, RawFrame, ToPayload, TypedFrame};
use servo_robot_protocol::log::LogLevel;

// ============================================================================
// 帧编解码
// ============================================================================

/// 帧头长度
const FRAME_HEADER_SIZE: usize = 4;
/// CRC 长度
const FRAME_CRC_SIZE: usize = 2;

/// TX 缓冲区大小
pub const TX_BUF_SIZE: usize = 512;
/// RX 缓冲区大小
pub const RX_BUF_SIZE: usize = 512;

/// 帧编码到缓冲区, 返回写入字节数
pub fn encode_frame(frame: &RawFrame, buf: &mut [u8]) -> usize {
    let payload_len = frame.payload.len();
    let total_len = FRAME_HEADER_SIZE + payload_len + FRAME_CRC_SIZE;

    if total_len > buf.len() {
        return 0;
    }

    let mut pos = 0;
    buf[pos] = FRAME_HEAD;
    pos += 1;
    buf[pos] = frame.frame_type.as_u8();
    pos += 1;

    let len_bytes = (payload_len as u16).to_le_bytes();
    buf[pos] = len_bytes[0];
    pos += 1;
    buf[pos] = len_bytes[1];
    pos += 1;

    buf[pos..pos + payload_len].copy_from_slice(&frame.payload);
    pos += payload_len;

    // CRC 覆盖 TYPE + LEN + PAYLOAD
    let crc_data = &buf[1..pos];
    let crc = crc16_ccitt_table(crc_data);
    let crc_bytes = crc.to_le_bytes();
    buf[pos] = crc_bytes[0];
    pos += 1;
    buf[pos] = crc_bytes[1];

    total_len
}

/// 从接收缓冲区中尝试解码一帧
///
/// 返回 `Ok((frame, consumed_bytes))` 或 `Err`
pub fn try_decode_frame(buf: &[u8]) -> Result<(RawFrame, usize), DecodeError> {
    // 查找帧头
    let header_pos = buf
        .iter()
        .position(|&b| b == FRAME_HEAD)
        .ok_or(DecodeError::NoHeader)?;

    if buf.len() - header_pos < FRAME_HEADER_SIZE {
        return Err(DecodeError::Incomplete);
    }

    let frame_type = FrameType::from_u8(buf[header_pos + 1]);
    let payload_len = u16::from_le_bytes([buf[header_pos + 2], buf[header_pos + 3]]) as usize;
    let total_len = FRAME_HEADER_SIZE + payload_len + FRAME_CRC_SIZE;

    if buf.len() - header_pos < total_len {
        return Err(DecodeError::Incomplete);
    }

    let payload_start = header_pos + FRAME_HEADER_SIZE;
    let payload_end = payload_start + payload_len;
    let payload = buf[payload_start..payload_end].to_vec();

    let crc_start = payload_end;
    let received_crc = u16::from_le_bytes([buf[crc_start], buf[crc_start + 1]]);

    let crc_data = &buf[header_pos + 1..payload_end];
    let calculated_crc = crc16_ccitt_table(crc_data);

    if received_crc != calculated_crc {
        return Err(DecodeError::CrcMismatch {
            expected: calculated_crc,
            got: received_crc,
        });
    }

    Ok((
        RawFrame {
            frame_type,
            payload,
        },
        header_pos + total_len,
    ))
}

/// 解码错误
#[derive(Debug, defmt::Format)]
pub enum DecodeError {
    /// 没有找到帧头
    NoHeader,
    /// 数据不完整
    Incomplete,
    /// CRC 校验失败
    CrcMismatch { expected: u16, got: u16 },
}

// ============================================================================
// TX 队列 (无锁环形缓冲区)
// ============================================================================

use core::sync::atomic::{AtomicU16, AtomicUsize, Ordering};

/// 发送队列大小 (必须是 2 的幂)
pub const TX_QUEUE_SIZE: usize = 2048;
/// 队列掩码
const TX_QUEUE_MASK: usize = TX_QUEUE_SIZE - 1;

/// 环形缓冲区
pub struct RingBuffer {
    buf: [u8; TX_QUEUE_SIZE],
    head: AtomicUsize,
    tail: AtomicUsize,
    overflow_count: AtomicU16,
}

/// 全局发送队列
pub static TX_QUEUE: RingBuffer = RingBuffer::new();

impl RingBuffer {
    pub const fn new() -> Self {
        Self {
            buf: [0; TX_QUEUE_SIZE],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            overflow_count: AtomicU16::new(0),
        }
    }

    /// 写入数据，返回实际写入的字节数
    pub fn write(&self, data: &[u8]) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        let available = TX_QUEUE_SIZE - head.wrapping_sub(tail);
        let to_write = data.len().min(available);

        if to_write == 0 {
            self.overflow_count.fetch_add(1, Ordering::Relaxed);
            return 0;
        }

        let write_pos = head & TX_QUEUE_MASK;
        let first_part = (TX_QUEUE_SIZE - write_pos).min(to_write);

        // SAFETY: 单写多读，head 只有生产者修改
        unsafe {
            core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.buf.as_ptr().add(write_pos) as *mut u8,
                first_part,
            );
        }

        if to_write > first_part {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    data.as_ptr().add(first_part),
                    self.buf.as_ptr() as *mut u8,
                    to_write - first_part,
                );
            }
        }

        self.head
            .store(head.wrapping_add(to_write), Ordering::Release);
        to_write
    }

    /// 读取数据，返回实际读取的字节数
    pub fn read(&self, buf: &mut [u8]) -> usize {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        let available = head.wrapping_sub(tail);
        let to_read = buf.len().min(available);

        if to_read == 0 {
            return 0;
        }

        let read_pos = tail & TX_QUEUE_MASK;
        let first_part = (TX_QUEUE_SIZE - read_pos).min(to_read);

        // SAFETY: 单读多写，tail 只有消费者修改
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.buf.as_ptr().add(read_pos),
                buf.as_mut_ptr(),
                first_part,
            );
        }

        if to_read > first_part {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    self.buf.as_ptr(),
                    buf.as_mut_ptr().add(first_part),
                    to_read - first_part,
                );
            }
        }

        self.tail
            .store(tail.wrapping_add(to_read), Ordering::Release);
        to_read
    }

    // 可读数据长度
    pub fn available(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Relaxed);
        head.wrapping_sub(tail)
    }

    /// 可写空间
    pub fn free_space(&self) -> usize {
        TX_QUEUE_SIZE - self.available()
    }

    /// 溢出计数
    pub fn overflow_count(&self) -> u16 {
        self.overflow_count.load(Ordering::Relaxed)
    }

    /// 清空队列
    pub fn clear(&self) {
        self.tail
            .store(self.head.load(Ordering::Relaxed), Ordering::Release);
    }
}

/// 帧编码并写入队列, 返回写入字节数
pub fn enqueue_frame(frame: &RawFrame) -> usize {
    let mut buf = [0u8; 512];
    let len = encode_frame(frame, &mut buf);
    if len > 0 {
        TX_QUEUE.write(&buf[..len])
    } else {
        0
    }
}

/// 从队列读取数据
pub fn dequeue_bytes(buf: &mut [u8]) -> usize {
    TX_QUEUE.read(buf)
}

/// 队列状态: (可用数据, 可用空间, 溢出次数)
pub fn queue_status() -> (usize, usize, u16) {
    (
        TX_QUEUE.available(),
        TX_QUEUE.free_space(),
        TX_QUEUE.overflow_count(),
    )
}

/// 编码 TypedFrame 为 RawFrame
pub fn encode_typed_frame(frame: &TypedFrame) -> Option<RawFrame> {
    match frame {
        TypedFrame::Imu(d) => Some(RawFrame {
            frame_type: FrameType::Imu,
            payload: d.to_bytes(),
        }),
        TypedFrame::Power(d) => Some(RawFrame {
            frame_type: FrameType::Power,
            payload: d.to_bytes(),
        }),
        TypedFrame::Battery(d) => Some(RawFrame {
            frame_type: FrameType::Battery,
            payload: d.to_bytes(),
        }),
        TypedFrame::System(d) => Some(RawFrame {
            frame_type: FrameType::System,
            payload: d.to_bytes(),
        }),
        TypedFrame::Event(d) => Some(RawFrame {
            frame_type: FrameType::Event,
            payload: d.to_bytes(),
        }),
        TypedFrame::Log(d) => Some(RawFrame {
            frame_type: FrameType::Log,
            payload: d.to_bytes(),
        }),
        TypedFrame::AckCfgWrite { success } => Some(RawFrame {
            frame_type: FrameType::AckCfgWrite,
            payload: alloc::vec![*success as u8],
        }),
        TypedFrame::AckCfgQuery(d) => Some(RawFrame {
            frame_type: FrameType::AckCfgQuery,
            payload: d.to_bytes(),
        }),
        TypedFrame::AckCfgQueryAll(d) => Some(RawFrame {
            frame_type: FrameType::AckCfgQueryAll,
            payload: d.to_bytes(),
        }),
        TypedFrame::AckServoCmd(d) => Some(RawFrame {
            frame_type: FrameType::AckServoCmd,
            payload: d.to_payload(),
        }),
        TypedFrame::AckFirmwareUpdate { success, offset } => {
            let mut payload = alloc::vec![*success as u8];
            payload.extend_from_slice(&offset.to_le_bytes());
            Some(RawFrame {
                frame_type: FrameType::AckFirmwareUpdate,
                payload,
            })
        }
        TypedFrame::AckCommand { success } => Some(RawFrame {
            frame_type: FrameType::AckCommand,
            payload: alloc::vec![*success as u8],
        }),
        _ => None,
    }
}

/// 编码 TypedFrame 并入 USB TX 队列, 返回写入字节数
pub fn encode_and_enqueue(frame: &TypedFrame) -> usize {
    match encode_typed_frame(frame) {
        Some(raw) => enqueue_frame(&raw),
        None => 0,
    }
}

/// 编码 TypedFrame 并同时入 USB + UART2 TX 队列
/// 返回 (usb_written, uart_written)
pub fn encode_and_enqueue_dual(frame: &TypedFrame) -> (usize, usize) {
    match encode_typed_frame(frame) {
        Some(raw) => {
            let usb = enqueue_frame(&raw);
            let uart = crate::hal::uart::enqueue_frame(&raw);
            (usb, uart)
        }
        None => (0, 0),
    }
}

// ============================================================================
// 配置处理
// ============================================================================

/// 处理配置写入命令
///
/// 返回 true 表示配置已修改，需要保存到 Flash
pub fn handle_config_write(
    pwr_servo_en: &mut impl embedded_hal::digital::StatefulOutputPin,
    bat_ext_out_en: &mut impl embedded_hal::digital::StatefulOutputPin,
    pwr_5v_en: &mut impl embedded_hal::digital::StatefulOutputPin,
    _pwr_key: &mut impl embedded_hal::digital::StatefulOutputPin,
    config: &mut BoardConfigSnapshot,
    protection_mgr: &mut crate::domain::protection::ProtectionManager,
    cfg: Config,
) -> bool {
    match cfg {
        Config::SwitchPowerServo(on) => {
            if on {
                pwr_servo_en.set_high().ok();
            } else {
                pwr_servo_en.set_low().ok();
            }
            config.power_servo_on = on;
            if on {
                protection_mgr.reset_servo_power();
            }
        }
        Config::SwitchPower5V(on) => {
            if on {
                pwr_5v_en.set_high().ok();
            } else {
                pwr_5v_en.set_low().ok();
            }
            config.power_5v_on = on;
            if on {
                protection_mgr.reset_5v_power();
            }
        }
        Config::SwitchCharge(on) => {
            config.charge_on = on;
        }
        Config::SwitchBatExtOut(on) => {
            if on {
                bat_ext_out_en.set_high().ok();
            } else {
                bat_ext_out_en.set_low().ok();
            }
            config.bat_ext_out_on = on;
        }
        Config::PowerServoCurrentLimitMa(v) => config.servo_current_limit_ma = v,
        Config::PowerServoTempLimit(v) => config.servo_temp_limit = v,
        Config::Power5vTempLimit(v) => config.temp_5v_limit = v,
        Config::ChargeMaxCurrentMa(v) => config.charge_max_current_ma = v,
        Config::ChargeTempDerating(v) => config.charge_temp_derating = v,
        Config::ChargeTempLimit(v) => config.charge_temp_limit = v,
        Config::ChargeStopVoltageMv(v) => config.charge_stop_voltage_mv = v,
        Config::ChargeStopSoc(v) => config.charge_stop_percentage = v,
        Config::TxLogLevel(level) => config.tx_log_level = level,
        Config::ServoBaudRate(v) => config.servo_baud_rate = v,
    }
    true
}

// ============================================================================
// 日志过滤
// ============================================================================

/// 检查日志等级是否应该发送
pub fn should_send_log(tx_log_level: LogLevel, msg_level: LogLevel) -> bool {
    if tx_log_level == LogLevel::OFF {
        return false;
    }
    (msg_level as u8) >= (tx_log_level as u8)
}

// ============================================================================
// 配置查询
// ============================================================================

/// 获取配置值
pub fn get_config_value(c: &BoardConfigSnapshot, ct: ConfigType) -> Config {
    match ct {
        ConfigType::SwitchServoPower => Config::SwitchPowerServo(c.power_servo_on),
        ConfigType::Switch5VPower => Config::SwitchPower5V(c.power_5v_on),
        ConfigType::SwitchCharge => Config::SwitchCharge(c.charge_on),
        ConfigType::SwitchBatExtOut => Config::SwitchBatExtOut(c.bat_ext_out_on),
        ConfigType::PowerServoCurrentLimitMa => {
            Config::PowerServoCurrentLimitMa(c.servo_current_limit_ma)
        }
        ConfigType::PowerServoTempLimit => Config::PowerServoTempLimit(c.servo_temp_limit),
        ConfigType::Power5vTempLimit => Config::Power5vTempLimit(c.temp_5v_limit),
        ConfigType::ChargeMaxCurrentMa => Config::ChargeMaxCurrentMa(c.charge_max_current_ma),
        ConfigType::ChargeTempDerating => Config::ChargeTempDerating(c.charge_temp_derating),
        ConfigType::ChargeTempLimit => Config::ChargeTempLimit(c.charge_temp_limit),
        ConfigType::ChargeStopVoltageMv => Config::ChargeStopVoltageMv(c.charge_stop_voltage_mv),
        ConfigType::ChargeStopSoc => Config::ChargeStopSoc(c.charge_stop_percentage),
        ConfigType::TxLogLevel => Config::TxLogLevel(c.tx_log_level),
        ConfigType::ServoBaudRate => Config::ServoBaudRate(c.servo_baud_rate),
    }
}
