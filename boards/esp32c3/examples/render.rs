#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use cyberpixie_app::{
    core::{
        io::image_reader::ImageLines,
        proto::types::{Hertz, ImageId},
        MAX_STRIP_LEN,
    },
    Configuration, Storage,
};
use cyberpixie_embedded_storage::StorageImpl;
use cyberpixie_esp32c3::{ws2812_spi, SpiType, DEFAULT_MEMORY_LAYOUT};
use cyberpixie_esp_common::singleton;
use embassy_executor::Executor;
use embassy_net::{
    tcp::TcpSocket, Config, IpListenEndpoint, Ipv4Address, Ipv4Cidr, Stack, StaticConfig,
};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};
use embassy_time::{Duration, Instant, Timer};
use embedded_svc::wifi::{AccessPointConfiguration, Wifi};
use esp_backtrace as _;
use esp_println::{logger::init_logger, print, println};
use esp_storage::FlashStorage;
use esp_wifi::wifi::{WifiController, WifiDevice, WifiEvent, WifiMode, WifiState};
use hal::{
    clock::{ClockControl, CpuClock},
    embassy,
    peripherals::Peripherals,
    prelude::*,
    systimer::SystemTimer,
    timer::TimerGroup,
    Rng, Rtc,
};
use smart_leds::RGB8;
use ws2812_async::Ws2812;

const RAW_IMAGE: &[u8] = include_bytes!("../../../assets/nyan_cat_dark_24.raw").as_slice();
const QUEUE_LEN: usize = 4;

type RGB8Line = heapless::Vec<RGB8, MAX_STRIP_LEN>;
type FramesChannel = Channel<CriticalSectionRawMutex, RGB8Line, QUEUE_LEN>;
type FramesSender<'a> = Sender<'a, CriticalSectionRawMutex, RGB8Line, QUEUE_LEN>;
type FramesReceiver<'a> = Receiver<'a, CriticalSectionRawMutex, RGB8Line, QUEUE_LEN>;

#[embassy_executor::task]
async fn render_task(spi: &'static mut SpiType<'static>, receiver: FramesReceiver<'static>) {
    const LED_BUF_LEN: usize = 12 * MAX_STRIP_LEN;

    let rate = Hertz(550);

    let mut ws: Ws2812<_, LED_BUF_LEN> = Ws2812::new(spi);
    // Cleanup strip
    ws.write(core::iter::repeat(RGB8::default()).take(MAX_STRIP_LEN))
        .await
        .unwrap();

    let frame_duration = Duration::from_hz(rate.0 as u64);

    Timer::after(Duration::from_secs(5)).await;
    log::info!("Start LED strip rendering task with refresh rate: {rate}hz");

    let mut total_render_time = 0;
    let mut dropped_frames = 0;
    let mut counts = 0;
    let mut max_render_time = 0;
    loop {
        let now = Instant::now();
        let line = receiver.recv().await.into_iter();
        ws.write(line).await.unwrap();
        let elapsed = now.elapsed();

        total_render_time += elapsed.as_micros();
        if elapsed <= frame_duration {
            let next_frame_time = now + frame_duration;
            Timer::at(next_frame_time).await;
        } else {
            dropped_frames += 1;
            max_render_time = core::cmp::max(max_render_time, elapsed.as_micros());
        }

        counts += 1;
        if counts == 10_000 {
            let line_render_time = total_render_time as f32 / counts as f32;
            log::info!("-> Refresh rate {rate}hz");
            log::info!("-> Total rendering time {total_render_time}us");
            log::info!("-> per line: {line_render_time}us");
            log::info!("-> max: {max_render_time}us");
            log::info!(
                "-> Average frame rendering frame rate is {}Hz",
                1_000_000f32 / line_render_time
            );
            log::info!(
                "-> dropped frames: {dropped_frames} [{}%]",
                dropped_frames as f32 * 100_f32 / counts as f32
            );

            dropped_frames = 0;
            total_render_time = 0;
            counts = 0;
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

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.get_capabilities());
    loop {
        match esp_wifi::wifi::get_wifi_state() {
            WifiState::ApStart => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::ApStop).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            other => println!("Got wifi state: {other:?}"),
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config =
                embedded_svc::wifi::Configuration::AccessPoint(AccessPointConfiguration {
                    ssid: "esp-wifi".into(),
                    ..Default::default()
                });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start().await.unwrap();
            println!("Wifi started!");
        }
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static>>) {
    stack.run().await
}

#[embassy_executor::task]
async fn task(stack: &'static Stack<WifiDevice<'static>>) {
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
    println!("Connect to the AP `esp-wifi` and point your browser to http://192.168.2.1:8080/");
    println!("Use a static IP in the range 192.168.2.2 .. 192.168.2.255, use gateway 192.168.2.1");

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(embassy_net::SmolDuration::from_secs(10)));
    loop {
        println!("Wait for connection...");
        let r = socket
            .accept(IpListenEndpoint {
                addr: None,
                port: 8080,
            })
            .await;
        println!("Connected...");

        if let Err(e) = r {
            println!("connect error: {:?}", e);
            continue;
        }

        use embedded_io::asynch::Write;

        let mut buffer = [0u8; 1024];
        let mut pos = 0;
        loop {
            match socket.read(&mut buffer).await {
                Ok(0) => {
                    println!("read EOF");
                    break;
                }
                Ok(len) => {
                    let to_print =
                        unsafe { core::str::from_utf8_unchecked(&buffer[..(pos + len)]) };

                    if to_print.contains("\r\n\r\n") {
                        print!("{}", to_print);
                        println!();
                        break;
                    }

                    pos += len;
                }
                Err(e) => {
                    println!("read error: {:?}", e);
                    break;
                }
            };
        }

        let r = socket
            .write_all(
                b"HTTP/1.0 200 OK\r\n\r\n\
            <html>\
                <body>\
                    <h1>Hello Rust! Hello esp-wifi!</h1>\
                </body>\
            </html>\r\n\
            ",
            )
            .await;
        if let Err(e) = r {
            println!("write error: {:?}", e);
        }

        let r = socket.flush().await;
        if let Err(e) = r {
            println!("flush error: {:?}", e);
        }
        Timer::after(Duration::from_millis(1000)).await;

        socket.close();
        Timer::after(Duration::from_millis(1000)).await;

        socket.abort();
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

    // Initialize and get Wifi device
    let timer = SystemTimer::new(peripherals.SYSTIMER).alarm0;
    esp_wifi::initialize(
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();
    let (wifi, _bluetooth) = peripherals.RADIO.split();

    // Network stack configuration.
    let config = Config::Static(StaticConfig {
        address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 2, 1), 24),
        gateway: Some(Ipv4Address::from_bytes(&[192, 168, 2, 1])),
        dns_servers: Default::default(),
    });

    // Initialize the network stack
    let (wifi_interface, controller) = esp_wifi::wifi::new_with_mode(wifi, WifiMode::Ap);
    let stack = singleton!(Stack::new(
        wifi_interface,
        config,
        singleton!(embassy_net::StackResources::<3>::new()),
        1234
    ));

    // Initialize storage
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

    // Initialize channel between rendering and reading tasks.
    let channel = singleton!(FramesChannel::new());

    // Initialize and run an Embassy executor.
    embassy::init(&clocks, timer_group0.timer0);
    let executor = singleton!(Executor::new());
    executor.run(|spawner| {
        spawner.must_spawn(render_task(spi, channel.receiver()));
        spawner.must_spawn(storage_read_task(storage, channel.sender()));
        // Wifi tasks
        spawner.must_spawn(connection(controller));
        spawner.must_spawn(net_task(stack));
        spawner.spawn(task(stack)).ok();
    })
}
