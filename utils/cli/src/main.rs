use std::{net::SocketAddr, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand};
use cyberpixie_core::proto::types::Hertz;
use cyberpixie_std_network::create_client;
use cyberpixie_cli::convert_image_to_raw;

/// Cyberpixie device manipulation utility
///
/// A command line application for interacting with the Cyberpixie device via WiFi connection
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = false)]
struct Cli {
    /// Device socket address
    #[arg(short, long, default_value = "127.0.0.1:80")]
    address: SocketAddr,
    /// Actual command
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Get information about device firmware
    FirmwareInfo,
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
    ShowImage {
        /// Image index
        index: usize,
    },
    /// Clear all images stored in the device memory
    ClearImages,
    /// Generate shell completions
    Completions {
        /// The shell to generate the completions for
        #[arg(value_enum)]
        shell: clap_complete_command::Shell,
    },
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let address = cli.address;
    match cli.command {
        Command::FirmwareInfo => {
            log::info!("Sending firmare info request to {}", address);

            let client = create_client(address)?;
            // TODO replace by the full firmware info.
            log::info!("Got {:#?} from the {}", client.device_info, address);
        }
        Command::AddImage { path, refresh_rate } => {
            let (strip_len, raw) = convert_image_to_raw(&path)?;

            log::info!("Sending image {:?}[{}] to {}", path, strip_len, address);
            let index = create_client(address)?.add_image(refresh_rate, strip_len as u16, &raw)?;
            log::info!(
                "Image loaded into the device {} with index {}",
                address,
                index
            );
        }
        Command::ShowImage { .. } => {}
        Command::ClearImages => {
            log::info!("Sending clear images command to {}", address);

            create_client(address)?.clear_images()?;
            log::trace!("Sent images clear command to {}", address);
        }

        Command::Completions { shell } => {
            shell.generate(&mut Cli::command(), &mut std::io::stdout());
        }
    }

    Ok(())
}
