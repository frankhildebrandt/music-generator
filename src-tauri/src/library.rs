use crate::{
    db::{safe_song_path, Database},
    error::{message, Result},
    types::{id, now},
};
use rusqlite::params;
use serde_json::{json, Value};
use std::{collections::HashSet, path::Path};

impl Database {
    pub fn folders(&self) -> Result<Value> {
        let con = self.lock();
        let mut stmt = con.prepare("SELECT f.id,f.name,(SELECT COUNT(*) FROM songs s WHERE s.folder_id=f.id) FROM folders f ORDER BY name COLLATE NOCASE")?;
        let folders = stmt.query_map([], |r| Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"count":r.get::<_,u32>(2)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let all: u32 = con.query_row("SELECT COUNT(*) FROM songs", [], |r| r.get(0))?;
        let none: u32 = con.query_row("SELECT COUNT(*) FROM songs WHERE folder_id IS NULL OR folder_id NOT IN (SELECT id FROM folders)", [], |r| r.get(0))?;
        Ok(json!({"folders":folders,"all":all,"none":none}))
    }

    pub fn folder_save(&self, fid: Option<&str>, name: &str) -> Result<Value> {
        let name: String = name.trim().chars().take(60).collect();
        if name.is_empty() {
            return Err(message("Please enter a folder name"));
        }
        let fid = fid.map(str::to_string).unwrap_or_else(id);
        self.lock().execute("INSERT INTO folders(id,name,created_at) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET name=excluded.name", params![fid,name,now()])?;
        Ok(json!({"id":fid,"name":name}))
    }

    pub fn folder_delete(&self, fid: &str) -> Result<()> {
        let mut con = self.lock();
        let tx = con.transaction()?;
        tx.execute("UPDATE songs SET folder_id=NULL WHERE folder_id=?1", [fid])?;
        tx.execute("DELETE FROM folders WHERE id=?1", [fid])?;
        tx.commit()?;
        Ok(())
    }

    pub fn queue(&self) -> Result<Value> {
        let dir = self.settings()?.songs_dir;
        let con = self.lock();
        let mut stmt = con.prepare("SELECT id,status,message,caption FROM songs WHERE status IN ('queued','running') ORDER BY created_at,rowid")?;
        let rows = stmt.query_map([], |r| Ok(json!({"id":r.get::<_,String>(0)?,"status":r.get::<_,String>(1)?,"message":r.get::<_,Option<String>>(2)?,"caption":r.get::<_,String>(3)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let bytes: u64 = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|p| p.metadata().ok())
            .filter(|m| m.is_file())
            .map(|m| m.len())
            .sum();
        let count: u64 = con.query_row(
            "SELECT COUNT(*) FROM songs WHERE file IS NOT NULL",
            [],
            |r| r.get(0),
        )?;
        Ok(
            json!({"running":rows.iter().filter(|s| s["status"]=="running").collect::<Vec<_>>(),"queued":rows.iter().filter(|s| s["status"]=="queued").count(),"library":{"count":count,"bytes":bytes}}),
        )
    }

    pub fn change_songs_dir(&self, new: &Path) -> Result<()> {
        if !new.is_absolute() {
            return Err(message("Please select an absolute folder"));
        }
        if self.busy()? {
            return Err(message(
                "Finish or cancel queued songs before changing folders",
            ));
        }
        std::fs::create_dir_all(new)?;
        let old = std::path::PathBuf::from(self.settings()?.songs_dir);
        if old.canonicalize().ok() == new.canonicalize().ok() {
            return Ok(());
        }
        let con = self.lock();
        let mut stmt = con.prepare("SELECT file FROM songs WHERE file IS NOT NULL")?;
        let files = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(stmt);
        drop(con);
        let mut copied = Vec::new();
        let result = (|| -> Result<()> {
            for name in &files {
                let src = safe_song_path(&old, name)?;
                let dest = safe_song_path(new, name)?;
                if dest.exists() {
                    return Err(message(format!("Destination already contains {name}")));
                }
                if src.is_file() {
                    std::fs::copy(&src, &dest)?;
                    copied.push(dest);
                }
            }
            let mut settings = self.settings()?;
            settings.songs_dir = new.to_string_lossy().into();
            self.save_settings(&settings)?;
            Ok(())
        })();
        if result.is_err() {
            for path in copied {
                let _ = std::fs::remove_file(path);
            }
        } else {
            for name in files {
                if let Ok(path) = safe_song_path(&old, &name) {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        result
    }

    pub fn export_audio(&self, sid: &str, destination: &Path) -> Result<()> {
        let song = self.song(sid)?;
        let path = song.path.ok_or_else(|| message("Song has no audio file"))?;
        if Path::new(&path).canonicalize()? == destination.canonicalize().unwrap_or_default() {
            return Err(message(
                "Choose a destination outside the source audio file",
            ));
        }
        std::fs::copy(path, destination)?;
        Ok(())
    }

    pub fn export_lyrics(&self, sid: &str, destination: &Path) -> Result<()> {
        let song = self.song(sid)?;
        let lyrics = song
            .result_meta
            .as_ref()
            .and_then(|m| m["lyrics"].as_str())
            .or(song.lyrics.as_deref())
            .unwrap_or("");
        std::fs::write(destination, lyrics)?;
        Ok(())
    }

    pub fn export_zip(&self, folder: &str, destination: &Path) -> Result<()> {
        // Export all matching rows, including songs outside the UI's display limit.
        let con = self.lock();
        let mut stmt = con.prepare("SELECT id FROM songs WHERE status='done' AND (?1='' OR folder_id=?1 OR (?1='none' AND (folder_id IS NULL OR folder_id NOT IN (SELECT id FROM folders)))) ORDER BY created_at,rowid")?;
        let ids = stmt
            .query_map([folder], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(stmt);
        drop(con);
        if ids.is_empty() {
            return Err(message("No finished songs to export"));
        }
        let temp = destination.with_extension(format!("{}.part", id()));
        let outcome = (|| -> Result<()> {
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&temp)?);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            let mut names = HashSet::new();
            for sid in ids {
                let song = self.song(&sid)?;
                let Some(path) = song.path else {
                    continue;
                };
                let mut file = std::fs::File::open(&path)?;
                let title = clean_name(song.title.as_deref().unwrap_or(&song.caption));
                let base = format!(
                    "{}{}",
                    title,
                    if song.letter.is_empty() {
                        String::new()
                    } else {
                        format!(" {}", song.letter)
                    }
                );
                let ext = Path::new(&path)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("wav");
                let mut name = format!("{base}.{ext}");
                if !names.insert(name.to_lowercase()) {
                    name = format!("{base}-{}.{ext}", song.id);
                    names.insert(name.to_lowercase());
                }
                zip.start_file(name, options)
                    .map_err(|e| message(e.to_string()))?;
                std::io::copy(&mut file, &mut zip)?;
            }
            zip.finish()
                .map_err(|e| message(e.to_string()))?
                .sync_all()?;
            std::fs::rename(&temp, destination)?;
            Ok(())
        })();
        if outcome.is_err() {
            let _ = std::fs::remove_file(temp);
        }
        outcome
    }
}

pub fn clean_name(name: &str) -> String {
    let clean: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || [' ', '-', '_'].contains(c))
        .take(80)
        .collect();
    if clean.trim().is_empty() {
        "Song".into()
    } else {
        clean.trim().into()
    }
}
