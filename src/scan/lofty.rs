use std::path::PathBuf;

use crate::scan::{FormatScanner, MetaData, UNKNOWN};
use color_eyre::{Result, Section, eyre::Context};
use lofty::{file::TaggedFileExt, probe::read_from_path, tag::Accessor};

pub struct Scanner;

impl Scanner {
    pub const fn new() -> Self {
        Scanner
    }
}

impl FormatScanner for Scanner {
    fn scan(&self, path: PathBuf) -> Result<Option<MetaData>> {
        let tagged_file = read_from_path(&path)
            .wrap_err("Could not open file for reading metadata")
            .with_note(|| format!("path is: {}", path.display()))?;

        let Some(tag) = tagged_file.primary_tag() else {
            return Ok(None);
        };

        Ok(Some(MetaData {
            title: tag.title().unwrap_or(UNKNOWN.into()).to_string(),
            file: path,
            artist: tag.artist().unwrap_or(UNKNOWN.into()).to_string(),
        }))
    }
}
