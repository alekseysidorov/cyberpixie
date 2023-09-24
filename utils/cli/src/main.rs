use std::{path::PathBuf, time::Duration};

use clap::{CommandFactory, Parser, Subcommand};
use cyberpixie_cli::convert_image_to_raw;
use cyberpixie_network::{
    core::{
        proto::{
            packet::{FromPacket, PackedSize, Packet},
            types::{Hertz, ImageId},
            ResponseHeader,
        },
        Error as CyberpixieError,
    },
    tokio::TokioStack,
    Client, NetworkStack, SocketAddr,
};

/// Cyberpixie device manipulation utility
///
/// A command line application for interacting with the Cyberpixie device via WiFi connection
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = false)]
struct Cli {
    /// Device socket address
    #[arg(short, long, default_value = "192.168.71.1:1800")]
    address: String,
    /// Actual command
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Get information about device firmware
    DeviceInfo,
    /// Add a new image to device memory
    AddImage {
        /// Image path
        #[arg(value_name = "FILE")]
        path: PathBuf,
        /// Refresh rate of the single image line
        #[arg(short, long = "refresh-rate", default_value = "300", value_name = "Hz")]
        refresh_rate: Hertz,
    },
    /// Show image
    Start {
        /// Image index
        image_id: u16,
    },
    /// Hide currently showing image
    Stop,
    /// Clear all images stored in the device memory
    ClearImages,
    /// Generate shell completions
    Completions {
        /// The shell to generate the completions for
        #[arg(value_enum)]
        shell: clap_complete_command::Shell,
    },
    /// Bluetooth low energy test
    BLE,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let address: SocketAddr = cli
        .address
        .parse()
        .map_err(|err| anyhow::anyhow!("{err}"))?;

    let mut stack = TokioStack;
    // Allocate socket.
    let mut socket = stack.socket();
    match cli.command {
        Command::DeviceInfo => {
            log::info!("Sending firmware info request to {}", address);

            let peer_info = Client::connect(&mut socket, address)
                .await?
                .peer_info()
                .await?;
            // TODO replace by the full firmware info.
            log::info!("Got {:#?} from the {}", peer_info, address);
        }

        Command::AddImage { path, refresh_rate } => {
            let (strip_len, raw) = convert_image_to_raw(&path)?;

            log::info!("Sending image {:?}[{}] to {}", path, strip_len, address);
            let index = Client::connect(&mut socket, address)
                .await?
                .add_image(refresh_rate, strip_len as u16, &raw)
                .await?;
            log::info!(
                "Image loaded into the device {} with index {}",
                address,
                index
            );
        }

        Command::Start { image_id } => {
            log::info!("Sending show image command to {address}");
            Client::connect(&mut socket, address)
                .await?
                .start(ImageId(image_id))
                .await?;
            log::info!("Showing image with id {image_id}");
        }

        Command::Stop => {
            log::info!("Sending hide image command to {address}");
            Client::connect(&mut socket, address).await?.stop().await?;
            log::info!("Hide a currently showing image");
        }

        Command::ClearImages => {
            log::info!("Sending clear images command to {address}");

            Client::connect(&mut socket, address)
                .await?
                .clear_images()
                .await?;
            log::trace!("Sent images clear command to {address}");
        }

        Command::Completions { shell } => {
            shell.generate(&mut Cli::command(), &mut std::io::stdout());
        }

        Command::BLE => check_ble().await?,
    }

    Ok(())
}

async fn check_ble() -> anyhow::Result<()> {
    use btleplug::{
        api::{
            bleuuid::uuid_from_u16, Central, Manager as _, Peripheral as _, ScanFilter, WriteType,
        },
        platform::{Adapter, Manager, Peripheral},
    };

    log::info!("Checking bluetooth low energy");

    let manager = btleplug::platform::Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().nth(0).unwrap();

    // start scanning for devices
    central.start_scan(ScanFilter::default()).await?;

    let cyberpixie;
    loop {
        // instead of waiting, you can use central.events() to get a stream which will
        // notify you of new devices, for an example of that see examples/event_driven_discovery.rs
        tokio::time::sleep(Duration::from_secs(3)).await;
        // find the device we're interested in
        if let Some(value) = find_cyberpixie(&central).await {
            cyberpixie = value;
            break;
        }
    }
    log::info!("found {cyberpixie:?}");

    // connect to the device
    cyberpixie.connect().await?;

    // // discover services and characteristics
    cyberpixie.discover_services().await?;

    // find the characteristic we want
    let chars = cyberpixie.characteristics();

    log::info!("Chars: {chars:?}");

    let board_info_char = chars
        .iter()
        .find(|c| c.uuid == "937312e0-2354-11eb-9f10-fbc30a62cf38".parse().unwrap())
        .expect("Unable to find characterics");

    for i in 0..500 {
        let buf = cyberpixie.read(board_info_char).await?;
        // Decode packet
        let packet = Packet::from_bytes(&buf[0..Packet::PACKED_LEN]);
        log::trace!("Got a next packet {packet:?}");

        // Read header
        let header_len = packet.header_len as usize;
        if header_len >= Packet::MAX_LEN {
            return Err(CyberpixieError::Decode.into());
        }

        let buf = &buf[Packet::PACKED_LEN..];
        let header =
            ResponseHeader::from_bytes(&buf[0..header_len]).map_err(CyberpixieError::decode)?;

        log::info!("[{i}] Got info: {header:?}");
    }

    Ok(())
}

async fn find_cyberpixie(
    central: &btleplug::platform::Adapter,
) -> Option<btleplug::platform::Peripheral> {
    use btleplug::api::{Central, Peripheral};

    for p in central.peripherals().await.unwrap() {
        if p.properties()
            .await
            .unwrap()
            .unwrap()
            .local_name
            .iter()
            .any(|name| name.contains("cyberpixie"))
        {
            return Some(p);
        }
    }
    None
}
