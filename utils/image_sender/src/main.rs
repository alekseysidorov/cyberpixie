use std::{net::SocketAddr, path::PathBuf};

use cyberpixie_proto::types::Hertz;
use structopt::StructOpt;

use image_sender::{
    convert_image_to_raw, run_transport_example, send_clear_images, send_firmware_info, send_image,
    send_show_image,
};

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
    /// Run transport example.
    #[structopt(name = "run-transport")]
    RunTransport { address: SocketAddr },

    /// Generate completions
    #[structopt(name = "gen-completions")]
    GenCompletions,
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
            send_image(strip_len, refresh_rate, raw, address)?;
        }

        Commands::ShowImage { address, index } => {
            log::info!("Sending show image {} command to {}", index, address);
            send_show_image(index, address)?;
        }

        Commands::ClearImages { address } => {
            log::info!("Sending clear images command to {}", address);
            send_clear_images(address)?;
        }

        Commands::FirmwareInfo { address } => {
            log::info!("Sending firmare info request to {}", address);
            send_firmware_info(address)?;
        }

        Commands::GenCompletions => {
            let _clap = Commands::clap();
            // clap.gen_completions(bin_name, for_shell, out_dir)
            todo!()
        }

        Commands::RunTransport { address } => {
            log::info!("Running transport example for {}", address);
            run_transport_example(address)?;
        }
    }

    Ok(())
}
