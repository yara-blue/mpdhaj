use std::sync::{Arc, Mutex};

use color_eyre::Section;
use color_eyre::eyre::{OptionExt, eyre};
use color_eyre::{Result, eyre::Context};
use futures::FutureExt;
use itertools::Itertools;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::task;
use tracing::{debug, info, warn};

use crate::mpd_protocol::{self, PlaybackState, SubSystem, response_format};
use crate::{mpd_protocol::Command, system::System};

pub(crate) async fn handle_clients(system: Arc<std::sync::Mutex<System>>, port: u16) -> Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;

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
                // use eprintln instead of tracing::warn as color_eyre gives
                // us pretty colors that we dont get to see with tracing
                eprintln!("error handling client: {e:?}");
            } else {
                info!("Client disconnected");
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

    while let Some(line) =
        reader.next_line().await.wrap_err("Could not get next line from client")?
    {
        if line == "command_list_ok_begin" {
            handle_command_list(&mut reader, &mut writer, &system, true).await?;
            continue;
        } else if line == "command_list_begin" {
            handle_command_list(&mut reader, &mut writer, &system, false).await?;
            continue;
        }

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
        let mut response = perform_command(command, &system)?;

        response.push_str("OK\n");
        debug!("reply: {response}");
        writer
            .write_all(response.as_bytes())
            .await
            .wrap_err("Failed to write response to client")?;
    }
    Ok(())
}

async fn handle_command_list(
    reader: &mut tokio::io::Lines<impl AsyncBufRead + Unpin>,
    writer: &mut (impl AsyncWrite + 'static + Unpin),
    system: &Arc<Mutex<System>>,
    ack_each_command: bool,
) -> Result<()> {
    debug!("handling command list");
    let mut command_executed = 0;
    loop {
        let line = reader
            .next_line()
            .await
            .wrap_err("Could not get next line from client")?
            .ok_or_eyre("Connection closed before command list ended")?;
        if line == "command_list_end" {
            if ack_each_command {
                for _ in 0..command_executed {
                    acknowledge_cmd_list_entry(writer).await?;
                }
            }
            return acknowledge(writer).await;
        }

        let command = Command::parse(&line)?;
        if matches!(command, Command::Idle(_) | Command::NoIdle) {
            return Err(eyre!("Idle and NoIde are not allowed in command lists"));
        }
        let response = perform_command(command, system)?;
        command_executed += 1;

        debug!("reply: {response}");
        writer
            .write_all(response.as_bytes())
            .await
            .wrap_err("Failed to write response to client")?;
    }
}

#[tracing::instrument(skip_all, fields(sub_systems))]
async fn handle_idle(
    reader: &mut tokio::io::Lines<impl AsyncBufRead + Unpin>,
    writer: &mut (impl AsyncWrite + 'static + Unpin),
    system: &Arc<Mutex<System>>,
    sub_systems: Vec<SubSystem>,
) -> Result<Option<Command>> {
    use futures_concurrency::prelude::*;
    debug!("Entering idle mode");

    let mut rx = system.lock().unwrap().idle(sub_systems);
    #[derive(Debug)]
    enum Potato {
        MpdEvent(Option<SubSystem>),
        NextLine(Result<Option<String>, std::io::Error>),
    }
    let next_line = reader.next_line().map(Potato::NextLine);
    let next_event = rx.recv().map(Potato::MpdEvent);

    Ok(Some(match (next_line, next_event).race().await {
        Potato::MpdEvent(Some(sub_system)) => {
            writer.write_all(response_format::subsystem(sub_system).as_bytes()).await?;
            let Some(line) = reader.next_line().await? else {
                return Ok(None);
            };
            Command::parse(&line)?
        }
        Potato::MpdEvent(None) => unreachable!("System should not drop ever"),
        Potato::NextLine(Ok(Some(line))) => {
            let command = Command::parse(&line)?;
            if let Command::NoIdle = command {
                acknowledge(writer).await?;
                debug!("Waiting for command after idle");
                let Some(line) = reader.next_line().await? else {
                    return Ok(None);
                };
                Command::parse(&line)?
            } else {
                warn!(
                    "bad client, sent something other than noidle after idle. \
                    The client send us: {command:?}"
                );
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

async fn acknowledge(writer: &mut (impl AsyncWrite + 'static + Unpin)) -> Result<()> {
    writer.write_all(b"OK\n").await.wrap_err("Failed to acknowledge cmd client")
}

async fn acknowledge_cmd_list_entry(
    writer: &mut (impl AsyncWrite + 'static + Unpin),
) -> Result<()> {
    writer.write_all(b"list_OK\n").await.wrap_err("Failed to acknowledge cmd list item to client")
}

pub fn perform_command(request: Command, system: &Mutex<System>) -> color_eyre::Result<String> {
    use Command::*;
    let mut system = system.lock().expect("No thread should ever panic");
    Ok(match &request {
        BinaryLimit(_) => String::new(),
        Commands => supported_command_list(),
        Status => {
            response_format::to_string(&system.status()).wrap_err("Failed to get system status")?
        }
        PlaylistInfo(_pos_or_range) => {
            response_format::to_string(&system.queue().wrap_err("Failed to get current queue")?)?
        }
        ListPlayLists => response_format::to_string(&system.playlists())
            .wrap_err("Failed to get list of playlists")?,
        ListPlaylistInfo(playlist_name, _range) => response_format::to_string(
            &system
                .get_playlist(playlist_name)
                .wrap_err("Failed to get playlist")
                .with_note(|| format!("playlist name: {playlist_name:?}"))?,
        )?,
        Clear => {
            system.clear()?;
            system.playing = PlaybackState::Stop;
            response_format::to_string(&system.status())?
        }
        ListAll(dir) => response_format::to_string(
            &system
                .list_all_in(&dir.clone().unwrap_or_default())
                .wrap_err("Failed to list all songs")?,
        )?,
        List(mpd_protocol::List { tag_to_list, query: _query, group_by }) => {
            if !group_by.is_empty() {
                return Err(eyre!("group_by argument in List command not yet supported"));
            }

            system
                .list_tag(tag_to_list)
                .wrap_err("Failed to list tags")
                .with_note(|| format!("Tag type: {tag_to_list}"))?
                .join("\n")
        }
        LsInfo(song) => response_format::to_string(
            &system
                .get_song_by_path(song)
                .wrap_err("Failed to get song info")
                .with_note(|| format!("song path: {song:?}"))?,
        )?,
        Volume(_volume_change) => todo!(),
        Play(_pos) => {
            // TODO: actually play
            system.playing = PlaybackState::Play;
            response_format::to_string(&system.status())?
        }
        Pause(_state) => {
            system.playing = PlaybackState::Pause;
            response_format::to_string(&system.status())?
        }
        Stop => {
            system.playing = PlaybackState::Stop;
            response_format::to_string(&system.status())?
        }
        Next => todo!(),
        Previous => todo!(),
        PlayId(_pos_in_playlist) => todo!(),
        Load(_playlist_name, _range, _position) => todo!(),
        add @ (Add(song, position) | AddId(song, position)) => {
            // TODO: handle add with directory (adds all recursively)
            let id = system
                .add_to_queue(song, position)
                .wrap_err("Failed to add song to queue")
                .with_note(|| format!("song path: {song:?}"))
                .with_note(|| format!("position: {position:?}"))?;
            if matches!(add, Add(..)) { String::new() } else { format!("Id: {}", id.0) }
        }
        Find(query, _sort, _range) => response_format::to_string(
            &system
                .handle_find(query)
                .wrap_err("Failed to handle find")
                .with_note(|| format!("query: {query:?}"))?,
        )?,
        FindAdd(query, _sort, _range, position) => {
            let results = system
                .handle_find(query)
                .wrap_err("Failed to handle find")
                .with_note(|| format!("query: {query:?}"))?;
            for result in results {
                system
                    .add_to_queue(&result.path, position)
                    .wrap_err("Could not add matching song to queue")
                    .with_note(|| format!("song: {result:?}"))?;
            }
            String::new()
        }
        CurrentSong => response_format::to_string(
            &system.current_song().wrap_err("Could not get current song")?,
        )?,
        Stats => todo!(), //{
        //     response_format::to_string(&system.stats().wrap_err("Could not gather statistics")?)?
        // }
        Idle(_) | NoIdle => panic!("These should be handled in the outer loop"),
        Ping => "OK".to_owned(),
        other => unimplemented!("{other:?}"),
    })
}

fn supported_command_list() -> String {
    use strum::VariantNames;
    let mut list = Command::VARIANTS
        .iter()
        .map(|name| name.replace("-", ""))
        .map(|command| format!("command: {command}"))
        .join("\n");
    list.push('\n');
    list
}
