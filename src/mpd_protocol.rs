// pub mod command_format;
pub mod command_parser;
pub mod query;
pub mod response_format;

use std::time::Duration;

use camino::Utf8PathBuf;
use jiff::Timestamp;
use rodio::{ChannelCount, SampleRate, nz};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString, VariantNames};
use tracing::instrument;

use crate::{mpd_protocol::query::Query, playlist::PlaylistName};

pub const VERSION: &str = "0.24.4";

// TODO: in general these should be using URIs instead of Utf8PathBuf

/// see <https://mpd.readthedocs.io/en/stable/protocol.html#command-reference>
#[derive(Debug, Default, VariantNames, EnumString, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum Command {
    // Query Status:
    ClearError,
    CurrentSong,
    Idle(Vec<SubSystem>),
    NoIdle,
    #[default]
    Status,
    Stats,

    // Playback Options:
    Consume(ConsumeState),
    Crossfade(u32), // seconds
    MixRampDB(f32),
    MixRampDelay(u32), // seconds
    Random(bool),
    Repeat(bool),
    SetVol(i8),
    GetVol,
    Single(bool),
    ReplayGainMode(ReplayGainMode),
    ReplayGainStatus,
    Volume(VolumeChange),

    // Control Playback:
    Next,
    Pause(Option<bool>), // 1 = pause, 0 = resume, None = toggle
    Play(Option<QueuePos>),
    PlayId(Option<QueueId>), // weird that this is optional
    Previous,
    Seek(QueuePos, f32),
    SeekId(QueueId, f32),
    SeekCur(TimeOrOffset),
    Stop,

    // Manipulate the Queue:
    /// Add an item to the queue
    Add(Utf8PathBuf, Option<Position>),
    AddId(Utf8PathBuf, Option<Position>),
    /// Remove all items from the Queue
    Clear,
    Delete(Option<PosOrRange>),
    DeleteId(QueueId),
    Move(Option<PosOrRange>, Position),
    MoveId(QueueId, Position),
    Playlist, // deprecated
    PlaylistFind(Query, Option<Sort>, Option<Range>),
    PlaylistId(Option<QueueId>),
    PlaylistInfo(Option<PosOrRange>),
    PlaylistSearch(Query, Option<Sort>, Option<Range>),
    PlChanges(u32, Option<Range>),
    PlChangesPosId(u32, Option<Range>),
    Prio(u8, Vec<Range>),
    PrioId(u8, Vec<QueueId>),
    RangeId(QueueId, Option<FloatRange>),
    Shuffle(Option<Range>),
    Swap(QueuePos, QueuePos), // TODO: can these be relative?
    SwapId(QueueId, QueueId),
    AddTagId(QueueId, Tag, String),
    ClearTagId(QueueId, Tag),

    // Manipulate Playlists:
    ListPlaylist(PlaylistName, Option<Range>),
    ListPlaylistInfo(PlaylistName, Option<Range>),
    SearchPlaylist(PlaylistName, Query, Option<Range>),
    ListPlayLists,
    Load(PlaylistName, Option<Range>, Option<Position>),
    PlaylistAdd(PlaylistName, Utf8PathBuf, Option<QueuePos>),
    PlaylistClear(PlaylistName),
    PlaylistDelete(PlaylistName, PosOrRange), // pos can't be relative
    PlaylistLength(PlaylistName),
    PlaylistMove(PlaylistName, Option<PosOrRange>, QueuePos), // pos can't be relative
    Rename(PlaylistName, PlaylistName),
    Rm(PlaylistName),
    Save(PlaylistName, Option<PlaylistSaveMode>),

    // Interact with database:
    AlbumArt(Utf8PathBuf, u64), // offset in bytes
    Count(Query, Option<Tag>),  // TODO: the group field here is weird, query can be optional?
    GetFingerprint(Utf8PathBuf),
    Find(Query, Option<Sort>, Option<core::ops::Range<u32>>),
    FindAdd(Query, Option<Sort>, Option<core::ops::Range<u32>>, Option<Position>),
    List(List),
    /// List everything in this dir
    ListAll(Option<Utf8PathBuf>),
    ListAllInfo(Option<Utf8PathBuf>),
    ListFiles(Utf8PathBuf),
    /// Mpd supports URI's here we only play files though so we use a path.
    LsInfo(Utf8PathBuf),
    ReadComments(Utf8PathBuf),
    ReadPicture(Utf8PathBuf, u64), // offset in bytes
    Search(Query, Option<Sort>, Option<Range>),
    SearchAdd(Query, Option<Sort>, Option<Range>, Option<Position>),
    SearchAddPl(
        PlaylistName,
        Query,
        Option<Sort>,
        Option<Range>,
        Option<Position>,
    ),
    SearchCount(Query, Option<Tag>),
    Update(Option<Utf8PathBuf>),
    Rescan(Option<Utf8PathBuf>),

    // Mounts and Neighbors:
    Mount(Utf8PathBuf, Utf8PathBuf),
    Unmount(Utf8PathBuf),
    ListMounts,
    ListNeighbors,

    // Stickers:
    StickerGet(StickerType, Utf8PathBuf, String),
    StickerSet(StickerType, Utf8PathBuf, String, String),
    StickerInc(StickerType, Utf8PathBuf, String, String),
    StickerDec(StickerType, Utf8PathBuf, String, String),
    StickerDelete(StickerType, Utf8PathBuf, Option<String>),
    StickerList(StickerType, Utf8PathBuf),
    StickerFind(
        StickerType,
        Utf8PathBuf,
        String,
        Option<Sort>,
        Option<Range>,
    ),
    StickerSearch(
        StickerType,
        Utf8PathBuf,
        String,
        Operator,
        String,
        Option<Sort>,
        Option<Range>,
    ),
    StickerNames,
    StickerTypes,
    StickerNamesTypes(Option<StickerType>),

    // Connection Settings:
    Close,
    Kill,
    Password(String),
    Ping,
    BinaryLimit(u64),
    TagTypes,
    TagTypesDisable(Vec<Tag>),
    TagTypesEnable(Vec<Tag>),
    TagTypesClear,
    TagTypesAll,
    TagTypesAvailable,
    TagTypesReset(Vec<Tag>),
    Protocol,
    ProtocolDisable(Vec<String>),
    ProtocolEnable(Vec<String>),
    ProtocolClear,
    ProtocolAll,
    ProtocolAvailable,

    // Partitions:
    Partition(String),
    ListPartitions,
    NewPartition(String),
    DelPartition(String),
    MoveOutput(String),

    //Audio Outputs:
    DisableOutput(u32),
    EnableOutput(u32),
    ToggleOutput(u32),
    Outputs,
    OutputSet(u32, String, String),

    // Reflection:
    Config,
    Commands,
    NotCommands,
    UrlHandlers,
    Decoders,

    // Client to client:
    // omg you can implement a chat client with MPD?! you love to see it...
    // imagine sending cute little messages to your friends using the
    // same server... wow that's gay.
    Subscribe(ChannelName),
    Unsubscribe(ChannelName),
    Channels,
    ReadMessages,
    SendMessage(ChannelName, String),
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, EnumIter, EnumString)]
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

#[derive(Debug, Default, PartialEq, Eq)]
pub struct List {
    // NOTE we can not parse mpd filters yet
    pub tag_to_list: Tag,
    pub query: Option<Query>,
    pub group_by: Vec<Tag>,
    // used for sending only part of the
    // query answer
    pub window: Option<core::ops::Range<u32>>,
}

/// see <https://mpd.readthedocs.io/en/stable/protocol.html#tags>
#[derive(
    Deserialize,
    Serialize,
    Display,
    EnumIter,
    EnumString,
    Debug,
    Default,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Hash,
)]
pub enum Tag {
    #[default]
    Artist,
    ArtistSort,
    Album,
    AlbumSort,
    AlbumArtist,
    AlbumArtistSort,
    Title,
    TitleSort,
    Track,
    Name,
    Genre,
    Mood,
    Date,
    OriginalDate,
    Composer,
    ComposerSort,
    Performer,
    Conductor,
    Work,
    Ensemble,
    Movement,
    MovementNumber,
    ShowMovement,
    Location,
    Grouping,
    Comment,
    Disc,
    Label,
    MusicbrainzArtistId,
    MusicbrainzAlbumId,
    MusicbrainzAlbumArtistId,
    MusicbrainzTrackId,
    MusicbrainzReleasegroupId,
    MusicbrainzReleaseTrackId,
    MusicbrainzWorkId,
}

impl Command {
    #[instrument(level = "debug", ret)]
    pub(crate) fn parse(line: &str) -> color_eyre::Result<Self> {
        command_parser::parse(line)
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

/// Unique Id for a song in the database. Set on scan.
///
/// Note:
/// Not the same as mpd's SongId
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SongId(pub u32);

/// Stable id for the queue. Adding the same song twice to the queue will assign
/// different id's to them
///
/// Note:
/// This is the same as Mpd's SongId
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq, Copy, Clone)]
pub struct QueueId(pub u32);

/// Position in the queue
///
/// Note:
/// This is the same as Mpd's SongNumber
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueuePos(pub u32);

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Play,
    Pause,
    #[default]
    Stop,
}

impl PlaybackState {
    pub fn toggle(self) -> Self {
        use PlaybackState::*;
        match self {
            Play => Pause,
            Pause => Play,
            Stop => Play,
        }
    }
}

// custom serialize as: samplerate:bits:channels
#[derive(Debug, Serialize)]
pub struct AudioParams {
    pub samplerate: SampleRate,
    pub bits: u64,
    pub channels: ChannelCount,
}

impl Default for AudioParams {
    fn default() -> Self {
        Self {
            samplerate: nz!(44100),
            bits: 16,
            channels: nz!(2),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct QueueInfo(pub Vec<QueueEntry>);

#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct QueueEntry {
    #[serde(rename = "file")]
    pub path: Utf8PathBuf,
    #[serde(rename = "Last-Modified")]
    pub last_modified: jiff::Timestamp, // as 2025-06-15T22:06:58Z
    pub added: jiff::Timestamp, // as 2025-06-15T22:06:58Z
    #[serde(serialize_with = "response_format::audio_params")]
    pub format: AudioParams,
    pub artist: String,
    pub album_artist: String,
    /// the song title
    pub title: String,
    pub album: String,
    /// the decimal track number within the album.
    pub track: u64,
    /// Release date usually 4 digit year
    pub date: String,
    /// the music genre
    pub genre: Option<String>,
    /// the name of the label or publisher
    pub label: String,
    pub disc: Option<u64>,
    #[serde(serialize_with = "response_format::duration_millis_precise")]
    #[serde(rename = "duration")]
    pub duration: Duration,
    pub pos: QueuePos,
    pub id: QueueId,
}

#[derive(Serialize, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
// TODO: check with yara, it doesn't seem to return directories on my machine? maybe mpc filters them out?
pub enum ListItem {
    Directory(Utf8PathBuf),
    File(Utf8PathBuf),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct FindResult {
    #[serde(rename = "file")]
    pub path: Utf8PathBuf,
    #[serde(rename = "Last-Modified")]
    pub last_modified: jiff::Timestamp,
    pub added: jiff::Timestamp,
    #[serde(serialize_with = "response_format::audio_params")]
    pub format: AudioParams,
    #[serde(serialize_with = "response_format::duration_millis_precise")]
    pub duration: Duration,
}

impl QueueEntry {
    /// almost all fields are todo!
    pub fn mostly_fake(pos: u32, id: QueueId, song: crate::system::Song) -> Self {
        Self {
            path: song.path,
            last_modified: Timestamp::constant(0, 0),
            added: Timestamp::constant(0, 0),
            format: AudioParams {
                samplerate: nz!(42),
                bits: 16,
                channels: nz!(42),
            },
            artist: song.artist.unwrap_or("unknown".to_owned()),
            album_artist: "todo".to_string(),
            title: song.title.unwrap_or("unknown".to_owned()),
            album: song.album.unwrap_or("unknown".to_owned()),
            track: 42,
            date: "todo".to_string(),
            genre: None,
            label: "todo".to_string(),
            disc: None,
            duration: song.playtime,
            pos: QueuePos(pos),
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
    /// 31-bit unsigned integer, the playlist version number
    pub playlist: u32, // TODO understand and implement?
    /// the length of queue
    pub playlistlength: u64,
    pub state: PlaybackState,
    pub lastloadedplaylist: Option<PlaylistName>,
    #[serde(serialize_with = "response_format::duration_seconds")]
    pub xfade: Duration,
    /// the current song stopped on or playing
    pub song: Option<QueuePos>,
    /// the current song stopped on or playing
    pub songid: Option<QueueId>,
    #[serde(serialize_with = "response_format::option_duration_millis_precise")]
    pub elapsed: Option<Duration>,
    pub bitrate: Option<u64>,
    /// Duration of the current song in seconds
    #[serde(serialize_with = "response_format::option_duration_millis_precise")]
    pub duration: Option<Duration>,
    #[serde(serialize_with = "response_format::option_audio_params")]
    pub audio: Option<AudioParams>,
    pub error: Option<String>,
    ///the next song to be played
    pub nextsong: Option<QueuePos>,
    ///the next song to be played
    pub nextsongid: Option<QueueId>,
}

#[derive(Serialize, Debug)]
pub struct Stats {
    pub artists: usize,
    pub albums: usize,
    pub songs: usize,
    #[serde(serialize_with = "response_format::duration_seconds")]
    pub uptime: Duration,
    #[serde(serialize_with = "response_format::duration_seconds")]
    pub db_playtime: Duration,
    #[serde(serialize_with = "response_format::unix_time")]
    pub db_update: jiff::Timestamp,
    #[serde(serialize_with = "response_format::duration_seconds")]
    pub playtime: Duration,
}

#[derive(Deserialize, Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum ReplayGainMode {
    #[default]
    Off,
    Track,
    Album,
    Auto,
}

#[derive(Deserialize, Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum ConsumeState {
    #[default]
    Off,
    #[serde(rename = "1")]
    On,
    Oneshot,
}

#[derive(Deserialize, Debug, Copy, Clone, PartialEq)]
pub enum TimeOrOffset {
    Absolute(f32),
    Relative(f32),
}

impl Default for TimeOrOffset {
    fn default() -> Self {
        Self::Absolute(0.0)
    }
}

#[derive(Deserialize, Debug, Copy, Clone, PartialEq)]
pub enum Position {
    Absolute(u32),
    // in mpd, +0 means after current and -0 means before current, for Relative(n)
    // zero means before current, 1 means after current, we parse "+n" as "Relative(n+1)"
    Relative(i32),
}

impl Default for Position {
    fn default() -> Self {
        Self::Absolute(0)
    }
}

#[derive(Deserialize, Debug, Copy, Clone, PartialEq, Default, Eq)]
pub struct Range {
    start: u32,
    end: Option<u32>,
}

#[derive(Deserialize, Debug, Copy, Clone, PartialEq)]
pub struct FloatRange {
    start: Option<f32>,
    end: Option<f32>,
}

#[derive(Deserialize, Debug, Copy, Clone, PartialEq)]
pub enum PosOrRange {
    Position(Position),
    Range(Range),
}

impl Default for PosOrRange {
    fn default() -> Self {
        Self::Range(Range::default())
    }
}

#[derive(Deserialize, Debug, Copy, Clone, PartialEq, Default)]
pub enum PlaylistSaveMode {
    #[default]
    Create,
    Append,
    Replace,
}

#[derive(Deserialize, Debug, Copy, Clone, PartialEq)]
pub struct Sort {
    reverse: bool,
    kind: SortType,
}

#[derive(Deserialize, Debug, Copy, Clone, PartialEq)]
enum SortType {
    Tag(Tag),
    Mtime,
    Prio,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct ChannelName(pub String);

#[derive(Default, Debug, PartialEq)]
pub enum StickerType {
    Song,
    #[default]
    Playlist,
    Tag(Tag),
    Query(Query),
}

#[derive(Deserialize, Debug, Copy, Clone, PartialEq, Default)]
pub enum Operator {
    #[default]
    Equal,
    LessThan,
    GreaterThan,
    Eq,
    Lt,
    Gt,
    StartsWith,
    Contains,
}
