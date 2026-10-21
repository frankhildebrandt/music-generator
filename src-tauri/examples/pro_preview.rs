//! Seed an isolated development profile with the three original listening-test stems.
use music_generator::{db::Database, hardware, pro::model::RATE, setup::fingerprint};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("Usage: pro_preview PROFILE MODEL_DIRECTORY STEM_DIRECTORY".into());
    }
    let root = std::env::current_dir()?.join(&args[1]);
    if root.join("music.db").exists() {
        return Err("Choose an empty preview profile; existing profiles are preserved".into());
    }
    let db = Database::open(&root)?;
    let mut settings = db.settings()?;
    settings.models_dir = PathBuf::from(&args[2])
        .canonicalize()?
        .to_string_lossy()
        .into();
    settings.setup_complete = true;
    settings.lm_model = "acestep-5Hz-lm-4B-Q8_0.gguf".into();
    settings.mode = "single".into();
    settings.tested_fingerprint = fingerprint(&settings.lm_model, &hardware::inspect().gpu);
    db.save_settings(&settings)?;
    let mut project = db.pro_create("First light · Base session", 120, "A minor", RATE * 16)?;
    for (file, name, instrument, color) in [
        ("01-drums.wav", "Drum room", "drums", "#dca766"),
        ("02-bass.wav", "Low tide", "bass", "#70c3b0"),
        ("03-chords.wav", "Warm Rhodes", "keyboard", "#9994e8"),
    ] {
        project = db.pro_import(&project.id, &PathBuf::from(&args[3]).join(file), None)?;
        let track = project.tracks.last_mut().unwrap();
        track.name = name.into();
        track.instrument = instrument.into();
        track.color = color.into();
        track.gain = 0.5;
        project = db.pro_save(&project)?;
    }
    println!(
        "Created isolated preview profile {} with project {}",
        root.display(),
        project.id
    );
    Ok(())
}
