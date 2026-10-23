use crate::{
    audio::{Audio, RATE},
    error::{message, Result},
};

/// Choose a bar-aligned region without a long silent lead-in or ending.
/// Explicit repair may repeat a shorter whole-bar divisor of the target cycle.
pub fn loop_region(
    audio: &Audio,
    frames: u64,
    bar_frames: f64,
    repeat: bool,
) -> Result<(u64, u64)> {
    if frames == 0 || !bar_frames.is_finite() || bar_frames < 1.0 {
        return Err(message("Invalid musical loop length"));
    }
    // Measure musical energy after the player's default high-pass. Model DC
    // offsets must not make an otherwise silent outro look like usable audio.
    let block = (RATE / 100) as usize;
    let alpha = (-2.0 * std::f64::consts::PI * 20.0 / RATE as f64).exp() as f32;
    let mut previous = audio.samples.first().copied().unwrap_or([0.0; 2]);
    let mut high = [0.0; 2];
    let energy: Vec<f32> = audio
        .samples
        .chunks(block)
        .map(|samples| {
            let mut sum = 0.0;
            for pair in samples {
                for ch in 0..2 {
                    high[ch] = alpha * (high[ch] + pair[ch] - previous[ch]);
                    previous[ch] = pair[ch];
                    sum += high[ch] * high[ch];
                }
            }
            (sum / (samples.len() * 2) as f32).sqrt()
        })
        .collect();
    let peak = energy.iter().copied().fold(0.0_f32, f32::max);
    let threshold = (peak * 0.003162278).max(0.00001);
    let tolerance = (bar_frames / 4.0).round().max(1.0) as usize;
    let mut length = frames;
    loop {
        let mut best = None;
        let mut best_silence = usize::MAX;
        for bar in 0..=(audio.samples.len() as f64 / bar_frames).floor() as usize {
            let start = (bar as f64 * bar_frames).round() as usize;
            let end = start.saturating_add(length as usize);
            if end > audio.samples.len() {
                break;
            }
            let first = start / block;
            let last = end.div_ceil(block).min(energy.len());
            let region = &energy[first..last];
            let leading = region
                .iter()
                .position(|v| *v > threshold)
                .map(|i| ((first + i) * block).saturating_sub(start))
                .unwrap_or(end - start);
            let trailing = region
                .iter()
                .rposition(|v| *v > threshold)
                .map(|i| end.saturating_sub((first + i + 1) * block))
                .unwrap_or(end - start);
            if leading < tolerance && trailing < tolerance && leading + trailing < best_silence {
                best = Some((start as u64, end as u64));
                best_silence = leading + trailing;
            }
        }
        if let Some(region) = best {
            return Ok(region);
        }
        let shorter = length / 2;
        if !repeat
            || shorter as f64 + 0.5 < bar_frames
            || (shorter as f64 / bar_frames - (shorter as f64 / bar_frames).round()).abs()
                > 1.0 / bar_frames
        {
            return Err(message("No complete musical loop without edge silence; regenerate or select a shorter phrase"));
        }
        length = shorter;
    }
}
