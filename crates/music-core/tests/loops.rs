use music_core::{
    audio::Audio,
    loops::{edit, ContextSource, EditRecipe, GenerationRequest},
};
fn audio() -> Audio {
    Audio {
        samples: (0..48000)
            .map(|n| {
                let x = (n as f32 * 0.05).sin() * 0.4;
                [x, x * 0.7]
            })
            .collect(),
    }
}
#[test]
fn seam_blend_keeps_length_and_stereo_endpoint() {
    let source = audio();
    let old = source.samples.clone();
    let output = edit(
        &source,
        &EditRecipe {
            frames: 24000,
            end: 24000,
            crossfade_ms: 5.0,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(output.samples.len(), 24000);
    assert_eq!(source.samples, old);
    assert_eq!(output.samples[0], output.samples[23999]);
    assert_eq!(&output.samples[240..23760], &old[240..23760]);
}
#[test]
fn normalizing_silence_never_produces_nan() {
    let a = Audio::silence(512);
    let output = edit(
        &a,
        &EditRecipe {
            normalize_db: Some(-1.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(output.peak(), 0.0);
}
#[test]
fn normalization_occurs_after_seam_render() {
    let out = edit(
        &audio(),
        &EditRecipe {
            crossfade_ms: 5.0,
            normalize_db: Some(-1.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!((out.peak() - 10_f32.powf(-1.0 / 20.0)).abs() < 1e-6);
}
#[test]
fn invalid_ranges_are_rejected() {
    assert!(edit(
        &audio(),
        &EditRecipe {
            start: 24000,
            end: 12000,
            ..Default::default()
        }
    )
    .is_err());
    assert!(edit(
        &audio(),
        &EditRecipe {
            crossfade_ms: f64::NAN,
            ..Default::default()
        }
    )
    .is_err());
}
#[test]
fn spectral_reduction_is_bounded_and_finite() {
    let source = Audio {
        samples: (0..24000)
            .map(|n| {
                let x = ((n * 1664525_u64 + 1013904223) % 65536) as f32 / 65536.0 - 0.5;
                [x * 0.1, x * 0.1]
            })
            .collect(),
    };
    let output = edit(
        &source,
        &EditRecipe {
            noise_start: 0,
            noise_end: 4096,
            noise_db: 12.0,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(source.samples.len(), output.samples.len());
    let energy = |a: &Audio| a.samples.iter().map(|s| s[0] * s[0]).sum::<f32>();
    assert!(energy(&output) < energy(&source));
    assert!(output.samples.iter().flatten().all(|s| s.is_finite()));
}
#[test]
fn context_cannot_include_target_slot() {
    let mut req:GenerationRequest=serde_json::from_value(serde_json::json!({"slot":36,"instrument":"bass","prompt":"funk","key":"A minor","bpm":120,"beats":16,"seed":42})).unwrap();
    assert!(req.validate().is_ok());
    assert_eq!(req.seconds(), 8.0);
    req.context.push(ContextSource {
        slot: 36,
        path: "bass.wav".into(),
    });
    assert!(req.validate().is_err());
}

#[test]
fn dc_removal_preserves_stereo_cycle_and_normalization() {
    let source = Audio {
        samples: (0..4800)
            .map(|n| {
                let tone = (n as f32 * 2.0 * std::f32::consts::PI / 48.0).sin() * 0.2;
                [tone + 0.3, tone - 0.15]
            })
            .collect(),
    };
    let output = edit(
        &source,
        &EditRecipe {
            remove_dc: true,
            normalize_db: Some(-1.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(source.samples.len(), output.samples.len());
    for ch in 0..2 {
        let mean =
            output.samples.iter().map(|s| s[ch] as f64).sum::<f64>() / output.samples.len() as f64;
        assert!(mean.abs() < 1e-6);
    }
    assert!((output.peak() - 10_f32.powf(-1.0 / 20.0)).abs() < 1e-6);
    assert_eq!(source.samples[0], [0.3, -0.15]);
}
