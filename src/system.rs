use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::{Context, OptionExt};
use color_eyre::{Report, Result, Section};
use etcetera::BaseStrategy;
use itertools::Itertools;
use jiff::Timestamp;
// use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use rodio::nz;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use strum::IntoEnumIterator;
use tokio::sync::mpsc;

use crate::mpd_protocol::query::Query;
use crate::mpd_protocol::{
    self, AudioParams, FindResult, ListItem, PlayList, PlaybackState, PlaylistEntry, PlaylistId,
    PlaylistInfo, SongId, SongNumber, SubSystem, Tag, Volume,
};
use crate::playlist::{self, PlaylistName};
use crate::scan::{self, MetaData, scan_path};
use crate::util::{LogError, WhatItertoolsIsMissing};

mod query;

// TODO: scan in the background/on restart
pub struct System {
    db: Connection,
    playing: PlaybackState,
    // queue: VecList<SongId>,
    // library: HashSet<Song>,
    // song_id_from_path: HashMap<Utf8PathBuf, SongId>,
    playlists: HashMap<PlaylistName, Vec<Utf8PathBuf>>,
    idlers: HashMap<SubSystem, Vec<mpsc::Sender<SubSystem>>>,
    music_dir: Utf8PathBuf,
}

impl System {
    pub fn new(music_dir: Utf8PathBuf, playlist_dir: Option<Utf8PathBuf>) -> Result<Self> {
        let dirs = etcetera::choose_base_strategy()?;
        let cache = dirs.cache_dir().join("mpdhaj").join("state.sqlite");
        let db = Connection::open(cache)?;
        db.execute_batch(
            "BEGIN;
            CREATE TABLE IF NOT EXISTS songs (
                id      INTEGER PRIMARY KEY AUTOINCREMENT,
                path    TEXT NOT NULL,
                mtime   TEXT NOT NULL, -- maybe use hash BLOB instead?
                title   TEXT,
                artist  TEXT,
                album   TEXT -- TODO: tags
            );
            CREATE TABLE IF NOT EXISTS queue_state (
                current INTEGER,
                first   INTEGER,
                last    INTEGER
            );
            CREATE TABLE IF NOT EXISTS queue (
                slot    INTEGER PRIMARY KEY AUTOINCREMENT,
                next    INTEGER,
                prev    INTEGER,
                id      INTEGER
            );
            -- TODO: persist mpd status like repeat/random/single/consume
            COMMIT;",
        )?;
        let playlist_dir = playlist_dir.unwrap_or_else(|| music_dir.join("playlists"));
        let playlists = match playlist::load_from_dir(&playlist_dir) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Playlists failed to load: {e:#}. Using an empty list...");
                Default::default()
            }
        };
        Ok(System {
            db,
            music_dir,
            playlists,
            playing: Default::default(),
            idlers: Default::default(),
        })
    }

    pub async fn rescan(&mut self) -> Result<()> {
        // TODO: maybe use async_walkdir instead? doesn't really matter if we only re-scan on startup
        let music_dir = &self.music_dir;
        for e in walkdir::WalkDir::new(music_dir) {
            if let Ok(e) = e
                && let Ok(metadata) = e.metadata()
                && !metadata.is_dir()
                && let Ok(Ok(mtime)) = metadata.modified().map(|t| Timestamp::try_from(t))
                && let Some(path) =
                    Utf8Path::from_path(e.path()).map(|p| p.strip_prefix(music_dir).unwrap())
            {
                // let hash = sha2::Sha256::digest(content).as_slice().to_vec();
                if let Ok((id, cached_mtime)) = self.db.query_one(
                    "SELECT id, mtime FROM songs WHERE path = ?1",
                    [path.as_str()],
                    |row| Ok((row.get::<_, u64>(0)?, row.get::<_, String>(1)?)),
                ) {
                    if let Ok(cached_mtime) = cached_mtime.parse()
                        && mtime != cached_mtime
                        && let Some(song_metadata) = scan_path(path).await
                    {
                        self.db.execute(
                            "REPLACE INTO songs (id, path, mtime, title, artist, album) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            (id, path.as_str(), &mtime.to_string(), song_metadata.title, song_metadata.artist, song_metadata.album),
                        )?;
                    }
                }
                else {

                }
            }
            // TODO: if path is present and this has a newer mtime, update it
        }
        Ok(())
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
                channels: nz!(2),
            },
            error: None,
            nextsong: SongNumber(1),
            nextsongid: SongId(1),
        }
    }

    pub fn queue(&self) -> Result<mpd_protocol::PlaylistInfo> {
        // let queue: Vec<_> = self
        //     .db
        //     .queue()
        //     .iter()
        //     .enumerate_ok()
        //     .collect::<Result<_, _>>()
        //     .wrap_err("Error loading queue from database")?;

        // let queue = queue
        //     .into_iter()
        //     .map(|(pos, song_id)| {
        //         let song = self
        //             .db
        //             .library()
        //             .get(song_id.0 as usize)
        //             .wrap_err("Could not get song from database")?
        //             .ok_or_eyre("Song id in queue was not found in library")
        //             .with_note(|| format!("Song id: {song_id:?}"))?;

        //         Ok::<_, Report>(PlaylistEntry::mostly_fake(pos, song_id, song))
        //     })
        //     .collect::<Result<Vec<_>, _>>()
        //     .wrap_err("Failed to resolve queue")?;

        // Ok(mpd_protocol::PlaylistInfo(queue))
        todo!()
    }

    pub fn playlists(&self) -> mpd_protocol::PlaylistList {
        let list = self
            .playlists
            .keys()
            .map(|name| PlayList::from_name(name.clone()))
            .collect_vec();
        mpd_protocol::PlaylistList(list)
    }

    pub fn get_playlist(&self, name: &PlaylistName) -> Result<mpd_protocol::PlaylistInfo> {
        let Some(paths) = self.playlists.get(name) else {
            tracing::warn!("No playlist found with name: {name:?}");
            return Ok(PlaylistInfo(Vec::new()));
        };

        todo!()
        // let song_ids: Vec<_> = paths
        //     .into_iter()
        //     .map(|path| {
        //         self.db
        //             .song_id_from_path()
        //             .get(path)
        //             .wrap_err("Could not read song_id lookup table")
        //             .and_then(|song_id| song_id.ok_or_eyre("Path is not in song_id lookup table"))
        //             .with_note(|| format!("path: {path}"))
        //     })
        //     .collect::<Result<_, _>>()?;

        // let songs = song_ids
        //     .into_iter()
        //     .enumerate()
        //     .map(|(pos, song_id)| {
        //         let song = self
        //             .db
        //             .library()
        //             .get(song_id.0 as usize)
        //             .wrap_err("Could not get song from database")?
        //             .ok_or_eyre("Song id in playlist was not found in library")
        //             .with_note(|| format!("Song id: {song_id:?}"))?;

        //         Ok::<_, Report>(PlaylistEntry::mostly_fake(pos, song_id, song))
        //     })
        //     .collect::<Result<_, _>>()?;

        // Ok(mpd_protocol::PlaylistInfo(songs))
    }

    pub fn song_info_from_path(&self, path: &Utf8Path) -> Result<Song> {
        // let song_id = self
        //     .db
        //     .song_id_from_path()
        //     .get(path)
        //     .wrap_err("Could not read song_id lookup table")
        //     .and_then(|song_id| song_id.ok_or_eyre("Path is not in song_id lookup table"))
        //     .with_note(|| format!("path: {path}"))?;
        // self.db
        //     .library()
        //     .get(song_id.0 as usize)
        //     .wrap_err("Could not get song from database")?
        //     .ok_or_eyre("Song id in playlist was not found in library")
        //     .with_note(|| format!("Song id: {song_id:?}"))
        todo!()
    }

    pub fn idle(&mut self, mut subsystems: Vec<SubSystem>) -> mpsc::Receiver<SubSystem> {
        if subsystems.is_empty() {
            subsystems.extend(SubSystem::iter());
        }

        let (tx, rx) = mpsc::channel(10);
        for subsystem in subsystems {
            self.idlers
                .entry(subsystem)
                .and_modify(|subscribers| subscribers.push(tx.clone()))
                .or_insert_with(|| vec![tx.clone()]);
        }
        rx
    }

    pub fn add_to_queue(&mut self, path: &Utf8Path) -> Result<()> {
        todo!()
        // let song_id = self
        //     .db
        //     .song_id_from_path()
        //     .get(path)
        //     .wrap_err("Could not read song_id lookup table")
        //     .and_then(|song_id| song_id.ok_or_eyre("Path is not in song_id lookup table"))
        //     .with_note(|| format!("path: {path}"))?;
        // self.db
        //     .queue()
        //     .push(&song_id)
        //     .wrap_err("Could not append song_id to queue")
    }

    pub fn list_all_in(&self, dir: &Utf8Path) -> Result<Vec<ListItem>> {
        // let mut paths = HashSet::new();
        // for path in self
        //     .db
        //     .library()
        //     .iter()
        //     .map_ok(|song| song.file)
        //     .filter_ok(|path| path.starts_with(&dir))
        // {
        //     let path = path.wrap_err("Error reading all library items from db")?;
        //     // annoyingly mpd's list all includes dirs... we dont store those
        //     // so create theme from the file paths here.
        //     paths.extend(path.parent().map(Path::to_owned).map(ListItem::Directory));
        //     paths.insert(ListItem::File(path));
        // }
        // Ok(paths.into_iter().collect_vec())
        todo!()
    }

    pub fn list_tags(&self, tag_to_list: &Tag) -> Result<String> {
        // let mut list = self
        //     .db
        //     .library()
        //     .iter()
        //     .map_ok(|song| match tag_to_list {
        //         Tag::Album => song.album,
        //         Tag::AlbumArtist => "todo".to_string(),
        //         Tag::Artist => song.artist,
        //     })
        //     .collect::<Result<HashSet<_>, _>>()
        //     .wrap_err("Database error while iterating over library")?
        //     .into_iter()
        //     .sorted_unstable()
        //     .map(|tag_value| format!("{tag_to_list}: {tag_value}"))
        //     .join("\n");
        // list.push('\n');
        // Ok(list)
        todo!()
    }

    pub fn handle_find(&self, query: &Query) -> Result<Vec<FindResult>> {
        query::handle_find(self, query)
    }

    pub fn current_song(&self) -> Result<Option<PlaylistEntry>> {
        // let Some(song_id) = self
        //     .db
        //     .queue()
        //     .get(0)
        //     .wrap_err("Could not get to item in queue")?
        // else {
        //     return Ok(None);
        // };

        // let song = self
        //     .db
        //     .library()
        //     .get(song_id.0 as usize)
        //     .wrap_err("Could not get current song from library")?
        //     .ok_or_eyre("Item in the queue was not in the library")?;
        // Ok(Some(PlaylistEntry::mostly_fake(0, song_id, song)))
        todo!()
    }
}

#[derive(Deserialize, Serialize, Hash)]
// #[derive(Archive, RkyvDeserialize, RkyvSerialize)]
pub struct Song {
    pub file: Utf8PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
}
