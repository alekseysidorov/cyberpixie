use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};
use cyberpixie_cli::convert_image_to_raw;
use cyberpixie_network::{
    core::proto::types::{Hertz, ImageId},
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
    }

    Ok(())
}
