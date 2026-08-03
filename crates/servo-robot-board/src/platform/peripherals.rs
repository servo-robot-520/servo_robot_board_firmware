//! Peripheral adapters and USB initialization.
//!
//! Contains `MpuSpiDevice` (SPI device adapter for the MPU6500),
//! `BusyDelay` (busy-wait delay), and USB OTG FS static-cell setup.

use static_cell::StaticCell;

// ---------------------------------------------------------------------------
// MpuSpiDevice: exclusive-bus SPI device adapter
// ---------------------------------------------------------------------------

/// Local `SpiDevice` adapter for the MPU6500's exclusive SPI bus and CS pin.
///
/// Because the MPU6500 is the only device on SPI1, no bus sharing/mutex is
/// needed -- this adapter simply toggles the CS pin around each transaction.
pub struct MpuSpiDevice<SPI, CS> {
    pub bus: SPI,
    pub cs: CS,
}

impl<SPI, CS> embedded_hal::spi::ErrorType for MpuSpiDevice<SPI, CS>
where
    SPI: embedded_hal::spi::ErrorType,
{
    type Error = SPI::Error;
}

impl<SPI, CS> embedded_hal::spi::SpiDevice<u8> for MpuSpiDevice<SPI, CS>
where
    SPI: embedded_hal::spi::SpiBus<u8>,
    CS: embedded_hal::digital::OutputPin<Error = core::convert::Infallible>,
{
    fn transaction(
        &mut self,
        operations: &mut [embedded_hal::spi::Operation<'_, u8>],
    ) -> Result<(), Self::Error> {
        if let Err(never) = self.cs.set_low() {
            match never {}
        }
        let result = (|| {
            for operation in operations {
                match operation {
                    embedded_hal::spi::Operation::Read(words) => self.bus.read(words)?,
                    embedded_hal::spi::Operation::Write(words) => self.bus.write(words)?,
                    embedded_hal::spi::Operation::Transfer(read, write) => {
                        self.bus.transfer(read, write)?
                    }
                    embedded_hal::spi::Operation::TransferInPlace(words) => {
                        self.bus.transfer_in_place(words)?
                    }
                    embedded_hal::spi::Operation::DelayNs(ns) => {
                        // Convert to microseconds to avoid ns*96 overflowing u32 at 96 MHz
                        let ns_val = *ns;
                        let us = ns_val / 1000;
                        let rem = ns_val % 1000;
                        if us > 0 {
                            cortex_m::asm::delay(us.saturating_mul(96));
                        }
                        if rem > 0 {
                            cortex_m::asm::delay(rem.saturating_mul(96) / 1000);
                        }
                    }
                }
            }
            self.bus.flush()
        })();
        if let Err(never) = self.cs.set_high() {
            match never {}
        }
        result
    }
}

// ---------------------------------------------------------------------------
// BusyDelay: busy-wait delay implementation
// ---------------------------------------------------------------------------

/// Busy-wait delay using `cortex_m::asm::delay` at 96 MHz.
pub struct BusyDelay;

impl embedded_hal::delay::DelayNs for BusyDelay {
    fn delay_ns(&mut self, ns: u32) {
        // Convert to microseconds to avoid ns*96 overflowing u32 at 96 MHz
        let us = ns / 1000;
        let rem = ns % 1000;
        if us > 0 {
            cortex_m::asm::delay(us.saturating_mul(96));
        }
        if rem > 0 {
            cortex_m::asm::delay(rem.saturating_mul(96) / 1000);
        }
    }
}

// ---------------------------------------------------------------------------
// ADC GPIO configuration
// ---------------------------------------------------------------------------

/// Configure ADC GPIO pins as analog inputs.
///
/// PA0 (TEMP_CHARGE), PA1 (TEMP_SERVO), PA4 (TEMP_5V), PB0 (BC_IOUT), PB1 (CV_ADC).
pub fn configure_adc_gpio() {
    let gpioa = unsafe { &*stm32f4xx_hal::pac::GPIOA::ptr() };
    gpioa
        .moder()
        .modify(|_, w| w.moder0().analog().moder1().analog().moder4().analog());
    let gpiob = unsafe { &*stm32f4xx_hal::pac::GPIOB::ptr() };
    gpiob
        .moder()
        .modify(|_, w| w.moder0().analog().moder1().analog());
}

// ---------------------------------------------------------------------------
// USB OTG FS static-cell initialization
// ---------------------------------------------------------------------------

/// USB endpoint memory buffer (must be in static scope for USB peripheral DMA access).
pub static EP_MEMORY: StaticCell<[u32; 128]> = StaticCell::new();

/// USB bus allocator storage (must be in static scope to produce `'static` references).
pub static USB_BUS_STORE: StaticCell<
    Option<usb_device::bus::UsbBusAllocator<stm32f4xx_hal::otg_fs::UsbBusType>>,
> = StaticCell::new();

/// Initialize USB endpoint memory and return the shared USB bus allocator.
///
/// The allocator is platform-owned because its static storage is required by
/// the OTG peripheral. Feature-specific USB classes are intentionally built
/// by `features::communication`, keeping this module free of protocol policy.
pub fn init_usb_bus(
    usb_periph: stm32f4xx_hal::otg_fs::USB,
) -> &'static usb_device::bus::UsbBusAllocator<stm32f4xx_hal::otg_fs::UsbBusType> {
    let ep_mem: &'static mut [u32; 128] = EP_MEMORY.init([0; 128]);
    let usb_bus = stm32f4xx_hal::otg_fs::UsbBus::new(usb_periph, ep_mem);
    let usb_bus_store: &'static mut Option<_> = USB_BUS_STORE.init(None);
    *usb_bus_store = Some(usb_bus);
    usb_bus_store.as_ref().expect("USB bus storage initialized")
}
