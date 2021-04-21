#![no_std]
#![no_main]

use gd32vf103xx_hal::{
    delay::McycleDelay,
    eclic::{EclicExt, Level, LevelPriorityBits, Priority, TriggerType},
    gpio::{
        gpioa::{PA5, PA6, PA7},
        Alternate, Floating, Input, PushPull,
    },
    pac::{self, Interrupt, ECLIC, SPI0, TIMER1},
    prelude::*,
    rcu::Rcu,
    serial::Serial,
    spi::Spi,
    time::Hertz,
    timer::{Event, Timer},
};
use pixel_poi_firmware::{
    config::SERIAL_PORT_CONFIG,
    generated, stdout,
    strip::{FixedImage, StripLineSource},
    sync::RwLock,
    uprint, uprintln,
};
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_spi::Ws2812;

use panic_halt as _;

type Spi0 = Spi<
    SPI0,
    (
        PA5<Alternate<PushPull>>,
        PA6<Input<Floating>>,
        PA7<Alternate<PushPull>>,
    ),
>;

struct StripDriver {
    output: Ws2812<Spi0>,
    image: FixedImage,
    timer: Option<Timer<TIMER1>>,
}

impl StripDriver {
    pub fn new(output: Ws2812<Spi0>, timer: TIMER1, rcu: &mut Rcu) -> Self {
        Self {
            output,
            image: FixedImage::empty(),
            timer: Some(Timer::timer1(timer, 1.hz(), rcu)),
        }
    }

    pub fn set_image(&mut self, data: &[RGB8], mut refresh_rate: Hertz, rcu: &mut Rcu) {
        self.image.reset(data);

        refresh_rate.0 = core::cmp::min(800, refresh_rate.0);
        uprintln!("Setting up {}hz refresh rate", refresh_rate.0);

        // Reset timer update period
        let mut timer = self.timer.take().unwrap();
        timer.unlisten(Event::Update);
        uprintln!("Unlisten old timer");
        timer = Timer::timer1(timer.free(), refresh_rate, rcu);
        uprintln!("Create new timer");
        timer.listen(Event::Update);
        uprintln!("Replace a timer");
        self.timer.replace(timer);
        uprintln!("Timer replaced");

        self.output
            .write(core::iter::repeat(RGB8::default()).take(144))
            .unwrap();
            uprintln!("Clear line");
    }

    pub fn refresh(&mut self) {
        self.timer.as_mut().unwrap().clear_update_interrupt_flag();
        self.output.write(self.image.next_line()).ok();
    }
}

static STRIP: RwLock<Option<StripDriver>> = RwLock::new(None);

#[export_name = "TIMER1"]
fn handle_timer_0_update() {
    STRIP
        .write(|mut inner| {
            inner.as_mut().unwrap().refresh();
        })
        .ok();
}

unsafe fn setup_timer1_interrupts() {
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
}

#[riscv_rt::entry]
fn main() -> ! {
    // Hardware initialization step.
    let dp = pac::Peripherals::take().unwrap();

    let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let gpioa = dp.GPIOA.split(&mut rcu);

    let (usb_tx, mut _usb_rx) = {
        let tx = gpioa.pa9.into_alternate_push_pull();
        let rx = gpioa.pa10.into_floating_input();

        let serial = Serial::new(dp.USART0, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };
    stdout::enable(usb_tx);

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
            3400.khz(),
            &mut rcu,
        )
    };

    let mut output = Ws2812::new(spi);
    output
        .write(core::iter::repeat(RGB8::default()).take(144))
        .unwrap();

    let mut rate = 800.hz();
    let mut driver = StripDriver::new(output, dp.TIMER1, &mut rcu);
    driver.set_image(&generated::DATA, rate, &mut rcu);
    STRIP.write(|mut inner| inner.replace(driver)).unwrap();

    unsafe { setup_timer1_interrupts() }

    let mut delay = McycleDelay::new(&rcu.clocks);
    loop {
        delay.delay_ms(1000);
        rate.0 += 50;

        STRIP
            .write(|mut inner| {
                uprintln!();
                uprintln!("Setting up {}hz image rate", rate.0);
                inner
                    .as_mut()
                    .unwrap()
                    .set_image(&generated::DATA, rate, &mut rcu)
            })
            .unwrap();
    }
}
