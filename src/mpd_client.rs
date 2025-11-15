use std::sync::{Arc, Mutex};

use color_eyre::{Result, eyre::Context};
use futures::FutureExt;
use itertools::Itertools;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::task;
use tracing::{info, warn};

use crate::mpd_protocol::{self, SubSystem, response_format};
use crate::{mpd_protocol::Command, system::System};

pub(crate) async fn handle_clients(system: Arc<std::sync::Mutex<System>>) -> Result<()> {
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
        let command = if let Command::Idle(sub_systems) = command {
            let Some(command_after_idle) =
                handle_idle(&mut reader, &mut writer, &system, sub_systems).await?
            else {
                return Ok(());
            };
            command_after_idle
        } else {
            command
        };
        info!("parsed request: {command:?}");
        let mut response = perform_command(command, &system)?;

        let response = if response.is_empty() {
            "OK\n".to_owned()
        } else {
            response.push_str("\nOK\n");
            response
        };
        eprintln!("reply: {response}");
        writer
            .write_all(response.as_bytes())
            .await
            .wrap_err("Failed to write response to client")?;
    }
    Ok(())
}

async fn handle_idle(
    reader: &mut tokio::io::Lines<impl AsyncBufRead + Unpin>,
    writer: &mut (impl AsyncWrite + 'static + Unpin),
    system: &Arc<Mutex<System>>,
    sub_systems: Vec<SubSystem>,
) -> Result<Option<Command>> {
    use futures_concurrency::prelude::*;

    let mut rx = system.lock().unwrap().idle(sub_systems);
    enum Potato {
        MpdEvent(Option<SubSystem>),
        NextLine(Result<Option<String>, std::io::Error>),
    }
    let next_line = reader.next_line().map(Potato::NextLine);
    let next_event = rx.recv().map(Potato::MpdEvent);

    Ok(Some(match (next_line, next_event).race().await {
        Potato::MpdEvent(Some(sub_system)) => {
            writer
                .write_all(response_format::subsystem(sub_system).as_bytes())
                .await?;
            let Some(line) = reader.next_line().await? else {
                return Ok(None);
            };
            Command::parse(&line)?
        }
        Potato::MpdEvent(None) => unreachable!("System should not drop ever"),
        Potato::NextLine(Ok(Some(line))) => {
            let command = Command::parse(&line)?;
            if let Command::NoIdle = command {
                let Some(line) = reader.next_line().await? else {
                    return Ok(None);
                };
                Command::parse(&line)?
            } else {
                warn!("bad client, sent something other than noidle after idle");
                command
            }
        }
        Potato::NextLine(Ok(None)) => {
            info!("client closed connection");
            return Ok(None);
        }
        Potato::NextLine(Err(e)) => Err(e).wrap_err("Could not get next line from client")?,
    }))
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
