use crate::scan::{FormatScanner, Metadata, UNKNOWN};
use camino::Utf8PathBuf;
use color_eyre::{Result, Section, eyre::Context};
use lofty::{
    file::{AudioFile, TaggedFileExt},
    probe::read_from_path,
    tag::Accessor,
};

pub struct Scanner;

impl Scanner {
    pub const fn new() -> Self {
        Scanner
    }
}

impl FormatScanner for Scanner {
    fn scan(&self, path: Utf8PathBuf) -> Result<Option<Metadata>> {
        let tagged_file = read_from_path(&path)
            .wrap_err("Could not open file for reading metadata")
            .with_note(|| format!("path is: {path}"))?;

        let Some(tag) = tagged_file.primary_tag() else {
            return Ok(None);
        };

        let playtime = tagged_file.properties().duration();

        Ok(Some(Metadata {
            title: tag.title().unwrap_or(UNKNOWN.into()).to_string(),
            file: path,
            artist: tag.artist().unwrap_or(UNKNOWN.into()).to_string(),
            album: tag.album().unwrap_or(UNKNOWN.into()).to_string(),
            playtime,
        }))
    }
}
