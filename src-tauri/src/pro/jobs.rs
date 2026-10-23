use super::{audio::Audio, model::*, render};
use crate::{
    db::Database,
    engine::client::EngineClient,
    error::{message, Result},
    state::AppState,
};
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

pub async fn run(
    state: &AppState,
    app: &AppHandle,
    jid: &str,
    project: Project,
    request: Generation,
    cancel: &CancellationToken,
) -> Result<()> {
    let client = state.start_engine(&state.db.settings()?, cancel).await?;
    let pid = project.id.clone();
    let result = execute(
        state.db.clone(),
        &client,
        jid,
        project,
        request,
        cancel,
        |_| {
            let _ = app.emit("pro-changed", &pid);
        },
    )
    .await;
    let _ = app.emit("pro-changed", &pid);
    result
}

/// Run the same persistent generation pipeline from the desktop worker or a hardware smoke test.
pub async fn execute(
    db: Arc<Database>,
    client: &EngineClient,
    jid: &str,
    project: Project,
    request: Generation,
    cancel: &CancellationToken,
    progress: impl Fn(String),
) -> Result<()> {
    let settings = db.settings()?;
    if !std::path::Path::new(&settings.models_dir)
        .join(BASE_MODEL)
        .is_file()
    {
        return Err(message("Download the Base model in Pro Mode first"));
    }
    let model = crate::manifest::manifest()
        .models
        .into_iter()
        .find(|m| m.kind == "pro")
        .ok_or_else(|| message("Missing Base model manifest"))?;
    let mut verifier = crate::verification::ModelVerifier::open(&db.root);
    if verifier
        .verify(
            &std::path::Path::new(&settings.models_dir).join(BASE_MODEL),
            &model.sha256,
            cancel,
            |_, _| {},
        )
        .await?
        == crate::verification::Verification::Invalid
    {
        return Err(message(
            "Base model checksum failed. Download the model again.",
        ));
    }
    let track = project
        .tracks
        .iter()
        .find(|t| t.id == request.track_id)
        .ok_or_else(|| message("Missing target track"))?;
    let render_db = db.clone();
    let snapshot = project.clone();
    let exclude = request.track_id.clone();
    let mut context = tauri::async_runtime::spawn_blocking(move || {
        render::render(&render_db, &snapshot, true, Some(&exclude), None)
    })
    .await
    .map_err(|e| message(e.to_string()))??;
    if cancel.is_cancelled() {
        return Err(crate::error::Error::Cancelled);
    }
    let has_context = context.peak() > 0.000001;
    context.headroom(0.8);
    let source = if has_context {
        let path = db.root.join(format!("context-{jid}.wav"));
        context.write(&path)?;
        Some(path)
    } else {
        None
    };
    drop(context);
    let task = if has_context { "lego" } else { "text2music" };
    let params = json!({"task_type":task,"track":track.instrument,"caption":format!("Isolated {} track. {}",track.instrument,request.prompt),
        "lyrics":"[Instrumental]","bpm":project.bpm,"duration":project.duration as f64/RATE as f64,
        "keyscale":project.key,"timesignature":project.beats_per_bar.to_string(),"seed":request.seed,
        "inference_steps":request.steps,"guidance_scale":1.0,"shift":1.0,"output_format":"wav32","synth_model":BASE_MODEL});
    let outcome = client
        .generate_stem(&params, source.as_deref(), cancel, |text| {
            let _ = db.pro_job_status(jid, "running", &text);
            progress(text);
        })
        .await;
    if let Some(path) = source {
        let _ = std::fs::remove_file(path);
    }
    let bytes = outcome?;
    if cancel.is_cancelled() {
        return Err(crate::error::Error::Cancelled);
    }
    let generated = Audio::decode(&bytes)?;
    if generated.samples.len() as u64 != project.duration {
        return Err(message(
            "Engine returned an unexpected duration; original take preserved",
        ));
    }
    let audio = if let Some(cid) = &request.clip_id {
        let clip = project
            .clips
            .iter()
            .find(|c| &c.id == cid)
            .ok_or_else(|| message("Missing source clip"))?;
        let take = project
            .takes
            .iter()
            .find(|t| t.id == clip.take_id)
            .ok_or_else(|| message("Missing source take"))?;
        render::splice(
            &Audio::read(&db.pro_path(&project.id, &take.file)?)?,
            &generated,
            clip,
            request.start,
            request.end,
        )?
    } else {
        generated
    };
    let take = Take::new(
        &track.id,
        &format!("{} · {}", track.name, crate::types::now()),
        audio.samples.len() as u64,
        json!({"generation":request,"engine":params,"model_sha256":model.sha256,
            "context":{"project_id":project.id,"revision":project.revision,"tracks":project.tracks,"clips":project.clips,
            "takes":project.takes.iter().map(|t|json!({"id":t.id,"file":t.file,"frames":t.frames})).collect::<Vec<_>>()}}),
        project.revision,
    );
    let path = db.pro_path(&project.id, &take.file)?;
    std::fs::create_dir_all(path.parent().unwrap())?;
    audio.write(&path)?;
    if cancel.is_cancelled() || !db.pro_complete(jid, &project.id, &take)? {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}
