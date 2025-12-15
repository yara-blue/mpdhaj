use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Section;
use color_eyre::eyre::Context;
use color_eyre::{Report, Result, eyre::eyre};
use etcetera::BaseStrategy;
use itertools::Itertools;
use jiff::Timestamp;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use tokio::sync::mpsc;
use tracing::instrument;

use std::collections::HashMap;
use std::time::Duration;

use crate::mpd_protocol::query::Query;
use crate::mpd_protocol::{
    self, AudioParams, FindResult, ListItem, PlayList, PlaybackState, Position, QueueEntry,
    QueueId, QueueInfo, QueuePos, SongId, SubSystem, Tag, Volume,
};
use crate::player::Player;
use crate::playlist::{self, PlaylistName};

mod query;

pub struct System {
    pub db: Connection,
    pub player: Player,
    pub playing: PlaybackState,
    pub playlists: HashMap<PlaylistName, Vec<Utf8PathBuf>>,
    pub idlers: HashMap<SubSystem, Vec<mpsc::Sender<SubSystem>>>,
    pub music_dir: Utf8PathBuf,
    #[allow(unused)]
    pub started_at: Timestamp, // for uptime
}

impl System {
    pub fn new(music_dir: Utf8PathBuf, playlist_dir: Option<Utf8PathBuf>) -> Result<Self> {
        let dirs = etcetera::choose_base_strategy()?;
        let cache = dirs.cache_dir().join("mpdhaj").join("state.sqlite");
        std::fs::create_dir_all(cache.parent().unwrap())?;
        let db = Connection::open(cache)?;
        db.execute_batch(include_str!("tables.sql"))?;
        let playlist_dir = playlist_dir.unwrap_or_else(|| music_dir.join("playlists"));
        let playlists = match playlist::load_from_dir(&playlist_dir) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Playlists failed to load: {e:#}. Using an empty list...");
                Default::default()
            }
        };
        let player = Player::new(0.5, false);
        Ok(System {
            db,
            music_dir,
            playlists,
            player,
            playing: Default::default(),
            idlers: Default::default(),
            started_at: Timestamp::now(),
        })
    }

    pub fn status(&self) -> Result<mpd_protocol::Status> {
        let (current, random, single, consume, repeat) = self.db.query_one(
            "SELECT current, random, single, consume, repeat FROM state",
            [],
            |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;
        let len = self
            .db
            .query_one("SELECT COUNT(*) FROM queue", [], |row| row.get::<_, u32>(0))?;
        let (mut queue_pos, mut queue_id, mut next_pos, mut next_id) = (None, None, None, None);
        if let Ok((id, pos)) = self.db.query_one(
            "SELECT song, position FROM QUEUE WHERE id = ?1",
            [current],
            |row| Ok((row.get::<_, u32>(0)?, row.get(1)?)),
        ) {
            queue_pos = Some(QueuePos(pos));
            queue_id = Some(QueueId(id));
            if let Ok(id) = self.db.query_one(
                "SELECT song FROM QUEUE WHERE position = ?1",
                [pos + 1],
                |row| Ok(row.get::<_, u32>(0)?),
            ) {
                next_pos = Some(QueuePos(pos + 1));
                next_id = Some(QueueId(id));
            }
        }
        Ok(mpd_protocol::Status {
            repeat: repeat,
            random: random,
            single: single,
            consume: consume,
            partition: "default".to_string(),
            volume: Volume::new(50), // TODO: persist
            playlist: 0,             // TODO
            playlistlength: len as u64,
            state: self.playing,
            lastloadedplaylist: None,
            xfade: Duration::from_secs(0),
            song: queue_pos,
            songid: queue_id,
            elapsed: None, // TODO
            bitrate: None,
            duration: None, // TODO
            audio: None,
            error: None,
            nextsong: next_pos,
            nextsongid: next_id,
        })
    }

    pub fn queue(&self) -> Result<mpd_protocol::QueueInfo> {
        let mut stmt = self.db.prepare(
            "SELECT q.id, q.position, s.path, s.title, s.artist, s.album
             FROM queue q
             JOIN songs s ON s.rowid = q.song
             ORDER BY q.position",
        )?;

        let songs = stmt
            .query_and_then([], |row| {
                let queue_id: u32 = row.get(0)?;
                let position: u32 = row.get(1)?;
                let song = Song {
                    path: row.get::<_, String>(2)?.into(),
                    title: row.get(3)?,
                    artist: row.get(4)?,
                    album: row.get(5)?,
                    ..Default::default()
                };
                Ok::<_, Report>(QueueEntry::mostly_fake(position, QueueId(queue_id), song))
            })?
            .collect::<Result<_, _>>()?;

        Ok(mpd_protocol::QueueInfo(songs))
    }

    pub fn playlists(&self) -> mpd_protocol::PlaylistList {
        let list = self
            .playlists
            .keys()
            .map(|name| PlayList::from_name(name.clone()))
            .collect_vec();
        mpd_protocol::PlaylistList(list)
    }

    fn song_id_from_path(&self, path: &Utf8Path) -> Result<SongId> {
        Ok(self.db.query_one(
            "SELECT rowid FROM songs WHERE path = ?1",
            [path.as_str()],
            |row| row.get(0).map(SongId),
        )?)
    }

    pub fn get_song(&self, id: SongId) -> Result<Song> {
        self.db
            .query_one(
                "SELECT path, title, artist, album FROM songs WHERE rowid = ?1",
                [id.0],
                |row| {
                    Ok(Song {
                        path: row.get::<_, String>(0)?.into(),
                        title: row.get(1)?,
                        artist: row.get(2)?,
                        album: row.get(3)?,
                        ..Default::default()
                    })
                },
            )
            .wrap_err("Couldn't find song in database")
            .with_note(|| format!("song id: {id:?}"))
    }

    pub fn get_song_by_path(&self, path: &Utf8Path) -> Result<Song> {
        Ok(self.db.query_one(
            "SELECT title, artist, album FROM songs WHERE path = ?1",
            [path.as_str()],
            |r| {
                Ok(Song {
                    path: path.to_owned(),
                    title: r.get(0)?,
                    artist: r.get(1)?,
                    album: r.get(2)?,
                    ..Default::default()
                })
            },
        )?)
    }

    pub fn get_playlist(&self, name: &PlaylistName) -> Result<mpd_protocol::QueueInfo> {
        let Some(paths) = self.playlists.get(name) else {
            tracing::warn!("No playlist found with name: {name:?}");
            return Ok(QueueInfo(Vec::new()));
        };

        let song_ids: Vec<_> = paths
            .iter()
            .map(|path| self.song_id_from_path(path))
            .collect::<Result<_, _>>()?;

        let songs = song_ids
            .into_iter()
            .enumerate()
            .map(|(pos, song_id)| {
                let song = self.get_song(song_id)?;
                Ok::<_, Report>(QueueEntry::mostly_fake(
                    pos as u32,
                    QueueId(42), // TODO
                    song,
                ))
            })
            .collect::<Result<_, _>>()?;

        Ok(mpd_protocol::QueueInfo(songs))
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

    pub fn add_to_queue(&self, path: &Utf8Path, position: &Option<Position>) -> Result<QueueId> {
        let song = self.song_id_from_path(path)?;
        if let Some(pos) = position {
            let pos: u32 = match pos {
                Position::Absolute(pos) => *pos,
                Position::Relative(offset) => {
                    let current = self
                        .db
                        .query_one("SELECT current FROM state", [], |row| row.get::<_, u32>(0))?;
                    if -offset > current as i32 {
                        return Err(eyre!(
                            "Position {offset} is invalid, current position is {current}"
                        ));
                    }
                    (current as i32 + offset) as u32
                }
            };
            self.db.execute(
                "UPDATE queue SET position = position + 1 WHERE position >= ?1",
                [pos],
            )?;
            let mut stmt = self
                .db
                .prepare("INSERT INTO queue (song, position) VALUES (?1, ?2)")?;
            Ok(stmt.insert([song.0, pos]).map(|n| QueueId(n as u32))?)
        } else {
            let mut stmt = self.db.prepare(
                "INSERT INTO queue (song, position)
                    VALUES (?1, COALESCE((SELECT MAX(position) FROM queue), 0) + 1)",
            )?;
            Ok(stmt.insert([song.0]).map(|n| QueueId(n as u32))?)
        }
    }

    pub fn list_all_in(&self, dir: &Utf8Path) -> Result<Vec<ListItem>> {
        let mut stmt = self.db.prepare(&format!(
            "SELECT path FROM songs WHERE path LIKE '{}%'",
            dir.as_str()
        ))?;
        stmt.query_and_then([], |row| {
            Result::Ok(ListItem::File(row.get::<_, String>(0)?.into()))
        })?
        .collect::<Result<Vec<_>, Report>>()
    }

    pub fn list_tag(&self, tag_to_list: &Tag) -> Result<Vec<String>> {
        let mut stmt = self.db.prepare(&format!(
            "SELECT DISTINCT {} FROM songs",
            tag_to_list.to_string().to_lowercase()
        ))?;
        Ok(stmt
            .query_and_then([], |row| dbg!(row.get::<_, String>(0)))?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn handle_find(&self, query: &Query) -> Result<Vec<FindResult>> {
        query::handle_find(self, query)
    }

    #[instrument(skip(self), ret)]
    pub fn current_song(&self) -> Result<Option<QueueEntry>> {
        let Ok(pos): Result<u32, _> = self
            .db
            .query_one("SELECT current FROM state", [], |row| row.get(0))
        else {
            return Ok(None);
        };
        if pos == 0 {
            return Ok(None);
        }
        self.song_by_pos(QueuePos(pos))
    }

    pub fn song_by_pos(&self, pos: QueuePos) -> Result<Option<QueueEntry>> {
        let Ok((song, id)) = self.db.query_one(
            "SELECT song, id FROM queue WHERE position = ?1",
            [pos.0],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ) else {
            return Err(eyre!("Couldn't find song #{} in the queue", pos.0));
        };
        let song = self.get_song(SongId(song))?;
        Ok(Some(QueueEntry::mostly_fake(pos.0, QueueId(id), song)))
    }

    pub fn song_by_id(&self, id: QueueId) -> Result<Option<QueueEntry>> {
        let Ok((song, pos)) = self.db.query_one(
            "SELECT song, position FROM queue WHERE id = ?1",
            [id.0],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ) else {
            return Err(eyre!("Couldn't find song id {} in the queue", id.0));
        };
        let song = self.get_song(SongId(song))?;
        Ok(Some(QueueEntry::mostly_fake(pos, id, song)))
    }

    pub fn clear(&self) -> Result<()> {
        self.db.execute_batch(
            "BEGIN;
            UPDATE state SET current = 0;
            DELETE FROM queue;
            COMMIT;",
        )?;
        Ok(())
    }

    // pub fn stats(&self) -> Result<Stats> {
    //     #[derive(Default)]
    //     struct Counter {
    //         artists: HashSet<String>,
    //         albums: HashSet<String>,
    //         songs: usize,
    //         db_playtime: Duration,
    //     }

    //     let counter = self
    //         .db
    //         .library()
    //         .iter()
    //         .fold_ok(Counter::default(), |mut counter, song| {
    //             counter.artists.insert(song.artist);
    //             counter.albums.insert(song.album);
    //             counter.songs += 1;
    //             counter.db_playtime += song.playtime;
    //             counter
    //         })
    //         .wrap_err("Could not read items from db for counting")?;

    //     dbg!(self.queue()?);

    //     let playtime = self
    //         .queue()
    //         .wrap_err("Could not get queue")?
    //         .0
    //         .into_iter()
    //         .map(|entry| dbg!(entry).duration)
    //         .sum();

    //     Ok(Stats {
    //         artists: counter.artists.len(),
    //         albums: counter.albums.len(),
    //         songs: counter.songs,
    //         uptime: self.up_since.elapsed(),
    //         db_playtime: counter.db_playtime,
    //         db_update: self
    //             .db
    //             .last_db_update()
    //             .get()
    //             .wrap_err("Could not read last db update")?
    //             .unwrap_or_default(),
    //         playtime,
    //     })
    // }
}

#[derive(Deserialize, Serialize, Hash, Default)]
pub struct Song {
    pub path: Utf8PathBuf,
    pub mtime: Timestamp,
    pub generation: u64,

    pub play_count: u32,
    pub skip_count: u32,
    pub date_added: Timestamp,

    pub title: Option<String>,
    pub artist: Option<String>,
    pub artist_sort: Option<String>,
    pub album: Option<String>,
    pub album_sort: Option<String>,
    pub album_artist: Option<String>,
    pub album_artist_sort: Option<String>,
    pub title_sort: Option<String>,
    pub track: Option<u8>,
    pub name: Option<String>,
    pub genre: Option<String>,
    pub mood: Option<String>,
    pub date: Option<String>,
    pub original_date: Option<String>,
    pub composer: Option<String>,
    pub composer_sort: Option<String>,
    pub performer: Option<String>,
    pub conductor: Option<String>,
    pub work: Option<String>,
    pub ensemble: Option<String>,
    pub movement: Option<String>,
    pub movement_number: Option<String>,
    pub show_movement: Option<bool>,
    pub location: Option<String>,
    pub grouping: Option<String>,
    pub comment: Option<String>,
    pub disc: Option<u8>,
    pub label: Option<String>,
    pub playtime: Duration,

    pub musicbrainz_artist_id: Option<String>,
    pub musicbrainz_album_id: Option<String>,
    pub musicbrainz_album_artist_id: Option<String>,
    pub musicbrainz_track_id: Option<String>,
    pub musicbrainz_releasegroup_id: Option<String>,
    pub musicbrainz_release_track_i: Option<String>,
    pub musicbrainz_work_id: Option<String>,
}

impl QueueEntry {
    fn from_song(s: Song, pos: QueuePos, id: QueueId) -> Self {
        QueueEntry {
            path: s.path,
            last_modified: s.mtime,
            added: s.date_added,
            format: AudioParams::default(), // TODO:
            artist: s.artist.unwrap_or_default(),
            album_artist: s.album_artist.unwrap_or_default(),
            title: s.title.unwrap_or_default(),
            album: s.album.unwrap_or_default(),
            track: s.track.unwrap_or_default() as u64,
            date: s.date.unwrap_or_default(),
            genre: s.genre,
            label: s.label.unwrap_or_default(),
            disc: s.disc.map(|n| n as u64),
            duration: s.playtime,
            pos,
            id,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::setup_tracing;

    use super::*;

    #[test]
    fn wtfwhynooo() {
        color_eyre::install().unwrap();
        setup_tracing();

        // TODO: use in-memory database for tests, pass connection into system::new instead of creating in
        // there. also disable scanning?
        let system = System::new("~/Music".into(), None).unwrap();
        system
            .add_to_queue(
                Utf8Path::new("The Sims Complete Collection/Disc 1/01 - Now Entering.mp3"),
                &None,
            )
            .unwrap();

        let queue = system.queue().unwrap();
        let first = &queue.0[0];
        assert!(first.path.as_str().contains("Sims"));
    }
}
