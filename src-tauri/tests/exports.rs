use music_generator::{db::Database, exports::export, types::GenerateRequest};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;

fn library(root: &std::path::Path) -> (Arc<Database>, String, PathBuf) {
    let db = Arc::new(Database::open(root).unwrap());
    let id = db
        .enqueue(&GenerateRequest {
            prompt: "rain".into(),
            variants: 1,
            ..Default::default()
        })
        .unwrap()[0]
        .clone();
    db.claim_next().unwrap();
    let dir = PathBuf::from(db.settings().unwrap().songs_dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("original.wav");
    std::fs::write(&file, b"lossless original").unwrap();
    db.finish(&id, "original.wav", &json!({})).unwrap();
    (db, id, file)
}

#[tokio::test]
async fn wav_audio_and_zip_preserve_original_audio_bytes() {
    let root = tempfile::tempdir().unwrap();
    let (db, id, original) = library(root.path());
    let audio = root.path().join("saved.wav");
    export(
        db.clone(),
        root.path().into(),
        "audio".into(),
        Some(id),
        None,
        audio.clone(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(audio).unwrap(), b"lossless original");
    let zip = root.path().join("songs.zip");
    export(
        db,
        root.path().into(),
        "zip".into(),
        None,
        None,
        zip.clone(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip).unwrap()).unwrap();
    let mut entry = archive.by_index(0).unwrap();
    assert!(entry.name().ends_with(".wav"));
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut entry, &mut bytes).unwrap();
    assert_eq!(bytes, b"lossless original");
    assert_eq!(std::fs::read(original).unwrap(), b"lossless original");
}

#[tokio::test]
async fn unavailable_mp3_converter_does_not_damage_existing_files() {
    let root = tempfile::tempdir().unwrap();
    let (db, id, original) = library(root.path());
    let mut settings = db.settings().unwrap();
    settings.export_format = "mp3".into();
    db.save_settings(&settings).unwrap();
    let destination = root.path().join("saved.mp3");
    std::fs::write(&destination, b"keep existing destination").unwrap();
    let error = export(
        db.clone(),
        root.path().into(),
        "audio".into(),
        Some(id.clone()),
        None,
        destination.clone(),
        CancellationToken::new(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "mp3_failed");
    assert_eq!(std::fs::read(&original).unwrap(), b"lossless original");
    assert_eq!(
        std::fs::read(destination).unwrap(),
        b"keep existing destination"
    );
    assert!(export(
        db,
        root.path().into(),
        "audio".into(),
        Some(id),
        None,
        original.clone(),
        CancellationToken::new()
    )
    .await
    .is_err());
    assert_eq!(std::fs::read(original).unwrap(), b"lossless original");
}
