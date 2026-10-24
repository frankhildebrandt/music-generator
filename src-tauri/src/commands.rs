use crate::{
    error::{message, Result},
    state::AppState,
    types::{GenerateRequest, Settings, Song, SongFilter},
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};
use tauri::{AppHandle, Manager, State};

type Desktop<'a> = State<'a, Arc<AppState>>;

#[tauri::command]
pub fn remember_folder(path: String) -> Result<()> {
    crate::sandbox::remember(std::path::Path::new(&path))
}

#[tauri::command]
pub async fn bootstrap(state: Desktop<'_>, app: AppHandle) -> Result<Value> {
    let _operation = state.operation.lock().await;
    let started = std::time::Instant::now();
    let hardware = crate::hardware::inspect();
    tracing::info!(
        elapsed_ms = started.elapsed().as_millis(),
        "Startup hardware check finished"
    );
    let settings = state.db.settings()?;
    let needs_setup = !settings.setup_complete;
    if !needs_setup {
        app.asset_protocol_scope()
            .allow_directory(&settings.songs_dir, false)
            .map_err(|e| message(e.to_string()))?;
    }
    state.restore_setup()?;
    tracing::info!(
        elapsed_ms = started.elapsed().as_millis(),
        needs_setup,
        "Startup setup check finished"
    );
    Ok(
        json!({"needs_setup":needs_setup,"settings":settings,"hardware":hardware,"manifest":crate::manifest::manifest(),"error":null,"version":env!("CARGO_PKG_VERSION"),"data_dir":state.db.root}),
    )
}

#[tauri::command]
pub async fn setup_models(
    state: Desktop<'_>,
    app: AppHandle,
    request: crate::setup::SetupRequest,
) -> Result<Value> {
    crate::setup::configure(state.inner().clone(), app, request).await
}

#[tauri::command]
pub fn setup_estimate(request: crate::setup::SetupRequest) -> Result<Value> {
    let models = crate::manifest::selected(&request.lm, request.xl)?;
    let root = PathBuf::from(&request.models_dir);
    if !root.is_absolute() {
        return Err(message("Select an absolute model folder"));
    }
    let ancestor = root
        .ancestors()
        .find(|p| p.is_dir())
        .ok_or_else(|| message("Folder is unavailable"))?;
    let missing: u64 = models
        .iter()
        .filter(|m| !root.join(&m.file).is_file())
        .map(|m| {
            m.estimated_bytes.saturating_sub(
                root.join(format!("{}.part", m.file))
                    .metadata()
                    .map(|m| m.len())
                    .unwrap_or(0),
            )
        })
        .sum();
    Ok(json!({"missing_bytes":missing,"free_bytes":fs2::available_space(ancestor)?}))
}

#[tauri::command]
pub fn cancel_setup(state: Desktop<'_>) {
    if let Some(token) = state.setup_cancel.lock().unwrap().as_ref() {
        token.cancel();
    }
}

#[tauri::command]
pub async fn import_library(state: Desktop<'_>, app: AppHandle, root: String) -> Result<Value> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| message("Please wait for the current operation"))?;
    let db = state.db.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        crate::import::import_legacy(&db, &PathBuf::from(root))
    })
    .await
    .map_err(|e| message(e.to_string()))??;
    state.changed(&app);
    Ok(result)
}

#[tauri::command]
pub fn list_songs(state: Desktop<'_>, filter: SongFilter) -> Result<Vec<Song>> {
    state.db.songs(&filter)
}

#[tauri::command]
pub fn get_song(state: Desktop<'_>, id: String) -> Result<Song> {
    state.db.song(&id)
}

#[tauri::command]
pub fn generate(state: Desktop<'_>, app: AppHandle, mut request: GenerateRequest) -> Result<Value> {
    if !state.ready.load(Ordering::SeqCst) || state.is_setting_up() {
        return Err(message("Complete model setup before generating music"));
    }
    let settings = state.db.settings()?;
    let selection = request
        .model_selection
        .clone()
        .unwrap_or_else(|| crate::catalog::Selection::from_settings(&settings));
    if !crate::catalog::cached(
        &selection,
        std::path::Path::new(&settings.models_dir),
        &crate::verification::ModelVerifier::open(&state.db.root),
    )? {
        return Err(message(
            "Open the ModelPicker and download or verify the selected models first",
        ));
    }
    request.model_selection = Some(selection);
    let ids = state.db.enqueue(&request)?;
    state.wake.notify_one();
    state.changed(&app);
    Ok(json!({"ids":ids}))
}

#[tauri::command]
pub fn patch_song(state: Desktop<'_>, app: AppHandle, id: String, patch: Value) -> Result<Song> {
    state.db.patch(&id, &patch)?;
    state.changed(&app);
    state.db.song(&id)
}

#[tauri::command]
pub fn delete_song(state: Desktop<'_>, app: AppHandle, id: String) -> Result<Value> {
    let cancelled = state.db.cancel_or_delete(&id)?;
    if cancelled {
        state.cancel(&id);
    }
    state.changed(&app);
    Ok(json!({"cancelled":cancelled,"deleted":!cancelled}))
}

#[tauri::command]
pub fn retry_song(state: Desktop<'_>, app: AppHandle, id: String) -> Result<Song> {
    state.db.retry(&id)?;
    state.wake.notify_one();
    state.changed(&app);
    state.db.song(&id)
}

#[tauri::command]
pub fn list_folders(state: Desktop<'_>) -> Result<Value> {
    state.db.folders()
}

#[tauri::command]
pub fn save_folder(
    state: Desktop<'_>,
    app: AppHandle,
    id: Option<String>,
    name: String,
) -> Result<Value> {
    let folder = state.db.folder_save(id.as_deref(), &name)?;
    state.changed(&app);
    Ok(folder)
}

#[tauri::command]
pub fn delete_folder(state: Desktop<'_>, app: AppHandle, id: String) -> Result<()> {
    state.db.folder_delete(&id)?;
    state.changed(&app);
    Ok(())
}

#[tauri::command]
pub fn queue(state: Desktop<'_>) -> Result<Value> {
    let mut queue = state.db.queue()?;
    queue["active"] = json!(state.operation.try_lock().is_err());
    Ok(queue)
}

#[tauri::command]
pub fn get_settings(state: Desktop<'_>) -> Result<Value> {
    let settings = state.db.settings()?;
    let mut result = serde_json::to_value(&settings)?;
    result["songs_dir_default"] = json!(state.db.root.join("songs"));
    result["mp3_available"] = json!(crate::audio_export::codec_path(&state.resources).is_file());
    result["cpu_only"] = json!(crate::hardware::cpu_only());
    result["dit_xl_available"] = json!(PathBuf::from(settings.models_dir)
        .join("acestep-v15-xl-sft-Q8_0.gguf")
        .is_file());
    Ok(result)
}

#[tauri::command]
pub async fn save_settings(state: Desktop<'_>, app: AppHandle, values: Value) -> Result<Settings> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| message("Please wait for the current generation or analysis"))?;
    let mut settings = state.db.settings()?;
    let old_device = settings.lm_device.clone();
    if let Some(device) = values.get("lm_device") {
        settings.lm_device = device
            .as_str()
            .ok_or_else(|| message("Invalid language model device"))?
            .into();
    }
    if let Some(format) = values.get("export_format") {
        settings.export_format = format
            .as_str()
            .ok_or_else(|| message("Invalid export format"))?
            .into();
    }
    if settings.export_format == "mp3"
        && !crate::audio_export::codec_path(&state.resources).is_file()
    {
        return Err(message("mp3_failed"));
    }
    if let Some(timeout) = values.get("timeout_minutes") {
        settings.timeout_minutes = timeout.as_f64().ok_or_else(|| message("Invalid timeout"))?;
    }
    if let Some(dit) = values.get("dit") {
        settings.dit = dit
            .as_str()
            .ok_or_else(|| message("Invalid synthesis model"))?
            .into();
        settings.synthesis_model.clear();
    }
    if settings.synthesis_model.is_empty()
        && settings.dit == "xl"
        && !PathBuf::from(&settings.models_dir)
            .join("acestep-v15-xl-sft-Q8_0.gguf")
            .is_file()
    {
        return Err(message("Download the XL model in model setup first"));
    }
    crate::state::validate_settings(&settings)?;
    if let Some(dir) = values.get("songs_dir").and_then(Value::as_str) {
        let dir = if dir.is_empty() {
            state.db.root.join("songs")
        } else {
            PathBuf::from(dir)
        };
        if dir != std::path::Path::new(&settings.songs_dir) {
            crate::sandbox::remember(&dir)?;
            let db = state.db.clone();
            let target = dir.clone();
            tauri::async_runtime::spawn_blocking(move || db.change_songs_dir(&target))
                .await
                .map_err(|e| message(e.to_string()))??;
            settings.songs_dir = dir.to_string_lossy().into();
        }
    }
    state.db.save_settings(&settings)?;
    if old_device != settings.lm_device {
        state.stop_engine().await;
        state.restore_setup()?;
    }
    app.asset_protocol_scope()
        .allow_directory(&settings.songs_dir, false)
        .map_err(|e| message(e.to_string()))?;
    // Apply timing changes without losing the synthesis log used by diagnostics.
    if let Some(engine) = state.engine.lock().await.as_mut() {
        engine.client.poll = std::time::Duration::from_secs_f64(settings.poll_interval);
        engine.client.timeout = std::time::Duration::from_secs_f64(settings.timeout_minutes * 60.0);
    }
    state.changed(&app);
    Ok(settings)
}

#[tauri::command]
pub async fn analyze(
    state: Desktop<'_>,
    data: Vec<u8>,
    filename: String,
    selection: Option<crate::catalog::Selection>,
) -> Result<Value> {
    if data.len() > 60 * 1024 * 1024 {
        return Err(message("Audio files must be smaller than 60 MB"));
    }
    let extension = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    if !["wav", "mp3", "flac", "m4a", "ogg"].contains(&extension.as_str()) {
        return Err(message("Please choose an audio file"));
    }
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| message("Please wait for the current generation or analysis"))?;
    if !state.ready.load(Ordering::SeqCst) {
        return Err(message("Complete model setup first"));
    }
    let mut settings = state.db.settings()?;
    if let Some(selection) = selection {
        selection.apply(&mut settings)?;
    }
    let cancel = state.shutdown.child_token();
    let engine = state.start_engine(&settings, &cancel).await?;
    let _awake = crate::power::KeepAwake::new();
    engine
        .analyze(
            data,
            filename,
            &crate::catalog::Selection::from_settings(&settings),
            &cancel,
        )
        .await
}

#[tauri::command]
pub async fn health(state: Desktop<'_>) -> Result<Value> {
    let mut engine = state.engine.lock().await;
    if let Some(engine) = engine.as_mut() {
        if !engine.alive() {
            return Ok(json!({"ok":false,"error":"Model engine stopped unexpectedly"}));
        }
        let client = &engine.client;
        let synth = client
            .http
            .get(format!("{}/health", client.synth))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;
        let lm = client
            .http
            .get(format!("{}/health", client.lm))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;
        let props = match client
            .http
            .get(format!("{}/props", client.synth))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => response.json::<Value>().await.ok(),
            Err(_) => None,
        };
        return Ok(
            json!({"ok":synth.is_ok_and(|r| r.status().is_success()) && lm.is_ok_and(|r| r.status().is_success()),"props":props}),
        );
    }
    let idle = state.ready.load(Ordering::SeqCst);
    Ok(
        json!({"ok":idle,"idle":idle,"error":if idle { None } else { Some("Complete model setup first") }}),
    )
}

#[tauri::command]
pub async fn export_file(
    state: Desktop<'_>,
    kind: String,
    id: Option<String>,
    folder: Option<String>,
    destination: String,
) -> Result<()> {
    let destination = PathBuf::from(destination);
    if !destination.is_absolute() {
        return Err(message("Select an absolute destination"));
    }
    crate::exports::export(
        state.db.clone(),
        state.resources.clone(),
        kind,
        id,
        folder,
        destination,
        state.shutdown.child_token(),
    )
    .await
}

#[tauri::command]
pub async fn check_update() -> Result<Value> {
    crate::updates::check().await
}

#[tauri::command]
pub fn exit_app(state: Desktop<'_>, app: AppHandle) {
    state.begin_shutdown();
    app.exit(0);
}
