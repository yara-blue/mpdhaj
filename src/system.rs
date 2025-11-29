use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::Context;
use color_eyre::{Report, Result, eyre::eyre};
use etcetera::BaseStrategy;
use itertools::Itertools;
use rodio::nz;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use tokio::sync::mpsc;

use std::collections::HashMap;
use std::time::Duration;

use crate::mpd_protocol::query::Query;
use crate::mpd_protocol::{
    self, AudioParams, FindResult, ListItem, PlayList, PlaybackState, PlaylistEntry, PlaylistId,
    PlaylistInfo, SongId, SongNumber, SubSystem, Tag, Volume,
};
use crate::playlist::{self, PlaylistName};

mod query;

pub struct System {
    pub db: Connection,
    pub playing: PlaybackState,
    pub playlists: HashMap<PlaylistName, Vec<Utf8PathBuf>>,
    pub idlers: HashMap<SubSystem, Vec<mpsc::Sender<SubSystem>>>,
    pub music_dir: Utf8PathBuf,
}

impl System {
    pub fn new(music_dir: Utf8PathBuf, playlist_dir: Option<Utf8PathBuf>) -> Result<Self> {
        let dirs = etcetera::choose_base_strategy()?;
        let cache = dirs.cache_dir().join("mpdhaj").join("state.sqlite");
        let db = Connection::open(cache)?;
        db.execute_batch(
            "BEGIN;
            CREATE TABLE IF NOT EXISTS songs (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                path        TEXT NOT NULL,
                mtime       TEXT NOT NULL,
                generation  INTEGER DEFAULT 0,
                title       TEXT,
                artist      TEXT,
                album       TEXT
                -- TODO: update as we add more tags
                -- TODO: playcount/skipcount
            );
            CREATE TABLE IF NOT EXISTS state (
                id          INTEGER PRIMARY KEY,
                generation  INTEGER,
                current     INTEGER,
                head        INTEGER,
                tail        INTEGER
                -- TODO: persist mpd status like repeat/random/single/consume
            );
            INSERT OR IGNORE INTO state
                (id, generation, current, head, tail) VALUES (0, 0, 0, 0, 0);
            CREATE TABLE IF NOT EXISTS queue (
                -- can't use id as primary key, need to support duplicates
                slot    INTEGER PRIMARY KEY AUTOINCREMENT,
                id      INTEGER,
                next    INTEGER,
                prev    INTEGER
            );
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
        let mut ids = Vec::new();
        let mut cur = self
            .db
            .query_one("SELECT head FROM state", [], |row| row.get::<_, u32>(0))?;
        while cur != 0 {
            let (next, id) =
                self.db
                    .query_one("SELECT next, id FROM queue WHERE slot = ?1", [cur], |row| {
                        Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?))
                    })?;
            ids.push(id);
            cur = next;
        }

        let songs = ids
            .into_iter()
            .enumerate()
            .map(|(pos, song_id)| {
                let song = self.song_from_id(song_id)?;
                Ok::<_, Report>(PlaylistEntry::mostly_fake(pos, SongId(song_id), song))
            })
            .collect::<Result<_, _>>()?;

        Ok(mpd_protocol::PlaylistInfo(songs))
    }

    pub fn playlists(&self) -> mpd_protocol::PlaylistList {
        let list = self
            .playlists
            .keys()
            .map(|name| PlayList::from_name(name.clone()))
            .collect_vec();
        mpd_protocol::PlaylistList(list)
    }

    fn song_id_from_path(&self, path: &Utf8Path) -> Result<u32> {
        Ok(self.db.query_one(
            "SELECT id FROM songs WHERE path = ?1",
            [path.as_str()],
            |row| row.get(0),
        )?)
    }

    fn song_from_id(&self, id: u32) -> Result<Song> {
        self.db
            .query_one(
                "SELECT path, title, artist, album FROM songs WHERE id = ?1",
                [id],
                |row| {
                    Ok(Song {
                        path: row.get::<_, String>(0)?.into(),
                        title: row.get(1)?,
                        artist: row.get(2)?,
                        album: row.get(3)?,
                    })
                },
            )
            .context("Couldn't find song in database: {id}")
    }

    pub fn get_playlist(&self, name: &PlaylistName) -> Result<mpd_protocol::PlaylistInfo> {
        let Some(paths) = self.playlists.get(name) else {
            tracing::warn!("No playlist found with name: {name:?}");
            return Ok(PlaylistInfo(Vec::new()));
        };

        let song_ids: Vec<_> = paths
            .iter()
            .map(|path| self.song_id_from_path(path))
            .collect::<Result<_, _>>()?;

        let songs = song_ids
            .into_iter()
            .enumerate()
            .map(|(pos, song_id)| {
                let song = self.song_from_id(song_id)?;
                Ok::<_, Report>(PlaylistEntry::mostly_fake(pos, SongId(song_id), song))
            })
            .collect::<Result<_, _>>()?;

        Ok(mpd_protocol::PlaylistInfo(songs))
    }

    pub fn song_info_from_path(&self, path: &Utf8Path) -> Result<Song> {
        Ok(self.db.query_one(
            "SELECT title, artist, album FROM songs WHERE path = ?1",
            [path.as_str()],
            |r| {
                Ok(Song {
                    path: path.to_owned(),
                    title: r.get(0)?,
                    artist: r.get(1)?,
                    album: r.get(2)?,
                })
            },
        )?)
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
        let id = self.song_id_from_path(path)?;
        let tail = self
            .db
            .query_one("SELECT tail FROM state", [], |row| row.get::<_, u32>(0))?;
        let mut stmt = self
            .db
            .prepare("INSERT INTO queue (id, prev, next) VALUES (?1, ?2, 0)")?;
        let slot = stmt.insert([id, tail])?;
        if tail == 0 {
            self.db
                .execute("UPDATE state SET head = ?1, tail = ?2", [slot, slot])?;
        } else {
            self.db.execute("UPDATE state SET tail = ?1", [slot])?;
            self.db
                .execute("UPDATE queue SET next = ?1 WHERE id = ?2", (slot, tail))?;
        }
        Ok(())
    }

    pub fn list_all_in(&self, dir: &Utf8Path) -> Result<Vec<ListItem>> {
        // TODO: check that params work like this, might have to just `format!()` the query.
        // could also just do the startswith filtering in rust
        let mut stmt = self
            .db
            .prepare("SELECT path FROM songs WHERE path LIKE ?1 + '%'")?;
        stmt.query_and_then([dir.as_str()], |row| {
            Result::Ok(ListItem::File(row.get::<_, String>(0)?.into()))
        })?
        .collect::<Result<Vec<_>, Report>>()
    }

    pub fn list_tag(&self, tag_to_list: &Tag) -> Result<Vec<String>> {
        let mut stmt = self.db.prepare("SELECT DISTINCT ?1 FROM songs")?;
        Ok(stmt
            .query_and_then([tag_to_list.to_string().to_lowercase()], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn handle_find(&self, query: &Query) -> Result<Vec<FindResult>> {
        query::handle_find(self, query)
    }

    pub fn current_song(&self) -> Result<Option<PlaylistEntry>> {
        let Ok(index) = self
            .db
            .query_one("SELECT current FROM state", [], |row| row.get::<_, u32>(0))
        else {
            return Ok(None);
        };
        if index == 0 {
            return Ok(None);
        }
        let Ok(id) = self
            .db
            .query_one("SELECT id FROM queue WHERE slot = ?1", [index], |row| {
                row.get::<_, u32>(0)
            })
        else {
            return Err(eyre!(
                "Couldn't find the currently song in the queue {index}"
            ));
        };
        let song = self
            .db
            .query_one(
                "SELECT path,title,artist,album FROM songs WHERE id = ?1",
                [id],
                |row| {
                    Ok(Song {
                        path: row.get::<_, String>(0)?.into(),
                        title: row.get(1)?,
                        artist: row.get(2)?,
                        album: row.get(3)?,
                    })
                },
            )
            .context("Couldn't find song in database: {id}")?;
        Ok(Some(PlaylistEntry::mostly_fake(0, SongId(id), song)))
    }

    pub fn clear(&self) -> Result<()> {
        self.db.execute_batch(
            "BEGIN
            UPDATE state SET current = 0;
            DELETE FROM queue;
            COMMIT;",
        )?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Hash)]
pub struct Song {
    pub path: Utf8PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
}
