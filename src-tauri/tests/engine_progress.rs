use axum::{
    body::Body,
    http::header::CONTENT_TYPE,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use music_generator::engine::{
    client::EngineClient,
    progress::{JobLogs, LogLines, Progress},
};
use serde_json::json;
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;

#[test]
fn logs_are_scoped_to_the_job_and_expose_only_measured_steps() {
    let mut logs = JobLogs::new("current");
    assert!(logs.parse("[DiT] Step 49/50 t=0.1").is_none());
    logs.parse("[Server] Job old created (synth)");
    assert!(logs.parse("[DiT] Step 49/50 t=0.1").is_none());
    logs.parse("[Server] Job current created (synth)");
    assert_eq!(
        logs.parse("[GGUF] model.gguf: 10 tensors").unwrap().phase,
        "loading"
    );
    let update = logs.parse("[DiT] Step 3/50 t=0.9").unwrap();
    assert_eq!(
        (update.phase, update.step, update.total_steps),
        ("dit", Some(3), Some(50))
    );
    assert!(logs.parse("[DiT] Step 3/0").is_none());
    assert!(logs.parse("[DiT] Step 51/50").is_none());
    assert!(logs.parse("[DiT] Step -1/50").is_none());
    let lm = logs
        .parse("[LM-Phase2] Step 10, 1 active, 10 total codes, 20 tok/s")
        .unwrap();
    assert_eq!((lm.phase, lm.step, lm.total_steps), ("lm", Some(10), None));
    let vae = logs.parse("[VAE] Graph: 479 nodes, T_latent=400").unwrap();
    assert_eq!((vae.phase, vae.total_steps), ("vae", None));
    logs.parse("[Server] Job current done (synth)");
    assert!(logs.parse("[DiT] Step 4/50").is_none());
    logs.parse("[Server] Job other created (synth)");
    assert!(logs.parse("[DiT] Step 4/50").is_none());
}

#[test]
fn sse_log_lines_handle_fragmented_utf8_crlf_comments_and_size_limits() {
    let mut lines = LogLines::default();
    let input = ": heartbeat\r\nevent: log\r\ndata: 模型\r\n\r\ndata:[DiT] Step 1/8\n\n";
    let decoded: Vec<_> = input
        .as_bytes()
        .chunks(1)
        .flat_map(|chunk| lines.push(chunk).unwrap())
        .collect();
    assert_eq!(decoded, ["模型", "[DiT] Step 1/8"]);
    assert!(lines.push(&vec![b'x'; 65_537]).is_err());
}

async fn serve(app: Router) -> (EngineClient, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (
        EngineClient::new(base.clone(), base, 0.02, 1.0).unwrap(),
        server,
    )
}

#[tokio::test]
async fn progress_stream_reconnects_and_keeps_result_polling_authoritative() {
    let connections = Arc::new(AtomicU32::new(0));
    let log_connections = connections.clone();
    let finished = Arc::new(AtomicU32::new(0));
    let job_finished = finished.clone();
    let app = Router::new()
        .route("/synth", post(|| async { Json(json!({"id":"current"})) }))
        .route("/logs", get(move || {
            let attempt = log_connections.fetch_add(1, Ordering::SeqCst);
            async move {
                let data = if attempt == 0 {
                    "data: [Server] Job old created (synth)\n\ndata: [DiT] Step 49/50\n\ndata: [Server] Job current created (synth)\n\ndata: [DiT] Step 1/8\n\n"
                } else {
                    "data: [Server] Job old created (synth)\n\ndata: [DiT] Step 49/50\n\ndata: [Server] Job current created (synth)\n\ndata: [DiT] Step 8/8\n\ndata: [VAE] Graph: 479 nodes\n\n"
                };
                let chunks = data.as_bytes().chunks(3).map(|c| Ok::<_, std::convert::Infallible>(c.to_vec())).collect::<Vec<_>>();
                Response::builder().header(CONTENT_TYPE, "text/event-stream").body(Body::from_stream(futures_util::stream::iter(chunks))).unwrap()
            }
        }))
        .route("/job", get(move || {
            let done = job_finished.load(Ordering::SeqCst) > 0;
            async move { Json(json!({"status":if done {"done"} else {"running"}})) }
        }));
    let (client, server) = serve(app).await;
    let updates = Mutex::new(Vec::<Progress>::new());
    let result = tokio::time::timeout(
        Duration::from_secs(8),
        client.operation(
            &client.synth,
            "synth",
            client.http.post(format!("{}/synth", client.synth)),
            &CancellationToken::new(),
            |update| {
                if update.phase == "vae" {
                    finished.store(1, Ordering::SeqCst);
                }
                updates.lock().unwrap().push(update);
            },
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(result.0, "application/json");
    assert_eq!(connections.load(Ordering::SeqCst), 2);
    let updates = updates.lock().unwrap();
    assert!(updates
        .iter()
        .any(|u| u.step == Some(1) && u.total_steps == Some(8)));
    assert!(updates
        .iter()
        .any(|u| u.step == Some(8) && u.total_steps == Some(8)));
    assert!(!updates.iter().any(|u| u.total_steps == Some(50)));
    server.abort();
}

#[tokio::test]
async fn engines_without_logs_still_finish_with_indeterminate_progress() {
    let polls = Arc::new(AtomicU32::new(0));
    let count = polls.clone();
    let app = Router::new()
        .route("/synth", post(|| async { Json(json!({"id":"legacy"})) }))
        .route("/job", get(move || {
            let attempt = count.fetch_add(1, Ordering::SeqCst);
            async move { Json(json!({"status": if attempt >= 2 { "done" } else { "running" }})) }
        }));
    let (client, server) = serve(app).await;
    let updates = Mutex::new(Vec::new());
    client
        .operation(
            &client.synth,
            "synth",
            client.http.post(format!("{}/synth", client.synth)),
            &CancellationToken::new(),
            |u| updates.lock().unwrap().push(u),
        )
        .await
        .unwrap();
    let updates = updates.lock().unwrap();
    assert!(!updates.is_empty());
    assert!(updates
        .iter()
        .all(|u| u.step.is_none() && u.total_steps.is_none()));
    server.abort();
}
