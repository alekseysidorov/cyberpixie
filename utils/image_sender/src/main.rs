use std::{fs::File, io::BufReader, net::SocketAddr, path::PathBuf};

use structopt::StructOpt;

use image_sender::{convert_image_to_raw, send_image};

#[derive(Debug, StructOpt)]
enum Commands {
    /// Resize and send image to the device.
    #[structopt(name = "send")]
    Send {
        host: SocketAddr,
        #[structopt(name = "image-file")]
        image_path: PathBuf,
        #[structopt(short, long, default_value = "24")]
        strip_len: u16,
        #[structopt(short, long = "refresh-rate", default_value = "50")]
        refresh_rate: u32,
    },
    /// Generate completions
    #[structopt(name = "gen-completions")]
    GenCompletions,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let opts = Commands::from_args();
    match opts {
        Commands::Send {
            host,
            image_path,
            strip_len,
            refresh_rate,
        } => {
            let file = File::open(image_path)?;
            let reader = BufReader::new(file);

            let raw = convert_image_to_raw(reader)?;
            send_image(strip_len, refresh_rate, raw, host)?;
        }

        Commands::GenCompletions => {
            let _clap = Commands::clap();
            // clap.gen_completions(bin_name, for_shell, out_dir)
            todo!()
        }
    }

    Ok(())
}
