//! Exercise full-track and region generation with real Base-model inference in an empty profile.
use music_generator::{
    db::Database,
    engine::{process::supervise, Engine},
    pro::{
        audio::Audio,
        jobs::execute,
        model::{Generation, RATE},
        render,
    },
};
use std::{path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(code) = supervise() {
        std::process::exit(code);
    }
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("Usage: pro_engine_smoke EMPTY_PROFILE MODELS RESOURCES STEMS".into());
    }
    let root = std::env::current_dir()?.join(&args[1]);
    if root.join("music.db").exists() {
        return Err("Choose an empty smoke-test profile".into());
    }
    let db = Arc::new(Database::open(&root)?);
    let mut settings = db.settings()?;
    settings.models_dir = PathBuf::from(&args[2])
        .canonicalize()?
        .to_string_lossy()
        .into();
    db.save_settings(&settings)?;
    let mut project = db.pro_create("Pro hardware smoke", 120, "A minor", RATE * 16)?;
    for name in ["01-drums.wav", "02-bass.wav", "03-chords.wav"] {
        project = db.pro_import(&project.id, &PathBuf::from(&args[4]).join(name), None)?;
    }
    project.tracks[0].instrument = "drums".into();
    project.tracks[1].instrument = "bass".into();
    project.tracks[2].instrument = "keyboard".into();
    for track in &mut project.tracks {
        track.gain = 0.5;
    }
    project = db.pro_save(&project)?;
    let cancel = CancellationToken::new();
    let engine = Engine::start(
        &PathBuf::from(&args[3]).canonicalize()?,
        &root.join("logs"),
        &settings,
        &cancel,
    )
    .await?;
    let mut request=Generation{track_id:project.tracks[1].id.clone(),clip_id:None,prompt:"Isolated clean electric bass guitar, dry fingerstyle funk groove in A minor at 120 BPM, tightly following the drums, bass only".into(),seed:20261024,steps:50,variants:1,start:0,end:project.duration};
    db.pro_enqueue(&project.id, &request)?;
    let (jid, snapshot, queued) = db.pro_claim()?.ok_or("Missing job")?;
    execute(
        db.clone(),
        &engine.client,
        &jid,
        snapshot,
        queued,
        &cancel,
        |s| println!("Full track: {s}"),
    )
    .await?;
    assert_eq!(db.pro_jobs(&project.id)?[0].status, "done");
    let updated = db.pro_project(&project.id)?;
    assert_eq!(
        updated.clips[1].take_id, project.clips[1].take_id,
        "Results must not auto-activate"
    );
    let full = updated.takes.last().ok_or("Missing generated take")?;
    let generated = Audio::read(&db.pro_path(&project.id, &full.file)?)?;
    assert!(generated.peak() > 0.0001);
    assert_eq!(generated.samples.len() as u64, project.duration);
    let mut activated = updated.clone();
    activated.clips[1].take_id = full.id.clone();
    project = db.pro_save(&activated)?;
    request.clip_id = Some(project.clips[1].id.clone());
    request.start = RATE * 6;
    request.end = RATE * 10;
    request.seed += 1;
    db.pro_enqueue(&project.id, &request)?;
    let (jid, snapshot, queued) = db.pro_claim()?.ok_or("Missing region job")?;
    execute(
        db.clone(),
        &engine.client,
        &jid,
        snapshot,
        queued,
        &cancel,
        |s| println!("Region: {s}"),
    )
    .await?;
    let updated = db.pro_project(&project.id)?;
    let region = updated.takes.last().ok_or("Missing region take")?;
    let changed = Audio::read(&db.pro_path(&project.id, &region.file)?)?;
    assert_eq!(
        &changed.samples[..(RATE * 6) as usize],
        &generated.samples[..(RATE * 6) as usize]
    );
    assert_eq!(
        &changed.samples[(RATE * 10) as usize..],
        &generated.samples[(RATE * 10) as usize..]
    );
    assert_ne!(
        &changed.samples[(RATE * 7) as usize..(RATE * 9) as usize],
        &generated.samples[(RATE * 7) as usize..(RATE * 9) as usize]
    );
    render::export(&db, &updated, &root.join("mix.wav"), false)?;
    render::export(&db, &updated, &root.join("stems.zip"), true)?;
    println!("PASS: full track and 6–10s region generated; outside samples bit-identical; old clips preserved; mix and stems exported. Profile: {}",root.display());
    Ok(())
}
