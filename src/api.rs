use std::sync::{Arc, Mutex};

use crate::{protocol::Command, system::System};

pub fn perform_request(request: Command, state: &Mutex<System>) -> Vec<u8> {
    let lines = match request {
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
    };
    todo!()
}

fn supported_command_list() -> Vec<String> {
    use strum::VariantNames;
    Command::VARIANTS
        .into_iter()
        .map(|name| name.replace("-", ""))
        .map(|command| format!("command: {command}\n"))
        .collect()
}
