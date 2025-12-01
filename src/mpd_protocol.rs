pub mod command_format;
pub mod command_parser;
#[allow(unused)]
pub mod query;
pub mod response_format;

use std::time::Duration;

use camino::Utf8PathBuf;
use jiff::Timestamp;
use rodio::{ChannelCount, SampleRate, nz};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{mpd_protocol::query::Query, playlist::PlaylistName};

pub const VERSION: &str = "0.24.4";

// TODO: in general these should be using URIs instead of Utf8PathBuf

/// see <https://mpd.readthedocs.io/en/stable/protocol.html#command-reference>
#[derive(
    Debug, Default, strum_macros::VariantNames, strum_macros::EnumString, PartialEq,
)]
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
    Play(Option<PosInPlaylist>),
    PlayId(Option<SongId>), // weird that this is optional
    Previous,
    Seek(PosInPlaylist, f32),
    SeekId(SongId, f32),
    SeekCur(TimeOrOffset),
    Stop,

    // Manipulate the Queue:
    /// Add an item to the queue
    Add(Utf8PathBuf, Option<Position>),
    AddId(Utf8PathBuf, Option<Position>),
    /// Remove all items from the Queue
    Clear,
    Delete(Option<PosOrRange>),
    DeleteId(SongId),
    Move(Option<PosOrRange>, Position),
    MoveId(SongId, Position),
    Playlist, // deprecated
    PlaylistFind(Query, Option<Sort>, Option<Range>),
    PlaylistId(Option<SongId>),
    PlaylistInfo(Option<PosOrRange>),
    PlaylistSearch(Query, Option<Sort>, Option<Range>),
    PlChanges(u32, Option<Range>),
    PlChangesPosId(u32, Option<Range>),
    Prio(u8, Vec<Range>),
    PrioId(u8, Vec<SongId>),
    RangeId(SongId, Option<FloatRange>),
    Shuffle(Option<Range>),
    Swap(PosInPlaylist, PosInPlaylist), // TODO: can these be relative?
    SwapId(SongId, SongId),
    AddTagId(SongId, Tag, String),
    ClearTagId(SongId, Tag),

    // Manipulate Playlists:
    ListPlaylist(PlaylistName, Option<Range>),
    ListPlaylistInfo(PlaylistName, Option<Range>),
    SearchPlaylist(PlaylistName, Query, Option<Range>),
    ListPlayLists,
    Load(PlaylistName, Option<Range>, Option<Position>),
    PlaylistAdd(PlaylistName, Utf8PathBuf, Option<PosInPlaylist>),
    PlaylistClear(PlaylistName),
    PlaylistDelete(PlaylistName, PosOrRange), // pos can't be relative
    PlaylistLength(PlaylistName),
    PlaylistMove(PlaylistName, Option<PosOrRange>, PosInPlaylist), // pos can't be relative
    Rename(PlaylistName, PlaylistName),
    Rm(PlaylistName),
    Save(PlaylistName, Option<PlaylistSaveMode>),

    // Interact with database:
    AlbumArt(Utf8PathBuf, u64), // offset in bytes
    Count(Query, Option<Tag>), // TODO: the group field here is weird, query can be optional?
    GetFingerprint(Utf8PathBuf),
    Find(Query, Option<Sort>, Option<Range>),
    FindAdd(Query, Option<Sort>, Option<Range>, Option<Position>),
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
    SearchAddPl(PlaylistName, Query, Option<Sort>, Option<Range>, Option<Position>),
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
    StickerFind(StickerType, Utf8PathBuf, String, Option<Sort>, Option<Range>),
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
    TagTypesDisable(Vec<String>),
    TagTypesEnable(Vec<String>),
    TagTypesClear,
    TagTypesAll,
    TagTypesAvailable,
    TagTypesReset(Vec<String>),
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

#[derive(
    Debug, Deserialize, Serialize, PartialEq, Eq, Hash, strum::EnumIter, strum::EnumString,
)]
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
    pub query: Query,
    pub group_by: Vec<Tag>,
}

/// see <https://mpd.readthedocs.io/en/stable/protocol.html#tags>
#[derive(
    Deserialize,
    Serialize,
    strum_macros::Display,
    Debug,
    Default,
    PartialEq,
    Eq,
    Clone,
    Copy,
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
        PlayList { playlist: name, last_modified: jiff::Timestamp::new(42, 42).unwrap() }
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
    #[allow(unused)]
    pub fn get(&self) -> u8 {
        self.0
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SongId(pub u32);
#[derive(Debug, Serialize)]
pub struct SongNumber(pub u32);

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PosInPlaylist(u32);

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub enum PlaybackState {
    Play,
    Pause,
    #[default]
    Stop,
}

// custom serialize as: samplerate:bits:channels
#[derive(Debug, Serialize)]
pub struct AudioParams {
    pub samplerate: SampleRate,
    pub bits: u64,
    pub channels: ChannelCount,
}

#[derive(Serialize, Debug)]
pub struct PlaylistInfo(pub Vec<PlaylistEntry>);

#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PlaylistEntry {
    #[serde(rename = "file")]
    pub path: Utf8PathBuf,
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
    track: u64,
    /// Release date usually 4 digit year
    date: String,
    /// the music genre
    genre: Option<String>,
    /// the name of the label or publisher
    label: String,
    disc: Option<u64>,
    #[serde(serialize_with = "response_format::duration_millis_precise")]
    #[serde(rename = "duration")]
    pub duration: Duration,
    pos: PosInPlaylist,
    id: SongId,
}

#[derive(Serialize, Debug, Hash, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
// TODO: check with yara, it doesn't seem to return directories on my machine? maybe mpc filters them out?
pub enum ListItem {
    #[allow(unused)]
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

impl PlaylistEntry {
    /// almost all fields are todo!
    pub fn mostly_fake(pos: u32, id: SongId, song: crate::system::Song) -> Self {
        Self {
            path: song.path,
            last_modified: Timestamp::constant(0, 0),
            added: Timestamp::constant(0, 0),
            format: AudioParams { samplerate: nz!(42), bits: 16, channels: nz!(42) },
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
            pos: PosInPlaylist(pos),
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
    pub playlistlength: u64,
    pub state: PlaybackState,
    pub lastloadedplaylist: Option<PlaylistName>,
    #[serde(serialize_with = "response_format::duration_seconds")]
    pub xfade: Duration,
    pub song: SongNumber,
    pub songid: SongId,
    #[serde(serialize_with = "response_format::duration_millis_precise")]
    pub elapsed: Duration,
    pub bitrate: u64,
    /// Duration of the current song in seconds
    #[serde(serialize_with = "response_format::duration_millis_precise")]
    pub duration: Duration,
    #[serde(serialize_with = "response_format::audio_params")]
    pub audio: AudioParams,
    pub error: Option<String>,
    pub nextsong: SongNumber,
    pub nextsongid: SongId,
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

#[derive(Deserialize, Debug, Copy, Clone, PartialEq)]
pub struct Range {
    start: u32,
    end: Option<u32>,
}

impl Default for Range {
    fn default() -> Self {
        Self { start: 0, end: None }
    }
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

#[derive(Deserialize, Default, Debug, PartialEq)]
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
