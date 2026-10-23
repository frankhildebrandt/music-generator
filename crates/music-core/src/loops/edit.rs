use super::EditRecipe;
use crate::{
    audio::{Audio, MAX_FRAMES, RATE},
    error::{message, Result},
};
use rustfft::{num_complex::Complex, FftPlanner};

pub fn seam_score(audio: &Audio, start: usize, length: usize) -> f32 {
    if length == 0 || start + length > audio.samples.len() {
        return f32::INFINITY;
    }
    let a = audio.samples[start];
    let b = audio.samples[start + length - 1];
    (a[0] - b[0]).abs()
        + (a[1] - b[1]).abs()
        + (a[0].abs() + a[1].abs() + b[0].abs() + b[1].abs()) * 0.25
}
/// Find one stereo-safe start while retaining the exact requested cycle length.
pub fn zero_start(audio: &Audio, start: usize, length: usize) -> usize {
    let upper = start
        .saturating_add(480)
        .min(audio.samples.len().saturating_sub(length));
    (start.saturating_sub(480)..=upper)
        .min_by(|a, b| seam_score(audio, *a, length).total_cmp(&seam_score(audio, *b, length)))
        .unwrap_or(start)
}
/// Render immutable source edits; seam blending preserves the cycle's frame count.
pub fn edit(source: &Audio, recipe: &EditRecipe) -> Result<Audio> {
    let end = if recipe.end == 0 {
        source.samples.len() as u64
    } else {
        recipe.end
    };
    let frames = if recipe.frames == 0 {
        end.saturating_sub(recipe.start)
    } else {
        recipe.frames
    };
    if recipe.start >= end
        || end > source.samples.len() as u64
        || frames == 0
        || frames > MAX_FRAMES
        || !recipe.crossfade_ms.is_finite()
        || !(0.0..=1000.0).contains(&recipe.crossfade_ms)
        || !recipe.noise_db.is_finite()
        || !(0.0..=30.0).contains(&recipe.noise_db)
        || recipe
            .normalize_db
            .is_some_and(|d| !d.is_finite() || !(-60.0..=0.0).contains(&d))
    {
        return Err(message("Invalid loop edit"));
    }
    let selected = &source.samples[recipe.start as usize..end as usize];
    let mut output = Audio {
        samples: (0..frames as usize)
            .map(|n| selected[n % selected.len()])
            .collect(),
    };
    if recipe.noise_db > 0.0 {
        denoise(source, &mut output, recipe)?;
    }
    let fade = ((recipe.crossfade_ms * RATE as f64 / 1000.0).round() as usize)
        .min(output.samples.len() / 2);
    let size = output.samples.len();
    for n in 0..fade {
        let w = 0.75 - 0.25 * (std::f32::consts::PI * n as f32 / fade.max(1) as f32).cos();
        let a = output.samples[n];
        let b = output.samples[size - 1 - n];
        for ch in 0..2 {
            output.samples[n][ch] = a[ch] * w + b[ch] * (1.0 - w);
            output.samples[size - 1 - n][ch] = b[ch] * w + a[ch] * (1.0 - w);
        }
    }
    if let Some(db) = recipe.normalize_db {
        let peak = output.peak();
        if peak > 1e-12 {
            let gain = 10.0_f32.powf(db as f32 / 20.0) / peak;
            for s in output.samples.iter_mut().flatten() {
                *s *= gain;
            }
        }
    }
    Ok(output)
}
/// Stereo-linked spectral subtraction with overlap-add and an explicit noise profile.
fn denoise(source: &Audio, output: &mut Audio, recipe: &EditRecipe) -> Result<()> {
    const N: usize = 1024;
    const H: usize = 256;
    if recipe.noise_end <= recipe.noise_start
        || recipe.noise_end > source.samples.len() as u64
        || recipe.noise_end - recipe.noise_start < N as u64
    {
        return Err(message("Select at least 22 ms of noise for the profile"));
    }
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(N);
    let inverse = planner.plan_fft_inverse(N);
    let window: Vec<f32> = (0..N)
        .map(|n| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * n as f32 / N as f32).cos())
        .collect();
    let mut spectrum = vec![Complex::default(); N];
    let mut noise = vec![0.0; N];
    let mut count = 0;
    for offset in (recipe.noise_start as usize..=recipe.noise_end as usize - N).step_by(H) {
        for ch in 0..2 {
            for n in 0..N {
                spectrum[n] = Complex::new(source.samples[offset + n][ch] * window[n], 0.0);
            }
            fft.process(&mut spectrum);
            for n in 0..N {
                noise[n] += spectrum[n].norm_sqr();
            }
            count += 1;
        }
    }
    for p in &mut noise {
        *p /= count as f32;
    }
    let original = output.samples.clone();
    let len = original.len();
    output.samples.fill([0.0; 2]);
    let mut weights = vec![0.0; len];
    let mut left = vec![Complex::default(); N];
    let mut right = left.clone();
    let floor = 10.0_f32.powf(-recipe.noise_db as f32 / 20.0);
    for offset in (0..len + N).step_by(H) {
        let start = offset as isize - N as isize;
        for n in 0..N {
            let pos = start + n as isize;
            let pair = if pos >= 0 && (pos as usize) < len {
                original[pos as usize]
            } else {
                [0.0; 2]
            };
            left[n] = Complex::new(pair[0] * window[n], 0.0);
            right[n] = Complex::new(pair[1] * window[n], 0.0);
        }
        fft.process(&mut left);
        fft.process(&mut right);
        for n in 0..N {
            let power = (left[n].norm_sqr() + right[n].norm_sqr()) * 0.5;
            let gain = (1.0 - noise[n] / power.max(1e-20))
                .max(0.0)
                .sqrt()
                .max(floor);
            left[n] *= gain;
            right[n] *= gain;
        }
        inverse.process(&mut left);
        inverse.process(&mut right);
        for n in 0..N {
            let pos = start + n as isize;
            if pos >= 0 && (pos as usize) < len {
                let i = pos as usize;
                weights[i] += window[n] * window[n];
                output.samples[i][0] += left[n].re * window[n] / N as f32;
                output.samples[i][1] += right[n].re * window[n] / N as f32;
            }
        }
    }
    for (pair, w) in output.samples.iter_mut().zip(weights) {
        for v in pair {
            *v /= w.max(1e-12);
        }
    }
    Ok(())
}
