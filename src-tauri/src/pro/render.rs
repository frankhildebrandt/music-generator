use super::{
    audio::{mix, Audio, MixSource},
    model::*,
};
use crate::{
    db::Database,
    error::{message, Result},
};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

pub const BUFFER_BUDGET: u64 = RATE * 60 * 12;

/// Render either the audible arrangement or an independent selection of context tracks.
pub fn render(
    db: &Database,
    project: &Project,
    context: bool,
    exclude: Option<&str>,
    only: Option<&str>,
) -> Result<Audio> {
    project.validate()?;
    let any_solo = project.tracks.iter().any(|t| t.solo);
    let tracks: HashMap<_, _> = project
        .tracks
        .iter()
        .filter(|t| {
            if context {
                t.context && Some(t.id.as_str()) != exclude
            } else if let Some(id) = only {
                t.id == id
            } else {
                !t.mute && (!any_solo || t.solo)
            }
        })
        .map(|t| (&t.id, t))
        .collect();
    let clips: Vec<_> = project
        .clips
        .iter()
        .filter(|c| tracks.contains_key(&c.track_id))
        .collect();
    let used: HashSet<_> = clips.iter().map(|c| &c.take_id).collect();
    let takes: Vec<_> = project
        .takes
        .iter()
        .filter(|t| used.contains(&t.id))
        .collect();
    if takes.iter().map(|t| t.frames).sum::<u64>() > BUFFER_BUDGET {
        return Err(message(
            "Arrangement exceeds audio memory budget; use shorter takes",
        ));
    }
    let mut audio = HashMap::new();
    for take in takes {
        audio.insert(
            &take.id,
            Audio::read(&db.pro_path(&project.id, &take.file)?)?,
        );
    }
    let sources = clips
        .iter()
        .map(|clip| {
            let track = tracks[&clip.track_id];
            Ok(MixSource {
                audio: audio
                    .get(&clip.take_id)
                    .ok_or_else(|| message("Missing take audio"))?,
                start: clip.start,
                offset: clip.offset,
                length: clip.length,
                gain: clip.gain * track.gain,
                pan: track.pan,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(mix(project.duration, &sources))
}

/// Blend inside a selected range, leaving every sample outside it unchanged.
pub fn splice(
    original: &Audio,
    generated: &Audio,
    clip: &Clip,
    start: u64,
    end: u64,
) -> Result<Audio> {
    if start < clip.start
        || end > clip.start + clip.length
        || start >= end
        || end as usize > generated.samples.len()
    {
        return Err(message("Invalid regeneration range"));
    }
    let offset = clip.offset + start - clip.start;
    let length = end - start;
    if offset + length > original.samples.len() as u64 {
        return Err(message("Original take is shorter than the selected region"));
    }
    let mut output = original.clone();
    let fade = (RATE * 15 / 1000).min(length / 2).max(1);
    for n in 0..length {
        let weight =
            (((n + 1) as f32 / fade as f32).min((length - n) as f32 / fade as f32)).min(1.0);
        for ch in 0..2 {
            let old = original.samples[(offset + n) as usize][ch];
            output.samples[(offset + n) as usize][ch] =
                old + (generated.samples[(start + n) as usize][ch] - old) * weight;
        }
    }
    Ok(output)
}

pub fn export(db: &Database, project: &Project, destination: &Path, stems: bool) -> Result<()> {
    if !stems {
        return render(db, project, false, None, None)?.write(destination);
    }
    let temporary = destination.with_extension(format!("{}.part", crate::types::id()));
    let result = (|| -> Result<()> {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&temporary)?);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (index, track) in project.tracks.iter().enumerate() {
            let audio = render(db, project, false, None, Some(&track.id))?;
            let scratch = db.root.join(format!("{}.wav", crate::types::id()));
            audio.write(&scratch)?;
            let result = (|| -> Result<()> {
                zip.start_file(
                    format!(
                        "{:02}-{}.wav",
                        index + 1,
                        crate::exports::clean_name(&track.name)
                    ),
                    options,
                )
                .map_err(|e| message(e.to_string()))?;
                std::io::copy(&mut std::fs::File::open(&scratch)?, &mut zip)?;
                Ok(())
            })();
            let _ = std::fs::remove_file(scratch);
            result?;
        }
        zip.start_file("project.json", options)
            .map_err(|e| message(e.to_string()))?;
        std::io::Write::write_all(&mut zip, &serde_json::to_vec_pretty(project)?)?;
        zip.finish()
            .map_err(|e| message(e.to_string()))?
            .sync_all()?;
        std::fs::rename(&temporary, destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}
