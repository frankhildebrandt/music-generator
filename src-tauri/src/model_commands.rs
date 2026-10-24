use crate::{
    catalog::{self, Selection},
    error::{message, Result},
    state::AppState,
    verification::ModelVerifier,
};
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn model_catalog(state: State<'_, Arc<AppState>>) -> Result<Value> {
    let settings = state.db.settings()?;
    let root = Path::new(&settings.models_dir);
    let verifier = ModelVerifier::open(&state.db.root);
    let catalog = catalog::catalog();
    let mut result = serde_json::to_value(&catalog)?;
    result["models"] = Value::Array(
        catalog
            .models
            .iter()
            .map(|model| {
                let mut value = serde_json::to_value(model).expect("model serialization");
                let path = root.join(&model.file);
                value["verified"] = json!(verifier.is_verified(&path, &model.sha256));
                value["present"] = json!(path.is_file());
                value["partial_bytes"] = json!(root
                    .join(format!("{}.part", model.file))
                    .metadata()
                    .map(|m| m.len())
                    .unwrap_or(0)
                    .min(model.estimated_bytes));
                value
            })
            .collect(),
    );
    result["title_model"] = crate::titles::status(root, &verifier);
    result["selection"] = json!(Selection::from_settings(&settings));
    result["hardware"] = json!(crate::hardware::inspect());
    result["free_bytes"] = json!(fs2::available_space(root).ok());
    result["models_dir"] = json!(settings.models_dir);
    result["busy"] = json!(state.operation.try_lock().is_err());
    Ok(result)
}

/// Verify/download a complete profile before making it the default. The old profile
/// remains durable on errors, cancellation and shutdown. Queue rows are never rewritten.
#[tauri::command]
pub async fn use_models(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    selection: Selection,
) -> Result<Selection> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| message("Please wait for the current generation, analysis or download"))?;
    let settings = state.db.settings()?;
    if !settings.setup_complete {
        return Err(message("Complete the first-run assistant first"));
    }
    if !catalog::compatible(&selection, &crate::hardware::inspect())?
        && selection != Selection::from_settings(&settings)
    {
        return Err(message(
            "This model combination exceeds the estimated RAM budget. Choose smaller models",
        ));
    }
    let cancel = state.shutdown.child_token();
    *state.setup_cancel.lock().unwrap() = Some(cancel.clone());
    // Stopping first releases resident model memory and refreshes the engine's file
    // registry on the next generation, including newly downloaded models.
    state.stop_engine().await;
    let outcome =
        catalog::install_profile(&catalog::catalog(), &state.db, &selection, &cancel, |p| {
            let _ = app.emit("model-download-progress", p);
        })
        .await
        .map(|()| selection);
    *state.setup_cancel.lock().unwrap() = None;
    if !state.shutdown.is_cancelled() {
        state.restore_setup()?;
    }
    let _ = app.emit("models-changed", ());
    state.changed(&app);
    outcome
}

#[tauri::command]
pub fn cancel_model_download(state: State<'_, Arc<AppState>>) {
    if let Some(cancel) = state.setup_cancel.lock().unwrap().as_ref() {
        cancel.cancel();
    }
}

/// Shared auxiliary model; installing it never switches the active music profile.
#[tauri::command]
pub async fn download_title_model(state: State<'_, Arc<AppState>>, app: AppHandle) -> Result<()> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| message("Please wait for the current generation, analysis or download"))?;
    let settings = state.db.settings()?;
    if !settings.setup_complete {
        return Err(message("Complete the first-run assistant first"));
    }
    let cancel = state.shutdown.child_token();
    *state.setup_cancel.lock().unwrap() = Some(cancel.clone());
    state.stop_engine().await;
    let outcome = crate::titles::install(
        &state.db.root,
        Path::new(&settings.models_dir),
        &cancel,
        |p| {
            let _ = app.emit("model-download-progress", p);
        },
    )
    .await;
    *state.setup_cancel.lock().unwrap() = None;
    if !state.shutdown.is_cancelled() {
        state.restore_setup()?;
    }
    let _ = app.emit("models-changed", ());
    outcome
}
