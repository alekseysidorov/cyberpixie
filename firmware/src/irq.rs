use embedded_hal::serial::Read;
use gd32vf103xx_hal::{
    eclic::{EclicExt, Level, LevelPriorityBits, Priority, TriggerType},
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

pub fn init_interrupts(mut usart1: Usart1) -> BufferedRx {
    // Safety: we can enter this section only once during the interrupts
    // initialization routine.
    unsafe {
        riscv::interrupt::disable();

        // Create USART1 interrupt context.
        usart1.timer.listen(gd32vf103xx_hal::timer::Event::Update);
        USART1_IRQ_CONTEXT.replace(usart1);

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

        riscv::interrupt::enable();
    }

    BufferedRx(())
}

// IRQ handlers

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
