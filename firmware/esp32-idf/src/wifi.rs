//! Wifi support for the esp32-idf target

use esp_idf_hal::modem::Modem;
use esp_idf_svc::{eventloop::EspSystemEventLoop, wifi::EspWifi};
use esp_idf_sys::EspError;
use log::info;

/// Supported Wifi auth methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod<'a> {
    Open,
    WPA1 { password: &'a str },
    WPA2Personal { password: &'a str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config<'a> {
    pub ssid: &'a str,
    pub hidden: bool,
    pub auth_method: AuthMethod<'a>,
}

impl<'a> Default for Config<'a> {
    fn default() -> Self {
        Self {
            ssid: "cyberpixie-1",
            hidden: false,
            auth_method: AuthMethod::Open,
        }
    }
}

impl<'a> Config<'a> {
    const fn auth_method(&self) -> (embedded_svc::wifi::AuthMethod, &'a str) {
        match self.auth_method {
            AuthMethod::Open => (embedded_svc::wifi::AuthMethod::None, ""),
            AuthMethod::WPA1 { password } => (embedded_svc::wifi::AuthMethod::WPA, password),
            AuthMethod::WPA2Personal { password } => {
                (embedded_svc::wifi::AuthMethod::WPA2Personal, password)
            }
        }
    }

    fn to_softap_config(self) -> embedded_svc::wifi::Configuration {
        let (auth_method, password) = self.auth_method();
        embedded_svc::wifi::Configuration::AccessPoint(
            embedded_svc::wifi::AccessPointConfiguration {
                ssid: self.ssid.into(),
                password: password.into(),
                auth_method,
                ssid_hidden: self.hidden,
                ..embedded_svc::wifi::AccessPointConfiguration::default()
            },
        )
    }

    fn to_client_config(self) -> embedded_svc::wifi::Configuration {
        let (auth_method, password) = self.auth_method();
        embedded_svc::wifi::Configuration::Client(embedded_svc::wifi::ClientConfiguration {
            ssid: self.ssid.into(),
            password: password.into(),
            auth_method,
            ..embedded_svc::wifi::ClientConfiguration::default()
        })
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

    pub fn establish_softap(&mut self, config: Config<'_>) -> anyhow::Result<()> {
        self.inner.set_configuration(&config.to_softap_config())?;
        self.inner.start()?;

        info!(
            "SoftAp started with the SSID {}, with sta {:?}, ap {:?}",
            config.ssid,
            self.inner.sta_netif().get_ip_info()?,
            self.inner.ap_netif().get_ip_info()?
        );
        Ok(())
    }

    pub fn connect_to(&mut self, config: Config<'_>) -> anyhow::Result<()> {
        self.inner.set_configuration(&config.to_client_config())?;
        self.inner.start()?;
        self.inner.connect()?;

        info!(
            "Wifi connected with the SSID {}, with sta {:?}, ap {:?}",
            config.ssid,
            self.inner.sta_netif().get_ip_info()?,
            self.inner.ap_netif().get_ip_info()?
        );
        Ok(())
    }

    pub fn mac_addr(&self) -> Result<[u8; 6], EspError> {
        self.inner.sta_netif().get_mac()
    }
}
