use crate::{
    db::Database,
    engine::{client::EngineClient, Engine},
    error::{message, Result},
    types::Settings,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

pub struct AppState {
    pub db: Arc<Database>,
    pub resources: PathBuf,
    pub engine: tokio::sync::Mutex<Option<Engine>>,
    pub operation: tokio::sync::Mutex<()>,
    pub running: Mutex<Option<(String, CancellationToken)>>,
    pub setup_cancel: Mutex<Option<CancellationToken>>,
    pub shutdown: CancellationToken,
    pub ready: AtomicBool,
    pub wake: Notify,
}

impl AppState {
    pub fn new(db: Database, resources: PathBuf) -> Self {
        Self {
            db: Arc::new(db),
            resources,
            engine: tokio::sync::Mutex::new(None),
            operation: tokio::sync::Mutex::new(()),
            running: Mutex::new(None),
            setup_cancel: Mutex::new(None),
            shutdown: CancellationToken::new(),
            ready: AtomicBool::new(false),
            wake: Notify::new(),
        }
    }

    pub fn changed(&self, app: &AppHandle) {
        let _ = app.emit("library-changed", ());
        let _ = app.emit("pro-changed", ());
    }

    // Restore the durable wizard result without reading models or starting the engine.
    pub fn restore_setup(&self) -> Result<bool> {
        let complete = self.db.settings()?.setup_complete;
        self.ready.store(complete, Ordering::SeqCst);
        if complete {
            self.wake.notify_one();
        }
        Ok(complete)
    }

    pub async fn start_engine(
        &self,
        settings: &Settings,
        cancel: &CancellationToken,
    ) -> Result<EngineClient> {
        let mut engine = self.engine.lock().await;
        if engine.as_mut().is_some_and(Engine::alive) {
            return Ok(engine.as_ref().unwrap().client.clone());
        }
        engine.take();
        if settings.setup_complete
            && !self.is_setting_up()
            && !crate::setup::models_valid(self, cancel).await?
        {
            return Err(message(
                "Models are missing or invalid. Open Models and hardware test in Settings.",
            ));
        }
        let started = Engine::start(
            &self.resources,
            &self.db.root.join("logs"),
            settings,
            cancel,
        )
        .await?;
        let client = started.client.clone();
        *engine = Some(started);
        Ok(client)
    }

    pub async fn stop_engine(&self) {
        self.ready.store(false, Ordering::SeqCst);
        self.engine.lock().await.take();
    }

    pub fn cancel(&self, sid: &str) {
        if let Some((running, token)) = self.running.lock().unwrap().as_ref() {
            if running == sid {
                token.cancel();
            }
        }
    }

    pub fn is_setting_up(&self) -> bool {
        self.setup_cancel.lock().unwrap().is_some()
    }

    pub fn begin_shutdown(&self) {
        self.shutdown.cancel();
        if let Some((_, token)) = self.running.lock().unwrap().as_ref() {
            token.cancel();
        }
        if let Some(token) = self.setup_cancel.lock().unwrap().as_ref() {
            token.cancel();
        }
        self.wake.notify_one();
    }
}

pub async fn worker(state: Arc<AppState>, app: AppHandle) {
    loop {
        let wake = state.wake.notified();
        if state.shutdown.is_cancelled() {
            break;
        }
        if !state.ready.load(Ordering::SeqCst) {
            tokio::select! { _ = state.shutdown.cancelled() => break, _ = wake => continue }
        }
        let operation = state.operation.lock().await;
        if state.shutdown.is_cancelled() {
            break;
        }
        if !state.ready.load(Ordering::SeqCst) {
            drop(operation);
            continue;
        }
        let next = match state.db.claim_next() {
            Ok(next) => next,
            Err(e) => {
                tracing::error!("Queue: {e}");
                None
            }
        };
        let Some((sid, params)) = next else {
            match state.db.pro_claim() {
                Ok(Some((jid, project, request))) => {
                    let cancel = state.shutdown.child_token();
                    *state.running.lock().unwrap() = Some((jid.clone(), cancel.clone()));
                    if !state.db.pro_running(&jid).unwrap_or(false) {
                        cancel.cancel();
                    }
                    state.changed(&app);
                    let _awake = crate::power::KeepAwake::new();
                    let outcome =
                        crate::pro::jobs::run(&state, &app, &jid, project, request, &cancel).await;
                    *state.running.lock().unwrap() = None;
                    if let Err(error) = outcome {
                        if !state.shutdown.is_cancelled() {
                            let _ = state.db.pro_job_status(&jid, "error", &error.to_string());
                        }
                    }
                    state.changed(&app);
                    drop(operation);
                    continue;
                }
                Err(error) => tracing::error!("Pro queue: {error}"),
                _ => {}
            }
            drop(operation);
            tokio::select! { _ = state.shutdown.cancelled() => break, _ = wake => continue }
        };
        let cancel = state.shutdown.child_token();
        *state.running.lock().unwrap() = Some((sid.clone(), cancel.clone()));
        if state.db.song(&sid).is_ok_and(|s| s.status != "running") {
            cancel.cancel();
        }
        state.changed(&app);
        let _awake = crate::power::KeepAwake::new();
        let outcome = async {
            let settings = state.db.settings()?;
            let client = state.start_engine(&settings, &cancel).await?;
            let dit = if settings.dit == "xl" {
                "acestep-v15-xl-sft-Q8_0.gguf"
            } else {
                "acestep-v15-sft-Q8_0.gguf"
            };
            let (audio, ext, meta) = client
                .generate(&params, &settings.lm_model, dit, &cancel, |text| {
                    let _ = state.db.progress(&sid, &text);
                    state.changed(&app);
                })
                .await?;
            let dir = PathBuf::from(settings.songs_dir);
            tokio::fs::create_dir_all(&dir).await?;
            let filename = format!("{sid}.{ext}");
            let path = dir.join(&filename);
            let temp = dir.join(format!("{sid}.part"));
            tokio::fs::write(&temp, audio).await?;
            if cancel.is_cancelled() {
                let _ = tokio::fs::remove_file(temp).await;
                return Err(crate::error::Error::Cancelled);
            }
            tokio::fs::rename(&temp, &path).await?;
            if !state.db.finish(&sid, &filename, &meta)? {
                let _ = tokio::fs::remove_file(path).await;
            }
            Ok::<_, crate::error::Error>(())
        }
        .await;
        *state.running.lock().unwrap() = None;
        if let Err(error) = outcome {
            if !state.shutdown.is_cancelled() {
                let _ = state.db.fail(&sid, &error.to_string());
            }
        }
        state.changed(&app);
        drop(operation);
    }
    state.stop_engine().await;
}

pub fn validate_settings(settings: &Settings) -> Result<()> {
    if !["auto", "cpu"].contains(&settings.lm_device.as_str()) {
        return Err(message("Unknown language model device"));
    }
    if !["wav", "mp3"].contains(&settings.export_format.as_str()) {
        return Err(message("Unknown export format"));
    }
    if !settings.poll_interval.is_finite()
        || !(0.1..=60.0).contains(&settings.poll_interval)
        || !settings.timeout_minutes.is_finite()
        || !(1.0..=600.0).contains(&settings.timeout_minutes)
    {
        return Err(message("Invalid timeout or poll interval"));
    }
    if !["standard", "xl"].contains(&settings.dit.as_str()) {
        return Err(message("Unknown synthesis model"));
    }
    for path in [&settings.models_dir, &settings.songs_dir] {
        if !std::path::Path::new(path).is_absolute() {
            return Err(message("Select an absolute data folder"));
        }
    }
    Ok(())
}
