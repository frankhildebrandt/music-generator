use crate::error::{message, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub file: String,
    pub estimated_bytes: u64,
    pub sha256: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub base_url: String,
    pub models: Vec<Model>,
    pub engines: serde_json::Value,
}

pub fn manifest() -> Manifest {
    serde_json::from_str(include_str!("../../models/manifest.json"))
        .expect("embedded model manifest")
}

pub fn selected(lm: &str, xl: bool) -> Result<Vec<Model>> {
    if !["0.6B", "1.7B", "4B"].contains(&lm) {
        return Err(message("Unknown language model"));
    }
    Ok(manifest()
        .models
        .into_iter()
        .filter(|m| m.kind == "required" || (xl && m.kind == "xl") || m.kind == lm)
        .collect())
}

pub fn lm_file(size: &str) -> String {
    format!("acestep-5Hz-lm-{size}-Q8_0.gguf")
}
