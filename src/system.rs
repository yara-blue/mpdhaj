use color_eyre::eyre::{Context, OptionExt};
use color_eyre::{Report, Result, Section};
use etcetera::BaseStrategy;
use itertools::Itertools;
use rodio::nz;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use strum::IntoEnumIterator;
use tokio::sync::mpsc;
use tracing::instrument;

use crate::mpd_protocol::query::Query;
use crate::mpd_protocol::{
    self, AudioParams, FindResult, ListItem, PlayList, PlaybackState, PlaylistEntry, PlaylistId,
    PlaylistInfo, SongId, SongNumber, Stats, SubSystem, Tag, Volume,
};
use crate::playlist::{self, PlaylistName};
use crate::scan::{self, MetaData};
use crate::util::WhatItertoolsIsMissing;

mod query;

// If this ever gets too slow we can see what we need to cache
#[dbstruct::dbstruct(db=sled)]
pub struct State {
    #[dbstruct(Default = "PlaybackState::Stop")]
    playing: PlaybackState,
    queue: Vec<SongId>,

    /// All songs currently 'scanned'. Scanning MUST occur before anything
    /// else atm. The `SongId` is the index in this vector.
    ///
    /// TODO: make it so scanning happens non stop in the background
    /// (using io-notify and friends).
    library: Vec<Song>,

    // just rebuild this on rescan
    song_id_from_path: HashMap<PathBuf, SongId>,

    last_db_update: Option<jiff::Timestamp>,
}

pub struct System {
    up_since: Instant,
    db: State,
    playlists: HashMap<PlaylistName, Vec<PathBuf>>,
    idlers: HashMap<SubSystem, Vec<mpsc::Sender<SubSystem>>>,
    music_dir: PathBuf,
}

impl System {
    pub fn new(music_dir: PathBuf, playlist_dir: Option<PathBuf>) -> Result<Self> {
        let dirs = etcetera::choose_base_strategy()?;
        let cache = dirs.cache_dir().join("mpdhaj").join("database");
        let state = State::open_path(&cache)
            .wrap_err("Could not open db")
            .with_note(|| format!("path: {}", cache.display()))
            .suggestion(
                "The database format probably changed, \
                try removing the database folder",
            )?;
        let playlist_dir = playlist_dir.unwrap_or_else(|| music_dir.join("playlists"));
        let playlists = match playlist::load_from_dir(&playlist_dir) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Playlists failed to load: {e:#}. Using an empty list...");
                Default::default()
            }
        };
        dbg!(state.queue().len());
        dbg!(
            state
                .ds
                .iter()
                .keys()
                .filter_ok(|key| key.starts_with(&[3]))
                .collect_vec()
        );

        dbg!(::dbstruct::traits::data_store::Ordered::get_lt(
            &state.ds,
            &::dbstruct::wrapper::VecPrefixed::max(3),
        )?
        .map(|(key, _): (::dbstruct::wrapper::VecPrefixed, SongId)| dbg!(key))
        .filter(|key| key.prefix() == 3)
        .map(|key| key.index() + 1) // a vecs len is index + 1
        .unwrap_or(0));

        // state.queue().clear().unwrap();
        Ok(System {
            up_since: Instant::now(),
            db: state,
            playlists,
            idlers: Default::default(),
            music_dir,
        })
    }

    pub async fn scan(&mut self) -> Result<()> {
        // Song ids will change, reset db
        dbg!(self.db.queue().len());
        dbg!(
            self.db
                .ds
                .iter()
                .keys()
                .filter_ok(|key| key.starts_with(&[3]))
                .collect_vec()
        );
        self.db.queue().clear().unwrap();
        dbg!(
            self.db
                .ds
                .iter()
                .keys()
                .filter_ok(|key| key.starts_with(&[3]))
                .collect_vec()
        );
        self.db.song_id_from_path().clear().unwrap();

        scan::scan_dir(&self.music_dir, |mut metadata: MetaData| {
            metadata.file = metadata
                .file
                .strip_prefix(&self.music_dir)
                .unwrap()
                .to_path_buf();

            self.db
                .song_id_from_path()
                .insert(&metadata.file, &SongId(self.db.library().len() as u32))
                .unwrap();
            self.db
                .library()
                .push(&Song {
                    file: metadata.file,
                    title: metadata.title,
                    artist: metadata.artist,
                    album: metadata.album,
                    playtime: metadata.playtime,
                })
                .unwrap();
        })
        .await?;

        self.db
            .last_db_update()
            .set(Some(&jiff::Timestamp::now()))
            .wrap_err("Could not set last db update")?;

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
        let queue: Vec<_> = self
            .db
            .queue()
            .iter()
            .enumerate_ok()
            .collect::<Result<_, _>>()
            .wrap_err("Error loading queue from database")?;

        dbg!(&queue);
        let queue = queue
            .into_iter()
            .map(|(pos, song_id)| {
                let song = self
                    .db
                    .library()
                    .get(song_id.0 as usize)
                    .wrap_err("Could not get song from database")?
                    .ok_or_eyre("Song id in queue was not found in library")
                    .with_note(|| format!("Song id: {song_id:?}"))?;

                Ok::<_, Report>(PlaylistEntry::mostly_fake(pos, song_id, song))
            })
            .collect::<Result<Vec<_>, _>>()
            .wrap_err("Failed to resolve queue")?;

        Ok(mpd_protocol::PlaylistInfo(queue))
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

        let song_ids: Vec<_> = paths
            .into_iter()
            .map(|path| {
                self.db
                    .song_id_from_path()
                    .get(path)
                    .wrap_err("Could not read song_id lookup table")
                    .and_then(|song_id| song_id.ok_or_eyre("Path is not in song_id lookup table"))
                    .with_note(|| format!("path: {}", path.display()))
            })
            .collect::<Result<_, _>>()?;

        let songs = song_ids
            .into_iter()
            .enumerate()
            .map(|(pos, song_id)| {
                let song = self
                    .db
                    .library()
                    .get(song_id.0 as usize)
                    .wrap_err("Could not get song from database")?
                    .ok_or_eyre("Song id in playlist was not found in library")
                    .with_note(|| format!("Song id: {song_id:?}"))?;

                Ok::<_, Report>(PlaylistEntry::mostly_fake(pos, song_id, song))
            })
            .collect::<Result<_, _>>()?;

        Ok(mpd_protocol::PlaylistInfo(songs))
    }

    pub fn song_info_from_path(&self, path: &Path) -> Result<Song> {
        let song_id = self
            .db
            .song_id_from_path()
            .get(path)
            .wrap_err("Could not read song_id lookup table")
            .and_then(|song_id| song_id.ok_or_eyre("Path is not in song_id lookup table"))
            .with_note(|| format!("path: {}", path.display()))?;
        self.db
            .library()
            .get(song_id.0 as usize)
            .wrap_err("Could not get song from database")?
            .ok_or_eyre("Song id in playlist was not found in library")
            .with_note(|| format!("Song id: {song_id:?}"))
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

    #[instrument(skip(self))]
    pub fn add_to_queue(&mut self, path: &Path) -> Result<()> {
        let song_id = self
            .db
            .song_id_from_path()
            .get(path)
            .wrap_err("Could not read song_id lookup table")
            .and_then(|song_id| song_id.ok_or_eyre("Path is not in song_id lookup table"))
            .with_note(|| format!("path: {}", path.display()))?;
        dbg!("hihihihi");
        self.db
            .queue()
            .push(&song_id)
            .wrap_err("Could not append song_id to queue")?;
        dbg!("hihihihi");
        self.db.ds.flush().unwrap();
        dbg!(self.db.queue().iter().collect_vec());
        tracing::debug!("Adding {song_id:?} to queue");

        Ok(())
    }

    pub fn list_all_in(&self, dir: PathBuf) -> Result<Vec<ListItem>> {
        let mut paths = HashSet::new();
        for path in self
            .db
            .library()
            .iter()
            .map_ok(|song| song.file)
            .filter_ok(|path| path.starts_with(&dir))
        {
            let path = path.wrap_err("Error reading all library items from db")?;
            // annoyingly mpd's list all includes dirs... we dont store those
            // so create theme from the file paths here.
            paths.extend(path.parent().map(Path::to_owned).map(ListItem::Directory));
            paths.insert(ListItem::File(path));
        }
        Ok(paths.into_iter().collect_vec())
    }

    pub fn list_tags(&self, tag_to_list: &Tag) -> Result<String> {
        let mut list = self
            .db
            .library()
            .iter()
            .map_ok(|song| match tag_to_list {
                Tag::Album => song.album,
                Tag::AlbumArtist => "todo".to_string(),
                Tag::Artist => song.artist,
            })
            .collect::<Result<HashSet<_>, _>>()
            .wrap_err("Database error while iterating over library")?
            .into_iter()
            .sorted_unstable()
            .map(|tag_value| format!("{tag_to_list}: {tag_value}"))
            .join("\n");
        list.push('\n');
        Ok(list)
    }

    pub fn handle_find(&self, query: &Query) -> Result<Vec<FindResult>> {
        query::handle_find(self, query)
    }

    pub fn current_song(&self) -> Result<Option<PlaylistEntry>> {
        let Some(song_id) = self
            .db
            .queue()
            .get(0)
            .wrap_err("Could not get to item in queue")?
        else {
            return Ok(None);
        };

        let song = self
            .db
            .library()
            .get(song_id.0 as usize)
            .wrap_err("Could not get current song from library")?
            .ok_or_eyre("Item in the queue was not in the library")?;
        Ok(Some(PlaylistEntry::mostly_fake(0, song_id, song)))
    }

    pub fn stats(&self) -> Result<Stats> {
        #[derive(Default)]
        struct Counter {
            artists: HashSet<String>,
            albums: HashSet<String>,
            songs: usize,
            db_playtime: Duration,
        }

        let counter = self
            .db
            .library()
            .iter()
            .fold_ok(Counter::default(), |mut counter, song| {
                counter.artists.insert(song.artist);
                counter.albums.insert(song.album);
                counter.songs += 1;
                counter.db_playtime += song.playtime;
                counter
            })
            .wrap_err("Could not read items from db for counting")?;

        dbg!(self.queue()?);

        let playtime = self
            .queue()
            .wrap_err("Could not get queue")?
            .0
            .into_iter()
            .map(|entry| dbg!(entry).duration)
            .sum();

        Ok(Stats {
            artists: counter.artists.len(),
            albums: counter.albums.len(),
            songs: counter.songs,
            uptime: self.up_since.elapsed(),
            db_playtime: counter.db_playtime,
            db_update: self
                .db
                .last_db_update()
                .get()
                .wrap_err("Could not read last db update")?
                .unwrap_or_default(),
            playtime,
        })
    }
}

#[derive(Deserialize, Serialize)]
pub struct Song {
    pub file: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playtime: Duration,
}

#[cfg(test)]
mod tests {
    use crate::setup_tracing;

    use super::*;

    #[test]
    fn wtfwhynooo() {
        color_eyre::install().unwrap();
        setup_tracing();

        let mut system = System::new("~/Music".into(), None).unwrap();
        system
            .add_to_queue(Path::new(
                "The Sims Complete Collection/Disc 1/01 - Now Entering.mp3",
            ))
            .unwrap();

        let queue = system.queue().unwrap();
        let first = &queue.0[0];
        assert!(first.file.to_string_lossy().contains("Sims"));
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
