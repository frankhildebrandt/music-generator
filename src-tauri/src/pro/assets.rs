use super::model::{valid_id, Take};
use crate::{
    db::{safe_song_path, Database},
    error::{message, Result},
};
use std::path::{Path, PathBuf};

pub fn library_path(root: &Path, name: &str) -> Result<PathBuf> {
    let Some((folder, file)) = name.split_once('/') else {
        return safe_song_path(root, name);
    };
    let pid = folder
        .strip_prefix("pro-")
        .filter(|pid| valid_id(pid))
        .ok_or_else(|| message("Invalid project folder"))?;
    if !file.ends_with(".wav") || !valid_id(file.trim_end_matches(".wav")) {
        return Err(message("Invalid take filename"));
    }
    let directory = safe_song_path(root, &format!("pro-{pid}"))?;
    safe_song_path(&directory, file)
}
impl Database {
    pub fn pro_asset_names(&self) -> Result<Vec<String>> {
        let con = self.lock();
        let mut stmt = con.prepare("SELECT project_id,document FROM pro_takes")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        rows.map(|row| {
            let (pid, document) = row?;
            let take: Take = serde_json::from_str(&document)?;
            if !valid_id(&pid)
                || !take.file.ends_with(".wav")
                || !valid_id(take.file.trim_end_matches(".wav"))
            {
                return Err(message("Invalid project asset"));
            }
            Ok(format!("pro-{pid}/{}", take.file))
        })
        .collect()
    }
}
