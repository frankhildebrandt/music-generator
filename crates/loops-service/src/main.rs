mod engine;
mod imports;
mod jobs;
mod models;
mod progress;
mod transport;
#[cfg(windows)]
mod windows_pipe;
use music_core::{
    error::{message, Result},
    loops::GenerationRequest,
};
use serde_json::{json, Value};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

struct Service {
    root: PathBuf,
    resources: PathBuf,
    models: Mutex<PathBuf>,
    jobs: Mutex<HashMap<String, jobs::Job>>,
    serial: Semaphore,
    engine: Mutex<Option<engine::Engine>>,
    last_request: std::sync::atomic::AtomicU64,
}
fn root() -> PathBuf {
    if let Some(p) = std::env::var_os("MUSIC_LOOPS_PROFILE") {
        return p.into();
    }
    #[cfg(windows)]
    {
        PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default())
            .join("MusicGeneratorLoops")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
            .join("Library/Application Support/MusicGeneratorLoops")
    }
}
fn seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[tokio::main]
async fn main() -> Result<()> {
    if let Some(code) = music_core::process::supervise() {
        std::process::exit(code);
    }
    let root = root();
    std::fs::create_dir_all(&root)?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("service.lock"))?;
    if fs2::FileExt::try_lock_exclusive(&lock).is_err() {
        return Ok(());
    }
    let resources = std::env::var_os("MUSIC_LOOPS_RESOURCES")
        .map(PathBuf::from)
        .unwrap_or(
            std::env::current_exe()?
                .parent()
                .ok_or_else(|| message("Missing service directory"))?
                .join("resources"),
        );
    let models = models::resolve_from(&models::app_roots()?);
    let service = Arc::new(Service {
        root,
        resources,
        models: Mutex::new(models),
        jobs: Mutex::new(HashMap::new()),
        serial: Semaphore::new(1),
        engine: Mutex::new(None),
        last_request: std::sync::atomic::AtomicU64::new(seconds()),
    });
    transport::serve(service).await
}
async fn dispatch(service: Arc<Service>, req: Value) -> Result<Value> {
    service
        .last_request
        .store(seconds(), std::sync::atomic::Ordering::Relaxed);
    if req["version"] != 1 {
        return Err(message("Incompatible plugin/service protocol"));
    }
    let instance = req["instance"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() < 100)
        .ok_or_else(|| message("Missing instance ID"))?
        .to_owned();
    let data = &req["data"];
    let action = req["action"].as_str().unwrap_or("");
    if ["status", "models", "install", "generate"].contains(&action) {
        models::refresh(&service).await?;
    }
    match action {
        "status" => Ok(
            json!({"version":1,"dc_removal":true,"sound_idea":true,"models":service.models.lock().await.to_string_lossy(),"root":service.root}),
        ),
        "models" => {
            let folder = service.models.lock().await.clone();
            Ok(
                json!({"folder":folder,"default_model":music_core::loops::DEFAULT_MODEL,"models":music_core::manifest::manifest().models.into_iter().filter(|m|music_core::loops::REQUIRED_MODELS.contains(&m.file.as_str())).map(|m|json!({"file":m.file,"bytes":m.estimated_bytes,"present":folder.join(&m.file).is_file()})).collect::<Vec<_>>()}),
            )
        }
        "install" => jobs::enqueue_install(service, instance).await,
        "generate" => {
            let request: GenerationRequest = serde_json::from_value(data.clone())?;
            request.validate()?;
            jobs::enqueue_generation(service, instance, request).await
        }
        "job" => {
            let jobs = service.jobs.lock().await;
            let job = jobs
                .get(data["id"].as_str().unwrap_or(""))
                .filter(|j| j.instance == instance)
                .ok_or_else(|| message("Unknown job"))?;
            Ok(job.value.clone())
        }
        "cancel" | "detach" => {
            let mut jobs = service.jobs.lock().await;
            for (id, job) in jobs.iter_mut().filter(|(id, j)| {
                j.instance == instance
                    && (req["action"] == "detach" || Some(id.as_str()) == data["id"].as_str())
            }) {
                let _ = id;
                job.cancel.cancel();
                if ["queued", "running"].contains(&job.value["status"].as_str().unwrap_or("")) {
                    job.value["status"] = json!("cancelled");
                }
            }
            Ok(json!({}))
        }
        "import" => {
            let path = PathBuf::from(
                data["path"]
                    .as_str()
                    .ok_or_else(|| message("Missing import path"))?,
            );
            let root = service.root.clone();
            tokio::task::spawn_blocking(move || imports::import(&root, &path))
                .await
                .map_err(|e| message(e.to_string()))?
        }
        "edit" | "zero" => {
            let source = PathBuf::from(
                data["path"]
                    .as_str()
                    .ok_or_else(|| message("Missing source"))?,
            );
            let root = service.root.clone();
            let data = data.clone();
            let zero = req["action"] == "zero";
            tokio::task::spawn_blocking(move || {
                let audio = music_core::audio::Audio::read(&source)?;
                if zero {
                    let start = data["start"].as_u64().unwrap_or(0) as usize;
                    let frames = data["frames"]
                        .as_u64()
                        .unwrap_or(audio.samples.len() as u64)
                        as usize;
                    let cut = music_core::loops::zero_start(&audio, start, frames);
                    let score = music_core::loops::seam_score(&audio, cut, frames);
                    return Ok(json!({"start":cut,"score":score}));
                }
                let mut recipe: music_core::loops::EditRecipe =
                    serde_json::from_value(data["recipe"].clone())?;
                if data["fit"].as_bool().unwrap_or(false) {
                    let bar_frames = data["bar_frames"].as_f64().unwrap_or(0.0);
                    (recipe.start, recipe.end) =
                        music_core::loops::loop_region(&audio, recipe.frames, bar_frames, true)?;
                }
                let edited = music_core::loops::edit(&audio, &recipe)?;
                let path = root
                    .join("assets")
                    .join(format!("{}.wav", music_core::types::id()));
                std::fs::create_dir_all(path.parent().unwrap())?;
                edited.write(&path)?;
                Ok(json!({"path":path,"recipe":recipe}))
            })
            .await
            .map_err(|e| message(e.to_string()))?
        }
        _ => Err(message("Unknown service action")),
    }
}
