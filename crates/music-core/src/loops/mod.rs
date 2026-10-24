mod edit;
mod import;
mod window;
use crate::error::{message, Result};
pub use edit::{edit, seam_score, zero_start};
pub use import::import_audio;
use serde::{Deserialize, Serialize};
pub use window::loop_region;

pub const DEFAULT_MODEL: &str = "acestep-v15-xl-sft-Q8_0.gguf";
pub const REQUIRED_MODELS: [&str; 3] = [
    DEFAULT_MODEL,
    "Qwen3-Embedding-0.6B-Q8_0.gguf",
    "vae-BF16.gguf",
];
pub const INSTRUMENTS: &[&str] = &[
    "drums",
    "bass",
    "keyboard",
    "guitar",
    "synth",
    "strings",
    "percussion",
    "vocals",
    "backing_vocals",
    "fx",
    "brass",
    "woodwinds",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoundIdea {
    pub name: String,
    pub prompt: String,
    pub revision: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub slot: u8,
    pub instrument: String,
    pub prompt: String,
    pub key: String,
    pub bpm: f64,
    pub beats: f64,
    #[serde(default = "four")]
    pub numerator: u32,
    #[serde(default = "four")]
    pub denominator: u32,
    pub seed: i64,
    #[serde(default = "steps")]
    pub steps: u32,
    #[serde(default = "one")]
    pub variants: u32,
    #[serde(default)]
    pub context: Vec<ContextSource>,
    #[serde(default)]
    pub sound_idea: Option<SoundIdea>,
}
fn four() -> u32 {
    4
}
fn steps() -> u32 {
    80
}
fn one() -> u32 {
    1
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextSource {
    pub slot: u8,
    pub path: String,
}
impl GenerationRequest {
    pub fn seconds(&self) -> f64 {
        self.beats * 60.0 / self.bpm
    }
    pub fn validate(&self) -> Result<()> {
        if self.sound_idea.as_ref().is_some_and(|idea| {
            idea.name.len() > 1024 || idea.prompt.len() > 8000 || idea.revision == 0
        }) {
            return Err(message("Invalid sound idea snapshot"));
        }
        if self.slot > 127
            || !INSTRUMENTS.contains(&self.instrument.as_str())
            || self.prompt.len() > 16100
            || self.key.len() > 80
            || !self.bpm.is_finite()
            || !(30.0..=300.0).contains(&self.bpm)
            || !self.beats.is_finite()
            || !(0.25..=1024.0).contains(&self.beats)
            || self.seconds() > 600.0
            || !(1..=12).contains(&self.numerator)
            || ![1, 2, 4, 8, 16].contains(&self.denominator)
            || !(1..=100).contains(&self.steps)
            || !(1..=8).contains(&self.variants)
            || self.context.len() > 32
            || self.context.iter().any(|c| c.slot == self.slot)
        {
            return Err(message(
                "Invalid loop generation request or target included in context",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EditRecipe {
    pub start: u64,
    pub end: u64,
    pub frames: u64,
    pub crossfade_ms: f64,
    pub normalize_db: Option<f64>,
    pub noise_start: u64,
    pub noise_end: u64,
    pub noise_db: f64,
    pub remove_dc: bool,
}
