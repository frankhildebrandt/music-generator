use super::{client::EngineClient, Engine};
use crate::{
    error::{Error, Result},
    types::Settings,
};
use serde_json::{json, Value};
use std::{path::Path, sync::Mutex, time::Duration};
use tokio_util::sync::CancellationToken;

/// Owns resident processes; configuration readiness belongs to the application runtime.
#[derive(Default)]
pub(crate) struct EngineManager {
    engine: Mutex<Option<Engine>>,
}

impl EngineManager {
    // Callers hold the runtime's exclusive operation permit while starting an engine.
    pub async fn start(
        &self,
        resources: &Path,
        logs: &Path,
        settings: &Settings,
        cancel: &CancellationToken,
    ) -> Result<EngineClient> {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        {
            let mut engine = self.engine.lock().unwrap();
            if engine.as_mut().is_some_and(Engine::alive) {
                return Ok(engine.as_ref().unwrap().client.clone());
            }
            engine.take();
        }
        let started = Engine::start(resources, logs, settings, cancel).await?;
        let mut engine = self.engine.lock().unwrap();
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let client = started.client.clone();
        *engine = Some(started);
        Ok(client)
    }

    pub fn stop(&self) {
        self.engine.lock().unwrap().take();
    }

    pub fn apply_settings(&self, before: &Settings, after: &Settings) {
        let mut engine = self.engine.lock().unwrap();
        if before.lm_device != after.lm_device
            || before.mode != after.mode
            || before.models_dir != after.models_dir
            || before.vae_chunk != after.vae_chunk
        {
            engine.take();
        } else if let Some(engine) = engine.as_mut() {
            engine.client.poll = Duration::from_secs_f64(after.poll_interval);
            engine.client.timeout = Duration::from_secs_f64(after.timeout_minutes * 60.0);
        }
    }

    pub async fn health(&self, ready: bool) -> Value {
        let client = {
            let mut engine = self.engine.lock().unwrap();
            match engine.as_mut() {
                Some(engine) => {
                    if !engine.alive() {
                        return json!({"ok":false,"error":"Model engine stopped unexpectedly"});
                    }
                    Some(engine.client.clone())
                }
                None => None,
            }
        };
        let Some(client) = client else {
            return json!({"ok":ready,"idle":ready,"error":if ready { None } else { Some("Complete model setup first") }});
        };
        let synth = client
            .http
            .get(format!("{}/health", client.synth))
            .timeout(Duration::from_secs(5))
            .send()
            .await;
        let lm = client
            .http
            .get(format!("{}/health", client.lm))
            .timeout(Duration::from_secs(5))
            .send()
            .await;
        let props = match client
            .http
            .get(format!("{}/props", client.synth))
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => response.json::<Value>().await.ok(),
            Err(_) => None,
        };
        json!({"ok":synth.is_ok_and(|r| r.status().is_success()) && lm.is_ok_and(|r| r.status().is_success()),"props":props})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn resident() -> EngineManager {
        let mut client =
            EngineClient::new("http://unused".into(), "http://unused".into(), 1.5, 30.0).unwrap();
        client.synth_log = Some("synthesis.log".into());
        EngineManager {
            engine: Mutex::new(Some(Engine {
                processes: Vec::new(),
                client,
            })),
        }
    }
    #[test]
    fn timing_changes_keep_the_resident_engine_and_its_diagnostics() {
        let manager = resident();
        let before = Settings::default();
        let mut after = before.clone();
        after.timeout_minutes = 45.0;
        after.poll_interval = 2.0;
        manager.apply_settings(&before, &after);
        let engine = manager.engine.lock().unwrap();
        let client = &engine.as_ref().unwrap().client;
        assert_eq!(client.timeout, Duration::from_secs(2700));
        assert_eq!(client.poll, Duration::from_secs(2));
        assert_eq!(
            client.synth_log.as_deref(),
            Some(Path::new("synthesis.log"))
        );
    }
    #[test]
    fn changing_the_device_releases_the_old_engine() {
        let manager = resident();
        let before = Settings::default();
        let mut after = before.clone();
        after.lm_device = "cpu".into();
        manager.apply_settings(&before, &after);
        assert!(manager.engine.lock().unwrap().is_none());
    }
}
