use std::{net::SocketAddr, path::PathBuf};

use cyberpixie_proto::Hertz;
use cyberpixie_std_transport::create_client;
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

            let client = create_client(address).map_err(display_err)?;
            // TODO replace by the full firmware info.
            log::info!("Got {:#?} from the {}", client.device_info, address);
        }
        Commands::AddImage { image_path, address, refresh_rate } => todo!(),
        Commands::ShowImage { index, address } => todo!(),
        Commands::ClearImages { address } => todo!(),
    }

    Ok(())
}
