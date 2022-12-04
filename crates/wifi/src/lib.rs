//! Wifi support for the esp32-idf target

use embedded_svc::wifi::{AccessPointConfiguration, AuthMethod, Configuration};
use esp_idf_hal::modem::Modem;
use esp_idf_svc::{eventloop::EspSystemEventLoop, wifi::EspWifi};
use log::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoftApConfig<'a> {
    pub ssid: &'a str,
    pub password: &'a str,
    pub hidden: bool,
    pub auth_method: AuthMethod,
}

impl<'a> Default for SoftApConfig<'a> {
    fn default() -> Self {
        Self {
            ssid: "cyberpixie-1",
            password: "",
            hidden: false,
            auth_method: AuthMethod::WPA2Personal,
        }
    }
}

pub struct Wifi<'a> {
    inner: EspWifi<'a>,
}

impl<'a> Wifi<'a> {
    pub fn new(modem: Modem, sysloop: EspSystemEventLoop) -> anyhow::Result<Self> {
        let inner = EspWifi::new(modem, sysloop, None)?;
        info!("Created esp-idf Wifi stack");
        Ok(Self { inner })
    }

    pub fn establish_softap(&mut self, config: SoftApConfig<'_>) -> anyhow::Result<()> {
        let conf = Configuration::AccessPoint(AccessPointConfiguration {
            ssid: heapless::String::from(config.ssid),
            password: heapless::String::from(config.password),
            auth_method: AuthMethod::WPA2Personal,
            ssid_hidden: config.hidden,
            ..Default::default()
        });
        self.inner.set_configuration(&conf)?;
        self.inner.start()?;

        info!("SoftAP started with SSID {}", config.ssid);
        Ok(())
    }
}
