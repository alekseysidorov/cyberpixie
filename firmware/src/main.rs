#![no_std]
#![no_main]

use core::{
    panic::PanicInfo,
    sync::atomic::{self, Ordering},
};

use cyberpixie::{
    leds::{SmartLedsWrite, RGB8},
    time::{Microseconds, Milliseconds},
    AppConfig, DeadlineTimer, ImagesRepository,
};
use cyberpixie_firmware::{
    config::{MAX_LINES_COUNT, SERIAL_PORT_CONFIG, STRIP_LEDS_COUNT},
    splash::WanderingLight,
    storage::ImagesStorage,
    TimerImpl,
};
use embedded_hal::{digital::v2::OutputPin, serial::Read};
use esp8266_softap::{Adapter, SoftApConfig};
use gd32vf103xx_hal::{
    afio::Afio,
    eclic::{EclicExt, Level, LevelPriorityBits, Priority, TriggerType},
    exti::{Exti, ExtiLine, TriggerEdge},
    gpio::{
        gpioa::{PA2, PA3},
        Alternate, Floating, Input, PushPull,
    },
    pac::{self, Interrupt, ECLIC, EXTI, TIMER1, USART1},
    prelude::*,
    rcu::Rcu,
    serial::{Event as SerialEvent, Rx, Serial, Tx},
    spi::{Spi, MODE_0},
    timer::Timer,
};
use heapless::mpmc::Q64;
use stdio_serial::uprintln;
use ws2812_spi::Ws2812;

const MAX_IMAGE_BUF_SIZE: usize = MAX_LINES_COUNT * STRIP_LEDS_COUNT;

static UART_QUEUE: Q64<u8> = Q64::new();
static mut USART1_RX: Option<Rx<USART1>> = None;
static mut UART_TIMER: Option<Timer<TIMER1>> = None;

#[export_name = "TIMER1"]
unsafe fn handle_timer_1_update() {
    UART_TIMER.as_mut().unwrap().clear_update_interrupt_flag();

    let rx = USART1_RX.as_mut().unwrap();
    let mut cnt = 0;
    loop {
        match rx.read() {
            Ok(byte) => {
                UART_QUEUE.enqueue(byte).expect("Buffer overrun");
                cnt += 1;
            }
            Err(nb::Error::Other(err)) => {
                panic!("An error in the serial rx occurred: {:?}", err)
            }
            v => {
                if cnt > 0 {
                    uprintln!("{:?}: Got {} bytes from the uart", v, cnt);
                }
                break;
            }
        }
    }
}

// #[export_name = "USART1"]
// unsafe fn handle_new_byte() {
//     uprintln!("USART1 triggered");

//     if ECLIC::is_pending(Interrupt::USART1) {
//         ECLIC::unpend(Interrupt::USART1);

//         let rx = USART1_RX.as_mut().unwrap();
//         let mut cnt = 0;
//         loop {
//             match rx.read() {
//                 Ok(byte) => {
//                     UART_QUEUE.enqueue(byte).expect("Buffer overrun");
//                     cnt += 1;
//                 }
//                 Err(nb::Error::Other(err)) => {
//                     panic!("An error in the serial rx occurred: {:?}", err)
//                 }
//                 v => {
//                     uprintln!("{:?}: Got {} bytes from the uart", v, cnt);
//                     break;
//                 }
//             }
//         }
//     }
// }

struct BufferedRx;

impl Read<u8> for BufferedRx {
    type Error = <Rx<USART1> as Read<u8>>::Error;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        unsafe { USART1_RX.as_mut().unwrap().read() }
        // UART_QUEUE.dequeue().ok_or(nb::Error::WouldBlock)
    }
}

// Interrupts initialization step.
// unsafe fn init_uart_1_interrupted_mode(
//     tx: PA2<Alternate<PushPull>>,
//     rx: PA3<Input<Floating>>,
//     usart1: USART1,
//     exti: EXTI,
//     afio: &mut Afio,
//     rcu: &mut Rcu,
// ) -> (Tx<USART1>, BufferedRx) {
//     // IRQ
//     ECLIC::reset();
//     ECLIC::set_threshold_level(Level::L0);
//     // Use 3 bits for level, 1 for priority
//     ECLIC::set_level_priority_bits(LevelPriorityBits::L3P1);

//     // eclic_irq_enable(EXTI5_9_IRQn, 1, 1);
//     ECLIC::setup(
//         Interrupt::EXTI_LINE3,
//         TriggerType::Level,
//         Level::L1,
//         Priority::P1,
//     );

//     // gpio_exti_source_select(GPIO_PORT_SOURCE_GPIOA, GPIO_PIN_SOURCE_8);
//     let rx_pin = rx.pin_number();
//     afio.extiss(rx.port(), rx_pin);

//     // ECLIC::setup(Interrupt::TIMER0_UP, TriggerType::Level, Level::L0, Priority::P0);
//     ECLIC::unmask(Interrupt::EXTI_LINE3);

//     let mut exti = Exti::new(exti);

//     let extiline = ExtiLine::from_gpio_line(rx_pin).unwrap();
//     exti.listen(extiline, TriggerEdge::Both);
//     Exti::clear(extiline);

//     let serial = Serial::new(usart1, (tx, rx), SERIAL_PORT_CONFIG, afio, rcu);
//     let (tx, rx) = serial.split();

//     USART1_RX.replace(rx);
//     USART1_GPIO_LINE = rx_pin;

//     riscv::interrupt::enable();

//     (tx, BufferedRx)
// }

// Interrupts initialization step.
unsafe fn init_uart_1_interrupted_mode(
    mut timer: Timer<TIMER1>,
    tx: PA2<Alternate<PushPull>>,
    rx: PA3<Input<Floating>>,
    usart1: USART1,
    afio: &mut Afio,
    rcu: &mut Rcu,
) -> (Tx<USART1>, BufferedRx) {
    let serial = Serial::new(usart1, (tx, rx), SERIAL_PORT_CONFIG, afio, rcu);
    let (tx, rx) = serial.split();

    timer.listen(gd32vf103xx_hal::timer::Event::Update);
    USART1_RX.replace(rx);
    UART_TIMER.replace(timer);

    // IRQ
    ECLIC::reset();
    ECLIC::set_threshold_level(Level::L0);
    // Use 3 bits for level, 1 for priority
    ECLIC::set_level_priority_bits(LevelPriorityBits::L3P1);

    ECLIC::setup(
        Interrupt::TIMER1,
        TriggerType::RisingEdge,
        Level::L1,
        Priority::P1,
    );

    ECLIC::unmask(Interrupt::TIMER1);
    riscv::interrupt::enable();

    (tx, BufferedRx)
}

#[riscv_rt::entry]
fn main() -> ! {
    // Hardware initialization step.
    let dp = pac::Peripherals::take().unwrap();

    let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let mut timer = TimerImpl::from(Timer::timer0(dp.TIMER0, 1.mhz(), &mut rcu));

    let gpioa = dp.GPIOA.split(&mut rcu);
    let (usb_tx, mut _usb_rx) = {
        let tx = gpioa.pa9.into_alternate_push_pull();
        let rx = gpioa.pa10.into_floating_input();

        let mut serial = Serial::new(dp.USART0, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.listen(SerialEvent::Rxne);
        serial.split()
    };
    stdio_serial::init(usb_tx);

    timer.delay(Milliseconds(1_000)).unwrap();
    uprintln!("Serial port configured.");

    let spi = {
        let pins = (
            gpioa.pa5.into_alternate_push_pull(),
            gpioa.pa6.into_floating_input(),
            gpioa.pa7.into_alternate_push_pull(),
        );

        Spi::spi0(
            dp.SPI0,
            pins,
            &mut afio,
            ws2812_spi::MODE,
            2800.khz(),
            &mut rcu,
        )
    };

    let mut strip = Ws2812::new(spi);

    uprintln!("Led strip configured.");
    strip
        .write(core::iter::repeat(RGB8::default()).take(144))
        .ok();
    uprintln!("Led strip cleaned.");

    // SPI1_SCK(PB13), SPI1_MISO(PB14) and SPI1_MOSI(PB15) GPIO pin configuration
    let mut device = {
        let gpiob = dp.GPIOB.split(&mut rcu);
        let spi = Spi::spi1(
            dp.SPI1,
            (
                gpiob.pb13.into_alternate_push_pull(),
                gpiob.pb14.into_floating_input(),
                gpiob.pb15.into_alternate_push_pull(),
            ),
            MODE_0,
            20.mhz(), // 16.mzh()
            &mut rcu,
        );

        let mut cs = gpiob.pb12.into_push_pull_output();
        cs.set_low().unwrap();

        let mut device = embedded_sdmmc::SdMmcSpi::new(spi, cs);
        device.init().unwrap();
        device
    };
    let images_repository = ImagesStorage::open(&mut device).unwrap();

    uprintln!("Total images count: {}", images_repository.count());

    let uart_timer = Timer::timer1(dp.TIMER1, 10.khz(), &mut rcu);

    let (esp_tx, esp_rx) = unsafe {
        init_uart_1_interrupted_mode(
            uart_timer,
            gpioa.pa2.into_alternate_push_pull(),
            gpioa.pa3,
            dp.USART1,
            &mut afio,
            &mut rcu,
        )
    };
    uprintln!("esp32 serial communication port configured.");

    let ap = {
        let adapter = Adapter::new(esp_rx, esp_tx).unwrap();
        let config = SoftApConfig {
            ssid: "cyberpixie",
            password: "12345678",
            channel: 5,
            mode: 4,
        };
        config.start(adapter).unwrap()
    };
    let network = cyberpixie_firmware::network::into_service(ap);
    uprintln!("SoftAP has been successfuly configured.");

    uprintln!("Showing splash...");
    let splash = WanderingLight::<STRIP_LEDS_COUNT>::default();
    for (ticks, line) in splash {
        timer.set_deadline(Microseconds(ticks));
        strip.write(core::array::IntoIter::new(line)).ok();
        nb::block!(timer.wait_deadline()).unwrap();
    }
    uprintln!("Splash has been showed.");

    let mut buf: [RGB8; MAX_IMAGE_BUF_SIZE] = [RGB8::default(); MAX_IMAGE_BUF_SIZE];
    let app = AppConfig::<_, _, _, _, STRIP_LEDS_COUNT> {
        network,
        timer,
        images_repository,
        strip,
    }
    .into_app(&mut buf);

    app.run()
}

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    uprintln!();
    uprintln!("The firmware panicked!");
    uprintln!("- {}", info);

    loop {
        atomic::compiler_fence(Ordering::SeqCst);
    }
}
