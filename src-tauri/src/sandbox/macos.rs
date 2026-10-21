use crate::error::{message, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use objc2::{rc::Retained, runtime::Bool};
use objc2_foundation::{
    NSData, NSHomeDirectory, NSString, NSURLBookmarkCreationOptions as Creation,
    NSURLBookmarkResolutionOptions as Resolution, NSURL,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

struct Bookmarks {
    file: PathBuf,
    data: BTreeMap<String, Vec<u8>>,
    active: Vec<Retained<NSURL>>,
}
static BOOKMARKS: OnceLock<Mutex<Bookmarks>> = OnceLock::new();

pub fn data_root() -> PathBuf {
    #[cfg(debug_assertions)]
    if let Some(path) = std::env::var_os("MUSIC_GENERATOR_PROFILE")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        return path;
    }
    PathBuf::from(NSHomeDirectory().to_string())
        .join("Library/Application Support/de.alphatester.music-generator")
}

// Restore user-selected directory grants before touching models or the library.
pub fn initialize(root: &Path) -> Result<()> {
    let file = root.join("folder-bookmarks.json");
    let mut data: BTreeMap<String, Vec<u8>> = if file.exists() {
        serde_json::from_slice(&std::fs::read(&file)?)?
    } else {
        BTreeMap::new()
    };
    let mut active = Vec::new();
    let mut invalid = Vec::new();
    for (path, bytes) in &mut data {
        let mut stale = Bool::NO;
        let result = unsafe {
            NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
                &NSData::with_bytes(bytes),
                Resolution::WithSecurityScope | Resolution::WithoutUI,
                None,
                &mut stale,
            )
        };
        match result {
            Ok(url) if unsafe { url.startAccessingSecurityScopedResource() } => {
                if stale.as_bool() {
                    *bytes = bookmark(&url, Creation::WithSecurityScope)?;
                }
                active.push(url);
            }
            _ => {
                tracing::warn!(folder=%path, "Folder access could not be restored; select it again in the native dialog");
                invalid.push(path.clone());
            }
        }
    }
    for path in invalid {
        data.remove(&path);
    }
    std::fs::write(&file, serde_json::to_vec(&data)?)?;
    let _ = BOOKMARKS.set(Mutex::new(Bookmarks { file, data, active }));
    Ok(())
}

fn bookmark(url: &NSURL, options: Creation) -> Result<Vec<u8>> {
    url.bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
        options, None, None,
    )
    .map(|data| data.to_vec())
    .map_err(|e| {
        message(format!(
            "Select this folder using Browse to allow access: {e}"
        ))
    })
}

// A successful native folder selection grants session access; persist it for subsequent launches.
pub fn remember(path: &Path) -> Result<()> {
    let Some(store) = BOOKMARKS.get() else {
        return Ok(());
    };
    if path.starts_with(data_root()) {
        return Ok(());
    }
    let mut store = store.lock().unwrap();
    let key = path.to_string_lossy().into_owned();
    if store.data.contains_key(&key) {
        return Ok(());
    }
    let url = NSURL::fileURLWithPath(&NSString::from_str(&key));
    let bytes = bookmark(&url, Creation::WithSecurityScope)?;
    if unsafe { url.startAccessingSecurityScopedResource() } {
        store.active.push(url);
    }
    store.data.insert(key, bytes);
    let temp = store.file.with_extension("json.tmp");
    std::fs::write(&temp, serde_json::to_vec(&store.data)?)?;
    std::fs::rename(temp, &store.file)?;
    Ok(())
}

// Implicit security-scoped bookmarks transfer the parent's live Powerbox grant to the engine.
pub fn transfer(path: &Path) -> Result<String> {
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    Ok(STANDARD.encode(bookmark(&url, Creation::empty())?))
}

pub fn shutdown() {
    if let Some(store) = BOOKMARKS.get() {
        for url in store.lock().unwrap().active.drain(..) {
            unsafe {
                url.stopAccessingSecurityScopedResource();
            }
        }
    }
}
