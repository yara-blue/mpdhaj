use std::sync::{Arc, Mutex};

use crate::{mpd_protocol::Command, system::System};

pub fn perform_request(request: Command, state: &Mutex<System>) -> Vec<u8> {
    let reply_lines = match request {
        Command::BinaryLimit(_) => Vec::new(),
        Command::Commands => supported_command_list(),
        Command::Status => todo!(),
        Command::PlaylistInfo => todo!(),
        Command::ListPlayLists => todo!(),
        Command::Idle(sub_systems) => todo!(),
        Command::NoIdle => todo!(),
        Command::ListPlaylistInfo(playlist_names) => todo!(),
        Command::PlayId(pos_in_playlist) => todo!(),
        Command::Clear => todo!(),
        Command::Load(playlist_name) => todo!(),
        Command::LsInfo(path_buf) => todo!(),
        Command::Volume(volume_change) => todo!(),
    };
    let reply = if reply_lines.is_empty() {
        "OK\n".to_owned()
    } else {
        let mut reply = reply_lines.join("\n");
        reply.push_str("\nOK\n");
        reply
    };

    eprintln!("reply: {reply}");
    reply.into_bytes()
}

fn supported_command_list() -> Vec<String> {
    use strum::VariantNames;
    Command::VARIANTS
        .into_iter()
        .map(|name| name.replace("-", ""))
        .map(|command| format!("command: {command}"))
        .collect()
}
