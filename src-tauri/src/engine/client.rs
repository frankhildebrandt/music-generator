use super::progress::{stream_logs, Progress};
use crate::error::{message, Error, Result};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
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
        })
    }

    pub async fn operation(
        &self,
        base: &str,
        endpoint: &str,
        request: reqwest::RequestBuilder,
        cancel: &CancellationToken,
        progress: impl Fn(Progress),
    ) -> Result<(String, Vec<u8>)> {
        let response = tokio::select! { _ = cancel.cancelled() => return Err(Error::Cancelled), r = request.send() => r? };
        let job: Value = checked(response).await?.json().await?;
        let jid = match &job["id"] {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => return Err(message("Engine returned no job ID")),
        };
        let outcome = tokio::select! {
            _ = cancel.cancelled() => Err(Error::Cancelled),
            r = tokio::time::timeout(self.timeout, self.track(base, &jid, endpoint, &progress)) => r.unwrap_or_else(|_| Err(message(format!("{endpoint}: time limit exceeded")))),
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

    async fn track(
        &self,
        base: &str,
        jid: &str,
        endpoint: &str,
        progress: &impl Fn(Progress),
    ) -> Result<(String, Vec<u8>)> {
        let start = Instant::now();
        let phase = if endpoint == "lm" { "lm" } else { "synth" };
        let (latest, current) = tokio::sync::watch::channel(Progress::new(phase));
        let report = |mut update: Progress| {
            update.elapsed_seconds = start.elapsed().as_secs();
            if *latest.borrow() != update {
                latest.send_replace(update.clone());
                progress(update);
            }
        };
        progress(current.borrow().clone());
        tokio::select! {
            result = self.wait(base, jid, start, current, progress) => result,
            _ = stream_logs(&self.http, base, jid, report) => unreachable!(),
        }
    }

    async fn wait(
        &self,
        base: &str,
        jid: &str,
        start: Instant,
        current: tokio::sync::watch::Receiver<Progress>,
        progress: &impl Fn(Progress),
    ) -> Result<(String, Vec<u8>)> {
        let mut last = None;
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
            let mut update = current.borrow().clone();
            update.elapsed_seconds = start.elapsed().as_secs();
            if last.as_ref() != Some(&update) {
                progress(update.clone());
                last = Some(update);
            }
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
        progress: impl Fn(Progress),
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
        progress: impl Fn(Progress),
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

        let mut enriched = self.language_model(&request, cancel, &progress).await?;
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
        enriched["output_format"] = json!("wav16");
        enriched["synth_model"] = json!(dit);
        let (ctype, body) = self
            .operation(
                &self.synth,
                "synth",
                self.http
                    .post(format!("{}/synth", self.synth))
                    .json(&enriched),
                cancel,
                progress,
            )
            .await?;
        let (audio, extension) = extract_audio(&ctype, &body)?;
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
