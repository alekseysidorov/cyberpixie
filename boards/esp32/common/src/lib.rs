//! Common Cyberpixie esp code.

#![no_std]
#![feature(async_fn_in_trait, type_alias_impl_trait)]
// Linter configuration
#![warn(unsafe_code, missing_copy_implementations)]
#![warn(clippy::pedantic)]
#![warn(clippy::use_self)]
// Too many false positives.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation
)]

use cyberpixie_app::{
    core::proto::types::{FirmwareInfo, ImageId},
    network::{NetworkSocket, NetworkStack, SocketAddr},
    Board, Configuration, CyberpixieError, CyberpixieResult,
};
use cyberpixie_embedded_storage::MemoryLayout;
use cyberpixie_network::FromSocketAddress;
use embassy_net::{tcp::TcpSocket, IpListenEndpoint, Stack};
use embassy_time::Duration;
#[cfg(feature = "esp32c3")]
use esp32c3_hal as hal;
#[cfg(feature = "esp32s3")]
use esp32s3_hal as hal;
use esp_storage::FlashStorage;
use esp_wifi::wifi::WifiApDevice;
use render::RenderingHandle;

pub mod render;

// pub mod wifi;

// pub type WifiDevice = esp_wifi::wifi::WifiDevice<'static, WifiApDevice>;
pub type StorageImpl = cyberpixie_embedded_storage::StorageImpl<FlashStorage>;

// /// Default memory layout of internal Flash storage.
// pub const DEFAULT_MEMORY_LAYOUT: MemoryLayout = MemoryLayout {
//     base: 0x9000,
//     size: 0x0019_9000,
// };

// #[derive(Clone, Copy)]
// pub struct NetworkStackImpl {
//     stack: &'static Stack<WifiDevice>,
// }

// pub struct NetworkSocketImpl {
//     rx: [u8; 1024],
//     tx: [u8; 1024],
//     stack: &'static Stack<WifiDevice>,
// }

// impl NetworkSocket for NetworkSocketImpl {
//     type ConnectionError = embassy_net::tcp::Error;
//     type Connection<'a> = TcpSocket<'a>;

//     async fn accept(&mut self, port: u16) -> CyberpixieResult<Self::Connection<'_>> {
//         let mut socket = TcpSocket::new(self.stack, &mut self.rx, &mut self.tx);
//         socket.set_timeout(Some(Duration::from_secs(30)));

//         socket
//             .accept(IpListenEndpoint { addr: None, port })
//             .await
//             .map_err(CyberpixieError::network)?;
//         Ok(socket)
//     }

//     async fn connect(&mut self, addr: SocketAddr) -> CyberpixieResult<Self::Connection<'_>> {
//         let mut socket = TcpSocket::new(self.stack, &mut self.rx, &mut self.tx);
//         socket.set_timeout(Some(Duration::from_secs(30)));

//         let (addr, port) = FromSocketAddress::from_socket_address(addr);
//         socket
//             .connect((addr, port))
//             .await
//             .map_err(CyberpixieError::network)?;
//         Ok(socket)
//     }
// }

// impl NetworkStackImpl {
//     /// Creates a new network driver implementation.
//     pub fn new(stack: &'static Stack<WifiDevice>) -> Self {
//         Self { stack }
//     }
// }

// impl NetworkStack for NetworkStackImpl {
//     type Socket<'a> = NetworkSocketImpl where Self: 'a;

//     fn socket(&mut self) -> Self::Socket<'_> {
//         NetworkSocketImpl {
//             rx: [0_u8; 1024],
//             tx: [0_u8; 1024],
//             stack: self.stack,
//         }
//     }
// }

// /// Board support implementation for the Cyberpixie device.
// pub struct BoardImpl {
//     network: Option<NetworkStackImpl>,
//     storage: Option<StorageImpl>,
//     rendering_handle: RenderingHandle,
// }

// impl BoardImpl {
//     pub fn new(stack: &'static Stack<WifiDevice>, rendering_handle: RenderingHandle) -> Self {
//         let storage = StorageImpl::init(
//             Configuration::default(),
//             FlashStorage::new(),
//             DEFAULT_MEMORY_LAYOUT,
//             singleton!([0_u8; 512]),
//         )
//         .expect("Unable to create storage");

//         Self {
//             network: Some(NetworkStackImpl::new(stack)),
//             storage: Some(storage),
//             rendering_handle,
//         }
//     }
// }

// impl Board for BoardImpl {
//     type Storage = StorageImpl;
//     type NetworkStack = NetworkStackImpl;
//     type RenderTask = RenderingHandle;

//     fn take_components(&mut self) -> Option<(Self::Storage, Self::NetworkStack)> {
//         let storage = self.storage.take()?;
//         let stack = self.network.take()?;
//         Some((storage, stack))
//     }

//     async fn start_rendering(
//         &mut self,
//         storage: Self::Storage,
//         image_id: ImageId,
//     ) -> cyberpixie_app::CyberpixieResult<Self::RenderTask> {
//         self.rendering_handle.start(storage, image_id).await;
//         Ok(self.rendering_handle.clone())
//     }

//     async fn stop_rendering(
//         &mut self,
//         handle: Self::RenderTask,
//     ) -> cyberpixie_app::CyberpixieResult<Self::Storage> {
//         Ok(handle.stop().await)
//     }

//     fn firmware_info(&self) -> FirmwareInfo {
//         FirmwareInfo
//     }
// }

/// Creates a singleton value in the static memory and returns a mutable reference.
#[macro_export]
macro_rules! singleton {
    ($val:expr) => {{
        type T = impl Sized;
        static STATIC_CELL: static_cell::StaticCell<T> = static_cell::StaticCell::new();
        let (x,) = STATIC_CELL.init(($val,));
        x
    }};
}
