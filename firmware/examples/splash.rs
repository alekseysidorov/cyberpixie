#![no_std]
#![no_main]

use core::{
    panic::PanicInfo,
    sync::atomic::{self, Ordering},
    time::Duration,
};

use cyberpixie::time::{CountDown, CountDownEx, Microseconds, Milliseconds};
use cyberpixie_firmware::{config::SERIAL_PORT_CONFIG, splash::WanderingLight, TimerImpl};
use gd32vf103xx_hal::{pac, prelude::*, serial::Serial, spi::Spi, timer::Timer};
use smart_leds::{SmartLedsWrite, RGB8};
use stdio_serial::uprintln;
use ws2812_spi::Ws2812;

const TICK_DELAY: u32 = 1;

const STRIP_LEN: usize = 48;

#[riscv_rt::entry]
fn main() -> ! {
    // Hardware initialization step.
    let dp = pac::Peripherals::take().unwrap();

    let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let mut timer = TimerImpl::new(Timer::timer0(dp.TIMER0, 1.mhz(), &mut rcu));
    let mut timer2 = TimerImpl::new(Timer::timer1(dp.TIMER1, 1.mhz(), &mut rcu));

    let gpioa = dp.GPIOA.split(&mut rcu);
    let (usb_tx, mut _usb_rx) = {
        let tx = gpioa.pa9.into_alternate_push_pull();
        let rx = gpioa.pa10.into_floating_input();

        let serial = Serial::new(dp.USART0, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };
    stdio_serial::init(usb_tx);

    timer.delay_ms(Milliseconds(1_000));
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

    let splash = WanderingLight::<STRIP_LEN>::default();
    direct_executor::run_spinning(async move {
        futures::future::join(
            async {
                for (ticks, line) in splash.cycle() {
                    futures::future::join(
                        timer.delay_async(Duration::from_micros((TICK_DELAY * ticks) as u64)),
                        async {
                            strip.write(core::array::IntoIter::new(line)).ok();
                        },
                    )
                    .await;
                }
            },
            async {
                loop {
                    timer2.delay_async(Duration::from_secs(5)).await;
                    uprintln!("Five more seconds passed.");
                }
            },
        )
        .await
    });

    loop {
        atomic::compiler_fence(Ordering::SeqCst);
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
