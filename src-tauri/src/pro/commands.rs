use super::{model::*, render};
use crate::{
    error::{message, Result},
    state::AppState,
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};
use tauri::{AppHandle, Emitter, Manager, State};

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| message(format!("Missing {key}")))
}

#[tauri::command]
pub async fn pro_request(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    action: String,
    payload: Value,
) -> Result<Value> {
    if action == "install_base" {
        return install_base(state.inner().clone(), app).await;
    }
    if action == "model" {
        let settings = state.db.settings()?;
        let model = crate::manifest::manifest()
            .models
            .into_iter()
            .find(|m| m.kind == "pro")
            .ok_or_else(|| message("Missing Base manifest"))?;
        return Ok(
            json!({"installed":PathBuf::from(settings.models_dir).join(BASE_MODEL).is_file(),"bytes":model.estimated_bytes}),
        );
    }
    if action == "generate" {
        if !state.ready.load(Ordering::SeqCst) {
            return Err(message("Complete model setup first"));
        }
        let request: Generation = serde_json::from_value(payload["request"].clone())?;
        let ids = state.db.pro_enqueue(field(&payload, "id")?, &request)?;
        state.wake.notify_one();
        state.changed(&app);
        return Ok(json!({"ids":ids}));
    }
    if action == "cancel" {
        let jid = field(&payload, "id")?;
        state.db.pro_job_status(jid, "cancelled", "Cancelled")?;
        state.cancel(jid);
        state.changed(&app);
        return Ok(json!({}));
    }
    if action == "retry" {
        state.db.pro_retry(field(&payload, "id")?)?;
        state.wake.notify_one();
        state.changed(&app);
        return Ok(json!({}));
    }
    let db = state.db.clone();
    let mut result = tauri::async_runtime::spawn_blocking(move || -> Result<Value> {
        match action.as_str() {
            "list" => Ok(json!(db.pro_projects()?)),
            "create" => Ok(json!(db.pro_create(
                field(&payload, "name")?,
                payload["bpm"].as_u64().unwrap_or(120) as u32,
                payload["key"].as_str().unwrap_or("A minor"),
                payload["duration"].as_u64().unwrap_or(RATE * 16)
            )?)),
            "open" => {
                let mut project = db.pro_project(field(&payload, "id")?)?;
                for take in &mut project.takes {
                    take.path = Some(
                        db.pro_path(&project.id, &take.file)?
                            .to_string_lossy()
                            .into(),
                    );
                }
                Ok(json!(project))
            }
            "save" => {
                let project: Project = serde_json::from_value(payload)?;
                Ok(json!(db.pro_save(&project)?))
            }
            "history" => Ok(json!(db.pro_history(
                field(&payload, "id")?,
                payload["redo"].as_bool().unwrap_or(false)
            )?)),
            "jobs" => Ok(json!(db.pro_jobs(field(&payload, "id")?)?)),
            "import" => {
                let source = PathBuf::from(field(&payload, "path")?);
                crate::sandbox::remember(&source)?;
                Ok(json!(db.pro_import(
                    field(&payload, "id")?,
                    &source,
                    payload["track_id"].as_str()
                )?))
            }
            "export" => {
                let destination = PathBuf::from(field(&payload, "path")?);
                if !destination.is_absolute() {
                    return Err(message("Select an absolute destination"));
                }
                // The Save dialog grants session access; a new file cannot be bookmarked yet.
                let project = db.pro_project(field(&payload, "id")?)?;
                let resolved = destination
                    .canonicalize()
                    .unwrap_or_else(|_| destination.clone());
                if project.takes.iter().any(|t| {
                    db.pro_path(&project.id, &t.file)
                        .ok()
                        .is_some_and(|p| p.canonicalize().unwrap_or(p) == resolved)
                }) {
                    return Err(message("Choose a new export destination"));
                }
                render::export(
                    &db,
                    &project,
                    &destination,
                    payload["stems"].as_bool().unwrap_or(false),
                )?;
                Ok(json!({}))
            }
            _ => Err(message("Unknown Pro operation")),
        }
    })
    .await
    .map_err(|e| message(e.to_string()))??;
    if let Some(pid) = result.get("id").and_then(Value::as_str).map(str::to_string) {
        if let Some(takes) = result.get_mut("takes").and_then(Value::as_array_mut) {
            for take in takes {
                take["path"] = json!(state
                    .db
                    .pro_path(&pid, field(take, "file")?)?
                    .to_string_lossy());
            }
        }
    }
    let songs = state.db.settings()?.songs_dir;
    app.asset_protocol_scope()
        .allow_directory(&songs, true)
        .map_err(|e| message(e.to_string()))?;
    Ok(result)
}

async fn install_base(state: Arc<AppState>, app: AppHandle) -> Result<Value> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| message("Wait for the current generation to finish"))?;
    if state.is_setting_up() {
        return Err(message("Model setup is already running"));
    }
    let cancel = state.shutdown.child_token();
    *state.setup_cancel.lock().unwrap() = Some(cancel.clone());
    let was_ready = state.ready.load(Ordering::SeqCst);
    state.stop_engine().await;
    let result = async {
        let settings = state.db.settings()?;
        let model = crate::manifest::manifest()
            .models
            .into_iter()
            .find(|m| m.kind == "pro")
            .ok_or_else(|| message("Missing Base model"))?;
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(30))
            .read_timeout(std::time::Duration::from_secs(60))
            .build()?;
        let mut verifier = crate::verification::ModelVerifier::open(&state.db.root);
        crate::download::download_model(
            &client,
            &format!("{}/{}", crate::manifest::manifest().base_url, model.file),
            &PathBuf::from(settings.models_dir),
            &model,
            &mut verifier,
            &cancel,
            |progress| {
                let _ = app.emit("pro-download", progress);
            },
        )
        .await?;
        Ok::<_, crate::error::Error>(json!({"installed":true}))
    }
    .await;
    *state.setup_cancel.lock().unwrap() = None;
    if was_ready && !state.shutdown.is_cancelled() {
        let settings = state.db.settings()?;
        match state.start_engine(&settings, &state.shutdown).await {
            Ok(_) => {
                state.ready.store(true, Ordering::SeqCst);
                state.wake.notify_one();
            }
            Err(error) if result.is_ok() => {
                return Err(message(format!(
                    "Base model downloaded, but the engine could not restart: {error}"
                )))
            }
            Err(_) => {}
        }
    }
    result
}
