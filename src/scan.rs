use std::{ops::Deref, time::Duration};

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use jiff::Timestamp;
use rusqlite::{Connection, Transaction};
use tokio::task::spawn_blocking;
use tracing::{info, info_span, span, trace_span};

use crate::system::System;

mod lofty;
mod moosicbox_audiotags;

// TODO: this should probably just be the same struct as system::Song
// TODO: all fields should be optional instead of using the "unknown" string here, that should go in the protocol impl when they're None
#[derive(Debug)]
pub struct Metadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub file: Utf8PathBuf,
    pub playtime: Duration,
    // TODO: add other tags, genre/release date/etc.
}

pub const UNKNOWN: &str = "unknown";
trait FormatScanner: Send + Sync {
    fn scan(&self, path: Utf8PathBuf) -> Result<Option<Metadata>>;
}

// TODO scanners should augment eachoter (fill leftover None fields). That way
// the last scanner can be rodio reading the audio file duration.
const SCANNERS: &[&dyn FormatScanner] =
    &[&lofty::Scanner::new(), &moosicbox_audiotags::Scanner::new()];

#[tracing::instrument(level = "trace")]
pub async fn scan_path(path: &Utf8Path) -> Option<Metadata> {
    let path = path.to_path_buf();
    spawn_blocking(move || {
        SCANNERS.iter().filter_map(move |scanner| scanner.scan(path.clone()).ok().flatten()).next()
    })
    .await
    .expect("Scanning should never panic")
}

enum ScanResult {
    Cached,
    Updated,
    Added,
    NotASong,
}
async fn scan_song(
    db: &impl Deref<Target = Connection>,
    relpath: &Utf8Path,
    abspath: &Utf8Path,
    // TODO: just use number for this, no need to parse/make human readable
    mtime: Timestamp,
    generation: u32,
) -> Result<ScanResult> {
    let Ok((id, cached_mtime)) = trace_span!("path lookup").in_scope(|| {
        db.query_one("SELECT rowid, mtime FROM songs WHERE path = ?1", [relpath.as_str()], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
        })
    }) else {
        let Some(song_metadata) = scan_path(abspath).await else {
            return Ok(ScanResult::NotASong);
        };
        trace_span!("insertion").in_scope(|| {
            db.execute(
                "INSERT INTO songs (path, mtime, title, artist, album, generation)
                           VALUES (?1,   ?2,    ?3,    ?4,     ?5,    ?6)",
                (
                    relpath.as_str(),
                    mtime.to_string(),
                    song_metadata.title,
                    song_metadata.artist,
                    song_metadata.album,
                    generation,
                ),
            )
        })?;
        return Ok(ScanResult::Added);
    };

    if let Ok(cached_mtime) = cached_mtime.parse()
        && mtime != cached_mtime
        && let Some(song_metadata) = scan_path(abspath).await
    {
        trace_span!("update").in_scope(|| {
            db.execute(
                "UPDATE songs
                    SET mtime = ?2, title = ?3, artist = ?4, album = ?5, generation = ?6
                    WHERE rowid = ?1
                        ",
                (
                    id,
                    relpath.as_str(),
                    mtime.to_string(),
                    song_metadata.title,
                    song_metadata.artist,
                    song_metadata.album,
                    generation,
                ),
            )
        })?;
        Ok(ScanResult::Updated)
    } else {
        trace_span!("bump generation").in_scope(|| {
            db.execute("UPDATE songs SET generation = ?2 WHERE rowid = ?1", (id, generation))
        })?;
        Ok(ScanResult::Cached)
    }
}

impl System {
    pub async fn rescan(&mut self) -> Result<()> {
        let generation = self
            .db
            .query_one("SELECT generation FROM state", [], |row| Ok(row.get::<_, u32>(0)? + 1))?;
        let music_dir = &self.music_dir;
        let (mut cached, mut added, mut updated) = (0, 0, 0);
        let t = Transaction::new(&mut self.db, rusqlite::TransactionBehavior::Exclusive)?;
        for e in walkdir::WalkDir::new(music_dir) {
            if let Ok(e) = e
                && let Ok(metadata) = e.metadata()
                && !metadata.is_dir()
                && let Ok(Ok(mtime)) = metadata.modified().map(Timestamp::try_from)
                && let Some(abspath) = Utf8Path::from_path(e.path())
                && let Ok(relpath) = abspath.strip_prefix(music_dir)
            {
                match scan_song(&t, relpath, abspath, mtime, generation).await? {
                    ScanResult::Cached => cached += 1,
                    ScanResult::Added => added += 1,
                    ScanResult::Updated => updated += 1,
                    ScanResult::NotASong => {}
                }
            }
        }
        info_span!("commit scan transaction").in_scope(|| t.commit())?;
        let old_size =
            self.db.query_one("SELECT COUNT(*) FROM songs", [], |row| row.get::<_, usize>(0))?;
        self.db.execute("UPDATE state SET generation = ?1", [generation])?;
        self.db.execute("DELETE FROM songs WHERE generation < ?1", [generation])?;
        let new_size =
            self.db.query_one("SELECT COUNT(*) FROM songs", [], |row| row.get::<_, usize>(0))?;
        info!(
            "Scan complete: {new_size} songs - {cached} cached - {added} added - {updated} updated - {} removed",
            old_size - new_size
        );
        Ok(())
    }
}
