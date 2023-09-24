//! Bluetooth low energy cyberpixie service definition

use bleps::{gatt, asynch::Ble, ad_structure::{create_advertising_data, AdStructure, LE_GENERAL_DISCOVERABLE, BR_EDR_NOT_SUPPORTED}, async_attribute_server::AttributeServer};
use cyberpixie_app::{Configuration, CyberpixieResult};
use cyberpixie_network::core::proto::{types::{PeerInfo, DeviceInfo, DeviceRole}, ResponseHeader, Headers};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use esp32c3_hal::radio::Bluetooth;
use esp_println::println;
use esp_storage::FlashStorage;
use esp_wifi::{EspWifiInitialization, ble::controller::asynch::BleConnector};

use crate::{singleton, StorageImpl, DEFAULT_MEMORY_LAYOUT};

type Mutex<T> = embassy_sync::blocking_mutex::Mutex<CriticalSectionRawMutex, T>;

struct DummyApp {
    device_info: DeviceInfo,
    storage: Option<StorageImpl>,
}

impl DummyApp {
    fn new() -> CyberpixieResult<Self> {
        let mut storage = StorageImpl::init(
            Configuration::default(),
            FlashStorage::new(),
            DEFAULT_MEMORY_LAYOUT,
            singleton!([0_u8; 512]),
        )?;
        let device_info = cyberpixie_app::read_device_info(&mut storage)?;

        Ok(Self {
            device_info,
            storage: Some(storage),
        })
    }

    fn peer_info(&self) -> PeerInfo {
        PeerInfo {
            role: DeviceRole::Main,
            group_id: None,
            device_info: Some(DeviceInfo {
                active: false,
                ..self.device_info
            }),
        }
    }
}

pub async fn run_task(init: EspWifiInitialization, mut bluetooth: Bluetooth,) {
    let connector = BleConnector::new(&init, &mut bluetooth);
    let mut ble = Ble::new(connector, esp_wifi::current_millis);

    let mutex = singleton!(Mutex::new(DummyApp::new().expect("Unable to initialize dummy application")));

    let mut board_info = |offset: usize, data: &mut [u8]| {
        debug_assert!(offset == 0, "offset should be zero");

        let info: PeerInfo = mutex.lock(|app| {
            app.peer_info()
        });

        let header: Headers = ResponseHeader::Handshake(info).into();
        let header_buf = header.encode(data, 0);

        header_buf.len()
    };

    gatt!([service {
        uuid: "c8dcf377-cebe-44c8-b9fd-fa811db3f217",
        characteristics: [
            characteristic {
                uuid: "937312e0-2354-11eb-9f10-fbc30a62cf38",
                read: board_info,
            },
        ],
    },]);

    println!("{:?}", ble.init().await);
    println!("{:?}", ble.cmd_set_le_advertising_parameters().await);
    println!(
        "{:?}",
        ble.cmd_set_le_advertising_data(
            create_advertising_data(&[
                AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x1809)]),
                AdStructure::CompleteLocalName("cyberpixie"),
            ])
            .unwrap()
        )
        .await
    );
    println!("{:?}", ble.cmd_set_le_advertise_enable(true).await);

    println!("started advertising");

    let mut srv = AttributeServer::new(&mut ble, &mut gatt_attributes);

    loop {
        srv.do_work().await.expect("oops");
    }
}
