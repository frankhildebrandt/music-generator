use super::model::*;
use crate::{
    db::Database,
    error::{message, Result},
    types::{id, now},
};
use rusqlite::params;

impl Database {
    pub fn pro_running(&self, jid: &str) -> Result<bool> {
        Ok(self.lock().query_row(
            "SELECT status='running' FROM pro_jobs WHERE id=?1",
            [jid],
            |row| row.get(0),
        )?)
    }
    pub fn pro_jobs(&self, pid: &str) -> Result<Vec<Job>> {
        let con = self.lock();
        let mut stmt=con.prepare("SELECT id,status,message,take_id,request,source_revision,created_at FROM pro_jobs WHERE project_id=?1 ORDER BY rowid DESC LIMIT 100")?;
        let rows = stmt.query_map([pid], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, u64>(5)?,
                r.get::<_, String>(6)?,
            ))
        })?;
        rows.map(|row| {
            let (id, status, message, take_id, request, source_revision, created_at) = row?;
            Ok(Job {
                id,
                project_id: pid.into(),
                status,
                message,
                take_id,
                request: serde_json::from_str(&request)?,
                source_revision,
                created_at,
            })
        })
        .collect()
    }
    /// Freeze the source arrangement at enqueue time, independently of later edits.
    pub fn pro_enqueue(&self, pid: &str, request: &Generation) -> Result<Vec<String>> {
        let project = self.pro_project(pid)?;
        let track = project
            .tracks
            .iter()
            .find(|t| t.id == request.track_id)
            .ok_or_else(|| message("Select an instrument track"))?;
        if request.prompt.trim().is_empty()
            || request.prompt.len() > 8000
            || !(10..=200).contains(&request.steps)
            || !(1..=4).contains(&request.variants)
            || request.seed < -1
            || request.seed.checked_add(request.variants as i64).is_none()
            || request.end <= request.start
            || request.end > project.duration
            || project.duration < RATE * 10
        {
            return Err(message("Check prompt, range, steps and variants; generation requires a project of at least ten seconds"));
        }
        if let Some(cid) = &request.clip_id {
            let clip = project
                .clips
                .iter()
                .find(|c| &c.id == cid && c.track_id == track.id)
                .ok_or_else(|| message("Selected clip no longer exists"))?;
            if request.start < clip.start || request.end > clip.start + clip.length {
                return Err(message("Generation range must be inside the selected clip"));
            }
        }
        let mut con = self.lock();
        let tx = con.transaction()?;
        let mut ids = vec![];
        for variant in 0..request.variants {
            let jid = id();
            let mut req = request.clone();
            req.variants = 1;
            req.seed = if req.seed >= 0 {
                req.seed + variant as i64
            } else {
                i64::from_str_radix(&id()[..15], 16).unwrap()
            };
            tx.execute("INSERT INTO pro_jobs(id,project_id,status,message,request,snapshot,source_revision,created_at) VALUES(?1,?2,'queued','',?3,?4,?5,?6)",params![jid,pid,serde_json::to_string(&req)?,serde_json::to_string(&project)?,project.revision,now()])?;
            ids.push(jid);
        }
        tx.commit()?;
        Ok(ids)
    }
    pub fn pro_claim(&self) -> Result<Option<(String, Project, Generation)>> {
        let mut con = self.lock();
        let tx = con.transaction()?;
        let next = {
            let mut stmt=tx.prepare("SELECT id,snapshot,request FROM pro_jobs WHERE status='queued' ORDER BY rowid LIMIT 1")?;
            let mut rows = stmt.query([])?;
            rows.next()?
                .map(|r| {
                    Ok::<_, rusqlite::Error>((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })
                .transpose()?
        };
        let result = next
            .map(|(id, snapshot, request)| {
                Ok::<_, crate::error::Error>((
                    id,
                    serde_json::from_str(&snapshot)?,
                    serde_json::from_str(&request)?,
                ))
            })
            .transpose()?;
        if let Some((jid, _, _)) = &result {
            tx.execute(
                "UPDATE pro_jobs SET status='running',message='Preparing context' WHERE id=?1",
                [jid],
            )?;
        }
        tx.commit()?;
        Ok(result)
    }
    pub fn pro_job_status(&self, jid: &str, status: &str, text: &str) -> Result<()> {
        self.lock().execute("UPDATE pro_jobs SET status=?1,message=?2 WHERE id=?3 AND status IN ('queued','running')",params![status,text,jid])?;
        Ok(())
    }
    pub fn pro_complete(&self, jid: &str, pid: &str, take: &Take) -> Result<bool> {
        let mut con = self.lock();
        let tx = con.transaction()?;
        if tx.execute("UPDATE pro_jobs SET status='done',message='',take_id=?1 WHERE id=?2 AND status='running'",params![take.id,jid])?==0 {return Ok(false);}
        tx.execute(
            "INSERT INTO pro_takes(id,project_id,document) VALUES(?1,?2,?3)",
            params![take.id, pid, serde_json::to_string(take)?],
        )?;
        tx.commit()?;
        Ok(true)
    }
    pub fn pro_retry(&self, jid: &str) -> Result<()> {
        self.lock().execute("UPDATE pro_jobs SET status='queued',message='' WHERE id=?1 AND status IN ('error','cancelled')",[jid])?;
        Ok(())
    }
    pub fn pro_busy(&self) -> Result<bool> {
        Ok(self.lock().query_row(
            "SELECT count(*) FROM pro_jobs WHERE status IN ('queued','running')",
            [],
            |r| r.get::<_, u32>(0),
        )? > 0)
    }
}
