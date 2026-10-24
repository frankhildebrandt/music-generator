use music_generator::{db::Database, state::validate_settings};

#[test]
fn device_export_and_timeout_survive_restart_without_overwriting_fallback() {
    let root = tempfile::tempdir().unwrap();
    let db = Database::open(root.path()).unwrap();
    let mut settings = db.settings().unwrap();
    assert_eq!(settings.lm_device, "auto");
    assert_eq!(settings.export_format, "wav");
    assert_eq!(settings.timeout_minutes, 30.0);
    settings.lm_device = "cpu".into();
    settings.export_format = "mp3".into();
    settings.timeout_minutes = 75.0;
    validate_settings(&settings).unwrap();
    db.save_settings(&settings).unwrap();
    drop(db);
    let settings = Database::open(root.path()).unwrap().settings().unwrap();
    assert!(settings.cpu_lm());
    assert_eq!(settings.mode, "single");
    assert_eq!(settings.export_format, "mp3");
    assert_eq!(settings.timeout_minutes, 75.0);
    let mut settings = settings;
    settings.lm_device = "auto".into();
    assert!(!settings.cpu_lm());
    settings.mode = "split".into();
    assert!(settings.cpu_lm());
    settings.export_format = "flac".into();
    assert!(validate_settings(&settings).is_err());
    settings.export_format = "wav".into();
    settings.lm_device = "gpu".into();
    assert!(validate_settings(&settings).is_err());
}
