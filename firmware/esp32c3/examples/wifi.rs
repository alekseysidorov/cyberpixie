#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use cyberpixie_esp32c3::singleton;
use embassy_executor::Executor;
use embassy_net::{
    tcp::TcpSocket, Config, IpListenEndpoint, Ipv4Address, Ipv4Cidr, Stack, StaticConfig,
};
use embassy_time::{Duration, Timer};
use embedded_svc::wifi::{AccessPointConfiguration, Configuration, Wifi};
use esp_backtrace as _;
use esp_println::{logger::init_logger, print, println};
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

#[entry]
fn main() -> ! {
    init_logger(log::LevelFilter::Info);

    // Initialize peripherals
    let peripherals = Peripherals::take();
    let mut system = peripherals.SYSTEM.split();

    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();

    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    // Disable watchdog timers
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
    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

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

    // Initialize embassy async reactor.
    embassy::init(&clocks, timer_group0.timer0);

    // Network stack configuration.
    let config = Config::Static(StaticConfig {
        address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 88, 1), 24),
        gateway: Some(Ipv4Address::from_bytes(&[192, 168, 88, 1])),
        dns_servers: Default::default(),
    });
    // FIXME: There is no way to use DHCP in Ap mode at this moment :(
    // let config = Config::Dhcp(Default::default());

    // Initialize the network stack
    let (wifi_interface, controller) = esp_wifi::wifi::new_with_mode(wifi, WifiMode::Ap);
    let stack = singleton!(Stack::new(
        wifi_interface,
        config,
        singleton!(embassy_net::StackResources::<3>::new()),
        1234
    ));

    // Spawn Embassy executor
    let executor = singleton!(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(connection(controller)).ok();
        spawner.spawn(net_task(stack)).ok();
        spawner.spawn(task(stack)).ok();
    })
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.get_capabilities());

    match esp_wifi::wifi::get_wifi_state() {
        WifiState::ApStart => {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::ApStop).await;
            Timer::after(Duration::from_millis(5000)).await
        }
        other => {
            println!("Wifi state changed to {other:?}")
        }
    }
    if !matches!(controller.is_started(), Ok(true)) {
        let client_config = Configuration::AccessPoint(AccessPointConfiguration {
            ssid: "esp-wifi".into(),
            ..Default::default()
        });
        controller.set_configuration(&client_config).unwrap();
        println!("Starting wifi");
        controller.start().await.unwrap();
        println!("Wifi started!");
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

    println!("Network config is {:?}", stack.config());
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
