use std::fmt::Display;
use clap::{Parser, ValueEnum};
use xshell::{cmd, Shell};

#[derive(ValueEnum, Copy, Clone, PartialEq, Eq, Debug)]
#[value(rename_all = "lower")]
enum SupportedBoards {
    Esp32C3,
    Esp32S3,
}

impl Display for SupportedBoards {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupportedBoards::Esp32C3 => f.write_str("esp32c3"),
            SupportedBoards::Esp32S3 => f.write_str("esp32s3"),
        }
    }
}

impl SupportedBoards {
    fn working_directory(&self) -> String {
        match self {
            Self::Esp32C3 => "boards/esp32/esp32c3",
            Self::Esp32S3 => "boards/esp32/esp32s3",
        }
        .to_owned()
    }
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = false)]
struct Cli {
    board: SupportedBoards,
    #[clap(allow_hyphen_values = true)]
    cargo: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    let sh = Shell::new()?;
    // Go to the directory where board is situated.
    {
        let _d = sh.push_dir(cli.board.working_directory());
        let nix_command = format!("cargo {}", cli.cargo.join(" "));
        cmd!(sh, "nix-shell --run {nix_command}").run()?;
    }

    Ok(())
}
