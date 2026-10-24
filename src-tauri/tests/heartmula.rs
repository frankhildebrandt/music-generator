use music_generator::{
    catalog::{self, Selection},
    db::Database,
    heartmula::{self, Options},
    types::{GenerateRequest, Settings},
};
use serde_json::json;
use std::path::Path;
fn selection() -> Selection {
    Selection {
        provider: "heartmula".into(),
        synthesis: "heartmula-q8_0.gguf".into(),
        language: String::new(),
    }
}
#[test]
fn native_profile_contains_codec_and_no_ace_step_dependencies() {
    let assets = catalog::required(&selection()).unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].estimated_bytes, 7659762592);
    assert!(assets[0]
        .url(&catalog::catalog())
        .contains("/ef1ab2598f6d4d101486e4c5b9e605c687f20642/HeartMuLa-GGUF/"));
    let mut mixed = selection();
    mixed.language = "acestep-5Hz-lm-4B-Q8_0.gguf".into();
    assert!(catalog::required(&mixed).is_err());
    let mut settings = Settings::default();
    selection().apply(&mut settings).unwrap();
    assert_eq!(Selection::from_settings(&settings), selection());
}
#[test]
fn heart_sampling_is_separate_from_saved_ace_step_values() {
    let root = tempfile::tempdir().unwrap();
    let db = Database::open(root.path()).unwrap();
    let request = GenerateRequest {
        prompt: "soft piano".into(),
        instrumental: true,
        inference_steps: 80,
        lm_temperature: Some(1.2),
        model_selection: Some(selection()),
        heartmula_options: Some(Options::default()),
        ..Default::default()
    };
    let id = db.enqueue(&request).unwrap().remove(0);
    let (_, params) = db.claim_next().unwrap().unwrap();
    let mut settings = db.settings().unwrap();
    selection().apply(&mut settings).unwrap();
    let args = heartmula::arguments(&params, &settings, Path::new("output.wav")).unwrap();
    assert!(args
        .windows(2)
        .any(|w| w == ["--num-inference-steps", "10"]));
    assert!(args.windows(2).any(|w| w == ["--temperature", "1"]));
    assert!(args.windows(2).any(|w| w == ["--lyrics", "[Instrumental]"]));
    assert!(args.contains(&"heartmula.mem_saver=true".into()));
    assert!(!args.iter().any(|s| s.contains("--lm-model")));
    assert_eq!(params["inference_steps"], 80);
    assert_eq!(params["lm_temperature"], 1.2);
    db.finish(&id, "song.wav", &json!({"provider":"heartmula"}))
        .unwrap();
    assert_eq!(
        db.song(&id).unwrap().engine.as_deref(),
        Some("heartmula_audio_cpp")
    );
    assert_eq!(db.song(&id).unwrap().params, params);
}
#[test]
fn unsupported_requests_fail_before_enqueueing() {
    for value in [
        json!({"instrumental":false,"lyrics":""}),
        json!({"instrumental":true,"duration":241}),
        json!({"instrumental":true,"heartmula_options":{"codec_steps":0}}),
        json!({"instrumental":true,"heartmula_options":{"temperature":0}}),
    ] {
        assert!(heartmula::validate(&value).is_err());
    }
    let root = tempfile::tempdir().unwrap();
    let db = Database::open(root.path()).unwrap();
    assert!(db
        .enqueue(&GenerateRequest {
            prompt: "song".into(),
            instrumental: false,
            model_selection: Some(selection()),
            ..Default::default()
        })
        .is_err());
    assert!(!db.busy().unwrap());
    let p = json!({"caption":"bright pop","lyrics":"[Verse]\nA line with \"quotes\" and $HOME","instrumental":false,"duration":10,"seed":42});
    let before = p.clone();
    let mut s = Settings::default();
    selection().apply(&mut s).unwrap();
    let args = heartmula::arguments(&p, &s, Path::new("some output.wav")).unwrap();
    assert!(args.contains(&p["lyrics"].as_str().unwrap().to_string()));
    assert_eq!(p, before);
}
#[test]
fn decoder_outputs_are_validated_before_library_insertion() {
    let mut w = Vec::new();
    w.extend(b"RIFF");
    w.extend(40u32.to_le_bytes());
    w.extend(b"WAVEfmt ");
    w.extend(16u32.to_le_bytes());
    w.extend(1u16.to_le_bytes());
    w.extend(2u16.to_le_bytes());
    w.extend(48000u32.to_le_bytes());
    w.extend(192000u32.to_le_bytes());
    w.extend(4u16.to_le_bytes());
    w.extend(16u16.to_le_bytes());
    w.extend(b"data");
    w.extend(4u32.to_le_bytes());
    w.extend([1, 0, 2, 0]);
    let info = heartmula::wav_info(&w).unwrap();
    assert_eq!((info.1, info.2), (48000, 2));
    assert!(info.0 > 0.0);
    assert!(heartmula::wav_info(b"error: codec failed").is_err());
    w.pop();
    assert!(heartmula::wav_info(&w).is_err());
}

#[test]
fn provider_profiles_preserve_the_last_selection_after_reopening() {
    let root = tempfile::tempdir().unwrap();
    let db = Database::open(root.path()).unwrap();
    let mut settings = db.settings().unwrap();
    settings.lm_model = "acestep-5Hz-lm-4B-Q8_0.gguf".into();
    settings.dit = "xl".into();
    let ace = Selection::from_settings(&settings);
    selection().apply(&mut settings).unwrap();
    db.save_settings(&settings).unwrap();
    drop(db);
    let settings = Database::open(root.path()).unwrap().settings().unwrap();
    assert_eq!(settings.model_profiles["ace-step"], ace);
    assert_eq!(settings.model_profiles["heartmula"], selection());
}

#[test]
fn first_run_heartmula_does_not_download_ace_step_starter_weights() {
    let request = music_generator::setup::SetupRequest {
        selection: Some(selection()),
        lm: "0.6B".into(),
        xl: true,
        models_dir: "/models".into(),
        songs_dir: "/songs".into(),
    };
    let plan = music_generator::setup::plan(&request).unwrap();
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].0.file, "heartmula-q8_0.gguf");
    assert!(plan[0].1.contains("audio-cpp/audio.cpp-gguf/resolve/"));
}

#[tokio::test]
async fn heart_download_uses_its_pinned_asset_url_and_no_ace_dependencies() {
    use axum::{routing::get, Router};
    use tokio_util::sync::CancellationToken;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new().route("/heart", get(|| async { "abc" }));
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let root = tempfile::tempdir().unwrap();
    let db = Database::open(root.path()).unwrap();
    let mut settings = db.settings().unwrap();
    settings.models_dir = root.path().to_string_lossy().into();
    db.save_settings(&settings).unwrap();
    let mut fixture = catalog::catalog();
    fixture.base_url = "http://127.0.0.1:1/not-used".into();
    let model = fixture
        .models
        .iter_mut()
        .find(|m| m.file == selection().synthesis)
        .unwrap();
    model.url = Some(format!("http://{address}/heart"));
    model.sha256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into();
    model.estimated_bytes = 3;
    catalog::install_profile(
        &fixture,
        &db,
        &selection(),
        &CancellationToken::new(),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(
        std::fs::read(root.path().join("heartmula-q8_0.gguf")).unwrap(),
        b"abc"
    );
    assert_eq!(
        Selection::from_settings(&db.settings().unwrap()),
        selection()
    );
    task.abort();
}
