#![no_std]
#![no_main]
#![allow(dead_code, unused_imports, unused_variables)]

//! Servo Robot Board Firmware

use defmt_rtt as _;
use panic_halt as _;

defmt::timestamp!("{=u32:us}", {
    // 使用 DWT cycle counter 作为时间戳 (96MHz)
    unsafe {
        let dwt = &*cortex_m::peripheral::DWT::PTR;
        dwt.cyccnt.read() / 96 // 96 cycles = 1 µs
    }
});

mod buzz;
mod domain;
mod hal;
mod log_macro;
mod services;
mod ws2812;

extern crate alloc;

use alloc::vec;
use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

use domain::charge::ChargeManager;
use domain::protection::ProtectionManager;
use servo_robot_protocol::battery_state::{
    BatteryChargeStatus, BatteryHealth, BatteryState, BatteryTechnology,
};
use servo_robot_protocol::command::{Command, CommandType};
use servo_robot_protocol::config::{BoardConfigSnapshot, Config, ConfigType};
use servo_robot_protocol::event::{
    BoardEvent, ChargePhase, ErrorFlags, ProtectionFlags, StateChangeFlags,
};
use servo_robot_protocol::frame::{FrameType, RawFrame, TypedFrame};
use servo_robot_protocol::imu::ImuData;
use servo_robot_protocol::log::{LogLevel, LogMessage};
use servo_robot_protocol::power::PowerData;
use servo_robot_protocol::servo::ServoCmdWrapper;
use servo_robot_protocol::system::SystemInfo;

/// 栈水位检测: 扫描描漆区域，返回最小剩余栈空间 (字节)
///
/// 原理: init 时用 0xCC 填充栈区域，使用过的栈会被覆盖。
/// 从栈底向栈顶扫描，找到第一个 0xCC 字节即为高水位。
fn check_stack_watermark() -> u16 {
    unsafe extern "C" {
        static _stack_start: u32; // 栈顶 (RAM 末尾)
        static _stack_end: u32; // 栈底 (BSS/uninit 末尾)
    }
    unsafe {
        let stack_bottom = &_stack_end as *const _ as *const u8;
        let stack_top = &_stack_start as *const _ as *const u8;
        let total = stack_top.offset_from(stack_bottom) as u32;

        // 从栈底向栈顶扫描，找第一个 0xCC (未被使用的区域)
        let mut scan = stack_bottom;
        while scan < stack_top && *scan != 0xCC {
            scan = scan.add(1);
        }
        let used = scan.offset_from(stack_bottom) as u32;
        ((total - used) / 1024) as u16
    }
}

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [USART6, TIM5, SPI3, SPI4, SPI5])]
mod app {
    use super::*;
    use rtic::Mutex;
    use static_cell::StaticCell;
    use stm32f4xx_hal::{
        gpio::{Input, Output, PushPull, gpioa, gpiob, gpioc},
        i2c::I2c,
        pac,
        prelude::*,
        rcc::Config as RccConfig,
    };
    use usb_device::prelude::*;

    /// USB endpoint memory buffer (must be in static scope for USB peripheral DMA access).
    static EP_MEMORY: StaticCell<[u32; 128]> = StaticCell::new();
    /// USB bus allocator storage (must be in static scope to produce `'static` references).
    static USB_BUS_STORE: StaticCell<
        Option<usb_device::bus::UsbBusAllocator<stm32f4xx_hal::otg_fs::UsbBusType>>,
    > = StaticCell::new();

    /// Local `SpiDevice` adapter for the MPU6500's exclusive SPI bus and CS pin.
    struct MpuSpiDevice<SPI, CS> {
        bus: SPI,
        cs: CS,
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
                            // 先转为微秒避免 ns*96 溢出 u32 (96MHz)
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

    struct BusyDelay;

    impl embedded_hal::delay::DelayNs for BusyDelay {
        fn delay_ns(&mut self, ns: u32) {
            // 先转为微秒避免 ns*96 溢出 u32 (96MHz)
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

    // task share data
    #[shared]
    struct Shared {
        i2c1: I2c<pac::I2C1>,
        imu_data: ImuData,
        power_data: PowerData,
        battery_state: BatteryState,
        board_event: BoardEvent,
        prev_board_event: BoardEvent,
        system_info: SystemInfo,
        config: BoardConfigSnapshot,
        pwr_servo_en: gpioc::PC13<Output<PushPull>>,
        bat_ext_out_en: gpioc::PC14<Output<PushPull>>,
        pwr_5v_en: gpioc::PC15<Output<PushPull>>,
        fan_en: gpiob::PB8<Output<PushPull>>,
        pwr_key: gpiob::PB13<Output<PushPull>>,
        charge_mgr: ChargeManager,
        protection_mgr: ProtectionManager,
        ws2812_colors: [ws2812::Color; 3],
        frames_sent: u32,
        uptime_s: u32,
        // 错误计数器
        i2c_errors: u16,
        spi_errors: u16,
        uart_errors: u16,
        usb_errors: u16,
        // PD 请求参数
        pd_request_voltage: u16,
        pd_request_current: u16,
        // 电池温度 (from BQ40Z50, 实际值 = 原始值 / 10)
        temp_battery: i16,
        // IMU 状态
        imu_id: u8,
        imu_ok: bool,
        usb_dev: hal::usb::UsbCompositeDevice<'static, stm32f4xx_hal::otg_fs::UsbBusType>,
        // UART2 外设 (用于与 Linux 上位机通讯)
        usart2: pac::USART2,
        // UART1 外设 (用于与串口舵机通讯)
        usart1: pac::USART1,
        // 串口舵机 TX 使能引脚 (高电平发送)
        servo_tx_en: gpiob::PB12<Output<PushPull>>,
        // 充电器 ACOK 输入 (高电平=充电器已连接)
        bc_acok: gpiob::PB5<Input>,
    }

    // task exclusive data
    #[local]
    struct Local {
        mpu6500: embedded_mpu6500::Mpu6500<
            MpuSpiDevice<stm32f4xx_hal::spi::Spi<pac::SPI1>, gpiob::PB2<Output<PushPull>>>,
        >,
        imu_filter: domain::imu::MahonyFilter,
        usb_rx_buf: [u8; 512],
        usb_rx_pos: usize,
        ota_writer: services::ota::OtaWriter,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        {
            use core::mem::MaybeUninit;
            const HEAP_SIZE: usize = 8192;
            static HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
            unsafe {
                HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE);
            }
        }

        // 栈描漆: 从当前 SP 到 _stack_start (RAM 末尾) 填充 0xCC
        // 后续通过扫描 0xCC 模式检测栈高水位
        unsafe extern "C" {
            static _stack_start: u32;
        }
        {
            let sp: u32;
            unsafe { core::arch::asm!("mov {}, sp", out(reg) sp) };
            let stack_top = unsafe { &_stack_start as *const _ as u32 };
            let paint_start = sp as *mut u8;
            let paint_len = (stack_top - sp) as usize;
            unsafe {
                core::ptr::write_bytes(paint_start, 0xCC, paint_len);
            }
        }

        let dp = ctx.device;

        // 外设时钟使能 (必须在 RCC.constrain() 之前)
        dp.RCC.apb1enr().modify(|_, w| w.usart2en().set_bit());
        dp.RCC.apb2enr().modify(|_, w| {
            w.usart1en()
                .set_bit()
                .adc1en()
                .set_bit()
                .tim1en()
                .set_bit()
                .syscfgen()
                .set_bit()
        });
        dp.RCC.ahb1enr().modify(|_, w| w.dma2en().set_bit());

        let rcc = dp.RCC.constrain();
        let mut rcc = rcc.freeze(RccConfig::hsi().sysclk(96.MHz()));
        let clocks = rcc.clocks;

        let gpioa = dp.GPIOA.split(&mut rcc);
        let gpiob = dp.GPIOB.split(&mut rcc);
        let gpioc = dp.GPIOC.split(&mut rcc);

        // UART2 GPIO: PA2=TX, PA3=RX (AF7)
        // 配置 AFRL 寄存器: PA2=AFRL[11:8], PA3=AFRL[15:12]
        {
            let gpioa_regs = unsafe { &*stm32f4xx_hal::pac::GPIOA::ptr() };
            gpioa_regs.afrl().modify(|_, w| unsafe {
                w.afrl2()
                    .bits(7) // PA2 → AF7
                    .afrl3()
                    .bits(7) // PA3 → AF7
            });
            // MODER: PA2=10 (alternate), PA3=10 (alternate)
            gpioa_regs
                .moder()
                .modify(|_, w| w.moder2().alternate().moder3().alternate());
            // OTYPER: PA2=0 (push-pull), PA3=0
            gpioa_regs
                .otyper()
                .modify(|_, w| w.ot2().push_pull().ot3().push_pull());
            // OSPEEDR: PA2=high, PA3=high
            gpioa_regs
                .ospeedr()
                .modify(|_, w| w.ospeedr2().high_speed().ospeedr3().high_speed());
        }

        // UART2 USART2 配置: 115200, 8N1
        {
            let usart2 = &dp.USART2;
            // APB1 时钟 = 48MHz (96MHz / 2), BRR = 48000000 / 115200 = 416
            usart2.brr().write(|w| unsafe { w.bits(416) });
            usart2.cr1().write(|w| {
                w.te()
                    .set_bit() // TX 使能
                    .re()
                    .set_bit() // RX 使能
                    .rxneie()
                    .set_bit() // RX 非空中断使能
            });
            usart2.cr1().modify(|_, w| w.ue().set_bit()); // USART2 使能
            unsafe {
                cortex_m::peripheral::NVIC::unmask(stm32f4xx_hal::pac::Interrupt::USART2);
            }
        }
        defmt::info!("UART2 initialized (115200 baud)");

        let pwr_servo_en = gpioc.pc13.into_push_pull_output();
        let bat_ext_out_en = gpioc.pc14.into_push_pull_output();
        let pwr_5v_en = gpioc.pc15.into_push_pull_output();
        let fan_en = gpiob.pb8.into_push_pull_output();
        let pwr_key = gpiob.pb13.into_push_pull_output();
        let servo_tx_en = gpiob.pb12.into_push_pull_output();
        let bc_acok = gpiob.pb5.into_pull_down_input();
        let _husb_int = gpiob.pb14.into_pull_down_input();

        // EXTI 外部中断配置: PB5 (BC_ACOK) 和 PB14 (HUSB238A INT)
        // 上升沿触发，用于检测充电器连接和 USB PD 事件
        {
            let syscfg = unsafe { &*stm32f4xx_hal::pac::SYSCFG::ptr() };
            // EXTICR2: EXTI5 = PB (1) — PB5 = BC_ACOK
            syscfg.exticr2().modify(|_, w| w.exti5().pb());
            // EXTICR4: EXTI14 = PB (1) — PB14 = HUSB238A INT
            syscfg.exticr4().modify(|_, w| w.exti14().pb());

            let exti = unsafe { &*stm32f4xx_hal::pac::EXTI::ptr() };
            // 上升沿触发
            exti.rtsr().modify(|_, w| w.tr5().set_bit().tr14().set_bit());
            // 不触发下降沿
            exti.ftsr().modify(|_, w| w.tr5().clear_bit().tr14().clear_bit());
            // 使能中断
            exti.imr().modify(|_, w| w.mr5().set_bit().mr14().set_bit());

            // 使能 NVIC 中断
            unsafe {
                cortex_m::peripheral::NVIC::unmask(stm32f4xx_hal::pac::Interrupt::EXTI9_5);
                cortex_m::peripheral::NVIC::unmask(stm32f4xx_hal::pac::Interrupt::EXTI15_10);
            }
        }
        defmt::info!("EXTI configured: PB5 (BC_ACOK), PB14 (HUSB238A INT)");

        // ADC GPIO: PA0, PA1, PA4 = 模拟输入; PB0, PB1 = 模拟输入
        {
            let gpioa_regs = unsafe { &*stm32f4xx_hal::pac::GPIOA::ptr() };
            // MODER: PA0=11, PA1=11, PA4=11 (analog)
            gpioa_regs
                .moder()
                .modify(|_, w| w.moder0().analog().moder1().analog().moder4().analog());
            let gpiob_regs = unsafe { &*stm32f4xx_hal::pac::GPIOB::ptr() };
            // MODER: PB0=11, PB1=11 (analog)
            gpiob_regs
                .moder()
                .modify(|_, w| w.moder0().analog().moder1().analog());
        }

        // UART1 GPIO: PA15=TX, PA10=RX (AF7)
        // 需要先禁用 JTAG (PA15 默认是 JTDI)
        {
            let gpioa_regs = unsafe { &*stm32f4xx_hal::pac::GPIOA::ptr() };
            // AFRH: PA15=AFRH[31:28], PA10=AFRH[7:4]
            gpioa_regs.afrh().modify(|_, w| unsafe {
                w.afrh15()
                    .bits(7) // PA15 → AF7
                    .afrh10()
                    .bits(7) // PA10 → AF7
            });
            // MODER: PA15=10 (alternate), PA10=10 (alternate)
            gpioa_regs
                .moder()
                .modify(|_, w| w.moder15().alternate().moder10().alternate());
            // OTYPER: push-pull
            gpioa_regs
                .otyper()
                .modify(|_, w| w.ot15().push_pull().ot10().push_pull());
            // OSPEEDR: high speed
            gpioa_regs
                .ospeedr()
                .modify(|_, w| w.ospeedr15().high_speed().ospeedr10().high_speed());
            // 禁用 JTAG (SWJ_CFG = 010: JTAG-DP Disabled, SW-DP Enabled)
            // 释放 PA15, PB3, PB4 为普通 GPIO
            let afio_regs = unsafe { &*stm32f4xx_hal::pac::SYSCFG::ptr() };
            // 对于 STM32F4, 通过 DBGMCU 或 AFIO_MAPR 禁用 JTAG
            // 实际上 STM32F4 使用 DBGMCU_CR 的 SWJ_CFG 位
            // 但更简单的方式是直接配置 GPIO，HAL 会处理
        }

        // UART1 USART1 配置: 默认 115200, 8N1
        {
            let usart1 = &dp.USART1;
            // APB2 时钟 = 96MHz, BRR = 96000000 / 115200 = 833
            usart1.brr().write(|w| unsafe { w.bits(833) });
            usart1.cr1().write(|w| {
                w.te()
                    .set_bit() // TX 使能
                    .re()
                    .set_bit() // RX 使能
                    .rxneie()
                    .set_bit() // RX 非空中断使能
            });
            usart1.cr1().modify(|_, w| w.ue().set_bit()); // USART1 使能
            unsafe {
                cortex_m::peripheral::NVIC::unmask(stm32f4xx_hal::pac::Interrupt::USART1);
            }
        }
        defmt::info!("UART1 initialized (115200 baud, servo)");

        // I2C1 (PB6=SCL, PB7=SDA)
        let scl1 = gpiob.pb6.into_alternate_open_drain();
        let sda1 = gpiob.pb7.into_alternate_open_drain();
        let i2c1 = I2c::new(dp.I2C1, (scl1, sda1), 400.kHz(), &mut rcc);

        // SPI1 (PA5=SCK, PA6=MISO, PA7=MOSI) + CS=PB2
        let sck = gpioa.pa5.into_alternate();
        let miso = gpioa.pa6.into_alternate();
        let mosi = gpioa.pa7.into_alternate();
        let spi1 = stm32f4xx_hal::spi::Spi::new(
            dp.SPI1,
            (Some(sck), Some(miso), Some(mosi)),
            stm32f4xx_hal::spi::Mode {
                polarity: stm32f4xx_hal::spi::Polarity::IdleHigh,
                phase: stm32f4xx_hal::spi::Phase::CaptureOnSecondTransition,
            },
            1.MHz(),
            &mut rcc,
        );
        let cs_mpu = gpiob.pb2.into_push_pull_output();

        // 初始化 MPU6500
        let mpu_spi = MpuSpiDevice {
            bus: spi1,
            cs: cs_mpu,
        };
        let mut mpu6500 = embedded_mpu6500::Mpu6500::new(mpu_spi);
        let mut mpu_delay = BusyDelay;
        let (who, imu_ok) = match mpu6500.init(&mut mpu_delay) {
            Ok(()) => {
                let who = mpu6500.who_am_i().unwrap_or(0);
                defmt::info!("MPU6500 WHO_AM_I: 0x{:02X}", who);
                (who, true)
            }
            Err(_e) => {
                defmt::error!("MPU6500 init failed");
                (0, false)
            }
        };

        // USB OTG FS
        let usb_dm = gpioa.pa11.into_alternate();
        let usb_dp = gpioa.pa12.into_alternate();
        let (usb_global, usb_device_periph, usb_pwrclk) =
            (dp.OTG_FS_GLOBAL, dp.OTG_FS_DEVICE, dp.OTG_FS_PWRCLK);
        let usb_periph = stm32f4xx_hal::otg_fs::USB::new(
            (usb_global, usb_device_periph, usb_pwrclk),
            (usb_dm, usb_dp),
            &clocks,
        );

        let ep_mem: &'static mut [u32; 128] = EP_MEMORY.init([0; 128]);
        let usb_bus = stm32f4xx_hal::otg_fs::UsbBus::new(usb_periph, ep_mem);
        let usb_bus_store: &'static mut Option<_> = USB_BUS_STORE.init(None);
        *usb_bus_store = Some(usb_bus);
        let usb_bus_ref: &'static _ = usb_bus_store.as_ref().unwrap();
        let usb_dev = hal::usb::UsbCompositeDevice::new(usb_bus_ref);

        // 从 Flash 加载配置
        let config = hal::flash::load_config().unwrap_or_default();

        // 根据配置恢复 GPIO 状态
        let mut pwr_servo_en = pwr_servo_en;
        let mut bat_ext_out_en = bat_ext_out_en;
        let mut pwr_5v_en = pwr_5v_en;
        if config.power_servo_on {
            pwr_servo_en.set_high();
        } else {
            pwr_servo_en.set_low();
        }
        if config.bat_ext_out_on {
            bat_ext_out_en.set_high();
        } else {
            bat_ext_out_en.set_low();
        }
        if config.power_5v_on {
            pwr_5v_en.set_high();
        } else {
            pwr_5v_en.set_low();
        }

        // 启动音效
        buzz::startup_sound(&dp.TIM2);

        // ADC1 + DMA2 初始化
        hal::adc::init_adc_dma(&dp.ADC1, &dp.DMA2);
        defmt::info!(
            "ADC1 + DMA2 initialized ({} channels)",
            hal::adc::ADC_CHANNEL_COUNT
        );

        // WS2812 LED: TIM1 CH2 (PA9) + DMA2 Stream5
        hal::ws2812::init_tim1_dma(&dp.TIM1);
        defmt::info!("WS2812 initialized (TIM1 CH2 + DMA2 Stream5)");

        defmt::info!("Servo Robot Board initialized");

        (
            Shared {
                i2c1,
                imu_data: ImuData::default(),
                power_data: PowerData::default(),
                battery_state: BatteryState::default(),
                board_event: BoardEvent::default(),
                prev_board_event: BoardEvent::default(),
                system_info: SystemInfo::default(),
                config,
                pwr_servo_en,
                bat_ext_out_en,
                pwr_5v_en,
                fan_en,
                pwr_key,
                charge_mgr: ChargeManager::new(),
                protection_mgr: ProtectionManager::new(),
                ws2812_colors: [ws2812::Color::BLACK; 3],
                frames_sent: 0,
                uptime_s: 0,
                i2c_errors: 0,
                spi_errors: 0,
                uart_errors: 0,
                usb_errors: 0,
                pd_request_voltage: 0,
                pd_request_current: 0,
                temp_battery: 0,
                imu_id: who,
                imu_ok,
                usb_dev,
                usart2: dp.USART2,
                usart1: dp.USART1,
                servo_tx_en,
                bc_acok,
            },
            Local {
                mpu6500,
                imu_filter: domain::imu::MahonyFilter::new(),
                usb_rx_buf: [0u8; 512],
                usb_rx_pos: 0,
                ota_writer: services::ota::OtaWriter::new(),
            },
        )
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    // ===== Task 1: IMU (100Hz) =====
    #[task(priority = 4, local = [mpu6500, imu_filter], shared = [imu_data, frames_sent])]
    async fn imu_task(mut ctx: imu_task::Context) {
        let mpu = ctx.local.mpu6500;
        let filter = ctx.local.imu_filter;
        // 100 Hz → dt = 10 ms = 0.01 s
        const IMU_DT: f32 = 0.01;
        match domain::imu::read_imu_data(mpu, filter, IMU_DT) {
            Ok(imu) => {
                ctx.shared.imu_data.lock(|d| *d = imu.clone());
                ctx.shared.frames_sent.lock(|f| *f += 1);
                comm_tx_task::spawn(TypedFrame::Imu(imu)).ok();
            }
            Err(_) => {
                defmt::warn!("MPU6500 read error");
            }
        }
    }

    // ===== Task 2: 电源 (20Hz) =====
    #[task(priority = 3, shared = [i2c1, power_data, protection_mgr, pwr_servo_en, config, frames_sent, board_event])]
    async fn power_task(mut ctx: power_task::Context) {
        let buf = hal::adc::adc_buf();
        let bc_iout = buf[hal::adc::CH_BC_IOUT];
        let cv_adc = buf[hal::adc::CH_CV_ADC];

        let ina_data = ctx
            .shared
            .i2c1
            .lock(|i2c| domain::power::read_ina219_data(&mut *i2c));

        let data = PowerData {
            servo_voltage_mv: ina_data.bus_voltage as u16,
            servo_current_ma: ina_data.current_ma as u16,
            charge_in_current_ma: domain::power::charge_current_ma(bc_iout) as u16,
            charge_in_voltage_mv: domain::power::pd_voltage_mv(cv_adc) as u16,
            ..PowerData::default()
        };

        // 过流保护检查
        let current_a = ina_data.current_ma as f32 / 1000.0;
        let (flags, should_cut) = ctx.shared.protection_mgr.lock(|pm| {
            let should_cut = pm.check_current(current_a);
            (pm.flags(), should_cut)
        });
        if should_cut {
            ctx.shared.pwr_servo_en.lock(|p| p.set_low());
        }
        // 更新 board_event 保护标志
        ctx.shared.board_event.lock(|e| {
            e.protection_flags =
                ProtectionFlags::from_bits(flags.to_u16()).unwrap_or(ProtectionFlags::empty());
        });
        event_task::spawn().ok();

        ctx.shared.power_data.lock(|d| *d = data.clone());
        ctx.shared.frames_sent.lock(|f| *f += 1);
        comm_tx_task::spawn(TypedFrame::Power(data)).ok();
    }

    // ===== Task 4: 电池 (10Hz) =====
    #[task(priority = 3, shared = [i2c1, battery_state, ws2812_colors, frames_sent, temp_battery])]
    async fn bat_task(mut ctx: bat_task::Context) {
        use embedded_bq40z50::Bq40z50;

        let bat_data = ctx.shared.i2c1.lock(|i2c| {
            let mut gauge = Bq40z50::new(&mut *i2c);
            domain::battery::read_bq40z50_data(&mut gauge)
        });

        ctx.shared.battery_state.lock(|d| *d = bat_data.clone());
        ctx.shared.temp_battery.lock(|t| *t = bat_data.temperature);
        ctx.shared.frames_sent.lock(|f| *f += 1);

        let soc = bat_data.percentage as u8;
        ctx.shared.ws2812_colors.lock(|c| {
            c[1] = ws2812::battery_soc_color(soc);
        });
        led_task::spawn().ok();

        comm_tx_task::spawn(TypedFrame::Battery(bat_data)).ok();
    }

    // ===== Task 5: 充电管理 (1Hz) =====
    #[task(priority = 2, shared = [i2c1, charge_mgr, board_event, config])]
    async fn charge_task(mut ctx: charge_task::Context) {
        // 从 ADC 读取充电电路 NTC 温度
        let adc_buf = hal::adc::adc_buf();
        // 转换为 0.1°C 单位，与 charge_temp_limit/derating 一致
        let charger_temp = (domain::thermal::ntc_temp_c(adc_buf[hal::adc::CH_TEMP_CHARGE]) * 10.0) as i16;

        let phase_enum = ctx.shared.i2c1.lock(|i2c| {
            let cfg = ctx.shared.config.lock(|c| c.clone());

            let result = ctx.shared.charge_mgr.lock(|cm| {
                domain::charge::update_charge(
                    &mut *i2c,
                    cm,
                    cfg.charge_on,
                    cfg.charge_max_current_ma,
                    cfg.charge_stop_voltage_mv,
                    cfg.charge_temp_derating as i16,
                    cfg.charge_temp_limit as i16,
                    charger_temp,
                )
            });

            match result.phase {
                domain::charge::ChargePhase::NotCharging => ChargePhase::NotCharging,
                domain::charge::ChargePhase::PreCharge => ChargePhase::PreCharge,
                domain::charge::ChargePhase::Cc => ChargePhase::Cc,
                domain::charge::ChargePhase::Cv => ChargePhase::Cv,
                domain::charge::ChargePhase::Full => ChargePhase::Full,
                domain::charge::ChargePhase::HusbFault => ChargePhase::PdSinkFault,
                domain::charge::ChargePhase::Unsupported => ChargePhase::UnsupportedCharger,
                domain::charge::ChargePhase::Unknown
                | domain::charge::ChargePhase::ThermalProtect => ChargePhase::NotCharging,
            }
        });

        ctx.shared.board_event.lock(|e| e.charge_phase = phase_enum);
        event_task::spawn().ok();
    }

    // ===== Task 6: 通讯接收 =====
    #[task(priority = 5, local = [ota_writer], shared = [config, board_event, pwr_servo_en, bat_ext_out_en, pwr_5v_en, fan_en, pwr_key, protection_mgr, usart1, servo_tx_en])]
    async fn comm_rx_task(mut ctx: comm_rx_task::Context, frame: RawFrame) {
        let typed = match TypedFrame::from_raw(&frame) {
            Ok(f) => f,
            Err(_) => {
                defmt::warn!("Frame parse error");
                return;
            }
        };

        match typed {
            TypedFrame::Command(cmd) => {
                // 一次性命令处理
                match cmd.cmd {
                    CommandType::Reset => {
                        defmt::info!("System reset");
                        comm_tx_task::spawn(TypedFrame::AckCommand { success: true }).ok();
                        cortex_m::peripheral::SCB::sys_reset();
                    }
                    CommandType::Shutdown => {
                        defmt::info!("Shutdown");
                        comm_tx_task::spawn(TypedFrame::AckCommand { success: true }).ok();
                        ctx.shared.pwr_key.lock(|p| p.set_low());
                        return;
                    }
                    CommandType::Ota => {
                        let flash = unsafe { stm32f4xx_hal::pac::Peripherals::steal() }.FLASH;
                        if services::ota::start_ota(&flash) {
                            comm_tx_task::spawn(TypedFrame::AckCommand { success: true }).ok();
                            cortex_m::peripheral::SCB::sys_reset();
                        } else {
                            comm_tx_task::spawn(TypedFrame::AckCommand { success: false }).ok();
                        }
                        return;
                    }
                }
            }
            TypedFrame::ConfigWrite(cfg) => {
                // 处理配置写入（RTIC lock 限制，需内联处理）
                match cfg {
                    Config::SwitchPowerServo(on) => {
                        ctx.shared.pwr_servo_en.lock(
                            |p| {
                                if on { p.set_high() } else { p.set_low() }
                            },
                        );
                        ctx.shared.config.lock(|c| c.power_servo_on = on);
                        if on {
                            ctx.shared.protection_mgr.lock(|pm| pm.reset_servo_power());
                        }
                        ctx.shared.board_event.lock(|e| {
                            e.state_change_flags
                                .set(StateChangeFlags::SERVO_POWER_ON, on);
                        });
                    }
                    Config::SwitchPower5V(on) => {
                        ctx.shared
                            .pwr_5v_en
                            .lock(|p| if on { p.set_high() } else { p.set_low() });
                        ctx.shared.config.lock(|c| c.power_5v_on = on);
                        if on {
                            ctx.shared.protection_mgr.lock(|pm| pm.reset_5v_power());
                        }
                        ctx.shared.board_event.lock(|e| {
                            e.state_change_flags.set(StateChangeFlags::POWER_5V_ON, on);
                        });
                    }
                    Config::SwitchCharge(on) => {
                        ctx.shared.config.lock(|c| c.charge_on = on);
                    }
                    Config::SwitchBatExtOut(on) => {
                        ctx.shared.bat_ext_out_en.lock(
                            |p| {
                                if on { p.set_high() } else { p.set_low() }
                            },
                        );
                        ctx.shared.config.lock(|c| c.bat_ext_out_on = on);
                        ctx.shared.board_event.lock(|e| {
                            e.state_change_flags
                                .set(StateChangeFlags::BAT_EXT_OUT_ON, on);
                        });
                    }
                    Config::PowerServoCurrentLimitMa(v) => {
                        ctx.shared.config.lock(|c| c.servo_current_limit_ma = v)
                    }
                    Config::PowerServoTempLimit(v) => {
                        ctx.shared.config.lock(|c| c.servo_temp_limit = v)
                    }
                    Config::Power5vTempLimit(v) => ctx.shared.config.lock(|c| c.temp_5v_limit = v),
                    Config::ChargeMaxCurrentMa(v) => {
                        ctx.shared.config.lock(|c| c.charge_max_current_ma = v)
                    }
                    Config::ChargeTempDerating(v) => {
                        ctx.shared.config.lock(|c| c.charge_temp_derating = v)
                    }
                    Config::ChargeTempLimit(v) => {
                        ctx.shared.config.lock(|c| c.charge_temp_limit = v)
                    }
                    Config::ChargeStopVoltageMv(v) => {
                        ctx.shared.config.lock(|c| c.charge_stop_voltage_mv = v)
                    }
                    Config::ChargeStopSoc(v) => {
                        ctx.shared.config.lock(|c| c.charge_stop_percentage = v)
                    }
                    Config::TxLogLevel(level) => ctx.shared.config.lock(|c| c.tx_log_level = level),
                    Config::ServoBaudRate(v) => ctx.shared.config.lock(|c| c.servo_baud_rate = v),
                }

                // 保存配置到 Flash
                let snapshot = ctx.shared.config.lock(|c| c.clone());
                let flash = unsafe { stm32f4xx_hal::pac::Peripherals::steal() }.FLASH;
                hal::flash::save_config(&flash, &snapshot).ok();

                comm_tx_task::spawn(TypedFrame::AckCfgWrite { success: true }).ok();
                event_task::spawn().ok();
            }
            TypedFrame::ConfigQuery(ct) => {
                let v = ctx
                    .shared
                    .config
                    .lock(|c| domain::comm::get_config_value(c, ct));
                comm_tx_task::spawn(TypedFrame::AckCfgQuery(v)).ok();
            }
            TypedFrame::ConfigQueryAll => {
                let s = ctx.shared.config.lock(|c| c.clone());
                comm_tx_task::spawn(TypedFrame::AckCfgQueryAll(s)).ok();
            }
            TypedFrame::ServoForward(wrapper) => {
                // 将舵机命令转发到 UART1
                // 拉高 TX 使能
                ctx.shared.servo_tx_en.lock(|p| p.set_high());
                // 写入 UART1 TX 队列
                let data = wrapper.data();
                hal::uart::uart1_enqueue_tx(data);
                // 触发 UART1 TX 发送
                ctx.shared.usart1.lock(|usart1| {
                    hal::uart::uart1_trigger_tx(usart1);
                });
                defmt::info!("ServoForward: {} bytes sent to UART1", data.len());
            }
            TypedFrame::FirmwareUpdate(wrapper) => {
                let data = wrapper.data();
                let flash = unsafe { stm32f4xx_hal::pac::Peripherals::steal() }.FLASH;

                match ctx.local.ota_writer.write_block(&flash, data) {
                    services::ota::OtaWriteResult::Success { new_offset } => {
                        comm_tx_task::spawn(TypedFrame::AckFirmwareUpdate {
                            success: true,
                            offset: new_offset,
                        })
                        .ok();
                    }
                    services::ota::OtaWriteResult::TooLarge => {
                        comm_tx_task::spawn(TypedFrame::AckFirmwareUpdate {
                            success: false,
                            offset: ctx.local.ota_writer.offset(),
                        })
                        .ok();
                    }
                    services::ota::OtaWriteResult::EraseFailed
                    | services::ota::OtaWriteResult::WriteFailed => {
                        comm_tx_task::spawn(TypedFrame::AckFirmwareUpdate {
                            success: false,
                            offset: ctx.local.ota_writer.offset(),
                        })
                        .ok();
                    }
                }
            }
            _ => {
                defmt::warn!("Unknown frame type");
            }
        }
    }

    // ===== Task 7: 通讯发送 (USB + UART2 双通道) =====
    #[task(priority = 1, shared = [frames_sent])]
    async fn comm_tx_task(mut ctx: comm_tx_task::Context, frame: TypedFrame) {
        let (usb_written, uart_written) = domain::comm::encode_and_enqueue_dual(&frame);
        if usb_written > 0 || uart_written > 0 {
            ctx.shared.frames_sent.lock(|f| *f += 1);
            if usb_written > 0 {
                tx_flush_task::spawn().ok();
            }
            if uart_written > 0 {
                uart_flush_task::spawn().ok();
            }
        } else {
            defmt::warn!("TX queue full");
        }
    }

    // ===== Task 8: 事件 (触发式) =====
    // 当 board_event 状态变化时由各任务 spawn，仅在有差异时发送事件帧
    #[task(priority = 2, shared = [board_event, prev_board_event])]
    async fn event_task(mut ctx: event_task::Context) {
        // 在同一把锁内完成 diff 检查和快照克隆，避免两次读取之间被高优先级任务修改
        let snapshot = ctx.shared.board_event.lock(|cur| {
            let changed = ctx.shared.prev_board_event.lock(|prev| {
                domain::event::diff_and_update(prev, cur)
            });
            if changed { Some(cur.clone()) } else { None }
        });
        if let Some(e) = snapshot {
            comm_tx_task::spawn(TypedFrame::Event(e)).ok();
        }
    }

    // ===== EXTI9_5 中断 (PB5 = BC_ACOK) =====
    #[task(priority = 3, binds = EXTI9_5)]
    fn exti9_5_task(_ctx: exti9_5_task::Context) {
        // 清除 EXTI5 pending bit (写 1 清除)
        let exti = unsafe { &*stm32f4xx_hal::pac::EXTI::ptr() };
        exti.pr().write(|w| w.pr5().clear_bit_by_one());

        // BC_ACOK 引脚状态由 sys_info_task 轮询更新，这里只触发事件
        defmt::info!("EXTI9_5: BC_ACOK changed");
        event_task::spawn().ok();
    }

    // ===== EXTI15_10 中断 (PB14 = HUSB238A INT) =====
    #[task(priority = 3, binds = EXTI15_10)]
    fn exti15_10_task(_ctx: exti15_10_task::Context) {
        // 清除 EXTI14 pending bit (写 1 清除)
        let exti = unsafe { &*stm32f4xx_hal::pac::EXTI::ptr() };
        exti.pr().write(|w| w.pr14().clear_bit_by_one());

        defmt::info!("EXTI15_10: HUSB238A INT");
        event_task::spawn().ok();
    }

    // ===== Task 9: 日志 =====
    #[task(priority = 1, shared = [config])]
    async fn log_task(mut ctx: log_task::Context, msg: LogMessage) {
        let should_send = ctx
            .shared
            .config
            .lock(|c| domain::comm::should_send_log(c.tx_log_level, msg.level));
        if should_send {
            comm_tx_task::spawn(TypedFrame::Log(msg)).ok();
        }
    }

    // ===== Task 10: 系统信息 + 温度监控 + 风扇控制 (1Hz) =====
    #[task(priority = 1, shared = [system_info, uptime_s, frames_sent, i2c_errors, spi_errors, uart_errors, usb_errors, pd_request_voltage, pd_request_current, imu_id, protection_mgr, pwr_servo_en, pwr_5v_en, config, ws2812_colors, fan_en, board_event, bc_acok, temp_battery])]
    async fn sys_info_task(mut ctx: sys_info_task::Context) {
        ctx.shared.uptime_s.lock(|t| *t += 1);

        let device_id = domain::sys_info::get_device_id();
        let uid = domain::sys_info::get_uid();
        let imu_id = ctx.shared.imu_id.lock(|id| *id);

        let heap_used = HEAP.used();
        let free_heap_kb = ((8192 - heap_used) / 1024) as u16;

        // 读取错误计数器
        let i2c_errors = ctx.shared.i2c_errors.lock(|e| *e);
        let spi_errors = ctx.shared.spi_errors.lock(|e| *e);
        let uart_errors = ctx.shared.uart_errors.lock(|e| *e);
        let usb_errors = ctx.shared.usb_errors.lock(|e| *e);

        // 读取 PD 请求参数
        let pd_voltage = ctx.shared.pd_request_voltage.lock(|v| *v);
        let pd_current = ctx.shared.pd_request_current.lock(|c| *c);

        // 读取温度数据
        let buf = hal::adc::adc_buf();
        let temp_charge = domain::thermal::ntc_temp_c(buf[hal::adc::CH_TEMP_CHARGE]);
        let temp_servo = domain::thermal::ntc_temp_c(buf[hal::adc::CH_TEMP_SERVO]);
        let temp_5v = domain::thermal::ntc_temp_c(buf[hal::adc::CH_TEMP_5V]);
        let mcu_temp = domain::thermal::mcu_temp_c(buf[hal::adc::CH_MCU_TEMP]);

        // 温度保护检查
        let (servo_limit, v5_limit) = ctx.shared.config.lock(|c| {
            (
                c.servo_temp_limit as f32 / 10.0,
                c.temp_5v_limit as f32 / 10.0,
            )
        });
        let prot_flags = ctx.shared.protection_mgr.lock(|pm| {
            pm.set_servo_temp_limit(servo_limit);
            pm.set_5v_temp_limit(v5_limit);
            let (s, v) = pm.check_thermal(temp_servo, temp_5v);
            if s {
                ctx.shared.pwr_servo_en.lock(|p| p.set_low());
            }
            if v {
                ctx.shared.pwr_5v_en.lock(|p| p.set_low());
            }
            pm.flags()
        });

        // 风扇控制: 任意温度超过 Limit - 10°C 开启，低于 Limit - 15°C 关闭
        let fan_should_on = temp_servo > (servo_limit - 10.0)
            || temp_5v > (v5_limit - 10.0)
            || temp_charge
                > (ctx
                    .shared
                    .config
                    .lock(|c| c.charge_temp_limit as f32 / 10.0)
                    - 10.0);
        let fan_should_off = temp_servo < (servo_limit - 15.0)
            && temp_5v < (v5_limit - 15.0)
            && temp_charge
                < (ctx
                    .shared
                    .config
                    .lock(|c| c.charge_temp_limit as f32 / 10.0)
                    - 15.0);
        let fan_on = ctx.shared.fan_en.lock(|f| {
            if fan_should_on {
                f.set_high();
                true
            } else if fan_should_off {
                f.set_low();
                false
            } else {
                f.is_set_high()
            }
        });

        // 读取充电器 ACOK 状态
        let charger_connected = ctx.shared.bc_acok.lock(|p| p.is_high());

        // 更新 board_event 各字段
        ctx.shared.board_event.lock(|e| {
            // 保护标志
            e.protection_flags =
                ProtectionFlags::from_bits(prot_flags.to_u16()).unwrap_or(ProtectionFlags::empty());
            // 风扇状态
            e.state_change_flags
                .set(StateChangeFlags::FAN_ENABLED, fan_on);
            // 充电器连接状态
            e.state_change_flags
                .set(StateChangeFlags::CHARGER_CONNECTED, charger_connected);
            // 错误标志
            let mut err = ErrorFlags::empty();
            if i2c_errors > 0 {
                err |= ErrorFlags::I2C1_ERROR;
            }
            if spi_errors > 0 {
                err |= ErrorFlags::SPI1_ERROR;
            }
            if uart_errors > 0 {
                err |= ErrorFlags::UART1_ERROR;
            }
            if usb_errors > 0 {
                err |= ErrorFlags::USB_ERROR;
            }
            e.error_flags = err;
        });
        event_task::spawn().ok();

        // WS2812 LED[0]: 充电温度指示
        ctx.shared.ws2812_colors.lock(|c| {
            c[0] = ws2812::battery_temp_color(temp_charge);
        });
        led_task::spawn().ok();

        // 组装 SystemInfo（包含温度数据）
        let info = ctx.shared.system_info.lock(|s| {
            s.device_id = device_id;
            s.uid = uid;
            s.imu_id = imu_id;
            s.uptime_s = ctx.shared.uptime_s.lock(|t| *t);
            // CPU 使用率: 需要 idle task 配合 DWT 周期计数，暂未实现
            s.cpu_usage_percent = 0;
            s.free_heap_kb = free_heap_kb;
            s.stack_watermark_min_kb = check_stack_watermark();
            s.i2c_error_count = i2c_errors;
            s.spi_error_count = spi_errors;
            s.uart_error_count = uart_errors;
            s.usb_error_count = usb_errors;
            s.frames_sent_total = ctx.shared.frames_sent.lock(|f| *f);
            s.pd_request_voltage_mv = pd_voltage;
            s.pd_request_current_ma = pd_current;
            // 温度数据 (i16, 实际值 = 原始值 / 10)
            s.temp_servo_power = (temp_servo * 10.0) as i16;
            s.temp_5v_power = (temp_5v * 10.0) as i16;
            s.temp_mcu = (mcu_temp * 10.0) as i16;
            s.temp_charge = (temp_charge * 10.0) as i16;
            s.temp_battery = ctx.shared.temp_battery.lock(|t| *t);
            s.clone()
        });

        comm_tx_task::spawn(TypedFrame::System(info)).ok();
    }

    // ===== WS2812 LED 输出 =====
    #[task(priority = 1, shared = [ws2812_colors])]
    async fn led_task(mut ctx: led_task::Context) {
        let colors = ctx.shared.ws2812_colors.lock(|c| *c);
        hal::ws2812::send_colors(&colors);
    }

    // ===== USB 轮询 (中断) =====
    #[task(priority = 6, binds = OTG_FS, shared = [usb_dev], local = [usb_rx_buf, usb_rx_pos])]
    fn usb_poll_task(mut ctx: usb_poll_task::Context) {
        let rx_buf = ctx.local.usb_rx_buf;
        let rx_pos = ctx.local.usb_rx_pos;

        ctx.shared.usb_dev.lock(|usb_dev| {
            // 轮询 USB 设备
            usb_dev.poll();

            // CDC 接收
            while let Some(frame) = usb_dev.try_receive_frame(rx_buf, rx_pos) {
                comm_rx_task::spawn(frame).ok();
            }

            // CDC 发送
            usb_dev.flush_tx_queue();
        });
    }

    // ===== USB TX 刷新 =====
    #[task(priority = 5, shared = [usb_dev])]
    async fn tx_flush_task(mut ctx: tx_flush_task::Context) {
        ctx.shared.usb_dev.lock(|usb_dev| {
            usb_dev.flush_tx_queue();
        });
    }

    // ===== UART2 中断处理 =====
    #[task(priority = 5, binds = USART2, shared = [usart2, uart_errors])]
    fn usart2_irq_task(mut ctx: usart2_irq_task::Context) {
        let had_rx = ctx.shared.usart2.lock(|usart2| {
            let sr = usart2.sr().read();
            let had_rx = sr.rxne().bit_is_set();
            hal::uart::handle_usart2_irq(usart2);
            had_rx
        });
        // 收到 RX 数据后，触发帧解析任务
        if had_rx {
            uart2_rx_check_task::spawn().ok();
        }
    }

    // ===== UART2 RX 帧解析 =====
    #[task(priority = 4)]
    async fn uart2_rx_check_task(_ctx: uart2_rx_check_task::Context) {
        // 尝试从 RX 缓冲区解码帧
        while let Some(frame) = hal::uart::uart2_try_decode_frame() {
            comm_rx_task::spawn(frame).ok();
        }
    }

    // ===== UART2 TX 刷新 =====
    #[task(priority = 5, shared = [usart2])]
    async fn uart_flush_task(mut ctx: uart_flush_task::Context) {
        ctx.shared.usart2.lock(|usart2| {
            hal::uart::trigger_tx(usart2);
        });
    }

    // ===== UART1 中断处理 (串口舵机) =====
    #[task(priority = 5, binds = USART1, shared = [usart1, servo_tx_en, uart_errors])]
    fn usart1_irq_task(mut ctx: usart1_irq_task::Context) {
        let had_rx = ctx.shared.usart1.lock(|usart1| {
            let sr = usart1.sr().read();
            let had_rx = sr.rxne().bit_is_set();
            hal::uart::handle_usart1_irq(usart1);
            had_rx
        });
        // TX 完成后拉低 SERVO_TX_EN
        if !hal::uart::uart1_tx_pending() {
            ctx.shared.servo_tx_en.lock(|p| p.set_low());
        }
        // 收到 RX 数据后，触发检查任务
        if had_rx {
            servo_rx_check_task::spawn().ok();
        }
    }

    // ===== UART1 TX 刷新 =====
    #[task(priority = 5, shared = [usart1, servo_tx_en])]
    async fn servo_tx_flush_task(mut ctx: servo_tx_flush_task::Context) {
        // 拉高 TX 使能
        ctx.shared.servo_tx_en.lock(|p| p.set_high());
        // 触发 UART1 TX
        ctx.shared.usart1.lock(|usart1| {
            hal::uart::uart1_trigger_tx(usart1);
        });
    }

    // ===== 串口舵机 RX 检查 (50Hz) =====
    #[task(priority = 3, shared = [frames_sent])]
    async fn servo_rx_check_task(ctx: servo_rx_check_task::Context) {
        let mut buf = [0u8; 512];
        let count = hal::uart::uart1_read_rx(&mut buf);
        if count > 0 {
            let wrapper = ServoCmdWrapper::new(buf[..count].to_vec());
            comm_tx_task::spawn(TypedFrame::AckServoCmd(wrapper)).ok();
            defmt::info!("Servo RX: {} bytes forwarded to host", count);
        }
    }

    // ===== 辅助函数 =====
}
