use super::*;
use crate::{db::Database, error::Error, types::GenerateRequest};
use std::cell::Cell;

struct Library {
    root: tempfile::TempDir,
    db: Database,
    original: PathBuf,
    proposed: Settings,
}

impl Library {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let db = Database::open(root.path()).unwrap();
        let mut settings = db.settings().unwrap();
        settings.setup_complete = true;
        db.save_settings(&settings).unwrap();
        let id = db
            .enqueue(&GenerateRequest {
                title: "Saved song".into(),
                prompt: "rain".into(),
                variants: 1,
                ..Default::default()
            })
            .unwrap()[0]
            .clone();
        db.claim_next().unwrap();
        let original = PathBuf::from(&settings.songs_dir).join("saved.wav");
        std::fs::create_dir_all(original.parent().unwrap()).unwrap();
        std::fs::write(&original, b"original audio").unwrap();
        db.finish(&id, "saved.wav", &json!({})).unwrap();
        let mut proposed = settings;
        proposed.songs_dir = root.path().join("new songs").to_string_lossy().into();
        proposed.tested_fingerprint = "new hardware test".into();
        Self {
            root,
            db,
            original,
            proposed,
        }
    }

    fn assert_unchanged(&self) {
        assert_eq!(std::fs::read(&self.original).unwrap(), b"original audio");
        assert_eq!(
            Path::new(&self.db.settings().unwrap().songs_dir),
            self.original.parent().unwrap()
        );
        assert!(self.db.settings().unwrap().setup_complete);
        assert_ne!(
            self.db.settings().unwrap().tested_fingerprint,
            self.proposed.tested_fingerprint
        );
        assert!(!Path::new(&self.proposed.songs_dir)
            .join("saved.wav")
            .exists());
    }
}

#[tokio::test]
async fn failed_or_cancelled_hardware_test_keeps_the_existing_library() {
    for error in [message("Hardware self-test failed"), Error::Cancelled] {
        let library = Library::new();
        let result = commit_configuration(
            &library.db,
            async {
                library.assert_unchanged();
                Err(error)
            },
            &CancellationToken::new(),
            |_| panic!("Failed setup must not grant new access"),
        )
        .await;
        assert!(result.is_err());
        library.assert_unchanged();
    }
}

#[tokio::test]
async fn cancellation_at_the_end_of_the_test_does_not_move_songs() {
    let library = Library::new();
    let cancel = CancellationToken::new();
    let result = commit_configuration(
        &library.db,
        async {
            cancel.cancel();
            Ok(library.proposed.clone())
        },
        &cancel,
        |_| panic!("Cancelled setup must not grant new access"),
    )
    .await;
    assert!(matches!(result, Err(Error::Cancelled)));
    library.assert_unchanged();
}

#[tokio::test]
async fn denied_playback_access_leaves_the_old_folder_and_settings_intact() {
    let library = Library::new();
    let result = commit_configuration(
        &library.db,
        async { Ok(library.proposed.clone()) },
        &CancellationToken::new(),
        |_| Err(message("Playback access denied")),
    )
    .await;
    assert!(result.is_err());
    library.assert_unchanged();
}

#[tokio::test]
async fn cancellation_after_playback_authorization_still_preserves_the_library() {
    let library = Library::new();
    let cancel = CancellationToken::new();
    let result = commit_configuration(
        &library.db,
        async { Ok(library.proposed.clone()) },
        &cancel,
        |_| {
            cancel.cancel();
            Ok(())
        },
    )
    .await;
    assert!(matches!(result, Err(Error::Cancelled)));
    library.assert_unchanged();
}

#[tokio::test]
async fn setup_without_a_folder_change_preserves_queued_jobs() {
    let library = Library::new();
    let id = library
        .db
        .enqueue(&GenerateRequest {
            prompt: "waiting song".into(),
            variants: 1,
            ..Default::default()
        })
        .unwrap()[0]
        .clone();
    let mut proposed = library.proposed.clone();
    proposed.songs_dir = library.db.settings().unwrap().songs_dir;
    commit_configuration(
        &library.db,
        async { Ok(proposed) },
        &CancellationToken::new(),
        |_| Ok(()),
    )
    .await
    .unwrap();
    assert_eq!(library.db.song(&id).unwrap().status, "queued");
    assert_eq!(std::fs::read(&library.original).unwrap(), b"original audio");
}

#[tokio::test]
async fn successful_setup_grants_playback_before_moving_and_saving() {
    let library = Library::new();
    let granted = Cell::new(false);
    let settings = commit_configuration(
        &library.db,
        async {
            library.assert_unchanged();
            assert!(!granted.get());
            Ok(library.proposed.clone())
        },
        &CancellationToken::new(),
        |songs| {
            library.assert_unchanged();
            assert_eq!(songs, library.root.path().join("new songs"));
            granted.set(true);
            Ok(())
        },
    )
    .await
    .unwrap();
    assert!(granted.get());
    assert!(!library.original.exists());
    assert_eq!(
        std::fs::read(Path::new(&settings.songs_dir).join("saved.wav")).unwrap(),
        b"original audio"
    );
    assert_eq!(
        library.db.settings().unwrap().tested_fingerprint,
        "new hardware test"
    );
    assert_eq!(library.db.settings().unwrap().songs_dir, settings.songs_dir);
}
