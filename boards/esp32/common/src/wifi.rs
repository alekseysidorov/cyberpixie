//! Wifi network tasks set.

use embassy_executor::Spawner;
use embassy_net::{IpAddress, Ipv4Cidr, Stack};
use embassy_time::{Duration, Timer};
use embedded_svc::wifi::Wifi;
pub use esp_wifi::wifi::WifiDevice;
use esp_wifi::{
    wifi::{WifiController, WifiEvent, WifiState, WifiMode},
    EspWifiTimer, EspWifiInitFor,
};

use crate::{hal::peripheral::Peripheral, singleton};

/// Supported Wifi configuration modes.
#[derive(PartialEq, Eq, Clone)]
pub enum Mode {
    /// Setup device as an access point.
    ///
    /// This mode uses a static IP configuration parameters that means that the client
    /// should setup Wifi connection manually.
    AccessPoint {
        ssid: heapless::String<32>,
        /// If the password is set the `WPA2` security model will be used; otherwise access point
        /// will be open.
        password: Option<heapless::String<64>>,
        /// The static IP address of access point and its gateway as well.
        address: IpAddress,
    },
}

impl Mode {
    /// Returns a network configuration corresponding to this mode.
    #[must_use]
    pub fn network_config(&self) -> embassy_net::Config {
        match self {
            Self::AccessPoint { address, .. } => {
                let IpAddress::Ipv4(address) = *address else {
                    unimplemented!("ipv6 is not supported by the embassy-net stack");
                };

                embassy_net::Config::Static(embassy_net::StaticConfig {
                    address: Ipv4Cidr::new(address, 24),
                    gateway: Some(address),
                    dns_servers: heapless::Vec::default(),
                })
            }
        }
    }

    /// Returns a Wifi configuration corresponding to this mode.
    #[must_use]
    pub fn wifi_config(&self) -> embedded_svc::wifi::Configuration {
        match self {
            Self::AccessPoint { ssid, password, .. } => {
                let auth_method = if password.is_some() {
                    embedded_svc::wifi::AuthMethod::WPA2Personal
                } else {
                    embedded_svc::wifi::AuthMethod::None
                };

                embedded_svc::wifi::Configuration::AccessPoint(
                    embedded_svc::wifi::AccessPointConfiguration {
                        ssid: ssid.clone(),
                        password: password.clone().unwrap_or_default(),
                        auth_method,
                        ..Default::default()
                    },
                )
            }
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Self::AccessPoint {
            ssid: "cybeprixie-0".into(),
            password: None,
            address: IpAddress::v4(192, 168, 1, 1),
        }
    }
}

pub struct WifiManager {
    mode: Mode,
    stack: &'static Stack<WifiDevice<'static>>,
    controller: WifiController<'static>,
}

impl WifiManager {
    /// Creates a new Wifi manager instance.
    #[must_use]
    pub fn new(
        mode: Mode,
        wifi: impl Peripheral<P = crate::hal::radio::Wifi> + 'static,
        timer: EspWifiTimer,
        mut rng: crate::hal::Rng,
        radio_clocks: crate::hal::system::RadioClockControl,
        clocks: &crate::hal::clock::Clocks,
    ) -> Self {
        // Generate a random seed.
        let seed = u64::from(rng.random());
        let init = esp_wifi::initialize(EspWifiInitFor::Wifi, timer, rng, radio_clocks, clocks)
            .expect("Unable to initialize WiFI");

        // Initialize the network stack
        let (device, controller) = esp_wifi::wifi::new_with_mode(&init, wifi, WifiMode::Ap);
        let stack = singleton!(Stack::new(
            device,
            mode.network_config(),
            singleton!(embassy_net::StackResources::<3>::new()),
            seed
        ));

        Self {
            mode,
            stack,
            controller,
        }
    }

    /// Spawns a Wifi manager tasks.
    #[must_use]
    pub fn must_spawn(self, spawner: Spawner) -> &'static Stack<WifiDevice<'static>> {
        spawner.must_spawn(connection_task(self.controller, self.mode.wifi_config()));
        spawner.must_spawn(network_stack_task(self.stack));
        self.stack
    }
}

#[embassy_executor::task]
async fn connection_task(
    mut controller: WifiController<'static>,
    wifi_config: embedded_svc::wifi::Configuration,
) {
    log::info!("start connection task");
    log::info!("Device capabilities: {:?}", controller.get_capabilities());

    log::info!("Waiting for a next wifi state!");
    match esp_wifi::wifi::get_wifi_state() {
        WifiState::ApStart => {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::ApStop).await;
            Timer::after(Duration::from_millis(5000)).await;
        }
        other => {
            log::info!("Wifi state changed to {other:?}");
        }
    }

    if !matches!(controller.is_started(), Ok(true)) {
        controller.set_configuration(&wifi_config).unwrap();
        log::info!("Starting wifi");
        controller.start().await.unwrap();
        log::info!("Wifi started!");
    }

    log::info!("Wifi connection task finished");
}

#[embassy_executor::task]
async fn network_stack_task(stack: &'static Stack<WifiDevice<'static>>) {
    stack.run().await;
}
