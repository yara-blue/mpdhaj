use std::path::PathBuf;

#[derive(clap::Parser)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    /// The port mpdhaj is running or proxying on
    #[clap(default_value_t = 6600)]
    pub(crate) port: u16,
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
    /// Look at the metadata of all the files in the folder and build
    /// and index.
    Scan(RunArgs),
}

#[derive(clap::Parser)]
pub struct RunArgs {
    pub(crate) playlist_dir: PathBuf,
    pub(crate) music_dir: PathBuf,
}
