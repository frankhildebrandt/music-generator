use crate::{
    engine::progress::Progress,
    runtime::{Operation, OperationKind},
    state::AppState,
};
use std::{path::PathBuf, sync::Arc};
use tauri::{AppHandle, Emitter};

fn report_progress(state: &AppState, app: &AppHandle, operation: &Operation, progress: Progress) {
    if let Some(update) = operation.report(progress) {
        let _ = state
            .db
            .progress(&update.job_id, &update.progress.message());
        let _ = app.emit("generation-progress", update);
    }
}

pub async fn worker(state: Arc<AppState>, app: AppHandle) {
    loop {
        if state.runtime.is_shutdown() {
            break;
        }
        if !state.runtime.is_ready() {
            state.runtime.wait().await;
            continue;
        }
        let Ok(operation) = state.runtime.begin(OperationKind::Generation).await else {
            break;
        };
        if state.runtime.is_shutdown() {
            break;
        }
        if !state.runtime.is_ready() {
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
            drop(operation);
            state.runtime.wait().await;
            continue;
        };
        let cancel = operation.cancellation();
        operation.start_song(&sid);
        if state.db.song(&sid).is_ok_and(|s| s.status != "running") {
            cancel.cancel();
        }
        report_progress(&state, &app, &operation, Progress::new("starting"));
        state.changed(&app);
        let _awake = crate::power::KeepAwake::new();
        let outcome = async {
            let mut settings = state.db.settings()?;
            // New jobs carry an immutable model snapshot. Legacy rows remain untouched.
            if let Some(selection) = params.get("model_selection").filter(|v| !v.is_null()) {
                let selection: crate::catalog::Selection =
                    serde_json::from_value(selection.clone())?;
                selection.apply(&mut settings)?;
            }
            let selection = crate::catalog::Selection::from_settings(&settings);
            let progress = |progress: Progress| {
                report_progress(&state, &app, &operation, progress);
            };
            // Existing installations lazily fetch only this shared model for blank titles.
            // Explicit titles require neither a title download nor title inference.
            if state
                .db
                .song(&sid)?
                .title
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                report_progress(&state, &app, &operation, Progress::new("title_download"));
                crate::titles::install(
                    &state.db.root,
                    std::path::Path::new(&settings.models_dir),
                    &cancel,
                    |p| {
                        let _ = app.emit("model-download-progress", p);
                    },
                )
                .await?;
                let _ = app.emit("models-changed", ());
            }
            let (audio, ext, mut meta) = {
                let client = state.start_engine(&operation, &settings).await?;
                client
                    .generate(
                        &params,
                        &selection.language,
                        &selection.synthesis,
                        &cancel,
                        progress,
                    )
                    .await?
            };
            if state
                .db
                .song(&sid)?
                .title
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                // Free resident music weights before loading the CPU-only text model.
                state.stop_engine();
                report_progress(&state, &app, &operation, Progress::new("title"));
                let input = crate::titles::request(&params, &meta);
                match crate::titles::generate(
                    &state.resources,
                    &state.db.root,
                    std::path::Path::new(&settings.models_dir),
                    &input,
                    &cancel,
                    progress,
                )
                .await
                {
                    Ok(title) => {
                        meta["generated_title"] = serde_json::json!(title);
                        meta["title_model"] = serde_json::json!(crate::titles::manifest().revision);
                    }
                    Err(crate::error::Error::Cancelled) => {
                        return Err(crate::error::Error::Cancelled)
                    }
                    Err(error) => {
                        tracing::warn!("Automatic title: {error}");
                        meta["title_generation_error"] = serde_json::json!(error.to_string());
                    }
                }
            }
            report_progress(&state, &app, &operation, Progress::new("saving"));
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
        if let Err(error) = outcome {
            if !state.runtime.is_shutdown() {
                let _ = state.db.fail(&sid, &error.to_string());
            }
        }
        drop(operation);
        state.changed(&app);
    }
    state.stop_engine();
}
