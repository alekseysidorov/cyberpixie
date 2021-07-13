use core::fmt::Debug;

use cyberpixie::{proto::DeviceRole, stdout::uprintln};
use embedded_hal::serial;
use esp8266_wifi_serial::{
    clock::SimpleClock, net::SocketAddr, JoinApConfig, Module, NetworkSession, SoftApConfig,
    WifiMode,
};
use serde::{Deserialize, Serialize};

const LISTEN_PORT: u16 = 333;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkConfig<'a> {
    SoftAp {
        ssid: &'a str,
        password: &'a str,
        channel: u8,
        mode: WifiMode,
    },
    JoinAp {
        ssid: &'a str,
        password: &'a str,
        address: &'a str,
    },
}

impl<'a> NetworkConfig<'a> {
    pub const LINK_ID: usize = 0;

    pub fn device_role(&self) -> DeviceRole {
        match self {
            NetworkConfig::SoftAp { .. } => DeviceRole::Main,
            NetworkConfig::JoinAp { .. } => DeviceRole::Secondary,
        }
    }

    pub fn establish<Rx, Tx, C, const N: usize>(
        self,
        adapter: Module<Rx, Tx, C, N>,
    ) -> esp8266_wifi_serial::Result<(NetworkSession<Rx, Tx, C, N>, SocketAddr)>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,
        C: SimpleClock,
    {
        match self {
            NetworkConfig::SoftAp {
                ssid,
                password,
                channel,
                mode,
            } => {
                uprintln!("Creating a new access point with ssid: \"{}\"", ssid);

                let mut session = SoftApConfig {
                    ssid,
                    password,
                    channel,
                    mode,
                }
                .start(adapter)?;
                session.listen(LISTEN_PORT)?;

                let ip = session.get_info()?.softap_address.unwrap();
                Ok((session, SocketAddr::new(ip, LISTEN_PORT)))
            }

            NetworkConfig::JoinAp {
                ssid,
                password,
                address,
            } => {
                uprintln!("Joining to the existing network with ssid: \"{}\"", ssid);

                let mut session = JoinApConfig { ssid, password }.join(adapter)?;
                session.listen(LISTEN_PORT)?;
                let ip = session.get_info()?.softap_address.unwrap();

                let address = address
                    .parse()
                    .expect("The socket address should be written as follows: \"ip_addr:port\"");
                session.connect(Self::LINK_ID, address)?;

                Ok((session, SocketAddr::new(ip, LISTEN_PORT)))
            }
        }
    }
}
