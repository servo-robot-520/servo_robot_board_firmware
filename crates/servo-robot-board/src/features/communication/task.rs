//! Communication task operations.
//!
//! RTIC wrappers retain priority, resource locking, and spawning. This module
//! owns protocol decoding plus USB/UART transport work, and returns explicit
//! actions for hardware effects that remain at the runtime boundary.

use super::ota::{OtaWriteResult, OtaWriter};
use super::transport::UsbCompositeDevice;
use servo_robot_protocol::command::CommandType;
use servo_robot_protocol::config::{Config, ConfigType};
use servo_robot_protocol::frame::{RawFrame, TypedFrame};
use servo_robot_protocol::servo::ServoCmdWrapper;

/// A decoded host request that the RTIC wrapper must apply with its resources.
pub enum ReceiveAction {
    Reset,
    Shutdown,
    StartOta,
    ConfigWrite(Config),
    ConfigQuery(ConfigType),
    ConfigQueryAll,
    ServoForward(ServoCmdWrapper),
    FirmwareUpdate(ServoCmdWrapper),
}

/// Decode a raw transport frame into a supported host action.
///
/// Unsupported or malformed frames are logged and ignored, matching the
/// previous task behavior without exposing protocol parsing to `main.rs`.
pub fn decode_inbound(frame: &RawFrame) -> Option<ReceiveAction> {
    let typed = match TypedFrame::from_raw(frame) {
        Ok(frame) => frame,
        Err(_) => {
            defmt::warn!("Frame parse error");
            return None;
        }
    };

    match typed {
        TypedFrame::Command(command) => match command.cmd {
            CommandType::Reset => Some(ReceiveAction::Reset),
            CommandType::Shutdown => Some(ReceiveAction::Shutdown),
            CommandType::Ota => Some(ReceiveAction::StartOta),
        },
        TypedFrame::ConfigWrite(config) => Some(ReceiveAction::ConfigWrite(config)),
        TypedFrame::ConfigQuery(config_type) => Some(ReceiveAction::ConfigQuery(config_type)),
        TypedFrame::ConfigQueryAll => Some(ReceiveAction::ConfigQueryAll),
        TypedFrame::ServoForward(command) => Some(ReceiveAction::ServoForward(command)),
        TypedFrame::FirmwareUpdate(block) => Some(ReceiveAction::FirmwareUpdate(block)),
        _ => {
            defmt::warn!("Unknown frame type");
            None
        }
    }
}

/// Write one OTA firmware block and create the protocol acknowledgement.
pub fn write_firmware_update(
    writer: &mut OtaWriter,
    flash: &stm32f4xx_hal::pac::FLASH,
    data: &[u8],
) -> TypedFrame {
    let result = writer.write_block(flash, data);
    let (success, offset) = match result {
        OtaWriteResult::Success { new_offset } => (true, new_offset),
        _ => (false, writer.offset()),
    };
    TypedFrame::AckFirmwareUpdate { success, offset }
}

/// Poll USB, deliver every complete inbound frame, then drain queued output.
pub fn poll_usb<B: usb_device::bus::UsbBus>(
    device: &mut UsbCompositeDevice<'_, B>,
    rx_buf: &mut [u8; 512],
    rx_pos: &mut usize,
    mut receive: impl FnMut(RawFrame),
) {
    device.poll();
    while let Some(frame) = device.try_receive_frame(rx_buf, rx_pos) {
        receive(frame);
    }
    device.flush_tx_queue();
}

/// Flush pending USB output.
pub fn flush_usb<B: usb_device::bus::UsbBus>(device: &mut UsbCompositeDevice<'_, B>) {
    device.flush_tx_queue();
}

/// Service the USART2 IRQ and report whether new bytes arrived.
pub fn handle_uart2_irq(usart2: &mut stm32f4xx_hal::pac::USART2) -> bool {
    let had_rx = usart2.sr().read().rxne().bit_is_set();
    crate::platform::uart2::handle_usart2_irq(usart2);
    had_rx
}

/// Decode every complete UART2 frame currently buffered.
pub fn drain_uart2_frames(mut receive: impl FnMut(RawFrame)) {
    while let Some(frame) = crate::platform::uart2::uart2_try_decode_frame() {
        receive(frame);
    }
}

/// Enable USART2 transmission when the byte queue is non-empty.
pub fn flush_uart2(usart2: &mut stm32f4xx_hal::pac::USART2) {
    crate::platform::uart2::trigger_tx(usart2);
}
