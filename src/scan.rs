use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use tokio::task::spawn_blocking;

use crate::util::LogError;

mod lofty;
mod moosicbox_audiotags;

#[derive(Debug)]
pub struct MetaData {
    pub title: String,
    pub artist: String,
    pub album: String,
    // TODO: add other tags, genre/release date/etc.
    pub file: Utf8PathBuf, // TODO: remove
}

pub const UNKNOWN: &str = "unknown";
trait FormatScanner: Send + Sync {
    fn scan(&self, path: Utf8PathBuf) -> Result<Option<MetaData>>;
}

const SCANNERS: &[&dyn FormatScanner] =
    &[&lofty::Scanner::new(), &moosicbox_audiotags::Scanner::new()];

pub async fn scan_path(path: &Utf8Path) -> Option<MetaData> {
    let path = path.to_path_buf();
    spawn_blocking(move || {
        SCANNERS
            .iter()
            .filter_map(move |scanner| scanner.scan(path.clone()).log_error().ok().flatten())
            .next()
    })
    .await
    .expect("Scanning should never panic")
}
