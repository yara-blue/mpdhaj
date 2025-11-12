#[derive(clap::Parser)]
pub(crate) struct Cli {
    /// Forward calls to another mpd server at this address
    #[clap(long)]
    pub(crate) proxy: Option<String>,
}
