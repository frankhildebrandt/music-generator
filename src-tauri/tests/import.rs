use music_generator::{
    db::Database,
    import::import_legacy,
    types::{GenerateRequest, SongFilter},
};
use serde_json::json;

#[test]
fn import_copies_audio_metadata_and_wal_without_touching_the_source() {
    let old = tempfile::tempdir().unwrap();
    let source = Database::open(&old.path().join("data")).unwrap();
    let ids = source
        .enqueue(&GenerateRequest {
            prompt: "old song".into(),
            ..Default::default()
        })
        .unwrap();
    source.claim_next().unwrap();
    source
        .finish(&ids[0], "old.wav", &json!({"lyrics":"hello"}))
        .unwrap();
    let songdir = old.path().join("data/songs");
    std::fs::create_dir_all(&songdir).unwrap();
    std::fs::write(songdir.join("old.wav"), b"old audio").unwrap();
    source
        .patch(&ids[0], &json!({"favorite":true,"title":"Original"}))
        .unwrap();
    let new = tempfile::tempdir().unwrap();
    let db = Database::open(new.path()).unwrap();
    assert_eq!(import_legacy(&db, old.path()).unwrap()["songs"], 1);
    assert_eq!(db.song(&ids[0]).unwrap().title.as_deref(), Some("Original"));
    assert!(db.song(&ids[0]).unwrap().favorite);
    assert_eq!(
        std::fs::read(new.path().join("songs/old.wav")).unwrap(),
        b"old audio"
    );
    assert_eq!(
        std::fs::read(songdir.join("old.wav")).unwrap(),
        b"old audio"
    );
    assert!(import_legacy(&db, old.path()).is_err());
}

#[test]
fn failed_import_leaves_the_destination_empty() {
    let old = tempfile::tempdir().unwrap();
    let source = Database::open(&old.path().join("data")).unwrap();
    let ids = source
        .enqueue(&GenerateRequest {
            prompt: "missing audio".into(),
            ..Default::default()
        })
        .unwrap();
    source.claim_next().unwrap();
    source.finish(&ids[0], "missing.wav", &json!({})).unwrap();
    let new = tempfile::tempdir().unwrap();
    let db = Database::open(new.path()).unwrap();
    assert!(import_legacy(&db, old.path()).is_err());
    assert!(db.songs(&SongFilter::default()).unwrap().is_empty());
}
