use std::sync::{Arc, Mutex};

use clap::Parser;
use color_eyre::{Result, eyre::Context};
use tracing_subscriber::fmt::format::FmtSpan;

use crate::{
    cli::{Cli, Commands},
    system::System,
};

mod cli;
mod mpd_client;
mod mpd_protocol;
mod playlist;
mod proxy;
mod scan;
mod system;

/// pub so doctests work
pub mod util;

#[allow(unexpected_cfgs)]
#[tokio::main(flavor = "local")]
async fn main() -> Result<()> {
    color_eyre::install().unwrap();
    setup_tracing();

    let options = Cli::parse();

    match options.command {
        Commands::Proxy { address } => proxy::handle_clients(options.port, &address).await?,
        Commands::Run(args) => {
            let system = Arc::new(Mutex::new({
                let mut s = System::new(args.music_dir, args.playlist_dir)
                    .wrap_err("Could not start system")?;
                s.rescan().await?;
                // s.add_to_queue(
                //     "0-singles/Good Kid - Mimi's Delivery Service.opus".into(),
                //     &None,
                // )?;
                // s.add_to_queue("0-singles/underscores - Music.ogg".into(), &None)?;
                s
            }));
            mpd_client::handle_clients(system, options.port).await?;
        }
        Commands::Scan(args) => {
            let mut system = System::new(args.music_dir, args.playlist_dir)
                .wrap_err("Could not start system")?;
            system.rescan().await?
        }
    };

    Ok(())
}

pub(crate) fn setup_tracing() {
    use tracing_subscriber::filter;
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;

    let filter = filter::EnvFilter::builder().from_env().unwrap();
    let fmt = fmt::layer().pretty().with_line_number(true).with_span_events(FmtSpan::CLOSE);

    let _ignore_err = tracing_subscriber::registry().with(fmt).with(filter).try_init();
}
