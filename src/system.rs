use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use strum::IntoEnumIterator;
use tokio::sync::mpsc;

use color_eyre::eyre::{Context, OptionExt};
use color_eyre::{Report, Result, Section};
use rodio::nz;

use crate::mpd_protocol::{
    self, AudioParams, PlayList, PlaybackState, PlaylistEntry, PlaylistId, PlaylistInfo, SongId,
    SongNumber, SubSystem, Tag, Volume,
};
use crate::playlist::{self, PlaylistName};
use crate::scan::{self, MetaData};
use crate::util::WhatItertoolsIsMissing;

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
}

pub struct System {
    db: State,
    playlists: HashMap<PlaylistName, Vec<PathBuf>>,
    idlers: HashMap<SubSystem, Vec<mpsc::Sender<SubSystem>>>,
    music_dir: PathBuf,
}

impl System {
    pub fn new(playlist_dir: &Path, music_dir: PathBuf) -> Result<Self> {
        let state = State::open_path("mpdhaj_database")
            .wrap_err("Could not open db")
            .note("path: mpdhaj_database")
            .suggestion(
                "The database format probably changed, \
                try removing the database folder",
            )?;
        let playlists =
            playlist::load_from_dir(playlist_dir).wrap_err("Failed to load playlists")?;
        Ok(System {
            db: state,
            playlists,
            idlers: Default::default(),
            music_dir,
        })
    }

    pub async fn scan(&mut self) -> Result<()> {
        // Song ids will change, reset db
        self.db.queue().clear().unwrap();
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
                })
                .unwrap();
        })
        .await?;

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
                channels: nz!(2)
            },
            error: "Failed to open \"usb dac attached to pi\" (alsa); Failed to open ALSA device \"hw:CARD=UD110v2,DEV=1\": No such device".to_string(),
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

    pub fn add_to_queue(&mut self, path: &Path) -> Result<()> {
        let song_id = self
            .db
            .song_id_from_path()
            .get(path)
            .wrap_err("Could not read song_id lookup table")
            .and_then(|song_id| song_id.ok_or_eyre("Path is not in song_id lookup table"))
            .with_note(|| format!("path: {}", path.display()))?;
        self.db
            .queue()
            .push(&song_id)
            .wrap_err("Could not append song_id to queue")
    }

    pub fn list_tags(&self, tag_to_list: &Tag) -> Result<String> {
        let mut list = self
            .db
            .library()
            .iter()
            .map_ok(|_song| match tag_to_list {
                Tag::Album => "todo".to_string(),
                Tag::AlbumArtist => "todo".to_string(),
                Tag::Artist => "todo".to_string(),
            })
            .map_ok(|tag_value| format!("{tag_to_list}: {tag_value}"))
            .collect::<Result<Vec<_>, _>>()
            .wrap_err("Database error while iterating over library")?
            .into_iter()
            .join("\n");
        list.push('\n');
        Ok(list)
    }
}

#[derive(Deserialize, Serialize)]
pub struct Song {
    pub file: PathBuf,
    pub title: String,
    pub artist: String,
}
