pub mod client;
pub mod device;
pub mod process;
use crate::{
    error::{message, Error, Result},
    types::Settings,
};
use client::EngineClient;
use process::OwnedProcess;
use std::{
    net::TcpListener,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio_util::sync::CancellationToken;

pub struct Engine {
    processes: Vec<OwnedProcess>,
    pub client: EngineClient,
}

pub fn engine_path(resources: &Path) -> PathBuf {
    resources.join("engine").join(if cfg!(windows) {
        "ace-server.exe"
    } else {
        "ace-server"
    })
}

impl Engine {
    pub async fn start(
        resources: &Path,
        logs: &Path,
        settings: &Settings,
        cancel: &CancellationToken,
    ) -> Result<Self> {
        let binary = engine_path(resources);
        if !binary.is_file() {
            return Err(message(
                "Bundled engine is missing. Run npm run engine:prepare before building the app.",
            ));
        }
        let mut processes = Vec::new();
        let (synth, process) = start_one(&binary, logs, settings, false, cancel).await?;
        processes.push(process);
        let lm = if settings.cpu_lm() {
            let (url, process) = start_one(&binary, logs, settings, true, cancel).await?;
            processes.push(process);
            url
        } else {
            synth.clone()
        };
        Ok(Self {
            processes,
            client: EngineClient::new(synth, lm, settings.poll_interval, settings.timeout_minutes)?,
        })
    }

    pub fn alive(&mut self) -> bool {
        self.processes
            .iter_mut()
            .all(|p| p.alive().unwrap_or(false))
    }
}

async fn start_one(
    binary: &Path,
    logs: &Path,
    settings: &Settings,
    cpu: bool,
    cancel: &CancellationToken,
) -> Result<(String, OwnedProcess)> {
    std::fs::create_dir_all(logs)?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()?;
    let cpu_args = if cpu {
        device::cpu_arguments(binary, cancel).await?
    } else {
        Vec::new()
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    for _ in 0..3 {
        if tokio::time::Instant::now() >= deadline {
            break;
        }
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        drop(listener);
        let mut args = vec![
            "--host".into(),
            "127.0.0.1".into(),
            "--port".into(),
            port.to_string(),
            "--models".into(),
            settings.models_dir.clone(),
            "--max-batch".into(),
            "1".into(),
        ];
        if let Some(chunk) = settings.vae_chunk {
            args.extend(["--vae-chunk".into(), chunk.to_string()]);
        }
        args.extend(cpu_args.iter().cloned());
        let mut process = OwnedProcess::spawn(
            binary,
            &args,
            cpu,
            &logs.join(if cpu { "ace-lm.log" } else { "ace-server.log" }),
        )?;
        let url = format!("http://127.0.0.1:{port}");
        while tokio::time::Instant::now() < deadline {
            if cancel.is_cancelled() {
                return Err(Error::Cancelled);
            }
            if !process.alive()? {
                break;
            }
            let response = tokio::select! {
                _ = cancel.cancelled() => return Err(Error::Cancelled),
                _ = tokio::time::sleep_until(deadline) => break,
                response = client.get(format!("{url}/health")).send() => response,
            };
            if response.is_ok_and(|r| r.status().is_success()) && process.alive()? {
                return Ok((url, process));
            }
            tokio::select! {
                _ = cancel.cancelled() => return Err(Error::Cancelled),
                _ = tokio::time::sleep(Duration::from_millis(500)) => {},
            }
        }
    }
    Err(message("Engine did not become ready. See the engine logs; check your graphics driver or run the hardware test again."))
}
