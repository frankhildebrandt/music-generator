use crate::{
    download::{download_model, ensure_space},
    engine::client::selftest_ok,
    error::{message, Result},
    hardware, manifest,
    state::AppState,
    verification::ModelVerifier,
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

pub fn plan(request: &SetupRequest) -> Result<Vec<(manifest::Model, String)>> {
    let mut selected = manifest::selected(&request.lm, request.xl)?
        .into_iter()
        .map(|m| {
            let url = format!("{}/{}", manifest::manifest().base_url, m.file);
            (m, url)
        })
        .collect::<Vec<_>>();
    let title = crate::titles::manifest();
    selected.push((title.model, title.url));
    Ok(selected)
}

pub fn fingerprint(lm: &str, gpu: &str) -> String {
    format!(
        "{}:{}:{}:{}:{:?}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        manifest::manifest().engines,
        lm,
        gpu
    )
}

pub async fn models_valid(state: &AppState, cancel: &CancellationToken) -> Result<bool> {
    let settings = state.db.settings()?;
    models_valid_for(state, &settings, cancel).await
}

pub async fn models_valid_for(
    state: &AppState,
    settings: &crate::types::Settings,
    cancel: &CancellationToken,
) -> Result<bool> {
    if !settings.setup_complete {
        return Ok(false);
    }
    let mut verifier = ModelVerifier::open(&state.db.root);
    crate::catalog::verify(
        &crate::catalog::Selection::from_settings(settings),
        Path::new(&settings.models_dir),
        &mut verifier,
        cancel,
    )
    .await
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
            // Restore readiness; the engine starts lazily on the next operation.
            state.restore_setup()?;
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
    let selected = plan(request)?;
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
        .filter(|(m, _)| !models.join(&m.file).exists())
        .map(|(m, _)| {
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
    for (model, url) in selected {
        download_model(&client, &url, &models, &model, &mut verifier, cancel, |p| {
            let _ = app.emit("setup-progress", p);
        })
        .await?;
    }
    crate::titles::probe(&state.resources, cancel).await?;
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
    settings.vae_chunk = hw.vae_chunk;
    settings.mode = "single".into();
    // The wizard deliberately returns to the supported starter profile.
    crate::catalog::Selection {
        provider: "ace-step".into(),
        synthesis: if request.xl && settings.dit == "xl" {
            "acestep-v15-xl-sft-Q8_0.gguf"
        } else {
            "acestep-v15-sft-Q8_0.gguf"
        }
        .into(),
        language: manifest::lm_file(&request.lm),
    }
    .apply(&mut settings)?;
    let fingerprint = fingerprint(&settings.lm_model, &hw.gpu);
    if hw.cpu_only {
        // The Intel engine contains no GPU backend. A GPU correctness benchmark is
        // inapplicable and can take minutes on older CPUs; verify server readiness.
        let _ = app.emit("setup-stage", "starting");
        state.start_engine(&settings, cancel).await?;
    } else {
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
