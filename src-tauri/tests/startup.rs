use music_generator::{db::Database, state::AppState};
use std::sync::atomic::Ordering;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn completed_setup_survives_restart_without_models_or_engine_startup() {
    let root = tempfile::tempdir().unwrap();
    let db = Database::open(root.path()).unwrap();
    let state = AppState::new(db, root.path().join("no-engine"));
    assert!(!state.restore_setup().unwrap());
    assert!(!state.ready.load(Ordering::SeqCst));
    let mut settings = state.db.settings().unwrap();
    settings.lm_model = "acestep-5Hz-lm-4B-Q8_0.gguf".into();
    settings.setup_complete = true;
    state.db.save_settings(&settings).unwrap();
    drop(state);

    let state = AppState::new(
        Database::open(root.path()).unwrap(),
        root.path().join("no-engine"),
    );
    assert!(state.restore_setup().unwrap());
    assert!(state.ready.load(Ordering::SeqCst));
    assert!(state.engine.lock().await.is_none());
    let error = state
        .start_engine(&settings, &CancellationToken::new())
        .await
        .err()
        .expect("missing models must be detected when needed");
    assert!(error.to_string().contains("Models and hardware test"));
    assert!(state.db.settings().unwrap().setup_complete);
}
