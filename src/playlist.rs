use std::{collections::HashMap, fs};

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::{
    Result, Section,
    eyre::{Context, ContextCompat, OptionExt},
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct PlaylistName(pub String);

// TODO: use walkdir to handle nested playlist dirs
// TODO: return valid playlists even when error is encountered
pub fn load_from_dir(path: &Utf8Path) -> Result<HashMap<PlaylistName, Vec<Utf8PathBuf>>> {
    fs::read_dir(path)
        .wrap_err("Could not read playlist dir")?
        .map_ok(|e| e.path())
        .filter_ok(|p| p.is_file())
        .map_ok(|p| {
            Utf8Path::from_path(&p)
                .wrap_err("non-utf8 path")
                .and_then(load_file)
        })
        .flatten()
        .collect()
}

fn load_file(path: &Utf8Path) -> Result<(PlaylistName, Vec<Utf8PathBuf>)> {
    let entries = fs::read_to_string(path)
        .wrap_err("Failed to read playlist from disk")
        .with_note(|| format!("path: {path}"))?
        .lines()
        .map(|l| l.to_owned().into())
        .collect();
    Ok((
        PlaylistName(
            path.file_name()
                .ok_or_eyre("Playlist file did not have a name")
                .with_note(|| format!("path: {path}"))?
                .to_string(),
        ),
        entries,
    ))
}
