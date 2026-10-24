use music_core::{
    audio::{Audio, RATE},
    loops::{edit, loop_region, EditRecipe},
};

fn fill_tone(source: &mut Audio, start: usize, end: usize, right_only: bool) {
    for (n, pair) in source.samples[start..end].iter_mut().enumerate() {
        let value = (n as f32 * 0.07).sin() * 0.25;
        *pair = [if right_only { 0.0 } else { value }, value * 0.8];
    }
}
fn assert_continuous(audio: &Audio) {
    for window in audio.samples.chunks(480) {
        let energy: f32 = window.iter().map(|s| s[1] * s[1]).sum();
        assert!(energy / window.len() as f32 > 0.001);
    }
}
#[test]
fn eight_bars_skip_silent_intro_and_outro_without_padding() {
    let bar = RATE * 2; // 4/4, 120 BPM
    let target = bar * 8;
    let mut source = Audio::silence(bar * 12);
    fill_tone(&mut source, bar as usize, (bar * 9) as usize, false);
    let (start, end) = loop_region(&source, target, bar as f64, false).unwrap();
    assert_eq!((start, end), (bar, bar * 9));
    let output = edit(
        &source,
        &EditRecipe {
            start,
            end,
            frames: target,
            crossfade_ms: 5.0,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(output.samples.len() as u64, target);
    assert_continuous(&output);
}

#[test]
fn incomplete_generation_is_rejected_instead_of_adding_silence() {
    let bar = RATE * 2;
    let mut source = Audio::silence(bar * 10);
    fill_tone(&mut source, 0, (bar * 6) as usize, false);
    assert!(loop_region(&source, bar * 8, bar as f64, false).is_err());
    assert!(loop_region(&Audio::silence(bar * 10), bar * 8, bar as f64, true).is_err());
}

#[test]
fn explicit_repair_repeats_whole_bars_and_preserves_internal_rests() {
    let bar = RATE * 2;
    let mut source = Audio::silence(bar * 8);
    fill_tone(&mut source, 0, (bar * 6) as usize, true);
    source.samples[bar as usize..(bar + RATE / 2) as usize].fill([0.0; 2]);
    let (start, end) = loop_region(&source, bar * 8, bar as f64, true).unwrap();
    assert_eq!((start, end), (0, bar * 4));
    let output = edit(
        &source,
        &EditRecipe {
            start,
            end,
            frames: bar * 8,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(output.samples.len() as u64, bar * 8);
    assert_eq!(
        &output.samples[..(bar * 4) as usize],
        &output.samples[(bar * 4) as usize..]
    );
    assert_eq!(output.samples[bar as usize], [0.0; 2]);
    assert_eq!(
        output.samples.last().unwrap(),
        &source.samples[(bar * 4 - 1) as usize]
    );
}

#[test]
fn repair_accepts_fractional_frame_bar_lengths() {
    let bar = RATE as f64 * 4.0 * 60.0 / 137.0;
    let frames = (bar * 8.0).round() as u64;
    let mut source = Audio::silence(frames);
    fill_tone(&mut source, 0, (bar * 5.0).round() as usize, false);
    let (start, end) = loop_region(&source, frames, bar, true).unwrap();
    let output = edit(
        &source,
        &EditRecipe {
            start,
            end,
            frames,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(output.samples.len() as u64, frames);
    assert_continuous(&output);
}

#[test]
fn model_dc_offset_does_not_hide_a_silent_ending() {
    let bar = RATE * 2;
    let mut source = Audio::silence(bar * 10);
    fill_tone(&mut source, 0, (bar * 6) as usize, false);
    for pair in &mut source.samples {
        pair[0] += 0.005;
        pair[1] -= 0.002;
    }
    assert!(loop_region(&source, bar * 8, bar as f64, false).is_err());
    let (start, end) = loop_region(&source, bar * 8, bar as f64, true).unwrap();
    assert_eq!(end - start, bar * 4);
}
