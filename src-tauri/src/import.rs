use crate::{
    db::Database,
    error::{message, Result},
    types::id,
};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

// Import from a SQLite backup, so WAL data is included without editing the old installation.
pub fn import_legacy(db: &Database, root: &Path) -> Result<Value> {
    if !db.pro_projects()?.is_empty()
        || db
            .lock()
            .query_row("SELECT COUNT(*) FROM songs", [], |r| r.get::<_, u32>(0))?
            > 0
        || db
            .lock()
            .query_row("SELECT COUNT(*) FROM folders", [], |r| r.get::<_, u32>(0))?
            > 0
    {
        return Err(message("Import requires an empty library"));
    }
    let desktop = root.join("music.db").is_file();
    let data = if desktop {
        root.to_path_buf()
    } else {
        root.join("data")
    };
    let source = data.join("music.db");
    let old = Connection::open_with_flags(&source, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let custom: Option<String> = old
        .query_row(
            "SELECT value FROM settings WHERE key='songs_dir'",
            [],
            |r| r.get::<_, String>(0),
        )
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());
    let songs_source = custom
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| data.join("songs"));
    let models_source = if desktop {
        old.query_row(
            "SELECT value FROM settings WHERE key='models_dir'",
            [],
            |r| r.get::<_, String>(0),
        )
        .ok()
        .and_then(|s| serde_json::from_str::<String>(&s).ok())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("models"))
    } else {
        root.join("engine/acestep.cpp/models")
    };
    if !songs_source.is_absolute() {
        return Err(message("Legacy song folder must be absolute"));
    }
    let staging = db.root.join(format!("import-{}", id()));
    std::fs::create_dir_all(&staging)?;
    let mut copied = Vec::new();
    let result = (|| -> Result<Value> {
        let mut backup = Connection::open(staging.join("music.db"))?;
        rusqlite::backup::Backup::new(&old, &mut backup)?.run_to_completion(
            100,
            std::time::Duration::from_millis(10),
            None,
        )?;
        drop(backup);
        let staged = Database::open(&staging)?;
        let con = staged.lock();
        for table in ["songs", "folders"] {
            let mut ids = con.prepare(&format!("SELECT id FROM {table}"))?;
            for id in ids.query_map([], |r| r.get::<_, String>(0))? {
                let id = id?;
                if id.is_empty()
                    || !id
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                {
                    return Err(message("Legacy library contains invalid identifiers"));
                }
            }
        }
        let mut stmt = con.prepare("SELECT file FROM songs WHERE file IS NOT NULL")?;
        let mut files = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(stmt);
        drop(con);
        files.extend(staged.pro_asset_names()?);
        let dest = PathBuf::from(db.settings()?.songs_dir);
        std::fs::create_dir_all(&dest)?;
        let bytes = files
            .iter()
            .map(|f| {
                crate::pro::assets::library_path(&songs_source, f)
                    .and_then(|p| Ok(p.metadata()?.len()))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .sum::<u64>();
        crate::download::ensure_space(&dest, bytes + 64 * 1024 * 1024)?;
        for name in &files {
            let to = crate::pro::assets::library_path(&dest, name)?;
            if to.exists() {
                return Err(message(format!("Song destination already contains {name}")));
            }
            let from = crate::pro::assets::library_path(&songs_source, name)?;
            std::fs::create_dir_all(to.parent().unwrap())?;
            copied.push(to.clone());
            std::fs::copy(from, to)?;
        }
        drop(staged);
        let mut con = db.lock();
        con.execute(
            "ATTACH DATABASE ?1 AS legacy",
            [staging.join("music.db").to_string_lossy().as_ref()],
        )?;
        let imported = (|| -> Result<usize> {
            let tx = con.transaction()?;
            let count = tx.execute("INSERT INTO songs(id,group_id,created_at,finished_at,status,message,title,caption,lyrics,seed,params,result_meta,engine,file,favorite,folder_id,started_at) SELECT id,group_id,created_at,finished_at,status,message,title,caption,lyrics,seed,params,result_meta,engine,file,favorite,folder_id,started_at FROM legacy.songs ORDER BY rowid", [])?;
            tx.execute(
                "INSERT INTO folders SELECT id,name,created_at FROM legacy.folders",
                [],
            )?;
            for table in ["pro_projects", "pro_takes", "pro_jobs"] {
                tx.execute(
                    &format!("INSERT INTO {table} SELECT * FROM legacy.{table}"),
                    [],
                )?;
            }
            tx.commit()?;
            Ok(count)
        })();
        con.execute_batch("DETACH DATABASE legacy")?;
        let count = imported?;
        Ok(json!({"songs":count,"models_dir":models_source.to_string_lossy()}))
    })();
    if result.is_err() {
        for path in copied {
            let _ = std::fs::remove_file(path);
        }
    }
    let _ = std::fs::remove_dir_all(staging);
    result
}
