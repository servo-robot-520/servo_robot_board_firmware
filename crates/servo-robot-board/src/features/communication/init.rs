//! Communication feature initialization.
//!
//! The platform owns USB peripheral memory and the bus allocator. This module
//! creates the protocol-aware CDC/MSC transport on that platform resource.

/// Build the USB composite transport on a platform-provided USB bus.
pub fn init_usb(
    bus: &'static usb_device::bus::UsbBusAllocator<stm32f4xx_hal::otg_fs::UsbBusType>,
) -> super::transport::UsbCompositeDevice<'static, stm32f4xx_hal::otg_fs::UsbBusType> {
    super::transport::UsbCompositeDevice::new(bus)
}
