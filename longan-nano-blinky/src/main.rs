#![no_std]
#![no_main]

use embedded_hal::digital::v2::{InputPin, OutputPin};
use longan_nano::{
    hal::{
        delay::McycleDelay,
        eclic::{EclicExt, Level, LevelPriorityBits, Priority, TriggerType},
        exti::{Exti, ExtiLine, TriggerEdge},
        gpio::{
            gpioa::{PA1, PA2, PA8},
            gpioc::PC13,
            Input, Output, PullUp, PushPull,
        },
        pac::{self, Interrupt, ECLIC},
        prelude::*,
    },
    sprintln,
};
use riscv::interrupt;

use panic_halt as _;

/// Generic LED
pub trait LedControl {
    /// Turns the LED off
    fn off(&mut self);

    /// Turns the LED on
    fn on(&mut self);
}

impl LedControl for PC13<Output<PushPull>> {
    fn on(&mut self) {
        self.set_low().unwrap();
    }

    fn off(&mut self) {
        self.set_high().unwrap();
    }
}

impl LedControl for PA1<Output<PushPull>> {
    fn on(&mut self) {
        self.set_low().unwrap();
    }

    fn off(&mut self) {
        self.set_high().unwrap();
    }
}

impl LedControl for PA2<Output<PushPull>> {
    fn on(&mut self) {
        self.set_low().unwrap();
    }

    fn off(&mut self) {
        self.set_high().unwrap();
    }
}

struct ButtonIrqData {
    boot_btn: PA8<Input<PullUp>>,
    current_led: usize,
}

impl ButtonIrqData {
    fn new(boot_btn: PA8<Input<PullUp>>) -> Self {
        Self {
            boot_btn,
            current_led: 0,
        }
    }

    fn handle_event(&mut self) {
        if self.boot_btn.is_high().unwrap() {
            self.current_led = (self.current_led + 1) % 3;
            sprintln!("Button pressed. Current led is {}", self.current_led);
        } else {
            sprintln!("Button released. Current led is {}", self.current_led);
        }
    }
}

static mut BUTTON_DATA: Option<ButtonIrqData> = None;

#[export_name = "EXTI_LINE9_5"]
fn handle_button_pressed() {
    let extiline = ExtiLine::from_gpio_line(8).unwrap();
    if Exti::is_pending(extiline) {
        Exti::unpend(extiline);
        Exti::clear(extiline);

        interrupt::free(|_| unsafe {
            BUTTON_DATA.as_mut().unwrap().handle_event();
        })
    }
}

#[riscv_rt::entry]
fn main() -> ! {
    // Hardware initialization step.

    let dp = pac::Peripherals::take().unwrap();

    let mut rcu = dp.RCU.configure().freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let gpioa = dp.GPIOA.split(&mut rcu);
    let gpioc = dp.GPIOC.split(&mut rcu);

    let mut led_1 = gpioc.pc13.into_push_pull_output();
    let mut led_2 = gpioa.pa1.into_push_pull_output();
    let mut led_3 = gpioa.pa2.into_push_pull_output();

    let boot_btn = gpioa.pa8.into_pull_up_input();

    longan_nano::stdout::configure(
        dp.USART0,
        gpioa.pa9,
        gpioa.pa10,
        115_200.bps(),
        &mut afio,
        &mut rcu,
    );

    // Interrupts initialization step.

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
    unsafe { ECLIC::unmask(Interrupt::EXTI_LINE9_5) };

    let mut exti = Exti::new(dp.EXTI);

    let extiline = ExtiLine::from_gpio_line(boot_btn.pin_number()).unwrap();
    exti.listen(extiline, TriggerEdge::Both);
    Exti::clear(extiline);

    interrupt::free(|_| unsafe {
        BUTTON_DATA.replace(ButtonIrqData::new(boot_btn));
    });

    unsafe { riscv::interrupt::enable() };

    // Leds preparation step.

    let mut leds: [&mut dyn LedControl; 3] = [&mut led_1, &mut led_2, &mut led_3];
    let mut delay = McycleDelay::new(&rcu.clocks);

    for led in &mut leds {
        led.off();
    }

    // Main routine.

    sprintln!("Start main program");

    let mut current = 0;
    loop {
        let next = interrupt::free(|_| unsafe { BUTTON_DATA.as_ref().unwrap().current_led });
        if next != current {
            leds[current].off();
            current = next;
        }

        leds[current].on();
        delay.delay_ms(150);
        leds[current].off();
        delay.delay_ms(150);
    }
}
