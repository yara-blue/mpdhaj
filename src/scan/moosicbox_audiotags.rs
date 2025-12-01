use std::time::Duration;

use camino::Utf8PathBuf;
use rodio::Source;

use crate::scan::Metadata;
use crate::scan::{FormatScanner, UNKNOWN};
use color_eyre::{Result, Section, eyre::Context};
use moosicbox_audiotags::{Error, Tag};

pub struct Scanner;

impl Scanner {
    pub const fn new() -> Self {
        Scanner
    }
}

impl FormatScanner for Scanner {
    fn scan(&self, path: Utf8PathBuf) -> Result<Option<Metadata>> {
        let tag = match Tag::new().read_from_path(&path) {
            Ok(tag) => tag,
            Err(
                Error::UnknownFileExtension(_)
                | Error::UnsupportedFormat(_)
                | Error::UnsupportedMimeType(_),
            ) => return Ok(None),
            Err(other) => {
                return Err(other)
                    .wrap_err("Could not parse metadata")
                    .with_note(|| format!("path: {path}"));
            }
        };

        let playtime = if let Some(duration) = tag.duration().map(Duration::from_secs_f64) {
            duration
        } else {
            let file = std::fs::File::open(&path).wrap_err("Could not open file")?;
            let source = rodio::Decoder::try_from(file).wrap_err("Can not decode music file")?;
            source.total_duration().unwrap_or_default()
        };

        Ok(Some(Metadata {
            title: tag.title().unwrap_or(UNKNOWN).to_string(),
            file: path,
            artist: tag.artist().unwrap_or(UNKNOWN).to_string(),
            album: tag.album().map(|album| album.title).unwrap_or(UNKNOWN).to_string(),
            playtime,
        }))
    }
}
