#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::{
    alloc::Layout,
    panic::PanicInfo,
    sync::atomic::{self, AtomicBool, Ordering},
};

use embedded_hal::digital::v2::OutputPin;
use gd32vf103xx_hal::{
    afio::Afio,
    delay::McycleDelay,
    eclic::{EclicExt, Level, LevelPriorityBits, Priority, TriggerType},
    exti::{Exti, ExtiLine, TriggerEdge},
    gpio::gpioa::PA8,
    pac::{self, Interrupt, ECLIC, EXTI},
    prelude::*,
    serial::Serial,
    spi::{Spi, MODE_0},
};
use pixel_poi_firmware::{
    allocator::{heap_bottom, RiscVHeap},
    config::SERIAL_PORT_CONFIG,
    stdout,
    storage::ImagesRepository,
    strip::{FixedImage, StripLineSource},
    uprintln,
};
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_spi::Ws2812;

#[global_allocator]
static ALLOCATOR: RiscVHeap = RiscVHeap::empty();

unsafe fn init_alloc() {
    // Initialize the allocator BEFORE you use it.
    let start = heap_bottom();
    let size = 1024; // in bytes
    ALLOCATOR.init(start, size)
}

const TOTAL_LED_STRIP_LEN: usize = 144;

static LED_STRIP_ENABLE: AtomicBool = AtomicBool::new(false);

#[export_name = "EXTI_LINE9_5"]
fn handle_button_pressed() {
    let extiline = ExtiLine::from_gpio_line(8).unwrap();
    if Exti::is_pending(extiline) {
        Exti::unpend(extiline);
        Exti::clear(extiline);

        let mut old = LED_STRIP_ENABLE.load(Ordering::Relaxed);
        let new = !old;
        loop {
            match LED_STRIP_ENABLE.compare_exchange_weak(
                old,
                new,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => old = x,
            }
        }
    }
}

// Interrupts initialization step.
unsafe fn init_button_interrupt<T>(boot_btn: PA8<T>, exti: EXTI, afio: &mut Afio) {
    // IRQ
    ECLIC::reset();
    ECLIC::set_threshold_level(Level::L0);
    // Use 3 bits for level, 1 for priority
    ECLIC::set_level_priority_bits(LevelPriorityBits::L3P1);

    // eclic_irq_enable(EXTI5_9_IRQn, 1, 1);
    ECLIC::setup(
        Interrupt::EXTI_LINE9_5,
        TriggerType::Level,
        Level::L1,
        Priority::P1,
    );

    // gpio_exti_source_select(GPIO_PORT_SOURCE_GPIOA, GPIO_PIN_SOURCE_8);
    afio.extiss(boot_btn.port(), boot_btn.pin_number());

    // ECLIC::setup(Interrupt::TIMER0_UP, TriggerType::Level, Level::L0, Priority::P0);
    ECLIC::unmask(Interrupt::EXTI_LINE9_5);

    let mut exti = Exti::new(exti);

    let extiline = ExtiLine::from_gpio_line(boot_btn.pin_number()).unwrap();
    exti.listen(extiline, TriggerEdge::Rising);
    Exti::clear(extiline);

    riscv::interrupt::enable();
}

#[riscv_rt::entry]
fn main() -> ! {
    unsafe { init_alloc() }

    // Hardware initialization step.
    let dp = pac::Peripherals::take().unwrap();

    let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let mut delay = McycleDelay::new(&rcu.clocks);

    let gpioa = dp.GPIOA.split(&mut rcu);
    let (usb_tx, mut _usb_rx) = {
        let tx = gpioa.pa9.into_alternate_push_pull();
        let rx = gpioa.pa10.into_floating_input();

        let serial = Serial::new(dp.USART0, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };
    stdout::enable(usb_tx);

    delay.delay_ms(1_000);
    uprintln!("Serial port configured.");

    let vec = alloc::vec![0_u8; 512];
    uprintln!("Successfuly allocated: {}", vec.len());
    drop(vec);

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
            3200.khz(),
            &mut rcu,
        )
    };
    let mut strip = Ws2812::new(spi);

    uprintln!("LED strip configured.");
    strip
        .write(core::iter::repeat(RGB8::default()).take(TOTAL_LED_STRIP_LEN))
        .ok();
    uprintln!("LED strip cleaned.");

    unsafe {
        init_button_interrupt(gpioa.pa8, dp.EXTI, &mut afio);
    }
    uprintln!("Toggle LED strip button enabled.");

    // SPI1_SCK(PB13), SPI1_MISO(PB14) and SPI1_MOSI(PB15) GPIO pin configuration
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

    let mut images = ImagesRepository::open(&mut device).unwrap();
    uprintln!("Total images count: {}", images.count());

    let image_num = 3;
    let (refresh_rate, data) = images.read_image(image_num);
    let mut source = FixedImage::from_raw(data, refresh_rate);
    uprintln!("Loaded {} image from the repository", image_num);

    let mut current_state = LED_STRIP_ENABLE.load(Ordering::SeqCst);
    loop {
        let next_state = LED_STRIP_ENABLE.load(Ordering::SeqCst);
        if current_state != next_state {
            uprintln!("LED poi enabled: {}", next_state);

            strip
                .write(core::iter::repeat(RGB8::default()).take(TOTAL_LED_STRIP_LEN))
                .ok();
            delay.delay_ms(100);

            current_state = next_state;
        }

        if current_state {
            let (us, line) = source.next_line();
            strip.write(line).ok();
            delay.delay_us(us.0);
        }
    }
}

#[alloc_error_handler]
fn oom(layout: Layout) -> ! {
    uprintln!("OOM: {:?}", layout);

    loop {
        continue;
    }
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
