pub mod command_format;
pub mod response_format;

use std::{path::PathBuf, time::Duration};

use color_eyre::{Section, eyre::Context};
use jiff::Timestamp;
use rodio::{ChannelCount, SampleRate, nz};
use serde::{Deserialize, Serialize};
use strum::EnumString;
use tracing::instrument;

use crate::playlist::PlaylistName;

pub const VERSION: &'static str = "0.24.4";

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, strum::EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum SubSystem {
    /// the song database has been modified after update.
    Database,
    /// a database update has started or finished. If the database was modified during the update, the database event is also emitted.
    Update,
    /// a stored playlist has been modified, renamed, created or deleted
    StoredPlaylist,
    /// the queue (i.e. the current playlist) has been modified
    Playlist,
    /// the player has been started, stopped or seeked or tags of the currently playing song have changed (e.g. received from stream)
    Player,
    /// the volume has been changed
    Mixer,
    /// an audio output has been added, removed or modified (e.g. renamed, enabled or disabled)
    Output,
    /// options like repeat, random, crossfade, replay gain
    Options,
    /// a partition was added, removed or changed
    Partition,
    /// the sticker database has been modified.
    Sticker,
    /// a client has subscribed or unsubscribed to a channel
    Subscription,
    /// a message was received on a channel this client is subscribed to; this event is only emitted when the client’s message queue is empty
    Message,
    /// a neighbor was found or lost
    Neighbor,
    /// the mount list has changed
    Mount,
}

#[derive(Debug, Deserialize, EnumString, strum_macros::VariantNames, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Command {
    BinaryLimit(usize),
    Commands,
    Status,
    PlaylistInfo,
    ListPlayLists,
    Idle(Vec<SubSystem>),
    NoIdle,
    ListPlaylistInfo(PlaylistName),
    PlayId(PosInPlaylist),
    /// Remove all items from the Queue
    Clear,
    Load(PlaylistName),
    /// Mpd supports URI's here we only play files though so we use a path.
    LsInfo(PathBuf),
    Volume(VolumeChange),
    /// Unpause
    Play,
    /// Add an item to the queue
    Add(PathBuf),
    List(List),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct List {
    // NOTE we can not parse mpd filters yet
    pub tag_to_list: Tag,
    pub group_by: Vec<Tag>,
}

#[derive(
    Debug, Default, Deserialize, Serialize, strum_macros::Display, PartialEq, Eq,
)]
// #[serde(rename_all = "lowercase")]
pub enum Tag {
    #[default]
    Album,
    AlbumArtist,
    Artist,
}

impl Command {
    #[instrument(level = "debug", ret)]
    pub(crate) fn parse(line: &str) -> color_eyre::Result<Self> {
        command_format::from_str(line)
            .wrap_err("Could not deserialize line")
            .with_note(|| format!("line was: {line}"))
    }
}

#[derive(Debug, Serialize)]
pub struct PlaylistList(pub Vec<PlayList>);

#[derive(Debug, Serialize)]
pub struct PlayList {
    playlist: PlaylistName,
    last_modified: jiff::Timestamp,
}
impl PlayList {
    pub(crate) fn from_name(name: PlaylistName) -> PlayList {
        PlayList {
            playlist: name,
            last_modified: jiff::Timestamp::new(42, 42).unwrap(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PlaylistId(pub u32);

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct VolumeChange(pub i8);

#[derive(Debug, Serialize)]
pub struct Volume(u8);

impl Volume {
    pub fn new(val: u8) -> Self {
        if (0..=100).contains(&val) {
            Self(val)
        } else {
            panic!("Volume value must be between 0 and 101")
        }
    }
    pub fn get(&self) -> u8 {
        self.0
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SongId(pub u32);
#[derive(Debug, Serialize)]
pub struct SongNumber(pub u32);

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PosInPlaylist(u32);

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Play,
    Pause,
    Stop,
}

// custom serialize as: samplerate:bits:channels
#[derive(Debug, Serialize)]
pub struct AudioParams {
    pub samplerate: SampleRate,
    pub bits: usize,
    pub channels: ChannelCount,
}

#[derive(Serialize)]
pub struct PlaylistInfo(pub Vec<PlaylistEntry>);

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaylistEntry {
    #[serde(rename = "file")]
    file: PathBuf,
    #[serde(rename = "Last-Modified")]
    last_modified: jiff::Timestamp, // as 2025-06-15T22:06:58Z
    added: jiff::Timestamp, // as 2025-06-15T22:06:58Z
    #[serde(serialize_with = "response_format::audio_params")]
    format: AudioParams,
    artist: String,
    album_artist: String,
    /// the song title
    title: String,
    album: String,
    /// the decimal track number within the album.
    track: usize,
    /// Release date usually 4 digit year
    date: String,
    /// the music genre
    genre: Option<String>,
    /// the name of the label or publisher
    label: String,
    disc: Option<usize>,
    #[serde(serialize_with = "response_format::duration_millis_precise")]
    #[serde(rename = "duration")]
    duration: Duration,
    pos: PosInPlaylist,
    id: SongId,
}

impl PlaylistEntry {
    /// almost all fields are todo!
    pub fn mostly_fake(pos: usize, id: SongId, song: crate::system::Song) -> Self {
        Self {
            file: song.file,
            last_modified: Timestamp::constant(0, 0),
            added: Timestamp::constant(0, 0),
            format: AudioParams {
                samplerate: nz!(42),
                bits: 16,
                channels: nz!(42),
            },
            artist: song.artist,
            album_artist: "todo".to_string(),
            title: song.title,
            album: "todo".to_string(),
            track: 42,
            date: "todo".to_string(),
            genre: None,
            label: "todo".to_string(),
            disc: None,
            duration: Duration::ZERO,
            pos: PosInPlaylist(
                pos.try_into()
                    .expect("You should not have 4 billion soungs"),
            ),
            id,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Status {
    pub repeat: bool,
    pub random: bool,
    pub single: bool,
    pub consume: bool,
    /// Name of the current partition
    ///
    /// A partition is one frontend of a multi-player MPD process: it has
    /// separate queue, player and outputs. A client is assigned to one
    /// partition at a time.
    ///
    /// We do not support this
    pub partition: String,
    pub volume: Volume,
    pub playlist: PlaylistId,
    pub playlistlength: usize,
    pub state: PlaybackState,
    pub lastloadedplaylist: Option<PlaylistName>,
    #[serde(serialize_with = "response_format::duration_seconds")]
    pub xfade: Duration,
    pub song: SongNumber,
    pub songid: SongId,
    #[serde(serialize_with = "response_format::duration_millis_precise")]
    pub elapsed: Duration,
    pub bitrate: usize,
    /// Duration of the current song in seconds
    #[serde(serialize_with = "response_format::duration_millis_precise")]
    pub duration: Duration,
    #[serde(serialize_with = "response_format::audio_params")]
    pub audio: AudioParams,
    pub error: String,
    pub nextsong: SongNumber,
    pub nextsongid: SongId,
}
