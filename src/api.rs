use itertools::Itertools;

use crate::mpd_protocol::response_format;
use std::sync::Mutex;

use crate::{mpd_protocol::Command, system::System};

pub fn perform_request(request: Command, system: &Mutex<System>) -> color_eyre::Result<String> {
    Ok(match request {
        Command::BinaryLimit(_) => String::new(),
        Command::Commands => supported_command_list(),
        Command::Status => response_format::to_string(&system.lock().unwrap().status())?,
        Command::PlaylistInfo => {
            response_format::to_string(&system.lock().unwrap().playlist_info())?
        }
        Command::ListPlayLists => todo!(),
        Command::Idle(sub_systems) => todo!(),
        Command::NoIdle => todo!(),
        Command::ListPlaylistInfo(playlist_names) => todo!(),
        Command::PlayId(pos_in_playlist) => todo!(),
        Command::Clear => todo!(),
        Command::Load(playlist_name) => todo!(),
        Command::LsInfo(path_buf) => todo!(),
        Command::Volume(volume_change) => todo!(),
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
