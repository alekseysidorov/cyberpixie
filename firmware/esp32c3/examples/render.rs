#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use cyberpixie_app::{
    core::{
        proto::types::{Hertz, ImageId},
        storage::ImageLines,
        MAX_STRIP_LEN,
    },
    Configuration, Storage,
};
use cyberpixie_embedded_storage::StorageImpl;
use cyberpixie_esp32c3::{singleton, ws2812_spi, SpiType, DEFAULT_MEMORY_LAYOUT};
use embassy_executor::Executor;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_println::logger::init_logger;
use esp_storage::FlashStorage;
use hal::{
    clock::{ClockControl, CpuClock},
    embassy,
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
    Rtc,
};
use smart_leds::RGB8;
use ws2812_async::Ws2812;

const RAW_IMAGE: &[u8] = include_bytes!("../../../assets/nyan_cat_24.raw").as_slice();

type RGB8Line = heapless::Vec<RGB8, MAX_STRIP_LEN>;
type FramesChannel = Channel<CriticalSectionRawMutex, RGB8Line, 2>;
type FramesSender<'a> = Sender<'a, CriticalSectionRawMutex, RGB8Line, 2>;
type FramesReceiver<'a> = Receiver<'a, CriticalSectionRawMutex, RGB8Line, 2>;

#[embassy_executor::task]
async fn render_task(spi: &'static mut SpiType<'static>, receiver: FramesReceiver<'static>) {
    const LED_BUF_LEN: usize = 12 * MAX_STRIP_LEN;

    let mut ws: Ws2812<_, LED_BUF_LEN> = Ws2812::new(spi);
    // Cleanup strip
    ws.write(core::iter::repeat(RGB8::default()).take(MAX_STRIP_LEN))
        .await
        .unwrap();

    let rate = Hertz(600);
    let frame_duration = Duration::from_hz(rate.0 as u64);

    log::info!("Start LED strip rendering task with refresh rate: {rate}hz");

    let mut total_render_time = 0;
    let mut dropped_frames = 0;
    let mut counts = 0;
    loop {
        let now = Instant::now();
        let line = receiver.recv().await;
        ws.write(line.into_iter()).await.unwrap();
        let elapsed = now.elapsed().as_micros();

        total_render_time += elapsed;
        if now.elapsed() <= frame_duration {
            let next_frame_time = now + frame_duration;
            Timer::at(next_frame_time).await;
        } else {
            dropped_frames += 1;
        }

        counts += 1;
        if counts % 10_000 == 0 {
            let line_render_time = total_render_time as f32 / counts as f32;
            log::info!("-> Total rendering time {total_render_time}us");
            log::info!("-> per line: {line_render_time}us");
            log::info!(
                "-> Average frame rendering frame rate is {}Hz",
                1_000_000f32 / line_render_time
            );
            log::info!("-> dropped frames: {dropped_frames}");
            Timer::after(Duration::from_millis(100)).await;
        }
    }
}

#[embassy_executor::task]
async fn storage_read_task(
    storage: &'static mut StorageImpl<FlashStorage>,
    sender: FramesSender<'static>,
) {
    let strip_len = storage.config().unwrap().strip_len;
    let image = storage.read_image(ImageId(0)).unwrap();

    let mut reader = ImageLines::new(image, strip_len, [0_u8; MAX_STRIP_LEN * 3]);
    loop {
        let line: RGB8Line = reader.next_line().unwrap().collect();
        sender.send(line).await;
    }
}

#[entry]
fn main() -> ! {
    init_logger(log::LevelFilter::Info);

    // Initialize peripherals
    let peripherals = Peripherals::take();
    let mut system = peripherals.SYSTEM.split();

    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();

    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(
        peripherals.TIMG0,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(
        peripherals.TIMG1,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt1 = timer_group1.wdt;

    // Disable watchdog timers
    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    let spi = singleton!(ws2812_spi(
        peripherals.SPI2,
        peripherals.GPIO,
        peripherals.IO_MUX,
        peripherals.DMA,
        &mut system.peripheral_clock_control,
        &clocks
    ));

    let storage = singleton!(StorageImpl::init(
        Configuration::default(),
        FlashStorage::new(),
        DEFAULT_MEMORY_LAYOUT,
        singleton!([0_u8; 512]),
    )
    .unwrap());

    storage.add_image(Hertz(50), RAW_IMAGE).unwrap();
    log::info!(
        "Image written: total count is {}",
        storage.images_count().unwrap()
    );

    // Initalize channel between rendering and reading tasks.
    let channel = singleton!(FramesChannel::new());

    // Initialize and run an Embassy executor.
    embassy::init(&clocks, timer_group0.timer0);
    let executor = singleton!(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(render_task(spi, channel.receiver())).unwrap();
        spawner
            .spawn(storage_read_task(storage, channel.sender()))
            .unwrap();
    })
}
