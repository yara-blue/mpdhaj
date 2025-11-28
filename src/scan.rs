use std::path::Path;

use camino::{Utf8Path, Utf8PathBuf};
use futures::FutureExt;
use tokio::{
    fs::{self},
    task::spawn_blocking,
};

use color_eyre::{
    Result, Section,
    eyre::{Context, ContextCompat},
};

use crate::util::LogError;

mod lofty;
mod moosicbox_audiotags;

#[derive(Debug)]
pub struct MetaData {
    pub title: String,
    pub artist: String,
    pub album: String,
    // TODO: add other tags, genre/release date/etc.
    pub file: Utf8PathBuf,
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

// TODO: maybe just use walkdir if we need more features like traversing symlinks
pub async fn scan_dir(music_dir: &Utf8PathBuf, add_to_db: impl Fn(MetaData)) -> Result<()> {
    scan_dir_recurse(music_dir.as_std_path(), &add_to_db).await?;
    Ok(())
}

pub async fn scan_dir_recurse(music_dir: &Path, add_to_db: &impl Fn(MetaData)) -> Result<()> {
    let mut read_dir = fs::read_dir(&music_dir)
        .await
        .wrap_err("Could not read directory")
        .with_note(|| format!("directory: {}", music_dir.display()))?;

    // TODO convert into highly concurrent stream
    while let Some(entry) = read_dir.next_entry().await? {
        match entry.file_type().await {
            Ok(ty) if ty.is_dir() => {
                scan_dir_recurse(&entry.path(), add_to_db)
                    .boxed_local()
                    .await
            }
            Ok(ty) if ty.is_file() => {
                let path = entry.path();
                if let Some(metadata) =
                    scan_path(Utf8Path::from_path(&path).wrap_err("non-utf8 path")?).await
                {
                    add_to_db(metadata)
                }
                Ok(())
            }
            Ok(_neither_file_nor_dir) => Ok(()),
            Err(e) => Err(e)
                .wrap_err("Could not get file type for dir entry")
                .with_note(|| format!("dir entry: {}", entry.path().display())),
        }?;
    }

    Ok(())
}

// pub async fn scan_dir_recurse(
//     &self,
//     music_dir: &Path,
//     on_meta: &impl Fn(MetaData) -> Result<()>,
// ) -> Result<Pin<Box<dyn Stream<Item = Result<MetaData>>>>> {
//     let read_dir = fs::read_dir(&music_dir)
//         .await
//         .wrap_err("Could not read directory")
//         .with_note(|| format!("directory: {}", music_dir.display()))?;
//     let dir_stream = ReadDirStream::new(read_dir);
//     Ok(TryStreamExt::into_stream(dir_stream).map_ok(async |entry| {
//         match entry.file_type().await {
//             Ok(ty) if ty.is_dir() => self.scan_dir_recurse(&entry.path(), &on_meta).await,
//             Ok(ty) if ty.is_file() => Ok({
//                 todo!()
//                 // if let Some(meta) = self.scan(&entry.path()).await {
//                 //     stream::once(ready((on_meta)(meta))).boxed_local()
//                 // } else {
//                 //     stream::empty().boxed_local()
//                 // }
//             }),
//             Ok(_neither_file_nor_dir) => Ok({
//                 todo!()
//                 // tracing::debug!(
//                 //     "skipping entry ({}) that is neither file not dir",
//                 //     entry.path().display()
//                 // );
//                 // stream::empty().boxed_local()
//             }),
//             Err(e) => Err(e)
//                 .wrap_err("Could not get file type for dir entry")
//                 .with_note(|| format!("dir entry: {}", entry.path().display())),
//         }
//     }))
// }
