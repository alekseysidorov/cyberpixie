use core::fmt::Debug;

use cyberpixie::{proto::DeviceRole, stdout::uprintln};
use embedded_hal::serial;
use esp8266_softap::{clock::SimpleClock, softap::JoinApConfig, Adapter, SoftApConfig, TcpSocket};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkConfig<'a> {
    SoftAp {
        ssid: &'a str,
        password: &'a str,
        channel: u8,
        mode: u8,
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
    ) -> esp8266_softap::Result<TcpSocket<Rx, Tx, C>, Rx::Error, Tx::Error>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,
        C: SimpleClock,

        Rx::Error: Debug,
        Tx::Error: Debug,
    {
        match self {
            NetworkConfig::SoftAp {
                ssid,
                password,
                channel,
                mode,
            } => {
                uprintln!("Creating a new access point with ssid: \"{}\"", ssid);

                SoftApConfig {
                    ssid,
                    password,
                    channel,
                    mode,
                }
                .start(adapter)
            }

            NetworkConfig::JoinAp {
                ssid,
                password,
                address,
            } => {
                uprintln!("Joining to the existing network with ssid: \"{}\"", ssid);

                JoinApConfig {
                    ssid,
                    password,
                    link_id: Self::LINK_ID,
                    address: address.parse().expect(
                        "The socket address should be written as follows: \"ip_addr:port\"",
                    ),
                }
                .join(adapter)
            }
        }
    }
}
