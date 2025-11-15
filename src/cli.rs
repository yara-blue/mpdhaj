use std::path::PathBuf;

#[derive(clap::Parser)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn proxy(&self) -> Option<&str> {
        if let Commands::Proxy { address } = &self.command {
            Some(address)
        } else {
            None
        }
    }
}

#[derive(clap::Subcommand)]
pub(crate) enum Commands {
    /// Forward calls to another mpd server at this address
    /// This is for testing only!
    Proxy {
        address: String,
    },
    Run(RunArgs),
}

#[derive(clap::Parser)]
pub struct RunArgs {
    pub(crate) playlist_dir: PathBuf,
}
