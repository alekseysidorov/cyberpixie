use std::{net::SocketAddr, path::PathBuf};

use cyberpixie_proto::types::Hertz;
use cyberpixie_std_network::create_client;
use image_sender::{convert_image_to_raw, display_err};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum Commands {
    /// Send info command to the device.
    #[structopt(name = "info")]
    FirmwareInfo { address: SocketAddr },
    /// Send image to the device.
    #[structopt(name = "add")]
    AddImage {
        #[structopt(name = "image-file")]
        image_path: PathBuf,
        address: SocketAddr,
        #[structopt(short, long = "refresh-rate", default_value = "50")]
        refresh_rate: Hertz,
    },
    /// Send show image command to the device.
    #[structopt(name = "show")]
    ShowImage {
        /// Image index.
        index: usize,
        address: SocketAddr,
    },
    /// Send clear images command to the device.
    #[structopt(name = "clear")]
    ClearImages { address: SocketAddr },
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let opts = Commands::from_args();
    match opts {
        Commands::FirmwareInfo { address } => {
            log::info!("Sending firmare info request to {}", address);

            let client = create_client(address)?;
            // TODO replace by the full firmware info.
            log::info!("Got {:#?} from the {}", client.device_info, address);
        }
        Commands::AddImage {
            image_path,
            address,
            refresh_rate,
        } => {
            let (strip_len, raw) = convert_image_to_raw(&image_path)?;

            log::info!(
                "Sending image {:?}[{}] to {}",
                image_path,
                strip_len,
                address
            );

            let index = create_client(address)?.add_image(refresh_rate, strip_len as u16, &raw)?;

            log::info!(
                "Image loaded into the device {} with index {}",
                address,
                index
            );
        }
        Commands::ShowImage { index, address } => {}
        Commands::ClearImages { address } => {
            log::info!("Sending clear images command to {}", address);

            create_client(address)?.clear_images()?;
            log::trace!("Sent images clear command to {}", address);
        }
    }

    Ok(())
}
