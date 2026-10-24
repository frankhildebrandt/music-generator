//! Native HeartMuLa + HeartCodec adapter. No interpreter, model server or Hub access.
use crate::{
    catalog::Selection,
    engine::process::OwnedProcess,
    error::{message, Error, Result},
    types::Settings,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    pub temperature: f64,
    pub cfg_scale: f64,
    pub top_k: u32,
    pub codec_steps: u32,
    pub codec_guidance: f64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            temperature: 1.0,
            cfg_scale: 1.5,
            top_k: 50,
            codec_steps: 10,
            codec_guidance: 1.25,
        }
    }
}
impl Options {
    pub fn validate(&self) -> Result<()> {
        if !self.temperature.is_finite()
            || !(0.05..=3.0).contains(&self.temperature)
            || !self.cfg_scale.is_finite()
            || !(0.1..=10.0).contains(&self.cfg_scale)
            || !self.codec_guidance.is_finite()
            || !(0.1..=10.0).contains(&self.codec_guidance)
            || !(1..=1000).contains(&self.top_k)
            || !(1..=100).contains(&self.codec_steps)
        {
            return Err(message("Invalid HeartMuLa sampling parameters"));
        }
        Ok(())
    }
}

pub fn executable(resources: &Path) -> PathBuf {
    resources.join("engine/heartmula").join(if cfg!(windows) {
        "audiocpp_cli.exe"
    } else {
        "audiocpp_cli"
    })
}

pub async fn probe(resources: &Path, cancel: &CancellationToken) -> Result<String> {
    let mut command = tokio::process::Command::new(executable(resources));
    command.arg("--list-devices").kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output = tokio::select! { _ = cancel.cancelled() => return Err(Error::Cancelled), r=tokio::time::timeout(Duration::from_secs(30),command.output())=>r.map_err(|_|message("HeartMuLa device test timed out"))?? };
    if !output.status.success() {
        return Err(message(format!(
            "HeartMuLa device test failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

pub fn validate(params: &Value) -> Result<Options> {
    let options: Options = params
        .get("heartmula_options")
        .filter(|v| !v.is_null())
        .cloned()
        .map(serde_json::from_value)
        .transpose()?
        .unwrap_or_default();
    options.validate()?;
    let duration = params["duration"].as_f64().unwrap_or(0.0);
    if !duration.is_finite() || !(0.0..=240.0).contains(&duration) {
        return Err(message("HeartMuLa supports at most 240 seconds per song"));
    }
    if !params["instrumental"].as_bool().unwrap_or(false)
        && params["lyrics"].as_str().unwrap_or("").trim().is_empty()
    {
        return Err(message(
            "HeartMuLa needs written lyrics. Enter lyrics or enable Instrumental",
        ));
    }
    if params["lyrics"].as_str().unwrap_or("").chars().count() > 8192 {
        return Err(message("HeartMuLa lyrics exceed 8192 characters"));
    }
    Ok(options)
}

pub fn arguments(params: &Value, settings: &Settings, output: &Path) -> Result<Vec<String>> {
    let options = validate(params)?;
    let selection = Selection::from_settings(settings);
    crate::catalog::required(&selection)?;
    if selection.provider != "heartmula" {
        return Err(message("Incorrect native provider adapter"));
    }
    let mut tags = params["caption"].as_str().unwrap_or("").trim().to_string();
    if tags.is_empty() {
        return Err(message("HeartMuLa needs a prompt or style tags"));
    }
    if params["instrumental"].as_bool().unwrap_or(false) {
        tags.push_str(", instrumental, no vocals");
    }
    if let Some(bpm) = params["bpm"].as_u64().filter(|n| *n > 0) {
        tags.push_str(&format!(", {bpm} bpm"));
    }
    for key in ["keyscale", "timesignature", "vocal_language"] {
        if let Some(value) = params[key].as_str().filter(|v| !v.is_empty()) {
            tags.push_str(", ");
            tags.push_str(value);
        }
    }
    // Keep the native Windows command line below its UTF-16 limit without truncating songs.
    if tags.chars().count() > 4096 {
        return Err(message("HeartMuLa style tags exceed 4096 characters"));
    }
    let duration = params["duration"]
        .as_f64()
        .filter(|n| *n > 0.0)
        .unwrap_or(120.0);
    let lyrics = if params["instrumental"].as_bool().unwrap_or(false) {
        "[Instrumental]"
    } else {
        params["lyrics"].as_str().unwrap_or("")
    };
    let backend = if settings.lm_device == "cpu" || crate::hardware::cpu_only() {
        "cpu"
    } else if cfg!(target_os = "macos") {
        "metal"
    } else {
        "best"
    };
    Ok(vec![
        "--task".into(),
        "gen".into(),
        "--family".into(),
        "heartmula".into(),
        "--model".into(),
        Path::new(&settings.models_dir)
            .join(&selection.synthesis)
            .to_string_lossy()
            .into(),
        "--backend".into(),
        backend.into(),
        "--lyrics".into(),
        lyrics.into(),
        "--request-option".into(),
        format!("tags={tags}"),
        "--duration-seconds".into(),
        duration.to_string(),
        "--request-option".into(),
        format!("duration_sec={duration}"),
        "--temperature".into(),
        options.temperature.to_string(),
        "--top-k".into(),
        options.top_k.to_string(),
        "--guidance-scale".into(),
        options.cfg_scale.to_string(),
        "--num-inference-steps".into(),
        options.codec_steps.to_string(),
        "--request-option".into(),
        format!("codec_guidance_scale={}", options.codec_guidance),
        "--session-option".into(),
        "heartmula.mem_saver=true".into(),
        "--seed".into(),
        params["seed"].as_i64().unwrap_or(0).max(0).to_string(),
        "--out".into(),
        output.to_string_lossy().into(),
    ])
}

pub async fn generate(
    resources: &Path,
    data_root: &Path,
    params: &Value,
    settings: &Settings,
    cancel: &CancellationToken,
    progress: impl Fn(String),
) -> Result<(Vec<u8>, String, Value)> {
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let binary = executable(resources);
    if !binary.is_file() {
        return Err(message(
            "Bundled HeartMuLa engine is missing. Run npm run engine:prepare",
        ));
    }
    let folder = data_root.join("heartmula-work").join(crate::types::id());
    tokio::fs::create_dir_all(&folder).await?;
    tokio::fs::create_dir_all(data_root.join("logs")).await?;
    let outcome = async {
        let output = folder.join("music.wav");
        let args = arguments(params, settings, &output)?;
        let log = data_root.join("logs/heartmula.log");
        let offset = log.metadata().map(|m| m.len()).unwrap_or(0);
        let mut process = OwnedProcess::spawn(&binary, &args, settings.lm_device == "cpu", &log)?;
        let started = Instant::now();
        let timeout = Duration::from_secs_f64(settings.timeout_minutes * 60.0);
        loop {
            if cancel.is_cancelled() { return Err(Error::Cancelled); }
            if started.elapsed() >= timeout { return Err(Error::EngineTimeout { operation: "HeartMuLa / HeartCodec".into(), minutes: settings.timeout_minutes.ceil() as u64 }); }
            if let Some(status) = process.status()? {
                if !status.success() {
                    use std::io::{Read, Seek, SeekFrom};
                    let mut file = std::fs::File::open(&log)?;
                    file.seek(SeekFrom::Start(offset.max(file.metadata()?.len().saturating_sub(8192))))?;
                    let mut bytes = Vec::new(); file.take(8192).read_to_end(&mut bytes)?;
                    return Err(message(format!("HeartMuLa failed ({status}): {}", String::from_utf8_lossy(&bytes))));
                }
                break;
            }
            progress(format!("HeartMuLa / HeartCodec · {}:{:02}", started.elapsed().as_secs()/60, started.elapsed().as_secs()%60));
            tokio::select! { _ = cancel.cancelled() => return Err(Error::Cancelled), _ = tokio::time::sleep(Duration::from_millis(500)) => {} }
        }
        let bytes = tokio::fs::read(output).await?;
        let (duration, rate, channels) = wav_info(&bytes)?;
        let meta = json!({"provider":"heartmula","model_selection":Selection::from_settings(settings),"caption":params["caption"],"lyrics":params["lyrics"],"duration":duration,"sample_rate":rate,"channels":channels,"heartmula_options":validate(params)?});
        Ok((bytes, "wav".into(), meta))
    }.await;
    let _ = tokio::fs::remove_dir_all(folder).await;
    outcome
}

/// Reject error text/truncated files before they can enter the library as audio.
pub fn wav_info(bytes: &[u8]) -> Result<(f64, u32, u16)> {
    if bytes.len() < 44 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(message("HeartCodec did not produce a WAV file"));
    }
    let mut offset = 12usize;
    let mut format = None;
    let mut data_size = None;
    while offset + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let start = offset + 8;
        let end = start
            .checked_add(size)
            .filter(|n| *n <= bytes.len())
            .ok_or_else(|| message("Truncated HeartCodec WAV"))?;
        if &bytes[offset..offset + 4] == b"fmt " && size >= 16 {
            let channels = u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap());
            let rate = u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap());
            let byte_rate = u32::from_le_bytes(bytes[start + 8..start + 12].try_into().unwrap());
            if channels == 0 || rate == 0 || byte_rate == 0 {
                return Err(message("Invalid HeartCodec WAV format"));
            }
            format = Some((rate, channels, byte_rate));
        }
        if &bytes[offset..offset + 4] == b"data" {
            data_size = Some(size);
        }
        offset = end + (size & 1);
    }
    let (rate, channels, byte_rate) =
        format.ok_or_else(|| message("Missing HeartCodec WAV format"))?;
    let data = data_size
        .filter(|s| *s > 0)
        .ok_or_else(|| message("Empty HeartCodec WAV"))?;
    Ok((data as f64 / byte_rate as f64, rate, channels))
}
