use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use color_eyre::{
    Result, Section,
    eyre::{Context, OptionExt},
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct PlaylistName(pub String);

pub fn load_from_dir(path: &Path) -> Result<HashMap<PlaylistName, Vec<PathBuf>>> {
    fs::read_dir(path)
        .wrap_err("Could not read playlist dir")?
        .map(|entry| entry.wrap_err("Could not read entry in playlist dir"))
        .map_ok(|entry| entry.path())
        .filter_ok(|path| path.is_file())
        .map_ok(|path| load_file(&path))
        .flatten()
        .collect()
}

fn load_file(path: &Path) -> Result<(PlaylistName, Vec<PathBuf>)> {
    let entries = fs::read_to_string(path)
        .wrap_err("Failed to read playlist from disk")
        .with_note(|| format!("path: {}", path.display()))?
        .lines()
        .map(|l| {
            PathBuf::from_str(l)
                .wrap_err("Entry in playlist is not a path")
                .with_note(|| format!("Entry: {l}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        PlaylistName(path.file_name()
            .ok_or_eyre("Playlist file did not have a name")
            .with_note(|| format!("path: {}", path.display()))?
            .to_string_lossy()
            .to_string()),
        entries,
    ))
}
