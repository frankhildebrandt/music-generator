use axum::{
    extract::Query,
    http::header::CONTENT_TYPE,
    routing::{get, post},
    Json, Router,
};
use music_generator::engine::client::{extract_audio, extract_json, selftest_ok, EngineClient};
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;

fn multipart() -> Vec<u8> {
    b"--song\r\nContent-Type: application/json\r\n\r\n{\"caption\":\"test\"}\r\n--song\r\nContent-Type: audio/wav\r\n\r\nRIFF\x00\xff\x01\r\n--song--\r\n".to_vec()
}

#[test]
fn multipart_preserves_binary_audio_and_extracts_metadata() {
    let ctype = "multipart/mixed; boundary=\"song\"";
    assert_eq!(
        extract_json(ctype, &multipart()).unwrap()["caption"],
        "test"
    );
    assert_eq!(
        extract_audio(ctype, &multipart()).unwrap(),
        (b"RIFF\x00\xff\x01".to_vec(), "wav".into())
    );
    assert!(extract_audio("application/json", b"{}").is_err());
    assert!(extract_json("application/json", b"[]").is_err());
}

#[test]
fn gpu_selftest_rejects_empty_and_repetitive_codes() {
    let codes = (0..50).map(|n| n.to_string()).collect::<Vec<_>>().join(",");
    assert!(selftest_ok(
        &json!({"audio_codes":codes,"bpm":37,"keyscale":"C major"})
    ));
    assert!(!selftest_ok(
        &json!({"audio_codes":"","bpm":90,"keyscale":"C major"})
    ));
    assert!(!selftest_ok(
        &json!({"audio_codes":vec!["1";50].join(","),"bpm":90,"keyscale":"C major"})
    ));
}

#[tokio::test]
async fn generation_chains_lm_and_synthesis_and_preserves_parameters() {
    let app = Router::new()
        .route(
            "/lm",
            post(|Json(request): Json<Value>| async move {
                assert_eq!(request["lm_model"], "test.gguf");
                assert_eq!(request["caption"], "piano");
                assert!(request.get("variants").is_none());
                Json(json!({"id":"lm"}))
            }),
        )
        .route(
            "/synth",
            post(|Json(request): Json<Value>| async move {
                assert_eq!(request["audio_codes"], "1,2,3");
                assert_eq!(request["output_format"], "wav16");
                assert_eq!(request["synth_model"], "sft.gguf");
                Json(json!({"id":"synth"}))
            }),
        )
        .route(
            "/job",
            get(
                |Query(query): Query<std::collections::HashMap<String, String>>| async move {
                    if !query.contains_key("result") {
                        return (
                            [(CONTENT_TYPE, "application/json")],
                            serde_json::to_vec(&json!({"status":"done"})).unwrap(),
                        );
                    }
                    if query["id"] == "lm" {
                        (
                            [(CONTENT_TYPE, "application/json")],
                            serde_json::to_vec(&json!([{"audio_codes":"1,2,3","bpm":90}])).unwrap(),
                        )
                    } else {
                        (
                            [(CONTENT_TYPE, "multipart/mixed; boundary=song")],
                            multipart(),
                        )
                    }
                },
            ),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = EngineClient::new(base.clone(), base, 0.1, 1.0).unwrap();
    let (audio, ext, meta) = client
        .generate(
            &json!({"caption":"piano","variants":2}),
            "test.gguf",
            "sft.gguf",
            &CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap();
    assert_eq!(audio, b"RIFF\x00\xff\x01");
    assert_eq!(ext, "wav");
    assert_eq!(meta["bpm"], 90);
    task.abort();
}

async fn pending_server() -> (EngineClient, Arc<AtomicU32>, tokio::task::JoinHandle<()>) {
    let count = Arc::new(AtomicU32::new(0));
    let cancelled = count.clone();
    let app = Router::new()
        .route("/lm", post(|| async { Json(json!({"id":"pending"})) }))
        .route(
            "/job",
            get(|| async { Json(json!({"status":"running"})) }).post(move || {
                let count = cancelled.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Json(json!({"ok":true}))
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (
        EngineClient::new(base.clone(), base, 0.01, 1.0).unwrap(),
        count,
        task,
    )
}

#[tokio::test]
async fn timeout_cancels_the_engine_job() {
    let (mut client, count, task) = pending_server().await;
    client.timeout = Duration::from_millis(80);
    assert!(client
        .language_model(&json!({}), &CancellationToken::new(), |_| {})
        .await
        .is_err());
    assert_eq!(count.load(Ordering::SeqCst), 1);
    task.abort();
}

#[tokio::test]
async fn user_cancellation_cancels_the_engine_job() {
    let (client, count, task) = pending_server().await;
    let cancel = CancellationToken::new();
    let signal = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(80)).await;
        signal.cancel();
    });
    assert!(client
        .language_model(&json!({}), &cancel, |_| {})
        .await
        .is_err());
    assert_eq!(count.load(Ordering::SeqCst), 1);
    task.abort();
}

#[cfg(unix)]
#[test]
fn closing_the_supervisor_pipe_terminates_the_engine() {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    let mut process = Command::new(env!("CARGO_BIN_EXE_music-generator"))
        .args([
            "--engine-supervisor",
            "/bin/sh",
            "-c",
            "echo $$; exec sleep 60",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(process.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let pid = line.trim();
    assert!(!pid.is_empty());
    drop(process.stdin.take());
    for _ in 0..100 {
        if process.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(process.try_wait().unwrap().is_some());
    assert!(!Command::new("kill")
        .args(["-0", pid])
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success());
}
