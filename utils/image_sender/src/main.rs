use std::{net::SocketAddr, path::PathBuf};

use cyberpixie_proto::types::Hertz;
use structopt::StructOpt;

use image_sender::{convert_image_to_raw, send_clear_images, send_image};

#[derive(Debug, StructOpt)]
enum Commands {
    /// Send image to the device.
    #[structopt(name = "send")]
    Send {
        #[structopt(name = "image-file")]
        image_path: PathBuf,
        address: SocketAddr,
        #[structopt(short, long, default_value = "24")]
        strip_len: u16,
        #[structopt(short, long = "refresh-rate", default_value = "50")]
        refresh_rate: Hertz,
    },
    /// Send clear images command to the device.
    #[structopt(name = "clear")]
    ClearImages { address: SocketAddr },
    /// Generate completions
    #[structopt(name = "gen-completions")]
    GenCompletions,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let opts = Commands::from_args();
    match opts {
        Commands::Send {
            address,
            image_path,
            strip_len,
            refresh_rate,
        } => {
            log::info!("Sending image {:?} to {}", image_path, address);

            let raw = convert_image_to_raw(image_path)?;
            send_image(strip_len, refresh_rate, raw, address)?;
        }

        Commands::ClearImages { address } => {
            log::info!("Sending clear images command to {}", address);
            send_clear_images(address)?;
        }

        Commands::GenCompletions => {
            let _clap = Commands::clap();
            // clap.gen_completions(bin_name, for_shell, out_dir)
            todo!()
        }
    }

    Ok(())
}
