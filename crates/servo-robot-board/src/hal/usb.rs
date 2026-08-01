//! USB 复合设备: CDC ACM (虚拟串口) + MSD (大容量存储)
//!
//! CDC: 与上位机通讯 (协议帧收发)
//! MSD: 虚拟 FAT12 磁盘, 暴露 FIRMWARE.BIN, 支持固件拖入更新

use usb_device::LangID;
use usb_device::UsbError;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid};
use usbd_serial::SerialPort;
use usbd_storage::subclass::scsi::{Scsi, ScsiCommand};
use usbd_storage::transport::bbb::BulkOnly; // used in type annotation

use crate::domain::comm;
use crate::hal::flash;

/// USB VID/PID
const USB_VID: u16 = 0x1209;
const USB_PID: u16 = 0x0001;

/// USB 发送缓冲区大小
pub const USB_TX_BUF_SIZE: usize = 512;

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
pub struct FirmwareReceiver {
    /// 固件总大小
    pub firmware_size: u32,
    /// 是否就绪 (传输完成)
    pub firmware_ready: bool,
    /// 写入偏移 (Flash OTA 区域内)
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
                    let addr = flash::OTA_TEMP_ADDR + 4 + flash_offset as u32;
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
                        if file_size > 0 {
                            self.firmware_size = file_size;
                            self.write_offset = 0;
                            self.firmware_ready = false;
                            flash::erase_ota_temp(flash_periph).ok();
                            let size_bytes = file_size.to_le_bytes();
                            flash::program_flash(flash_periph, flash::OTA_TEMP_ADDR, &size_bytes)
                                .ok();
                        }
                    }
                }
            }
            3..TOTAL_SECTORS => {
                if self.firmware_size > 0 {
                    let addr = flash::OTA_TEMP_ADDR + 4 + self.write_offset;
                    flash::program_flash(flash_periph, addr, data).ok();
                    self.write_offset += SECTOR_SIZE as u32;

                    if self.write_offset >= self.firmware_size {
                        self.firmware_ready = true;
                    }
                }
            }
            _ => {}
        }
    }

    /// 复位
    pub fn reset(&mut self) {
        self.firmware_size = 0;
        self.firmware_ready = false;
        self.write_offset = 0;
    }
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
            match comm::try_decode_frame(&rx_buf[..*rx_pos]) {
                Ok((frame, consumed)) => {
                    rx_buf.copy_within(consumed..*rx_pos, 0);
                    *rx_pos -= consumed;
                    return Some(frame);
                }
                Err(comm::DecodeError::NoHeader) => {
                    *rx_pos = 0;
                }
                Err(comm::DecodeError::Incomplete) => {}
                Err(comm::DecodeError::CrcMismatch { .. }) => {
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
        let len = comm::dequeue_bytes(&mut buf);
        if len > 0 {
            let _ = self.serial.write(&buf[..len]);
            let _ = self.serial.flush();
        }
    }
}
