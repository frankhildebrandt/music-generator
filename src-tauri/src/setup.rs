use crate::{
    download::{download_model, ensure_space},
    engine::client::selftest_ok,
    error::{message, Result},
    hardware, manifest,
    state::AppState,
    verification::{ModelVerifier, Verification},
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::{atomic::Ordering, Arc},
};
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Deserialize)]
pub struct SetupRequest {
    pub lm: String,
    pub xl: bool,
    pub models_dir: String,
    pub songs_dir: String,
}

pub fn fingerprint(lm: &str, gpu: &str) -> String {
    format!(
        "{}:{}:{}:{:?}",
        std::env::consts::OS,
        manifest::manifest().engines,
        lm,
        gpu
    )
}

pub async fn models_valid(state: &AppState, cancel: &CancellationToken) -> Result<bool> {
    let settings = state.db.settings()?;
    if !settings.setup_complete {
        return Ok(false);
    }
    let Some(size) = ["0.6B", "1.7B", "4B"]
        .into_iter()
        .find(|s| manifest::lm_file(s) == settings.lm_model)
    else {
        return Ok(false);
    };
    let mut verifier = ModelVerifier::open(&state.db.root);
    for model in manifest::selected(size, settings.dit == "xl")? {
        let path = PathBuf::from(&settings.models_dir).join(&model.file);
        let Ok(path) = path.canonicalize() else {
            return Ok(false);
        };
        if verifier
            .verify(&path, &model.sha256, cancel, |_, _| {})
            .await?
            == Verification::Invalid
        {
            return Ok(false);
        }
    }
    Ok(true)
}

pub async fn configure(
    state: Arc<AppState>,
    app: AppHandle,
    request: SetupRequest,
) -> Result<Value> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| message("Please wait for the current operation to finish"))?;
    if state.is_setting_up() {
        return Err(message("Setup is already running"));
    }
    let cancel = state.shutdown.child_token();
    *state.setup_cancel.lock().unwrap() = Some(cancel.clone());
    state.stop_engine().await;
    let outcome = configure_inner(&state, &app, &request, &cancel).await;
    *state.setup_cancel.lock().unwrap() = None;
    if outcome.is_err() {
        state.stop_engine().await;
        // Cancelling model management must not disable an already working installation.
        if !state.shutdown.is_cancelled()
            && models_valid(&state, &state.shutdown).await.unwrap_or(false)
        {
            let previous = state.db.settings()?;
            if state.start_engine(&previous, &state.shutdown).await.is_ok() {
                state.ready.store(true, Ordering::SeqCst);
                state.wake.notify_one();
            }
        }
    }
    outcome
}

async fn configure_inner(
    state: &AppState,
    app: &AppHandle,
    request: &SetupRequest,
    cancel: &CancellationToken,
) -> Result<Value> {
    let selected = manifest::selected(&request.lm, request.xl)?;
    let models = PathBuf::from(&request.models_dir);
    let songs = PathBuf::from(&request.songs_dir);
    if !models.is_absolute() || !songs.is_absolute() {
        return Err(message("Select absolute folders"));
    }
    crate::sandbox::remember(&models)?;
    crate::sandbox::remember(&songs)?;
    std::fs::create_dir_all(&models)?;
    std::fs::create_dir_all(&songs)?;
    let required: u64 = selected
        .iter()
        .filter(|m| !models.join(&m.file).exists())
        .map(|m| {
            m.estimated_bytes.saturating_sub(
                models
                    .join(format!("{}.part", m.file))
                    .metadata()
                    .map(|m| m.len())
                    .unwrap_or(0),
            )
        })
        .sum();
    ensure_space(&models, required + 1024 * 1024 * 1024)?;
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .read_timeout(std::time::Duration::from_secs(60))
        .build()?;
    let mut verifier = ModelVerifier::open(&state.db.root);
    for model in selected {
        let url = format!("{}/{}", manifest::manifest().base_url, model.file);
        download_model(&client, &url, &models, &model, &mut verifier, cancel, |p| {
            let _ = app.emit("setup-progress", p);
        })
        .await?;
    }
    let mut settings = state.db.settings()?;
    if Path::new(&settings.songs_dir) != songs && state.db.busy()? {
        return Err(message(
            "Finish or cancel queued songs before moving the song folder",
        ));
    }
    if Path::new(&settings.songs_dir) != songs {
        state.db.change_songs_dir(&songs)?;
    }
    let hw = hardware::inspect();
    settings.songs_dir = songs.to_string_lossy().into();
    settings.models_dir = models.to_string_lossy().into();
    settings.lm_model = manifest::lm_file(&request.lm);
    settings.vae_chunk = hw.vae_chunk;
    settings.mode = "single".into();
    let fingerprint = fingerprint(&settings.lm_model, &hw.gpu);
    let _ = app.emit("setup-stage", "testing");
    let test = json!({"caption":"calm ambient pad, soft piano","lyrics":"[Instrumental]","duration":10,"vocal_language":"unknown","seed":42,"lm_model":settings.lm_model});
    let accelerated = state.start_engine(&settings, cancel).await;
    let mut okay = false;
    if let Ok(mut engine) = accelerated {
        engine.timeout = std::time::Duration::from_secs(300);
        okay = engine
            .language_model(&test, cancel, |_| {})
            .await
            .is_ok_and(|v| selftest_ok(&v));
    }
    if cancel.is_cancelled() {
        return Err(crate::error::Error::Cancelled);
    }
    if !okay {
        state.stop_engine().await;
        settings.mode = "split".into();
        let _ = app.emit("setup-stage", "cpu-fallback");
        let mut engine = state.start_engine(&settings, cancel).await?;
        engine.timeout = std::time::Duration::from_secs(300);
        if !selftest_ok(&engine.language_model(&test, cancel, |_| {}).await?) {
            return Err(message(
                "Language model self-test failed on CPU. See the engine logs.",
            ));
        }
    }
    app.asset_protocol_scope()
        .allow_directory(&songs, false)
        .map_err(|e| message(e.to_string()))?;
    if cancel.is_cancelled() {
        return Err(crate::error::Error::Cancelled);
    }
    settings.tested_fingerprint = fingerprint;
    settings.setup_complete = true;
    state.db.save_settings(&settings)?;
    state.ready.store(true, Ordering::SeqCst);
    state.wake.notify_one();
    state.changed(app);
    Ok(json!({"ok":true,"mode":settings.mode}))
}
