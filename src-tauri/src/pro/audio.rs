use super::model::{MAX_FRAMES, RATE};
use crate::error::{message, Result};
use std::{fs, path::Path};

#[derive(Clone, Debug)]
pub struct Audio {
    pub samples: Vec<[f32; 2]>,
}
pub struct MixSource<'a> {
    pub audio: &'a Audio,
    pub start: u64,
    pub offset: u64,
    pub length: u64,
    pub gain: f32,
    pub pan: f32,
}

/// Mix source clips on the 48 kHz timeline without changing their relative levels.
pub fn mix(frames: u64, sources: &[MixSource<'_>]) -> Audio {
    let mut samples = vec![[0.0; 2]; frames as usize];
    for source in sources {
        let gain = [
            source.gain * (1.0 - source.pan).min(1.0),
            source.gain * (1.0 + source.pan).min(1.0),
        ];
        for n in 0..source.length {
            let Some(output) = samples.get_mut((source.start + n) as usize) else {
                break;
            };
            let Some(input) = source.audio.samples.get((source.offset + n) as usize) else {
                break;
            };
            for ch in 0..2 {
                output[ch] += input[ch] * gain[ch];
            }
        }
    }
    Audio { samples }
}
impl Audio {
    pub fn silence(frames: u64) -> Self {
        Self {
            samples: vec![[0.0; 2]; frames as usize],
        }
    }
    pub fn peak(&self) -> f32 {
        self.samples
            .iter()
            .flatten()
            .fold(0.0_f32, |peak, x| peak.max(x.abs()))
    }
    pub fn headroom(&mut self, target: f32) {
        let gain = (target / self.peak().max(0.000001)).min(1.0);
        for sample in self.samples.iter_mut().flatten() {
            *sample *= gain;
        }
    }
    /// Decode bounded PCM/float WAV files and resample mono/stereo sources to 48 kHz.
    pub fn read(path: &Path) -> Result<Self> {
        if fs::metadata(path)?.len() > 256 * 1024 * 1024 {
            return Err(message("WAV exceeds 256 MB"));
        }
        let bytes = fs::read(path)?;
        Self::decode(&bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        fn u16_at(b: &[u8], n: usize) -> u16 {
            u16::from_le_bytes([b[n], b[n + 1]])
        }
        fn u32_at(b: &[u8], n: usize) -> u32 {
            u32::from_le_bytes([b[n], b[n + 1], b[n + 2], b[n + 3]])
        }
        if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(message("Choose a PCM or float WAV file"));
        }
        let mut fmt = None;
        let mut data = None;
        let mut pos = 12;
        while pos + 8 <= bytes.len() {
            let len = u32_at(bytes, pos + 4) as usize;
            let start = pos + 8;
            let end = start
                .checked_add(len)
                .filter(|end| *end <= bytes.len())
                .ok_or_else(|| message("Truncated WAV chunk"))?;
            match &bytes[pos..pos + 4] {
                b"fmt " => fmt = Some(&bytes[start..end]),
                b"data" => data = Some(&bytes[start..end]),
                _ => {}
            }
            pos = end + len % 2;
        }
        let fmt = fmt
            .filter(|f| f.len() >= 16)
            .ok_or_else(|| message("Missing WAV format"))?;
        let data = data.ok_or_else(|| message("Missing WAV data"))?;
        let mut format = u16_at(fmt, 0);
        if format == 0xfffe && fmt.len() >= 40 {
            format = u16_at(fmt, 24);
        }
        let channels = u16_at(fmt, 2) as usize;
        let rate = u32_at(fmt, 4) as u64;
        let bits = u16_at(fmt, 14) as usize;
        if !(1..=2).contains(&channels)
            || !(8000..=192000).contains(&rate)
            || !(format == 1 && [16, 24, 32].contains(&bits) || format == 3 && bits == 32)
        {
            return Err(message(
                "Supported WAV: mono/stereo PCM 16/24/32 bit or float32, 8–192 kHz",
            ));
        }
        let stride = bits / 8 * channels;
        if u16_at(fmt, 12) as usize != stride || data.len() % stride != 0 {
            return Err(message("Invalid WAV alignment"));
        }
        let source_frames = data.len() / stride;
        let frames = source_frames as u64 * RATE / rate;
        if frames == 0 || frames > MAX_FRAMES {
            return Err(message("Audio must be between one sample and ten minutes"));
        }
        let mut source = Vec::with_capacity(source_frames);
        for frame in data.chunks_exact(stride) {
            let mut pair = [0.0; 2];
            for ch in 0..channels {
                let b = &frame[ch * bits / 8..];
                let value = match (format, bits) {
                    (3, 32) => f32::from_le_bytes(b[..4].try_into().unwrap()),
                    (1, 16) => i16::from_le_bytes(b[..2].try_into().unwrap()) as f32 / 32768.0,
                    (1, 24) => {
                        ((b[0] as i32 | (b[1] as i32) << 8 | (b[2] as i32) << 16) << 8 >> 8) as f32
                            / 8388608.0
                    }
                    _ => i32::from_le_bytes(b[..4].try_into().unwrap()) as f32 / 2147483648.0,
                };
                if !value.is_finite() || value.abs() > 16.0 {
                    return Err(message("Invalid WAV sample"));
                }
                pair[ch] = value;
            }
            if channels == 1 {
                pair[1] = pair[0];
            }
            source.push(pair);
        }
        if rate == RATE {
            return Ok(Self { samples: source });
        }
        let samples = (0..frames)
            .map(|n| {
                let position = n as f64 * rate as f64 / RATE as f64;
                let index = position as usize;
                let fraction = (position - index as f64) as f32;
                let a = source[index];
                let b = source[(index + 1).min(source.len() - 1)];
                [
                    a[0] + (b[0] - a[0]) * fraction,
                    a[1] + (b[1] - a[1]) * fraction,
                ]
            })
            .collect();
        Ok(Self { samples })
    }
    /// Save float WAV without clipping or silently normalizing the arrangement.
    pub fn write(&self, path: &Path) -> Result<()> {
        let len = self.samples.len() * 8;
        let mut bytes = Vec::with_capacity(44 + len);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + len as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt \x10\0\0\0\x03\0\x02\0");
        bytes.extend_from_slice(&(RATE as u32).to_le_bytes());
        bytes.extend_from_slice(&(RATE as u32 * 8).to_le_bytes());
        bytes.extend_from_slice(&8_u16.to_le_bytes());
        bytes.extend_from_slice(&32_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(len as u32).to_le_bytes());
        for value in self.samples.iter().flatten() {
            if !value.is_finite() {
                return Err(message("Invalid rendered sample"));
            }
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let temporary = path.with_extension(format!("{}.part", crate::types::id()));
        {
            use std::io::Write;
            let mut file = fs::File::create(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
        }
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::remove_file(temporary);
            return Err(error.into());
        }
        Ok(())
    }
}
