// #![feature(async_fn_in_trait)]

// use std::{convert::Infallible, thread::JoinHandle};

// use cyberpixie_app::{App, Board, Configuration, CyberpixieResult, Storage};
// use cyberpixie_core::{
//     proto::types::{DeviceRole, FirmwareInfo, ImageId},
//     ExactSizeRead,
// };
// use cyberpixie_network::blocking::Client;
// use embedded_io::{
//     blocking::{Read, Seek},
//     Io,
// };
// use std_embedded_nal::Stack;

// struct BoardStub;
// struct StorageStub;
// struct ImageReadStub;

// impl Io for ImageReadStub {
//     type Error = Infallible;
// }

// impl Read for ImageReadStub {
//     fn read(&mut self, _buf: &mut [u8]) -> Result<usize, Self::Error> {
//         unimplemented!()
//     }
// }

// impl ExactSizeRead for ImageReadStub {
//     fn bytes_remaining(&self) -> usize {
//         0
//     }
// }

// impl Seek for ImageReadStub {
//     fn seek(&mut self, _pos: embedded_io::SeekFrom) -> Result<u64, Self::Error> {
//         unimplemented!()
//     }
// }

// impl Storage for StorageStub {
//     type ImageRead<'a> = ImageReadStub;

//     fn config(&mut self) -> CyberpixieResult<Configuration> {
//         Ok(Configuration {
//             strip_len: 16,
//             current_image: None,
//         })
//     }

//     fn set_config(&mut self, _config: Configuration) -> CyberpixieResult<()> {
//         unimplemented!()
//     }

//     fn add_image<R: cyberpixie_core::ExactSizeRead>(
//         &mut self,
//         _refresh_rate: cyberpixie_core::proto::types::Hertz,
//         _image: R,
//     ) -> CyberpixieResult<ImageId> {
//         unimplemented!()
//     }

//     async fn add_image_async<R: embedded_io::asynch::Read + ExactSizeRead>(
//         &mut self,
//         _refresh_rate: cyberpixie_core::proto::types::Hertz,
//         _image: R,
//     ) -> CyberpixieResult<ImageId> {
//         unimplemented!()
//     }

//     fn read_image(
//         &mut self,
//         _id: ImageId,
//     ) -> CyberpixieResult<cyberpixie_app::ImageReader<'_, Self>> {
//         unimplemented!()
//     }

//     fn images_count(&mut self) -> CyberpixieResult<ImageId> {
//         Ok(ImageId(0))
//     }

//     fn clear_images(&mut self) -> CyberpixieResult<()> {
//         unimplemented!()
//     }
// }

// impl Board for BoardStub {
//     type Storage = StorageStub;
//     type NetworkStack = Stack;
//     type RenderTask = ();

//     fn take_components(&mut self) -> Option<(Self::Storage, Self::NetworkStack)> {
//         Some((StorageStub, std_embedded_nal::Stack))
//     }

//     fn start_rendering(
//         &mut self,
//         _storage: Self::Storage,
//         _image_id: ImageId,
//     ) -> CyberpixieResult<Self::RenderTask> {
//         unimplemented!()
//     }

//     fn stop_rendering(&mut self, _handle: Self::RenderTask) -> CyberpixieResult<Self::Storage> {
//         unimplemented!()
//     }

//     fn firmware_info(&self) -> FirmwareInfo {
//         FirmwareInfo
//     }
// }

// fn create_loopback(
//     stack: &mut Stack,
//     port: u16,
// ) -> (JoinHandle<CyberpixieResult<()>>, Client<Stack>) {
//     // Create a thread with an application instance
//     let app = App::with_port(BoardStub, port).unwrap();
//     let app_handle = std::thread::spawn(move || app.run());

//     // Create a Cyberpixie client and connect with an application.
//     let client = Client::connect(
//         stack,
//         embedded_nal::SocketAddr::new(embedded_nal::Ipv6Addr::localhost().into(), port),
//     )
//     .unwrap();
//     (app_handle, client)
// }

// #[test]
// fn test_simple_handshake() {
//     let mut stack = Stack;
//     let (_app, mut client) = create_loopback(&mut stack, 1234);

//     let info = client.peer_info(&mut stack).unwrap();
//     assert_eq!(info.role, DeviceRole::Main);

//     client.debug(&mut stack, "Hello debug").unwrap();
//     client.debug(&mut stack, "Hello debug 2").unwrap();
//     drop(client);
// }
