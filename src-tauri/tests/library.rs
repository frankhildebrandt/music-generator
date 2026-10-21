use music_generator::{
    db::Database,
    types::{GenerateRequest, SongFilter},
};
use serde_json::json;

fn request() -> GenerateRequest {
    GenerateRequest {
        prompt: "rainy night".into(),
        seed: 42,
        variants: 2,
        ..Default::default()
    }
}

#[test]
fn queue_is_fifo_and_keeps_variant_seeds() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let ids = db.enqueue(&request()).unwrap();
    let (first, p) = db.claim_next().unwrap().unwrap();
    assert_eq!(first, ids[0]);
    assert_eq!(p["seed"], 42);
    assert_eq!(p["lyrics"], "[Instrumental]");
    db.finish(&first, "song.wav", &json!({"bpm":90})).unwrap();
    let (second, p) = db.claim_next().unwrap().unwrap();
    assert_eq!(second, ids[1]);
    assert_eq!(p["seed"], 43);
    assert!(db.claim_next().unwrap().is_none());
    assert_eq!(db.song(&ids[0]).unwrap().letter, "A");
    assert_eq!(db.song(&ids[1]).unwrap().letter, "B");
}

#[test]
fn cancellation_cannot_be_overwritten_by_a_late_result() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let ids = db.enqueue(&request()).unwrap();
    db.claim_next().unwrap();
    assert!(db.cancel_or_delete(&ids[0]).unwrap());
    assert!(!db.finish(&ids[0], "late.wav", &json!({})).unwrap());
    db.fail(&ids[0], "late error").unwrap();
    assert_eq!(db.song(&ids[0]).unwrap().status, "cancelled");
    db.retry(&ids[0]).unwrap();
    assert_eq!(db.claim_next().unwrap().unwrap().0, ids[0]);
}

#[test]
fn interrupted_generation_is_resumed_after_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let ids = db.enqueue(&request()).unwrap();
    db.claim_next().unwrap();
    drop(db);
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.claim_next().unwrap().unwrap().0, ids[0]);
}

#[test]
fn folders_favorites_and_deletion_preserve_songs() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let folder = db.folder_save(None, " Ambient ").unwrap();
    let mut req = request();
    req.folder_id = Some(folder["id"].as_str().unwrap().into());
    let ids = db.enqueue(&req).unwrap();
    db.patch(&ids[0], &json!({"favorite":true,"title":"Evening"}))
        .unwrap();
    assert_eq!(
        db.songs(&SongFilter {
            favorites: true,
            ..Default::default()
        })
        .unwrap()
        .len(),
        1
    );
    assert_eq!(
        db.songs(&SongFilter {
            q: "Evening".into(),
            ..Default::default()
        })
        .unwrap()
        .len(),
        1
    );
    db.folder_delete(folder["id"].as_str().unwrap()).unwrap();
    assert_eq!(db.folders().unwrap()["none"], 2);
    assert_eq!(db.songs(&SongFilter::default()).unwrap().len(), 2);
}

#[test]
fn empty_prompt_and_invalid_variants_are_rejected_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    assert!(db.enqueue(&GenerateRequest::default()).is_err());
    let mut req = request();
    req.variants = 51;
    assert!(db.enqueue(&req).is_err());
    assert!(db.songs(&SongFilter::default()).unwrap().is_empty());
}

#[test]
fn direct_lookup_and_export_work_beyond_the_display_limit() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let mut req = request();
    req.variants = 50;
    let first = db.enqueue(&req).unwrap()[0].clone();
    for _ in 0..10 {
        db.enqueue(&req).unwrap();
    }
    assert_eq!(db.songs(&SongFilter::default()).unwrap().len(), 500);
    assert_eq!(db.song(&first).unwrap().id, first);
}

#[test]
fn folder_move_rolls_back_on_collision() {
    let root = tempfile::tempdir().unwrap();
    let db = Database::open(root.path()).unwrap();
    let sid = db.enqueue(&request()).unwrap()[0].clone();
    db.claim_next().unwrap();
    db.finish(&sid, "one.wav", &json!({})).unwrap();
    let old = std::path::PathBuf::from(db.settings().unwrap().songs_dir);
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join("one.wav"), b"audio").unwrap();
    let new = root.path().join("new");
    std::fs::create_dir_all(&new).unwrap();
    std::fs::write(new.join("one.wav"), b"other").unwrap();
    // Complete/cancel other queued variants before a move.
    let next = db.claim_next().unwrap().unwrap().0;
    db.cancel_or_delete(&next).unwrap();
    assert!(db.change_songs_dir(&new).is_err());
    assert_eq!(db.settings().unwrap().songs_dir, old.to_string_lossy());
    assert_eq!(std::fs::read(new.join("one.wav")).unwrap(), b"other");
}

#[test]
fn imported_paths_cannot_escape_the_song_directory() {
    let root = tempfile::tempdir().unwrap();
    for name in [
        "../song.wav",
        "/etc/passwd",
        "C:\\song.wav",
        ".",
        "foo/bar.wav",
    ] {
        assert!(music_generator::db::safe_song_path(root.path(), name).is_err());
    }
}
