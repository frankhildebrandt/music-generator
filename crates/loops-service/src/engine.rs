use super::*;
use music_core::{
    engine::client::EngineClient,
    verification::{ModelVerifier, Verification},
};
use std::process::{Child, Command, Stdio};
pub struct Engine {
    child: Child,
    pub client: EngineClient,
}
impl Drop for Engine {
    fn drop(&mut self) {
        drop(self.child.stdin.take());
        for _ in 0..60 {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Engine {
    pub async fn start(service: &Service, cancel: &CancellationToken) -> Result<Self> {
        let models = service.models.lock().await.clone();
        let mut verifier = ModelVerifier::open(&service.root);
        for model in music_core::manifest::manifest()
            .models
            .into_iter()
            .filter(|m| {
                m.kind == "pro"
                    || ["vae-BF16.gguf", "Qwen3-Embedding-0.6B-Q8_0.gguf"]
                        .contains(&m.file.as_str())
            })
        {
            if !models.join(&model.file).is_file()
                || verifier
                    .verify(&models.join(&model.file), &model.sha256, cancel, |_, _| {})
                    .await?
                    == Verification::Invalid
            {
                return Err(message(format!("Install or repair model {}", model.file)));
            }
        }
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        drop(listener);
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(service.root.join("engine.log"))?;
        let binary = service.resources.join("engine").join(if cfg!(windows) {
            "ace-server.exe"
        } else {
            "ace-server"
        });
        let mut command = Command::new(std::env::current_exe()?);
        command.arg("--engine-supervisor").arg(&binary);
        command
            .args([
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--models",
                &models.to_string_lossy(),
                "--max-batch",
                "1",
            ])
            .current_dir(binary.parent().unwrap())
            .stdout(log.try_clone()?)
            .stderr(log)
            .stdin(Stdio::piped());
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        command.env("GGML_BACKEND", "CPU");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let child = command.spawn()?;
        let base = format!("http://127.0.0.1:{port}");
        let mut engine = Self {
            child,
            client: EngineClient::new(base.clone(), base, 0.5, 30.0)?,
        };
        engine.client.synth_log = Some(service.root.join("engine.log"));
        let http = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(2))
            .build()?;
        for _ in 0..240 {
            if cancel.is_cancelled() {
                return Err(music_core::error::Error::Cancelled);
            }
            if engine.child.try_wait()?.is_some() {
                return Err(message("Engine exited; inspect engine.log"));
            }
            if http
                .get(format!("{}/health", engine.client.synth))
                .send()
                .await
                .is_ok_and(|r| r.status().is_success())
            {
                return Ok(engine);
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        Err(message("Engine startup timed out"))
    }
    pub fn alive(&mut self) -> bool {
        self.child.try_wait().is_ok_and(|s| s.is_none())
    }
}
