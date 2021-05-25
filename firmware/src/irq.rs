use cyberpixie::{HwEvent, HwEventSource};
use embedded_hal::serial::Read;
use gd32vf103xx_hal::{
    afio::Afio,
    eclic::{EclicExt, Level, LevelPriorityBits, Priority, TriggerType},
    exti::{Exti, ExtiLine, TriggerEdge},
    gpio::gpioa::PA8,
    pac::{Interrupt, ECLIC, TIMER1, USART1},
    serial::Rx,
    timer::Timer,
};

use heapless::mpmc::MpMcQueue;

// Usart Interrupt context

type UartError = <Rx<USART1> as Read<u8>>::Error;

pub struct Usart1 {
    pub rx: Rx<USART1>,
    // Quick and dirty buffered serial port implementation.
    // FIXME Rewrite it on the USART1 interrupts.
    pub timer: Timer<TIMER1>,
}

static UART_QUEUE: MpMcQueue<Result<u8, UartError>, 128> = MpMcQueue::new();
static mut USART1_IRQ_CONTEXT: Option<Usart1> = None;

pub struct BufferedRx(());

impl Read<u8> for BufferedRx {
    type Error = <Rx<USART1> as Read<u8>>::Error;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        let value = UART_QUEUE.dequeue();

        value
            .ok_or(nb::Error::WouldBlock)?
            .map_err(nb::Error::Other)
    }
}

// Button A8 Interrupt context

struct BtnContext {
    line: ExtiLine,
}

static HW_EVENTS: MpMcQueue<HwEvent, 1> = MpMcQueue::new();
static mut BTN_IRQ_CONTEXT: Option<BtnContext> = None;

pub struct HwEventsReceiver(());

impl HwEventSource for HwEventsReceiver {
    fn next_event(&self) -> Option<HwEvent> {
        HW_EVENTS.dequeue()
    }
}

pub struct Button<'a, T> {
    pub pin: PA8<T>,
    pub afio: &'a mut Afio,
    pub exti: Exti,
}

pub fn init_interrupts<T>(
    mut usart1: Usart1,
    mut button: Button<'_, T>,
) -> (BufferedRx, HwEventsReceiver) {
    // Safety: we can enter this section only once during the interrupts
    // initialization routine.
    unsafe {
        riscv::interrupt::disable();

        // Create USART1 interrupt context.
        usart1.timer.listen(gd32vf103xx_hal::timer::Event::Update);
        USART1_IRQ_CONTEXT.replace(usart1);

        // Create Button interrupt context.
        button
            .afio
            .extiss(button.pin.port(), button.pin.pin_number());
        let line = ExtiLine::from_gpio_line(button.pin.pin_number()).unwrap();
        button.exti.listen(line, TriggerEdge::Falling);
        Exti::clear(line);
        BTN_IRQ_CONTEXT.replace(BtnContext { line });

        // IRQ
        ECLIC::reset();
        ECLIC::set_threshold_level(Level::L0);
        // Use 3 bits for level, 1 for priority
        ECLIC::set_level_priority_bits(LevelPriorityBits::L3P1);

        ECLIC::setup(
            Interrupt::TIMER1,
            TriggerType::RisingEdge,
            Level::L3,
            Priority::P1,
        );

        ECLIC::unmask(Interrupt::TIMER1);
        ECLIC::unmask(Interrupt::EXTI_LINE9_5);
        
        riscv::interrupt::enable();
    }

    (BufferedRx(()), HwEventsReceiver(()))
}

// IRQ handlers

#[inline(always)]
pub fn handle_button_pressed() {
    let line = riscv::interrupt::free(|_| unsafe {
        let context = BTN_IRQ_CONTEXT
            .as_ref()
            .expect("the context should be initialized before getting");
        context.line
    });

    if Exti::is_pending(line) {
        Exti::unpend(line);
        Exti::clear(line);

        HW_EVENTS.enqueue(HwEvent::ShowNextImage).ok();
    }
}

#[inline(always)]
pub fn handle_usart1_update() {
    riscv::interrupt::free(|_| unsafe {
        let context = USART1_IRQ_CONTEXT
            .as_mut()
            .expect("the context should be initialized before getting");
        context.timer.clear_update_interrupt_flag();

        loop {
            let res = match context.rx.read() {
                Err(nb::Error::WouldBlock) => break,
                Ok(byte) => Ok(byte),
                Err(nb::Error::Other(err)) => Err(err),
            };

            UART_QUEUE.enqueue(res).expect("queue buffer overrun");
        }
    });
}
