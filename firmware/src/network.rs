use core::fmt::Debug;

use cyberpixie::{proto::DeviceRole, stdout::uprintln};
use embedded_hal::serial;
use esp8266_softap::{
    clock::SimpleClock, net::SocketAddr, Adapter, JoinApConfig, SoftApConfig, WifiMode, WifiSession,
};
use serde::{Deserialize, Serialize};

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

    pub fn establish<Rx, Tx, C>(
        self,
        adapter: Adapter<Rx, Tx, C>,
    ) -> esp8266_softap::Result<(WifiSession<Rx, Tx, C>, SocketAddr)>
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
                let ap_address = session.listen(333)?;

                Ok((session, ap_address))
            }

            NetworkConfig::JoinAp {
                ssid,
                password,
                address,
            } => {
                uprintln!("Joining to the existing network with ssid: \"{}\"", ssid);

                let mut session = JoinApConfig { ssid, password }.join(adapter)?;
                let ap_address = session.listen(333)?;

                let address = address
                    .parse()
                    .expect("The socket address should be written as follows: \"ip_addr:port\"");
                session.connect_to(Self::LINK_ID, address)?;
                Ok((session, ap_address))
            }
        }
    }
}
