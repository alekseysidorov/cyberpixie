use std::{net::SocketAddr, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand};
use cyberpixie_core::proto::types::Hertz;
use cyberpixie_std_network::create_client;
use image_sender::{convert_image_to_raw, display_err};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = false)]
struct Cli {
    /// Device socket address
    #[arg(short, long, default_value = "127.0.0.1:80")]
    address: SocketAddr,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Get firmware info from the device.
    FirmwareInfo,
    /// Add image to device.
    AddImage {
        /// Image path
        #[arg(value_name = "FILE")]
        path: PathBuf,
        /// Image refresh rate
        #[arg(short, long = "refresh-rate", default_value = "50", value_name = "Hz")]
        refresh_rate: Hertz,
    },
    /// Show image.
    ShowImage {
        /// Image index.
        index: usize,
    },
    /// Clear device images.
    ClearImages,
    /// Generate shell completions
    Completions {
        /// The shell to generate the completions for.
        #[arg(value_enum)]
        shell: clap_complete_command::Shell,
    },
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let address = cli.address;
    match cli.command {
        Commands::FirmwareInfo => {
            log::info!("Sending firmare info request to {}", address);

            let client = create_client(address)?;
            // TODO replace by the full firmware info.
            log::info!("Got {:#?} from the {}", client.device_info, address);
        }
        Commands::AddImage { path, refresh_rate } => {
            let (strip_len, raw) = convert_image_to_raw(&path)?;

            log::info!("Sending image {:?}[{}] to {}", path, strip_len, address);
            let index = create_client(address)?.add_image(refresh_rate, strip_len as u16, &raw)?;
            log::info!(
                "Image loaded into the device {} with index {}",
                address,
                index
            );
        }
        Commands::ShowImage { index } => {}
        Commands::ClearImages => {
            log::info!("Sending clear images command to {}", address);

            create_client(address)?.clear_images()?;
            log::trace!("Sent images clear command to {}", address);
        }

        Commands::Completions { shell } => {
            shell.generate(&mut Cli::command(), &mut std::io::stdout());
        }
    }

    Ok(())
}
