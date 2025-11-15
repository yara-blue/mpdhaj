use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use color_eyre::Result;
use color_eyre::eyre::Context;
use rodio::nz;

use crate::mpd_protocol::{
    self, AudioParams, PlaybackState, PlaylistId, SongId, SongNumber, Volume,
};
use crate::playlist::{self, PlaylistName};

#[dbstruct::dbstruct(db=sled)]
pub struct State {
    #[dbstruct(Default = "PlaybackState::Stop")]
    playing: PlaybackState,
    // queue: Vec<Song>,
}

pub struct System {
    state: State,
    playlists: HashMap<PlaylistName, Vec<PathBuf>>,
}

impl System {
    pub fn new(playlist_dir: &Path) -> Result<Self> {
        let state = State::open_path("mpdhaj_database").wrap_err("Could not open db")?;
        let playlists =
            playlist::load_from_dir(playlist_dir).wrap_err("Failed to load playlists")?;
        Ok(System { state, playlists })
    }

    pub fn status(&self) -> mpd_protocol::Status {
        mpd_protocol::Status {
            repeat: false,
            random: true,
            single: false,
            consume: true,
            partition: "default".to_string(),
            volume: Volume::new(50),
            playlist: PlaylistId(22),
            playlistlength: 0,
            state: PlaybackState::Stop,
            lastloadedplaylist: None,
            xfade: Duration::from_secs(5),
            song: SongNumber(5),
            songid: SongId(5),
            elapsed: Duration::from_secs(2),
            bitrate: 320_000,
            duration: Duration::from_secs(320),
            audio: AudioParams {
                samplerate: nz!(44100),
                bits: 24,
                channels: nz!(2)
            },
            error: "Failed to open \"usb dac attached to pi\" (alsa); Failed to open ALSA device \"hw:CARD=UD110v2,DEV=1\": No such device".to_string(),
            nextsong: SongNumber(1),
            nextsongid: SongId(1),
        }
    }
}
