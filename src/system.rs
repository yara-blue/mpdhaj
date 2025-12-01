use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::Context;
use color_eyre::{Report, Result, eyre::eyre};
use etcetera::BaseStrategy;
use itertools::Itertools;
use jiff::Timestamp;
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
    PlaylistInfo, Position, SongId, SongNumber, SubSystem, Tag, Volume,
};
use crate::playlist::{self, PlaylistName};

mod query;

pub struct System {
    pub db: Connection,
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
        Ok(System {
            db,
            music_dir,
            playlists,
            playing: Default::default(),
            idlers: Default::default(),
            started_at: Timestamp::now(),
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
            audio: AudioParams { samplerate: nz!(44100), bits: 24, channels: nz!(2) },
            error: None,
            nextsong: SongNumber(1),
            nextsongid: SongId(1),
        }
    }

    pub fn queue(&self) -> Result<mpd_protocol::PlaylistInfo> {
        let mut stmt = self.db.prepare(
            "SELECT q.id, q.position, s.path, s.title, s.artist, s.album
             FROM queue q
             JOIN songs s ON s.rowid = q.song
             ORDER BY q.position",
        )?;

        let songs = stmt
            .query_and_then([], |row| {
                let song_id: u32 = row.get(0)?;
                let position: u32 = row.get(1)?;
                let song = Song {
                    path: row.get::<_, String>(2)?.into(),
                    title: row.get(3)?,
                    artist: row.get(4)?,
                    album: row.get(5)?,
                    ..Default::default()
                };
                Ok::<_, Report>(PlaylistEntry::mostly_fake(position, SongId(song_id), song))
            })?
            .collect::<Result<_, _>>()?;

        Ok(mpd_protocol::PlaylistInfo(songs))
    }

    pub fn playlists(&self) -> mpd_protocol::PlaylistList {
        let list =
            self.playlists.keys().map(|name| PlayList::from_name(name.clone())).collect_vec();
        mpd_protocol::PlaylistList(list)
    }

    fn song_number_from_path(&self, path: &Utf8Path) -> Result<u32> {
        Ok(self.db.query_one(
            "SELECT rowid FROM songs WHERE path = ?1",
            [path.as_str()],
            |row| row.get(0),
        )?)
    }

    fn get_song(&self, id: u32) -> Result<Song> {
        self.db
            .query_one("SELECT path, title, artist, album FROM songs WHERE id = ?1", [id], |row| {
                Ok(Song {
                    path: row.get::<_, String>(0)?.into(),
                    title: row.get(1)?,
                    artist: row.get(2)?,
                    album: row.get(3)?,
                    ..Default::default()
                })
            })
            .context("Couldn't find song in database: {id}")
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

    pub fn get_playlist(&self, name: &PlaylistName) -> Result<mpd_protocol::PlaylistInfo> {
        let Some(paths) = self.playlists.get(name) else {
            tracing::warn!("No playlist found with name: {name:?}");
            return Ok(PlaylistInfo(Vec::new()));
        };

        let song_numbers: Vec<_> =
            paths.iter().map(|path| self.song_number_from_path(path)).collect::<Result<_, _>>()?;

        let songs = song_numbers
            .into_iter()
            .enumerate()
            .map(|(pos, song_number)| {
                let song = self.get_song(song_number)?;
                Ok::<_, Report>(PlaylistEntry::mostly_fake(pos as u32, SongId(song_number), song))
            })
            .collect::<Result<_, _>>()?;

        Ok(mpd_protocol::PlaylistInfo(songs))
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

    pub fn add_to_queue(&self, path: &Utf8Path, position: &Option<Position>) -> Result<SongId> {
        let song = self.song_number_from_path(path)?;
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
            self.db
                .execute("UPDATE queue SET position = position + 1 WHERE position >= ?1", [pos])?;
            let mut stmt = self.db.prepare("INSERT INTO queue (song, position) VALUES (?1, ?2)")?;
            Ok(stmt.insert([song, pos]).map(|n| SongId(n as u32))?)
        } else {
            let mut stmt = self.db.prepare(
                "INSERT INTO queue (song, position)
                    VALUES (?1, COALESCE((SELECT MAX(position) FROM queue), 0) + 1)",
            )?;
            Ok(stmt.insert([song]).map(|n| SongId(n as u32))?)
        }
    }

    pub fn list_all_in(&self, dir: &Utf8Path) -> Result<Vec<ListItem>> {
        let mut stmt = self
            .db
            .prepare(&format!("SELECT path FROM songs WHERE path LIKE '{}%'", dir.as_str()))?;
        stmt.query_and_then([], |row| Result::Ok(ListItem::File(row.get::<_, String>(0)?.into())))?
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
        let Ok(pos): Result<u32, _> =
            self.db.query_one("SELECT current FROM state", [], |row| row.get(0))
        else {
            return Ok(None);
        };
        if pos == 0 {
            return Ok(None);
        }
        let Ok((song, id)) =
            self.db.query_one("SELECT song, id FROM queue WHERE position = ?1", [pos], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
        else {
            return Err(eyre!("Couldn't find the currently song in the queue {pos}"));
        };
        let song = self.get_song(song)?;
        Ok(Some(PlaylistEntry::mostly_fake(0, SongId(id), song)))
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

#[cfg(test)]
mod tests {
    use crate::setup_tracing;

    use super::*;

    #[test]
    fn wtfwhynooo() {
        color_eyre::install().unwrap();
        setup_tracing();

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

    // #[test]
    // fn wtfwhyyyyyy() {
    //     let mut system = System::new("~/Music".into(), None).unwrap();
    //     system.db.queue().push(&SongId(1)).unwrap();

    //     dbg!(system.db.queue().get(0).unwrap());
    //     dbg!(system.db.queue().iter().collect_vec());
    // }
}

/*
running 1 test
[src/system.rs:414:9] system.db.queue().get(0).unwrap() = Some(
    SongId(
        1,
    ),
)
[src/system.rs:415:9] system.db.queue().iter().collect_vec() = [
    Ok(
        SongId(
            1,
        ),
    ),
    Ok(
        SongId(
            1,
        ),
    ),
]
 */
