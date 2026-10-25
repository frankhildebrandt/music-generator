use crate::{
    db::{safe_song_path, Database},
    error::{message, Error, Result},
    types::Settings,
};
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

/// Remove only files this attempt created if copying or the SQLite commit fails.
#[derive(Default)]
struct Copies {
    paths: Vec<PathBuf>,
    committed: bool,
}
impl Drop for Copies {
    fn drop(&mut self) {
        if !self.committed {
            for path in &self.paths {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

pub(super) fn commit(db: &Database, settings: &Settings, cancel: &CancellationToken) -> Result<()> {
    let old = PathBuf::from(db.settings()?.songs_dir);
    let new = Path::new(&settings.songs_dir);
    let same_directory = matches!(
        (old.canonicalize(), new.canonicalize()),
        (Ok(old), Ok(new)) if old == new
    );
    let moving = old != new && !same_directory;
    let mut copies = Copies::default();
    let mut originals = Vec::new();
    if moving {
        if db.busy()? {
            return Err(message(
                "Finish or cancel queued songs before changing folders",
            ));
        }
        std::fs::create_dir_all(new)?;
        for file in db.audio_files()? {
            if cancel.is_cancelled() {
                return Err(Error::Cancelled);
            }
            let source = safe_song_path(&old, &file)?;
            let destination = safe_song_path(new, &file)?;
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&destination)?;
            copies.paths.push(destination);
            let mut input = std::fs::File::open(&source)?;
            std::io::copy(&mut input, &mut output)?;
            output.sync_all()?;
            originals.push(source);
        }
    }
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    db.save_settings(settings)?;
    copies.committed = true;
    for original in originals {
        let _ = std::fs::remove_file(original);
    }
    Ok(())
}
