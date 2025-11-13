mod command_format;
mod response_format;

use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

use color_eyre::eyre::Context;
use rodio::{ChannelCount, SampleRate};
use serde::{Deserialize, Serialize};
use strum::EnumString;

pub const VERSION: &'static str = "0.24.4";

#[derive(Debug, Deserialize, PartialEq, Eq)]
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
}

impl Command {
    pub(crate) fn parse(line: &str) -> color_eyre::Result<Self> {
        command_format::from_str(line).wrap_err("Could not deserialize line")
    }
}

struct PlaylistList(Vec<PlayList>);

struct PlayList {
    playlist: String,
    last_modified: SystemTime,
}

#[derive(Debug, Serialize)]
struct PlaylistId(u32);

#[derive(Debug, Serialize)]
struct Volume(u8);

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

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct PlaylistName(String);

#[derive(Debug, Serialize)]
struct SongId(u32);
#[derive(Debug, Serialize)]
struct SongNumber(u32);

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PosInPlaylist(u32);
#[derive(Debug, Serialize)]
struct IdInPlaylist(u32);

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum State {
    Play,
    Pause,
    Stop,
}

// custom serialize as: samplerate:bits:channels
#[derive(Debug, Serialize)]
struct AudioParams {
    samplerate: SampleRate,
    bits: usize,
    channels: ChannelCount,
}

#[derive(Serialize)]
pub struct PlaylistInfo(Vec<PlaylistEntry>);

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaylistEntry {
    #[serde(rename = "file")]
    file: PathBuf,
    #[serde(rename = "Last-Modified")]
    last_modified: jiff::Timestamp, // as 2025-06-15T22:06:58Z
    added: jiff::Timestamp,         // as 2025-06-15T22:06:58Z
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
    id: IdInPlaylist,
}

#[derive(Serialize, Debug)]
struct Status {
    repeat: bool,
    random: bool,
    single: bool,
    consume: bool,
    /// Name of the current partition
    ///
    /// A partition is one frontend of a multi-player MPD process: it has
    /// separate queue, player and outputs. A client is assigned to one
    /// partition at a time.
    ///
    /// We do not support this
    partition: String,
    volume: Volume,
    playlist: PlaylistId,
    playlistlength: usize,
    state: State,
    lastloadedplaylist: Option<PlaylistName>,
    #[serde(serialize_with = "response_format::duration_seconds")]
    xfade: Duration,
    song: SongNumber,
    songid: SongId,
    #[serde(serialize_with = "response_format::duration_millis_precise")]
    elapsed: Duration,
    bitrate: usize,
    /// Duration of the current song in seconds
    #[serde(serialize_with = "response_format::duration_millis_precise")]
    duration: Duration,
    #[serde(serialize_with = "response_format::audio_params")]
    audio: AudioParams,
    error: String,
    nextsong: SongNumber,
    nextsongid: SongId,
}
