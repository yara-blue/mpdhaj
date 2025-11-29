use crate::scan::{FormatScanner, Metadata, UNKNOWN};

use camino::Utf8PathBuf;
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

        Ok(Some(Metadata {
            title: tag.title().unwrap_or(UNKNOWN).to_string(),
            artist: tag.artist().unwrap_or(UNKNOWN).to_string(),
            album: tag
                .album()
                .map(|album| album.title)
                .unwrap_or(UNKNOWN)
                .to_string(),
        }))
    }
}
