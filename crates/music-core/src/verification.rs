use crate::error::{message, Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Instant, UNIX_EPOCH},
};
use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;

#[derive(Debug, PartialEq, Eq)]
pub enum Verification {
    Cached,
    Hashed,
    Invalid,
}

#[derive(Serialize, Deserialize, Default)]
struct Entries {
    entries: HashMap<String, (String, u64, u128)>,
}

pub struct ModelVerifier {
    file: PathBuf,
    cache: Entries,
}

fn stamp(path: &Path) -> Result<(u64, u128)> {
    let metadata = path.metadata()?;
    if !metadata.is_file() {
        return Err(message("Model is not a regular file"));
    }
    Ok((
        metadata.len(),
        metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map_err(|e| message(e.to_string()))?
            .as_nanos(),
    ))
}

impl ModelVerifier {
    pub fn open(root: &Path) -> Self {
        let file = root.join("verified-models.json");
        let cache = std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self { file, cache }
    }

    // Persist each successful check, so retries and subsequent launches reuse completed work.
    pub async fn verify(
        &mut self,
        path: &Path,
        expected: &str,
        cancel: &CancellationToken,
        progress: impl Fn(u64, u64),
    ) -> Result<Verification> {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let path = path.canonicalize()?;
        let (length, modified) = stamp(&path)?;
        let key = path.to_string_lossy().into_owned();
        let entry = (expected.to_owned(), length, modified);
        if self.cache.entries.get(&key) == Some(&entry) {
            return Ok(Verification::Cached);
        }
        let valid = hash_file_with_progress(&path, cancel, progress).await? == expected;
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        if stamp(&path)? != (length, modified) {
            return Err(message("Model changed during verification; retry"));
        }
        if !valid {
            if self.cache.entries.remove(&key).is_some() {
                self.save()?;
            }
            return Ok(Verification::Invalid);
        }
        self.cache.entries.insert(key, entry);
        self.save()?;
        Ok(Verification::Hashed)
    }

    // Preserve the checksum proof when a verified partial download becomes a model file.
    pub(crate) fn promote(&mut self, partial: &Path, target: &Path) -> Result<()> {
        let key = partial.canonicalize()?.to_string_lossy().into_owned();
        let entry = self
            .cache
            .entries
            .get(&key)
            .cloned()
            .ok_or_else(|| message("Download has not been verified"))?;
        if stamp(partial)? != (entry.1, entry.2) {
            return Err(message("Download changed after verification"));
        }
        std::fs::rename(partial, target)?;
        self.cache.entries.remove(&key);
        self.cache
            .entries
            .insert(target.canonicalize()?.to_string_lossy().into_owned(), entry);
        self.save()
    }

    fn save(&self) -> Result<()> {
        let temporary = self
            .file
            .with_extension(format!("{}.tmp", crate::types::id()));
        std::fs::write(&temporary, serde_json::to_vec(&self.cache)?)?;
        std::fs::rename(temporary, &self.file)?;
        Ok(())
    }
}

async fn hash_file_with_progress(
    path: &Path,
    cancel: &CancellationToken,
    progress: impl Fn(u64, u64),
) -> Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let total = file.metadata().await?.len();
    let mut buffer = vec![0; 4 * 1024 * 1024];
    let mut hash = Sha256::new();
    let mut read = 0;
    let mut last = Instant::now();
    progress(0, total);
    loop {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let size = file.read(&mut buffer).await?;
        if size == 0 {
            break;
        }
        hash.update(&buffer[..size]);
        read += size as u64;
        if last.elapsed().as_millis() >= 200 {
            progress(read, total);
            last = Instant::now();
        }
    }
    progress(read, total);
    Ok(format!("{:x}", hash.finalize()))
}
