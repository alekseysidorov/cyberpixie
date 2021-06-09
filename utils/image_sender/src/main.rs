use std::{net::SocketAddr, path::PathBuf};

use cyberpixie_proto::Hertz;
use structopt::StructOpt;

use image_sender::{convert_image_to_raw, create_service, display_err};

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
        Commands::AddImage {
            address,
            image_path,
            refresh_rate,
        } => {
            log::info!("Sending image {:?} to {}", image_path, address);

            let (strip_len, raw) = convert_image_to_raw(image_path)?;

            let index = create_service(address)?
                .add_image(address, refresh_rate, strip_len, raw.into_iter())?
                .map_err(display_err)?;
            log::info!(
                "Image loaded into the device {} with index {}",
                address,
                index
            );
        }

        Commands::ShowImage { address, index } => {
            log::info!("Sending show image {} command to {}", index, address);

            create_service(address)?
                .show_image(address, index)?
                .map_err(display_err)?;
            log::trace!("Showing image at {} on device {}", index, address);
        }

        Commands::ClearImages { address } => {
            log::info!("Sending clear images command to {}", address);

            create_service(address)?
                .clear_images(address)?
                .map_err(display_err)?;
            log::trace!("Sent images clear command to {}", address);
        }

        Commands::FirmwareInfo { address } => {
            log::info!("Sending firmare info request to {}", address);

            let info = create_service(address)?
                .request_firmware_info(address)?
                .map_err(display_err)?;
            log::info!("Got {:#?} from the {}", info, address);
        }
    }

    Ok(())
}
