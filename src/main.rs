use std::sync::{Arc, Mutex};

use clap::Parser;
use color_eyre::{Result, eyre::Context};
use itertools::Itertools;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::task;
use tracing::info;

use crate::{
    cli::{Cli, Commands},
    mpd_protocol::{Command, response_format},
    system::System,
};

mod cli;
mod mpd_protocol;
mod playlist;
mod proxy;
mod system;

#[tokio::main(flavor = "local")]
async fn main() -> Result<()> {
    color_eyre::install().unwrap();
    setup_tracing();

    let options = Cli::parse();

    match options.command {
        Commands::Proxy { address } => proxy::handle_clients(options.port, &address).await?,
        Commands::Run(args) => {
            let system = Arc::new(Mutex::new(
                System::new(&args.playlist_dir).wrap_err("Could not start system")?,
            ));
            handle_clients(system).await?;
        }
    };

    Ok(())
}

async fn handle_clients(system: Arc<std::sync::Mutex<System>>) -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:6600").await?;

    loop {
        let stream = match listener.accept().await {
            Ok((stream, _addr)) => stream,
            Err(e) => return Err(e).wrap_err("Could not accept connection"),
        };
        let (reader, writer) = tokio::io::split(stream);
        let reader = BufReader::new(reader).lines();
        let system = Arc::clone(&system);
        task::spawn(async move {
            if let Err(e) = handle_client(reader, writer, system).await {
                info!("error handling client: {e:?}");
            }
        });
    }
}

async fn handle_client(
    mut reader: tokio::io::Lines<impl AsyncBufRead + Unpin>,
    mut writer: impl AsyncWrite + Send + 'static + Unpin + Send,
    system: Arc<std::sync::Mutex<System>>,
) -> Result<()> {
    writer
        .write_all(format!("OK MPD {}\n", mpd_protocol::VERSION).as_bytes())
        .await
        .wrap_err("Could not send handshake to client")?;

    while let Some(line) = reader
        .next_line()
        .await
        .wrap_err("Could not get next line from client")?
    {
        let command = Command::parse(&line)?;
        if let Command::Idle(sub_systems) = &command {
            // wait for next line if noidle ok else bad client send bad
            // &
            // rx.recv()

            // loop {
            //     if let Some() reader.next()
            //         rx.recv_timeout()
            // }

            system.lock().unwrap().idle(sub_systems);
            // Command::NoIdle => todo!(),
        }
        info!("parsed request: {command:?}");
        let mut response = perform_command(command, &system)?;

        let response = if response.is_empty() {
            "OK\n".to_owned()
        } else {
            response.push_str("\nOK\n");
            response
        };
        info!("reply: {response}");
        writer
            .write_all(response.as_bytes())
            .await
            .wrap_err("Failed to write response to client")?;
    }
    Ok(())
}

pub fn perform_command(request: Command, system: &Mutex<System>) -> color_eyre::Result<String> {
    Ok(match request {
        Command::BinaryLimit(_) => String::new(),
        Command::Commands => supported_command_list(),
        Command::Status => response_format::to_string(&system.lock().unwrap().status())?,
        Command::PlaylistInfo => response_format::to_string(&system.lock().unwrap().queue())?,
        Command::ListPlayLists => response_format::to_string(&system.lock().unwrap().playlists())?,
        Command::ListPlaylistInfo(playlist_names) => todo!(),
        Command::PlayId(pos_in_playlist) => todo!(),
        Command::Clear => todo!(),
        Command::Load(playlist_name) => todo!(),
        Command::LsInfo(path_buf) => todo!(),
        Command::Volume(volume_change) => todo!(),
        Command::Idle(_) | Command::NoIdle => panic!("These should be handled in the outer loop"),
    })
}

fn supported_command_list() -> String {
    use strum::VariantNames;
    Command::VARIANTS
        .into_iter()
        .map(|name| name.replace("-", ""))
        .map(|command| format!("command: {command}"))
        .join("\n")
}

pub fn setup_tracing() {
    use tracing_subscriber::filter;
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;

    let filter = filter::EnvFilter::builder().from_env().unwrap();
    let fmt = fmt::layer().pretty().with_line_number(true);

    let _ignore_err = tracing_subscriber::registry()
        .with(fmt)
        .with(filter)
        .try_init();
}
