//! 通讯传输层
//!
//! 帧编解码 + TX 队列 + USB 复合设备 (CDC + MSD) + 虚拟 FAT12 磁盘。
//!
//! 协议帧格式: [HEAD:1][TYPE:1][LEN:2][PAYLOAD:N][CRC:2]
//! HEAD = 0xAA, CRC = CRC16-CCITT

use servo_robot_protocol::crc::crc16_ccitt_table;
use servo_robot_protocol::frame::{FRAME_HEAD, FrameType, RawFrame, ToPayload, TypedFrame};

use crate::platform::flash;

/// Encode a raw protocol frame into `buf`.
///
/// Protocol framing is part of communication, not the UART hardware layer.
pub fn encode_frame(frame: &RawFrame, buf: &mut [u8]) -> usize {
    let payload_len = frame.payload.len();
    let total_len = FRAME_HEADER_SIZE + payload_len + FRAME_CRC_SIZE;
    if total_len > buf.len() {
        return 0;
    }

    buf[0] = FRAME_HEAD;
    buf[1] = frame.frame_type.as_u8();
    buf[2..4].copy_from_slice(&(payload_len as u16).to_le_bytes());
    buf[FRAME_HEADER_SIZE..FRAME_HEADER_SIZE + payload_len].copy_from_slice(&frame.payload);

    let crc_offset = FRAME_HEADER_SIZE + payload_len;
    let crc = crc16_ccitt_table(&buf[1..crc_offset]);
    buf[crc_offset..crc_offset + FRAME_CRC_SIZE].copy_from_slice(&crc.to_le_bytes());
    total_len
}

/// 帧头长度
const FRAME_HEADER_SIZE: usize = 4;
/// CRC 长度
const FRAME_CRC_SIZE: usize = 2;

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
    #[allow(dead_code)]
    pub fn available(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Relaxed);
        head.wrapping_sub(tail)
    }

    /// 可写空间
    #[allow(dead_code)]
    pub fn free_space(&self) -> usize {
        TX_QUEUE_SIZE - self.available()
    }

    /// 溢出计数
    #[allow(dead_code)]
    pub fn overflow_count(&self) -> u16 {
        self.overflow_count.load(Ordering::Relaxed)
    }

    /// 清空队列
    #[allow(dead_code)]
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

/// 编码 TypedFrame 并同时入 USB + UART2 TX 队列
/// 返回 (usb_written, uart_written)
pub fn encode_and_enqueue_dual(frame: &TypedFrame) -> (usize, usize) {
    match encode_typed_frame(frame) {
        Some(raw) => {
            let usb = enqueue_frame(&raw);
            let mut uart_buf = [0u8; 512];
            let uart = encode_frame(&raw, &mut uart_buf);
            let uart = if uart > 0 {
                crate::platform::uart2::enqueue_tx(&uart_buf[..uart])
            } else {
                0
            };
            (usb, uart)
        }
        None => (0, 0),
    }
}

// ============================================================================
// USB 复合设备: CDC ACM (虚拟串口) + MSD (大容量存储)
// ============================================================================
//
// CDC: 与上位机通讯 (协议帧收发)
// MSD: 虚拟 FAT12 磁盘, 暴露 FIRMWARE.BIN, 支持固件拖入更新

use usb_device::LangID;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid};
use usbd_serial::SerialPort;
use usbd_storage::subclass::scsi::{Scsi, ScsiCommand};
use usbd_storage::transport::bbb::BulkOnly; // used in type annotation

/// USB VID/PID
const USB_VID: u16 = 0x1209;
const USB_PID: u16 = 0x0001;

// ============================================================================
// MSD 虚拟 FAT12 磁盘
// ============================================================================

/// 虚拟磁盘扇区大小
const SECTOR_SIZE: usize = 512;
/// 总扇区数
const TOTAL_SECTORS: u32 = 64;
/// 数据区起始扇区 (引导扇区 + FAT + 根目录)
const DATA_START_SECTOR: u32 = 3;

fn make_boot_sector(buf: &mut [u8; 512]) {
    buf.fill(0);
    buf[0] = 0xEB;
    buf[1] = 0x3C;
    buf[2] = 0x90;
    buf[3..11].copy_from_slice(b"SERVO   ");
    buf[11..13].copy_from_slice(&512u16.to_le_bytes());
    buf[13] = 1;
    buf[14..16].copy_from_slice(&1u16.to_le_bytes());
    buf[16] = 1;
    buf[17..19].copy_from_slice(&16u16.to_le_bytes());
    buf[19..21].copy_from_slice(&(TOTAL_SECTORS as u16).to_le_bytes());
    buf[21] = 0xF8;
    buf[22..24].copy_from_slice(&1u16.to_le_bytes());
    buf[24..26].copy_from_slice(&1u16.to_le_bytes());
    buf[26..28].copy_from_slice(&1u16.to_le_bytes());
    buf[36] = 0x80;
    buf[38] = 0x29;
    buf[39..43].copy_from_slice(&0x12345678u32.to_le_bytes());
    buf[43..54].copy_from_slice(b"SERVO_ROBOT");
    buf[54..62].copy_from_slice(b"FAT12   ");
    buf[510] = 0x55;
    buf[511] = 0xAA;
}

fn write_fat12_entry(buf: &mut [u8; 512], index: u16, value: u16) {
    let byte_offset = (index as usize * 3) / 2;
    if index % 2 == 0 {
        buf[byte_offset] = (value & 0xFF) as u8;
        buf[byte_offset + 1] = (buf[byte_offset + 1] & 0xF0) | ((value >> 8) & 0x0F) as u8;
    } else {
        buf[byte_offset] = (buf[byte_offset] & 0x0F) | ((value << 4) & 0xF0) as u8;
        buf[byte_offset + 1] = ((value >> 4) & 0xFF) as u8;
    }
}

fn make_fat(buf: &mut [u8; 512], firmware_size: u32) {
    buf.fill(0);
    buf[0] = 0xF8;
    buf[1] = 0xFF;
    buf[2] = 0xFF;

    if firmware_size > 0 {
        let clusters = ((firmware_size as usize + SECTOR_SIZE - 1) / SECTOR_SIZE) as u16;
        if clusters > 1 {
            for i in 2..(2 + clusters - 1) {
                write_fat12_entry(buf, i, i + 1);
            }
            write_fat12_entry(buf, 2 + clusters - 1, 0xFFF);
        } else if clusters == 1 {
            write_fat12_entry(buf, 2, 0xFFF);
        }
    }
}

fn make_root_dir(buf: &mut [u8; 512], firmware_size: u32) {
    buf.fill(0);
    buf[0..11].copy_from_slice(b"SERVO_ROBOT");
    buf[11] = 0x08;

    if firmware_size > 0 {
        let entry = &mut buf[32..64];
        entry[0..11].copy_from_slice(b"FIRMWARE BIN");
        entry[11] = 0x20;
        entry[26..28].copy_from_slice(&2u16.to_le_bytes());
        entry[28..32].copy_from_slice(&firmware_size.to_le_bytes());
    }
}

// ============================================================================
// 固件接收器 (流式写入 Flash)
// ============================================================================

/// 固件接收状态
///
/// MSD 路径写入格式与 Protocol OTA 路径一致:
///   [0..16]   OTA 镜像头 (magic="OTAI", image_size, image_crc32)
///   [16..]    固件数据 (末尾 4 字节为 CRC32)
pub struct FirmwareReceiver {
    /// 固件总大小 (从 FAT 目录项读取, 包含末尾 CRC32)
    pub firmware_size: u32,
    /// 是否就绪 (传输完成)
    pub firmware_ready: bool,
    /// 写入偏移 (OTA 区域内, 从 OTA_IMAGE_HEADER_SIZE 开始)
    write_offset: u32,
}

impl FirmwareReceiver {
    pub const fn new() -> Self {
        Self {
            firmware_size: 0,
            firmware_ready: false,
            write_offset: 0,
        }
    }

    /// 处理 SCSI READ 命令: 读取虚拟磁盘扇区
    pub fn read_sector(&self, lba: u32, buf: &mut [u8; 512]) {
        match lba {
            0 => make_boot_sector(buf),
            1 => make_fat(buf, self.firmware_size),
            2 => make_root_dir(buf, self.firmware_size),
            3..TOTAL_SECTORS => {
                let flash_offset = ((lba - DATA_START_SECTOR) * SECTOR_SIZE as u32) as usize;
                if flash_offset < self.firmware_size as usize {
                    let addr =
                        flash::OTA_TEMP_ADDR + flash::OTA_IMAGE_HEADER_SIZE + flash_offset as u32;
                    let end = (flash_offset + SECTOR_SIZE).min(self.firmware_size as usize);
                    let len = end - flash_offset;
                    flash::read_flash(addr, &mut buf[..len]);
                    if len < SECTOR_SIZE {
                        buf[len..].fill(0);
                    }
                } else {
                    buf.fill(0);
                }
            }
            _ => buf.fill(0),
        }
    }

    /// 处理 SCSI WRITE 命令: 写入固件数据到 Flash
    ///
    /// 写入布局:
    ///   LBA 2: 检测 FIRMWARE BIN 目录项, 擦除 OTA 区域, 写入 OTA 镜像头 (占位)
    ///   LBA 3+: 固件数据写入 OTA_TEMP_ADDR + OTA_IMAGE_HEADER_SIZE
    ///   传输完成后回填 Header 中的 image_size 和 image_crc32
    pub fn write_sector(
        &mut self,
        lba: u32,
        data: &[u8; 512],
        flash_periph: &stm32f4xx_hal::pac::FLASH,
    ) {
        match lba {
            0 | 1 => { /* 忽略引导扇区和 FAT 写入 */ }
            2 => {
                if data.len() >= 64 {
                    let entry = &data[32..64];
                    if entry[0..11] == *b"FIRMWARE BIN" {
                        let file_size =
                            u32::from_le_bytes([entry[28], entry[29], entry[30], entry[31]]);
                        // 固件文件必须大于 Header 大小 + CRC32 大小
                        if file_size > flash::OTA_IMAGE_HEADER_SIZE + 4 {
                            self.firmware_size = file_size;
                            self.write_offset = 0;
                            self.firmware_ready = false;
                            flash::erase_ota_temp(flash_periph).ok();
                            // 写入 OTA 镜像头 占位 (image_size 和 CRC 后续回填)
                            patch_ota_header(
                                flash_periph,
                                0, // image_size placeholder
                                0, // image_crc32 placeholder
                            );
                        }
                    }
                }
            }
            3..TOTAL_SECTORS => {
                if self.firmware_size > 0 {
                    let addr =
                        flash::OTA_TEMP_ADDR + flash::OTA_IMAGE_HEADER_SIZE + self.write_offset;
                    flash::program_flash(flash_periph, addr, data).ok();
                    self.write_offset += SECTOR_SIZE as u32;

                    if self.write_offset >= self.firmware_size {
                        // 传输完成: 从 Flash 读取末尾 4 字节作为 CRC32,
                        // 并回填 Header 中的 image_size 和 image_crc32
                        let image_size = self.firmware_size - 4;
                        let crc_addr =
                            flash::OTA_TEMP_ADDR + flash::OTA_IMAGE_HEADER_SIZE + image_size;
                        let mut crc_buf = [0u8; 4];
                        flash::read_flash(crc_addr, &mut crc_buf);
                        let image_crc32 = u32::from_le_bytes(crc_buf);
                        patch_ota_header(flash_periph, image_size, image_crc32);
                        self.firmware_ready = true;
                    }
                }
            }
            _ => {}
        }
    }
}

/// 写入或回填 OTA 镜像头 到 OTA_TEMP_ADDR
fn patch_ota_header(flash_periph: &stm32f4xx_hal::pac::FLASH, image_size: u32, image_crc32: u32) {
    let mut hdr = [0u8; flash::OTA_IMAGE_HEADER_SIZE as usize];
    // magic: "OTAI"
    hdr[0..4].copy_from_slice(&flash::OTA_IMAGE_MAGIC.to_le_bytes());
    hdr[4] = flash::OTA_IMAGE_FORMAT_VERSION;
    hdr[5] = flash::OTA_TARGET_MCU_F411;
    // _reserved [6..8] = 0
    hdr[8..12].copy_from_slice(&image_size.to_le_bytes());
    hdr[12..16].copy_from_slice(&image_crc32.to_le_bytes());
    flash::program_flash(flash_periph, flash::OTA_TEMP_ADDR, &hdr).ok();
}

// ============================================================================
// USB 复合设备 (CDC + MSD)
// ============================================================================

/// MSD 块缓冲区 (static 保证地址稳定)
static mut MSD_BLOCK_BUF: [u8; SECTOR_SIZE] = [0; SECTOR_SIZE];

/// USB 复合设备 (CDC + MSD)
pub struct UsbCompositeDevice<'a, B: usb_device::bus::UsbBus> {
    usb_dev: UsbDevice<'a, B>,
    serial: SerialPort<'a, B>,
    msd: Scsi<BulkOnly<'a, B, &'static mut [u8]>>,
    firmware_receiver: FirmwareReceiver,
}

impl<'a, B: usb_device::bus::UsbBus> UsbCompositeDevice<'a, B> {
    /// 创建 USB 复合设备 (CDC + MSD)
    pub fn new(usb_bus: &'a UsbBusAllocator<B>) -> Self {
        let serial = SerialPort::new(usb_bus);

        // MSD: SCSI over Bulk-Only Transport
        let block_buf: &'static mut [u8] = unsafe { &mut *core::ptr::addr_of_mut!(MSD_BLOCK_BUF) };
        let msd = Scsi::new(usb_bus, 64, 0, block_buf).unwrap();

        let strings = StringDescriptors::new(LangID::EN)
            .manufacturer("ServoRobot")
            .product("ServoRobot Board")
            .serial_number("0001");

        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(USB_VID, USB_PID))
            .strings(&[strings])
            .unwrap()
            .composite_with_iads()
            .build();

        Self {
            usb_dev,
            serial,
            msd,
            firmware_receiver: FirmwareReceiver::new(),
        }
    }

    /// USB 轮询 (CDC + MSD)
    pub fn poll(&mut self) -> bool {
        let changed = self.usb_dev.poll(&mut [&mut self.serial, &mut self.msd]);

        // 处理 MSD SCSI 命令
        self.handle_msd_commands();

        changed
    }

    /// 处理 MSD SCSI 命令
    fn handle_msd_commands(&mut self) {
        let fw = &mut self.firmware_receiver as *mut FirmwareReceiver;
        let flash = unsafe { stm32f4xx_hal::pac::Peripherals::steal() }.FLASH;
        let flash_ref = &flash as *const _;

        unsafe {
            self.msd
                .poll_command(|mut cmd| {
                    let fw = &mut *fw;
                    let flash = &*flash_ref;

                    match cmd.kind {
                        ScsiCommand::TestUnitReady => {
                            cmd.pass(0);
                            Ok(())
                        }
                        ScsiCommand::Inquiry { .. } => {
                            let mut resp = [0u8; 36];
                            resp[0] = 0x00; // SBC
                            resp[1] = 0x80; // removable
                            resp[2] = 0x04; // SPC-2
                            resp[3] = 0x02; // response data format
                            resp[4] = 31; // additional length
                            resp[8..16].copy_from_slice(b"SERVO   ");
                            resp[16..32].copy_from_slice(b"Robot Board FW  ");
                            resp[32..36].copy_from_slice(b"1.00");
                            cmd.try_write_data_all(&resp)?;
                            cmd.pass(36);
                            Ok(())
                        }
                        ScsiCommand::ReadCapacity10 => {
                            let last_lba = TOTAL_SECTORS - 1;
                            let mut resp = [0u8; 8];
                            resp[0..4].copy_from_slice(&last_lba.to_be_bytes());
                            resp[4..8].copy_from_slice(&(SECTOR_SIZE as u32).to_be_bytes());
                            cmd.try_write_data_all(&resp)?;
                            cmd.pass(8);
                            Ok(())
                        }
                        ScsiCommand::ReadCapacity16 { .. } => {
                            let last_lba = TOTAL_SECTORS - 1;
                            let mut resp = [0u8; 12];
                            resp[0..4].copy_from_slice(&last_lba.to_be_bytes());
                            resp[4..8].copy_from_slice(&(SECTOR_SIZE as u32).to_be_bytes());
                            resp[8] = 0;
                            cmd.try_write_data_all(&resp)?;
                            cmd.pass(12);
                            Ok(())
                        }
                        ScsiCommand::Read { lba, len } => {
                            let blocks = len as u32;
                            let mut total = 0u32;
                            for i in 0..blocks {
                                let mut buf = [0u8; SECTOR_SIZE];
                                fw.read_sector(lba + i, &mut buf);
                                cmd.try_write_data_all(&buf)?;
                                total += SECTOR_SIZE as u32;
                            }
                            cmd.pass(total);
                            Ok(())
                        }
                        ScsiCommand::Write { lba, len } => {
                            let blocks = len as u32;
                            let mut total = 0u32;
                            for _ in 0..blocks {
                                let mut buf = [0u8; SECTOR_SIZE];
                                match cmd.read_data(&mut buf) {
                                    Ok(n) if n >= SECTOR_SIZE => {
                                        fw.write_sector(lba, &buf, flash);
                                        total += SECTOR_SIZE as u32;
                                    }
                                    _ => break,
                                }
                            }
                            cmd.pass(total);
                            Ok(())
                        }
                        ScsiCommand::RequestSense { .. } => {
                            let mut resp = [0u8; 18];
                            resp[0] = 0x70; // response code
                            resp[2] = 0x00; // sense key: no sense
                            resp[7] = 10; // additional sense length
                            cmd.try_write_data_all(&resp)?;
                            cmd.pass(18);
                            Ok(())
                        }
                        ScsiCommand::ModeSense6 { .. } => {
                            let resp = [0x03, 0x00, 0x00, 0x00];
                            cmd.try_write_data_all(&resp)?;
                            cmd.pass(4);
                            Ok(())
                        }
                        ScsiCommand::ModeSense10 { .. } => {
                            let resp = [0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
                            cmd.try_write_data_all(&resp)?;
                            cmd.pass(8);
                            Ok(())
                        }
                        ScsiCommand::ReadFormatCapacities { .. } => {
                            let mut resp = [0u8; 12];
                            resp[3] = 8;
                            resp[4..8].copy_from_slice(&TOTAL_SECTORS.to_be_bytes());
                            resp[8] = 0x02; // formatted media
                            let block_size_bytes = (SECTOR_SIZE as u16).to_be_bytes();
                            resp[9] = block_size_bytes[0];
                            resp[10] = block_size_bytes[1];
                            resp[11] = 0;
                            cmd.try_write_data_all(&resp)?;
                            cmd.pass(12);
                            Ok(())
                        }
                        _ => {
                            cmd.fail(0);
                            Ok(())
                        }
                    }
                })
                .ok();
        }
    }

    /// 检查是否有新固件 (MSD 写入完成)
    pub fn has_new_firmware(&self) -> bool {
        self.firmware_receiver.firmware_ready
    }

    /// 尝试从 USB CDC 接收完整的协议帧
    pub fn try_receive_frame(
        &mut self,
        rx_buf: &mut [u8],
        rx_pos: &mut usize,
    ) -> Option<servo_robot_protocol::frame::RawFrame> {
        let mut tmp = [0u8; 64];
        match self.serial.read(&mut tmp) {
            Ok(count) if count > 0 => {
                let space = rx_buf.len() - *rx_pos;
                let to_copy = count.min(space);
                if to_copy > 0 {
                    rx_buf[*rx_pos..*rx_pos + to_copy].copy_from_slice(&tmp[..to_copy]);
                    *rx_pos += to_copy;
                }
            }
            _ => {}
        }

        if *rx_pos > 0 {
            match try_decode_frame(&rx_buf[..*rx_pos]) {
                Ok((frame, consumed)) => {
                    rx_buf.copy_within(consumed..*rx_pos, 0);
                    *rx_pos -= consumed;
                    return Some(frame);
                }
                Err(DecodeError::NoHeader) => {
                    *rx_pos = 0;
                }
                Err(DecodeError::Incomplete) => {}
                Err(DecodeError::CrcMismatch { .. }) => {
                    rx_buf.copy_within(1..*rx_pos, 0);
                    *rx_pos = rx_pos.saturating_sub(1);
                }
            }
        }

        None
    }

    /// 刷新 TX 队列到 USB
    pub fn flush_tx_queue(&mut self) {
        let mut buf = [0u8; 512];
        let len = dequeue_bytes(&mut buf);
        if len > 0 {
            let _ = self.serial.write(&buf[..len]);
            let _ = self.serial.flush();
        }
    }
}
