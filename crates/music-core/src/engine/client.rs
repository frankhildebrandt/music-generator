use super::diagnostics::SynthesisLog;
use crate::error::{message, Error, Result};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct EngineClient {
    pub http: reqwest::Client,
    pub synth: String,
    pub lm: String,
    pub poll: Duration,
    pub timeout: Duration,
    pub synth_log: Option<PathBuf>,
}

impl EngineClient {
    pub fn new(synth: String, lm: String, poll: f64, timeout: f64) -> Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(120))
                .build()?,
            synth,
            lm,
            poll: Duration::from_secs_f64(poll),
            timeout: Duration::from_secs_f64(timeout * 60.0),
            synth_log: None,
        })
    }

    pub async fn operation(
        &self,
        base: &str,
        endpoint: &str,
        request: reqwest::RequestBuilder,
        cancel: &CancellationToken,
        progress: impl Fn(String),
    ) -> Result<(String, Vec<u8>)> {
        let response = tokio::select! { _ = cancel.cancelled() => return Err(Error::Cancelled), r = request.send() => r? };
        let job: Value = checked(response).await?.json().await?;
        let jid = match &job["id"] {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => return Err(message("Engine returned no job ID")),
        };
        let started = Instant::now();
        let outcome = tokio::select! {
            _ = cancel.cancelled() => Err(Error::Cancelled),
            r = tokio::time::timeout(self.timeout, self.wait(base, &jid, &progress)) => r.unwrap_or_else(|_| Err(timeout_error(endpoint, started.elapsed()))),
        };
        if outcome.is_err() {
            let _ = self
                .http
                .post(format!("{base}/job"))
                .query(&[("id", jid.as_str()), ("cancel", "1")])
                .timeout(Duration::from_secs(5))
                .send()
                .await;
        }
        outcome
    }

    async fn wait(
        &self,
        base: &str,
        jid: &str,
        progress: &impl Fn(String),
    ) -> Result<(String, Vec<u8>)> {
        let start = Instant::now();
        loop {
            let state: Value = self.get_job(base, jid, false).await?.json().await?;
            let status = state["status"].as_str().unwrap_or("").to_lowercase();
            if status == "done" {
                let response = self.get_job(base, jid, true).await?;
                let ctype = response
                    .headers()
                    .get("content-type")
                    .and_then(|s| s.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                return Ok((ctype, response.bytes().await?.to_vec()));
            }
            if ["err", "fail", "cancel"]
                .iter()
                .any(|word| status.contains(word))
            {
                return Err(message(format!("Engine job failed: {state}")));
            }
            let seconds = start.elapsed().as_secs();
            progress(format!("{}:{:02}", seconds / 60, seconds % 60));
            tokio::time::sleep(self.poll).await;
        }
    }

    async fn get_job(&self, base: &str, jid: &str, result: bool) -> Result<reqwest::Response> {
        let mut last = None;
        for attempt in 0..4 {
            let mut request = self.http.get(format!("{base}/job")).query(&[("id", jid)]);
            if result {
                request = request.query(&[("result", "1")]);
            }
            match request.send().await {
                Ok(response) => return checked(response).await,
                Err(e) => last = Some(e),
            }
            tokio::time::sleep(Duration::from_millis(300 * (attempt + 1))).await;
        }
        Err(last.unwrap().into())
    }

    pub async fn language_model(
        &self,
        request: &Value,
        cancel: &CancellationToken,
        progress: impl Fn(String),
    ) -> Result<Value> {
        let (ctype, body) = self
            .operation(
                &self.lm,
                "lm",
                self.http.post(format!("{}/lm", self.lm)).json(request),
                cancel,
                progress,
            )
            .await?;
        extract_json(&ctype, &body)
    }

    pub async fn generate(
        &self,
        params: &Value,
        lm_file: &str,
        dit: &str,
        cancel: &CancellationToken,
        progress: impl Fn(String),
    ) -> Result<(Vec<u8>, String, Value)> {
        const FIELDS: &[&str] = &[
            "caption",
            "lyrics",
            "bpm",
            "duration",
            "keyscale",
            "timesignature",
            "vocal_language",
            "seed",
            "lm_temperature",
            "lm_cfg_scale",
            "inference_steps",
            "guidance_scale",
            "use_cot_caption",
        ];
        let mut request = json!({"output_format":"wav16", "lm_model": lm_file, "synth_model": dit});
        for field in FIELDS {
            if let Some(value) = params
                .get(field)
                .filter(|v| !v.is_null() && v.as_str() != Some(""))
            {
                request[field] = value.clone();
            }
        }
        progress("LM · 0:00".into());
        let mut enriched = self
            .language_model(&request, cancel, |s| progress(format!("LM · {s}")))
            .await?;
        if enriched["audio_codes"]
            .as_str()
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            return Err(message(
                "Language model returned no audio codes. Run the hardware test again.",
            ));
        }
        // The synthesis model supports at most 600 seconds (3000 codes).
        // Trim occasional LM overproduction only in this outgoing payload;
        // saved song parameters remain unchanged.
        limit_audio_codes(&mut enriched, params);
        enriched["output_format"] = json!("wav16");
        enriched["synth_model"] = json!(dit);
        let log = self.synth_log.as_deref().map(SynthesisLog::capture);
        let result = self
            .operation(
                &self.synth,
                "synth",
                self.http
                    .post(format!("{}/synth", self.synth))
                    .json(&enriched),
                cancel,
                |s| progress(format!("Synth · {s}")),
            )
            .await
            .and_then(|(ctype, body)| extract_audio(&ctype, &body));
        let (audio, extension) = result.map_err(|error| {
            if matches!(error, Error::Cancelled | Error::EngineTimeout { .. }) {
                return error;
            }
            log.as_ref()
                .and_then(|log| log.hint(&error.to_string()))
                .map(message)
                .unwrap_or(error)
        })?;
        Ok((audio, extension, enriched))
    }

    pub async fn analyze(
        &self,
        data: Vec<u8>,
        filename: String,
        cancel: &CancellationToken,
    ) -> Result<Value> {
        let part = reqwest::multipart::Part::bytes(data).file_name(filename);
        let request = self
            .http
            .post(format!("{}/understand", self.lm))
            .multipart(reqwest::multipart::Form::new().part("audio", part));
        let (ctype, body) = self
            .operation(&self.lm, "understand", request, cancel, |_| {})
            .await?;
        extract_json(&ctype, &body)
    }

    /// Generate an isolated stem directly through DiT, with optional backing audio.
    pub async fn generate_stem(
        &self,
        params: &Value,
        source: Option<&std::path::Path>,
        cancel: &CancellationToken,
        progress: impl Fn(String),
    ) -> Result<Vec<u8>> {
        let mut request = self.http.post(format!("{}/synth", self.synth));
        if let Some(path) = source {
            let part = reqwest::multipart::Part::bytes(tokio::fs::read(path).await?)
                .file_name("context.wav");
            request = request.multipart(
                reqwest::multipart::Form::new()
                    .text("request", serde_json::to_string(params)?)
                    .part("audio", part),
            );
        } else {
            request = request.json(params);
        }
        let (ctype, body) = self
            .operation(&self.synth, "stem", request, cancel, progress)
            .await?;
        Ok(extract_audio(&ctype, &body)?.0)
    }
}

fn timeout_error(endpoint: &str, elapsed: Duration) -> Error {
    Error::EngineTimeout {
        operation: match endpoint {
            "lm" => "LM",
            "synth" => "Synthese / Synthesis",
            "understand" => "Analyse / Analysis",
            other => other,
        }
        .to_owned(),
        minutes: (elapsed.as_secs_f64() / 60.0).round().max(1.0) as u64,
    }
}

pub fn limit_audio_codes(enriched: &mut Value, params: &Value) {
    let seconds = params["duration"]
        .as_f64()
        .filter(|n| n.is_finite() && *n > 0.0)
        .or_else(|| {
            enriched["duration"]
                .as_f64()
                .filter(|n| n.is_finite() && *n > 0.0)
        })
        .unwrap_or(600.0)
        .min(600.0);
    let limit = (seconds * 5.0).round() as usize;
    let codes: Vec<_> = enriched["audio_codes"]
        .as_str()
        .unwrap_or("")
        .split(',')
        .filter(|code| !code.trim().is_empty())
        .collect();
    if codes.len() > limit {
        enriched["audio_codes"] = json!(codes[..limit].join(","));
    }
}

async fn checked(response: reqwest::Response) -> Result<reqwest::Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    Err(message(format!(
        "Engine HTTP {status}: {}",
        text.chars().take(500).collect::<String>()
    )))
}

pub fn extract_json(ctype: &str, body: &[u8]) -> Result<Value> {
    let bytes = if ctype.starts_with("application/json") {
        body.to_vec()
    } else {
        multipart_part(ctype, body, "application/json")?.0
    };
    let value: Value = serde_json::from_slice(&bytes)?;
    let value = if let Value::Array(mut items) = value {
        if items.is_empty() {
            return Err(message("Empty engine result"));
        }
        items.remove(0)
    } else {
        value
    };
    if !value.is_object() {
        return Err(message("Invalid engine metadata"));
    }
    Ok(value)
}

pub fn extract_audio(ctype: &str, body: &[u8]) -> Result<(Vec<u8>, String)> {
    let (bytes, kind) = if ctype.starts_with("audio/") {
        (body.to_vec(), ctype.into())
    } else {
        multipart_part(ctype, body, "audio/")?
    };
    if bytes.is_empty() {
        return Err(message("Engine returned empty audio"));
    }
    let ext = if kind.contains("mpeg") || kind.contains("mp3") {
        "mp3"
    } else {
        "wav"
    };
    Ok((bytes, ext.into()))
}

fn multipart_part(ctype: &str, body: &[u8], prefix: &str) -> Result<(Vec<u8>, String)> {
    if !ctype.to_lowercase().starts_with("multipart/") {
        return Err(message(format!("Unexpected engine response: {ctype}")));
    }
    let mut message_bytes =
        format!("Content-Type: {ctype}\r\nMIME-Version: 1.0\r\n\r\n").into_bytes();
    message_bytes.extend(body);
    let parsed = mailparse::parse_mail(&message_bytes).map_err(|e| message(e.to_string()))?;
    for part in &parsed.subparts {
        if part.ctype.mimetype.starts_with(prefix) {
            return Ok((
                part.get_body_raw().map_err(|e| message(e.to_string()))?,
                part.ctype.mimetype.clone(),
            ));
        }
    }
    Err(message(format!("No {prefix} part in engine response")))
}

pub fn selftest_ok(result: &Value) -> bool {
    let codes: Vec<&str> = result["audio_codes"]
        .as_str()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let distinct: HashSet<_> = codes.iter().collect();
    let bpm = result["bpm"]
        .as_f64()
        .or_else(|| result["bpm"].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0);
    bpm > 0.0
        && !result["keyscale"].as_str().unwrap_or("").is_empty()
        && (30..=70).contains(&codes.len())
        && distinct.len() as f64 / codes.len().max(1) as f64 > 0.3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_includes_stage_rounded_minutes_and_settings_guidance() {
        for (seconds, minutes) in [(0.08, 1), (60.0, 1), (90.0, 2), (1800.0, 30)] {
            for (endpoint, stage) in [
                ("lm", "LM"),
                ("synth", "Synthese"),
                ("understand", "Analyse"),
            ] {
                let error = timeout_error(endpoint, Duration::from_secs_f64(seconds));
                assert!(matches!(error, Error::EngineTimeout { minutes: n, .. } if n == minutes));
                let text = error.to_string();
                assert!(text.contains(stage));
                assert!(text.contains(&format!("nach {minutes} Min.")));
                assert!(text.contains("Timeout-Zeit erhöhen"));
                assert!(text.contains("Raise the timeout in the settings"));
                assert_eq!(serde_json::to_value(&error).unwrap(), json!(text));
            }
        }
    }
}
