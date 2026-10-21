use super::{audio::Audio, model::*};
use crate::{
    db::Database,
    error::{message, Result},
    types::{id, now},
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn load(con: &Connection, pid: &str) -> Result<Project> {
    let text: String = con.query_row(
        "SELECT document FROM pro_projects WHERE id=?1",
        [pid],
        |r| r.get(0),
    )?;
    let mut project: Project = serde_json::from_str(&text)?;
    if project.id != pid {
        return Err(message(
            "Project identity does not match its saved document",
        ));
    }
    let mut stmt =
        con.prepare("SELECT document FROM pro_takes WHERE project_id=?1 ORDER BY rowid")?;
    project.takes = stmt
        .query_map([pid], |r| r.get::<_, String>(0))?
        .map(|r| Ok(serde_json::from_str(&r?)?))
        .collect::<Result<_>>()?;
    project.validate()?;
    Ok(project)
}
fn document(project: &Project) -> Result<String> {
    let mut copy = project.clone();
    copy.takes.clear();
    Ok(serde_json::to_string(&copy)?)
}
fn history(con: &Connection, pid: &str) -> Result<(Vec<Project>, Vec<Project>)> {
    let (undo, redo): (String, String) = con.query_row(
        "SELECT undo,redo FROM pro_projects WHERE id=?1",
        [pid],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok((serde_json::from_str(&undo)?, serde_json::from_str(&redo)?))
}
fn push(stack: &mut Vec<Project>, project: &Project) {
    let mut copy = project.clone();
    copy.takes.clear();
    stack.push(copy);
    if stack.len() > 50 {
        stack.remove(0);
    }
}

impl Database {
    pub fn pro_create(&self, name: &str, bpm: u32, key: &str, duration: u64) -> Result<Project> {
        let project = Project::new(name, bpm, key, duration)?;
        self.lock().execute(
            "INSERT INTO pro_projects(id,name,document,updated_at) VALUES(?1,?2,?3,?4)",
            params![project.id, project.name, document(&project)?, now()],
        )?;
        Ok(project)
    }
    pub fn pro_projects(&self) -> Result<Vec<Value>> {
        let con = self.lock();
        let mut stmt =
            con.prepare("SELECT id,name,updated_at FROM pro_projects ORDER BY updated_at DESC")?;
        let rows = stmt.query_map([], |r| Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"updated_at":r.get::<_,String>(2)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(rows)
    }
    pub fn pro_project(&self, pid: &str) -> Result<Project> {
        load(&self.lock(), pid)
    }
    pub fn pro_save(&self, project: &Project) -> Result<Project> {
        let mut con = self.lock();
        let tx = con.transaction()?;
        let current = load(&tx, &project.id)?;
        if project.revision != current.revision {
            return Err(message(
                "Project changed elsewhere. Reopen the project before editing.",
            ));
        }
        let mut saved = project.clone();
        saved.takes = current.takes.clone();
        saved.validate()?;
        let (mut undo, _) = history(&tx, &saved.id)?;
        push(&mut undo, &current);
        saved.revision += 1;
        tx.execute("UPDATE pro_projects SET name=?1,document=?2,undo=?3,redo='[]',updated_at=?4 WHERE id=?5", params![saved.name,document(&saved)?,serde_json::to_string(&undo)?,now(),saved.id])?;
        tx.commit()?;
        Ok(saved)
    }
    pub fn pro_history(&self, pid: &str, redo: bool) -> Result<Project> {
        let mut con = self.lock();
        let tx = con.transaction()?;
        let current = load(&tx, pid)?;
        let (mut backwards, mut forwards) = history(&tx, pid)?;
        let mut next = if redo {
            let next = forwards.pop();
            if next.is_some() {
                push(&mut backwards, &current);
            }
            next
        } else {
            let next = backwards.pop();
            if next.is_some() {
                push(&mut forwards, &current);
            }
            next
        }
        .ok_or_else(|| message("No further history"))?;
        next.takes = current.takes;
        next.revision = current.revision + 1;
        next.validate()?;
        tx.execute(
            "UPDATE pro_projects SET name=?1,document=?2,undo=?3,redo=?4,updated_at=?5 WHERE id=?6",
            params![
                next.name,
                document(&next)?,
                serde_json::to_string(&backwards)?,
                serde_json::to_string(&forwards)?,
                now(),
                pid
            ],
        )?;
        tx.commit()?;
        Ok(next)
    }
    pub fn pro_path(&self, pid: &str, file: &str) -> Result<PathBuf> {
        if !valid_id(pid) || !file.ends_with(".wav") || !valid_id(file.trim_end_matches(".wav")) {
            return Err(message("Invalid project audio path"));
        }
        super::assets::library_path(
            &PathBuf::from(self.settings()?.songs_dir),
            &format!("pro-{pid}/{file}"),
        )
    }
    pub fn pro_import(&self, pid: &str, source: &Path, track_id: Option<&str>) -> Result<Project> {
        let audio = Audio::read(source)?;
        let mut project = self.pro_project(pid)?;
        let name = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Audio")
            .chars()
            .take(100)
            .collect::<String>();
        let track_id = match track_id {
            Some(tid) if project.tracks.iter().any(|t| t.id == tid) => tid.to_string(),
            Some(_) => return Err(message("Track no longer exists")),
            None => {
                let tid = id();
                project.tracks.push(Track {
                    id: tid.clone(),
                    name: name.clone(),
                    instrument: "keyboard".into(),
                    prompt: String::new(),
                    gain: 0.7,
                    pan: 0.0,
                    mute: false,
                    solo: false,
                    context: true,
                    color: "#70c3b0".into(),
                });
                tid
            }
        };
        let take = Take::new(
            &track_id,
            &name,
            audio.samples.len() as u64,
            json!({"import":true}),
            project.revision,
        );
        let path = self.pro_path(pid, &take.file)?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        audio.write(&path)?;
        self.lock().execute(
            "INSERT INTO pro_takes(id,project_id,document) VALUES(?1,?2,?3)",
            params![take.id, pid, serde_json::to_string(&take)?],
        )?;
        project.clips.push(Clip {
            id: id(),
            track_id,
            take_id: take.id,
            start: 0,
            offset: 0,
            length: take.frames.min(project.duration),
            gain: 1.0,
        });
        self.pro_save(&project)
    }
}
