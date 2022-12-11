use std::{
    net::{TcpListener, TcpStream},
    thread::JoinHandle,
};

use cyberpixie_proto::ng::{DeviceInfo, DeviceRole};
use cyberpixie_std_transport::ng::{Client, NetworkPart, SimpleDevice};

const CLIENT_INFO: DeviceInfo = DeviceInfo {
    role: DeviceRole::Client,
    group_id: None,
    strip_len: 36,
};

fn create_loopback<D>(device: D) -> anyhow::Result<(Client, JoinHandle<()>)>
where
    D: SimpleDevice + Send + 'static,
{
    let _ = env_logger::try_init();

    let listener = TcpListener::bind("0.0.0.0:0")?;
    let addr = listener.local_addr()?;

    let mut server = NetworkPart::new(device, listener)?;
    let handle = std::thread::spawn(move || loop {
        if let Err(nb::Error::Other(err)) = server.poll() {
            panic!("{err}");
        }
    });

    let client = Client::connect(CLIENT_INFO, TcpStream::connect(addr)?)?;
    Ok((client, handle))
}

struct DeviceStub;

impl SimpleDevice for DeviceStub {
    fn device_info(&self) -> DeviceInfo {
        DeviceInfo {
            role: DeviceRole::Main,
            group_id: None,
            strip_len: 36,
        }
    }
}

#[test]
fn test_simple_handshake() {
    let (mut client, _device) = create_loopback(DeviceStub).unwrap();
    assert_eq!(
        client.device_info,
        DeviceInfo {
            role: DeviceRole::Main,
            group_id: None,
            strip_len: 36,
        }
    );

    let info = client.resend_handshake().unwrap();
    assert_eq!(info, client.device_info);

    client.send_debug("Hello debug").unwrap();
    client.send_debug("Hello debug 2").unwrap();
}
