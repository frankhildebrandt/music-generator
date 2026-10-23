use crate::{
    audio::{Audio, MAX_FRAMES, RATE},
    error::{message, Result},
};
use std::path::Path;
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, formats::FormatOptions, io::MediaSourceStream,
    meta::MetadataOptions, probe::Hint,
};

/// Decode app WAV/MP3 exports into bounded, canonical stereo audio.
pub fn import_audio(path: &Path) -> Result<Audio> {
    if path.metadata()?.len() > 256 * 1024 * 1024 {
        return Err(message("Audio exceeds 256 MiB"));
    }
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("wav"))
    {
        return Audio::read(path);
    }
    let input = MediaSourceStream::new(Box::new(std::fs::File::open(path)?), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("mp3");
    let mut reader = symphonia::default::get_probe()
        .format(
            &hint,
            input,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| message(e.to_string()))?
        .format;
    let track = reader
        .default_track()
        .ok_or_else(|| message("No audio track"))?;
    let id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| message(e.to_string()))?;
    let mut samples = Vec::new();
    let mut rate = 0;
    loop {
        let packet = match reader.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(e) => return Err(message(e.to_string())),
        };
        if packet.track_id() != id {
            continue;
        }
        let decoded = decoder
            .decode(&packet)
            .map_err(|e| message(e.to_string()))?;
        let spec = *decoded.spec();
        let channels = spec.channels.count();
        if channels == 0
            || channels > 2
            || !(8000..=192000).contains(&spec.rate)
            || (rate != 0 && rate != spec.rate)
        {
            return Err(message("Unsupported audio layout"));
        }
        rate = spec.rate;
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        buffer.copy_interleaved_ref(decoded);
        for frame in buffer.samples().chunks_exact(channels) {
            if frame.iter().any(|v| !v.is_finite()) {
                return Err(message("Invalid audio sample"));
            }
            samples.push([frame[0], frame[channels - 1]]);
        }
        if samples.len() as u64 > spec.rate as u64 * 600 {
            return Err(message("Audio exceeds ten minutes"));
        }
    }
    if samples.is_empty() {
        return Err(message("Empty audio"));
    }
    let frames = (samples.len() as u64 * RATE / rate as u64).min(MAX_FRAMES);
    Ok(Audio {
        samples: (0..frames)
            .map(|n| {
                let p = n as f64 * rate as f64 / RATE as f64;
                let i = p as usize;
                let f = (p - i as f64) as f32;
                let a = samples[i];
                let b = samples[(i + 1).min(samples.len() - 1)];
                [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f]
            })
            .collect(),
    })
}
