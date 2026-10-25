use music_generator::runtime::OperationKind;
use music_generator::{db::Database, state::AppState};

#[tokio::test]
async fn completed_setup_survives_restart_without_models_or_engine_startup() {
    let root = tempfile::tempdir().unwrap();
    let db = Database::open(root.path()).unwrap();
    let state = AppState::new(db, root.path().join("no-engine"));
    assert!(!state.restore_setup().unwrap());
    assert!(!state.runtime.is_ready());
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
    assert!(state.runtime.is_ready());
    state.stop_engine();
    assert!(
        state.runtime.is_ready(),
        "Releasing model memory must not undo completed setup"
    );
    assert_eq!(state.health().await["idle"], true);
    let operation = state.runtime.try_begin(OperationKind::Analysis).unwrap();
    let error = state
        .start_engine(&operation, &settings)
        .await
        .err()
        .expect("missing models must be detected when needed");
    assert!(error.to_string().contains("Models and hardware test"));
    assert!(state.db.settings().unwrap().setup_complete);
}
