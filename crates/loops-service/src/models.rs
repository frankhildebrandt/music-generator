use music_core::error::{message, Result};
use std::path::{Path, PathBuf};

const APP_ID: &str = "de.alphatester.music-generator";
const REQUIRED: [&str; 3] = [
    music_core::loops::BASE_MODEL,
    "Qwen3-Embedding-0.6B-Q8_0.gguf",
    "vae-BF16.gguf",
];

pub fn app_roots() -> Result<Vec<PathBuf>> {
    if let Some(profile) = std::env::var_os("MUSIC_GENERATOR_PROFILE")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        return Ok(vec![profile]);
    }
    #[cfg(windows)]
    {
        let local = std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| message("Missing LOCALAPPDATA for shared app models"))?;
        let mut folders = vec![PathBuf::from(local).join(APP_ID)];
        if let Some(roaming) = std::env::var_os("APPDATA") {
            folders.push(PathBuf::from(roaming).join(APP_ID));
        }
        Ok(folders)
    }
    #[cfg(not(windows))]
    {
        let home = PathBuf::from(
            std::env::var_os("HOME")
                .ok_or_else(|| message("Missing HOME for shared app models"))?,
        );
        Ok(vec![
            home.join("Library/Containers")
                .join(APP_ID)
                .join("Data/Library/Application Support")
                .join(APP_ID),
            home.join("Library/Application Support").join(APP_ID),
        ])
    }
}

fn configured(app: &Path) -> Option<PathBuf> {
    let db = rusqlite::Connection::open_with_flags(
        app.join("music.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .ok()?;
    let value: String = db
        .query_row(
            "SELECT value FROM settings WHERE key='models_dir'",
            [],
            |row| row.get(0),
        )
        .ok()?;
    let path = PathBuf::from(serde_json::from_str::<String>(&value).unwrap_or(value));
    (path.is_absolute() && path.is_dir()).then_some(path)
}

/// Reuse the app's model directory, with the app's default as download destination.
pub fn resolve_from(apps: &[PathBuf]) -> PathBuf {
    let mut best = apps[0].join("models");
    let mut found = false;
    let mut best_count = 0;
    for app in apps {
        for path in configured(app)
            .into_iter()
            .chain(std::iter::once(app.join("models")))
        {
            let count = REQUIRED
                .iter()
                .filter(|file| path.join(file).is_file())
                .count();
            if !found || count > best_count {
                best = path;
                best_count = count;
                found = true;
            }
        }
    }
    best
}

/// Refresh after app installation/settings changes without interrupting active jobs.
pub async fn refresh(service: &crate::Service) -> Result<()> {
    let Ok(_permit) = service.serial.try_acquire() else {
        return Ok(());
    };
    let path = resolve_from(&app_roots()?);
    let mut current = service.models.lock().await;
    if *current != path {
        service.engine.lock().await.take();
        *current = path;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn models(path: &Path, count: usize) {
        std::fs::create_dir_all(path).unwrap();
        for name in REQUIRED.iter().take(count) {
            std::fs::write(path.join(name), b"fixture").unwrap();
        }
    }
    #[test]
    fn empty_primary_profile_does_not_hide_existing_app_downloads() {
        let root = tempfile::tempdir().unwrap();
        let primary = root.path().join("sandbox");
        let legacy = root.path().join("Application Support");
        models(&primary.join("models"), 0);
        models(&legacy.join("models"), 3);
        assert_eq!(
            resolve_from(&[primary, legacy.clone()]),
            legacy.join("models")
        );
    }
    #[test]
    fn reuses_custom_model_folder_saved_by_the_app() {
        let root = tempfile::tempdir().unwrap();
        let app = root.path().join("AppData/Local").join(APP_ID);
        let custom = root.path().join("Eigene Modelle ä");
        std::fs::create_dir_all(&app).unwrap();
        models(&custom, 3);
        let db = rusqlite::Connection::open(app.join("music.db")).unwrap();
        db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT);")
            .unwrap();
        db.execute(
            "INSERT INTO settings VALUES('models_dir',?1)",
            [serde_json::to_string(&custom.to_string_lossy()).unwrap()],
        )
        .unwrap();
        assert_eq!(resolve_from(&[app]), custom);
    }
    #[test]
    fn downloads_to_app_data_without_creating_an_app_database() {
        let root = tempfile::tempdir().unwrap();
        let app = root.path().join("AppData/Local").join(APP_ID);
        assert_eq!(resolve_from(std::slice::from_ref(&app)), app.join("models"));
        assert!(!app.exists());
    }
    #[test]
    fn stale_or_relative_app_setting_falls_back_to_app_models() {
        let root = tempfile::tempdir().unwrap();
        let app = root.path().join("app");
        models(&app.join("models"), 2);
        let db = rusqlite::Connection::open(app.join("music.db")).unwrap();
        db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT); INSERT INTO settings VALUES('models_dir','models');").unwrap();
        assert_eq!(resolve_from(std::slice::from_ref(&app)), app.join("models"));
        db.execute(
            "UPDATE settings SET value=?1",
            [root.path().join("removed").to_string_lossy().as_ref()],
        )
        .unwrap();
        assert_eq!(resolve_from(std::slice::from_ref(&app)), app.join("models"));
    }
}
