use crate::{
    error::{message, Result},
    types::{id, now},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

pub const RATE: u64 = 48000;
pub const MAX_FRAMES: u64 = RATE * 600;
pub const BASE_MODEL: &str = "acestep-v15-base-Q8_0.gguf";
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
pub struct Project {
    pub id: String,
    pub name: String,
    pub bpm: u32,
    pub key: String,
    pub beats_per_bar: u32,
    pub duration: u64,
    pub revision: u64,
    pub tracks: Vec<Track>,
    pub clips: Vec<Clip>,
    #[serde(default)]
    pub takes: Vec<Take>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub instrument: String,
    #[serde(default)]
    pub prompt: String,
    pub gain: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub context: bool,
    pub color: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clip {
    pub id: String,
    pub track_id: String,
    pub take_id: String,
    pub start: u64,
    pub offset: u64,
    pub length: u64,
    pub gain: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Take {
    pub id: String,
    pub track_id: String,
    pub name: String,
    pub file: String,
    pub frames: u64,
    pub created_at: String,
    pub request: Value,
    pub source_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Generation {
    pub track_id: String,
    pub clip_id: Option<String>,
    pub prompt: String,
    pub seed: i64,
    pub steps: u32,
    pub variants: u32,
    pub start: u64,
    pub end: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub project_id: String,
    pub status: String,
    pub message: String,
    pub take_id: Option<String>,
    pub request: Generation,
    pub source_revision: u64,
    pub created_at: String,
}

pub fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
impl Project {
    pub fn new(name: &str, bpm: u32, key: &str, duration: u64) -> Result<Self> {
        let project = Self {
            id: id(),
            name: name.trim().into(),
            bpm,
            key: key.into(),
            beats_per_bar: 4,
            duration,
            revision: 0,
            tracks: vec![],
            clips: vec![],
            takes: vec![],
        };
        project.validate()?;
        Ok(project)
    }
    pub fn validate(&self) -> Result<()> {
        if !valid_id(&self.id)
            || self.name.trim().is_empty()
            || self.name.len() > 200
            || self.key.len() > 80
            || !(30..=300).contains(&self.bpm)
            || !(1..=12).contains(&self.beats_per_bar)
            || !(RATE..=MAX_FRAMES).contains(&self.duration)
            || self.tracks.len() > 32
            || self.clips.len() > 1000
        {
            return Err(message("Invalid project; maximum 32 tracks and 10 minutes"));
        }
        let mut ids = HashSet::new();
        for take in &self.takes {
            if !valid_id(&take.id)
                || !valid_id(&take.track_id)
                || take.file != format!("{}.wav", take.id)
                || take.frames == 0
                || take.frames > MAX_FRAMES
            {
                return Err(message("Invalid take metadata"));
            }
        }
        for track in &self.tracks {
            if !valid_id(&track.id)
                || !ids.insert(&track.id)
                || !INSTRUMENTS.contains(&track.instrument.as_str())
                || track.name.len() > 120
                || track.prompt.len() > 8000
                || !track.gain.is_finite()
                || !(0.0..=2.0).contains(&track.gain)
                || !track.pan.is_finite()
                || !(-1.0..=1.0).contains(&track.pan)
                || track.color.len() != 7
                || !track.color.starts_with('#')
                || !track.color[1..].bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(message("Invalid instrument track"));
            }
        }
        let mut clips = HashSet::new();
        for clip in &self.clips {
            let take = self
                .takes
                .iter()
                .find(|t| t.id == clip.take_id)
                .ok_or_else(|| message("Clip refers to missing take"))?;
            if !valid_id(&clip.id)
                || !clips.insert(&clip.id)
                || !ids.contains(&clip.track_id)
                || take.track_id != clip.track_id
                || clip.length == 0
                || clip
                    .start
                    .checked_add(clip.length)
                    .is_none_or(|n| n > self.duration)
                || clip
                    .offset
                    .checked_add(clip.length)
                    .is_none_or(|n| n > take.frames)
                || !clip.gain.is_finite()
                || !(0.0..=2.0).contains(&clip.gain)
            {
                return Err(message("Invalid clip bounds or source"));
            }
        }
        Ok(())
    }
}
impl Take {
    pub fn new(track_id: &str, name: &str, frames: u64, request: Value, revision: u64) -> Self {
        let tid = id();
        Self {
            file: format!("{tid}.wav"),
            id: tid,
            track_id: track_id.into(),
            name: name.into(),
            frames,
            created_at: now(),
            request,
            source_revision: revision,
            path: None,
        }
    }
}
