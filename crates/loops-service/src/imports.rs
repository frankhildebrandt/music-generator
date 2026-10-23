use super::*;
use std::{io::Read, path::Path};
/// Read rendered app stems; project take paths never address files outside the ZIP.
pub fn import(root: &Path, path: &Path) -> Result<Value> {
    let assets = root.join("assets");
    std::fs::create_dir_all(&assets)?;
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
    {
        return Ok(json!({"results":[one(&assets,path,None)?]}));
    }
    let mut archive =
        zip::ZipArchive::new(std::fs::File::open(path)?).map_err(|e| message(e.to_string()))?;
    if archive.len() > 512 {
        return Err(message("Archive has too many entries"));
    }
    let mut total = 0_u64;
    for n in 0..archive.len() {
        let entry = archive.by_index(n).map_err(|e| message(e.to_string()))?;
        total = total
            .checked_add(entry.size())
            .ok_or_else(|| message("Archive size overflow"))?;
    }
    if total > 512 * 1024 * 1024 {
        return Err(message("Archive exceeds 512 MiB uncompressed"));
    }
    let project = if let Ok(mut entry) = archive.by_name("project.json") {
        if entry.size() > 1024 * 1024 {
            return Err(message("Project metadata too large"));
        }
        let mut b = vec![];
        entry.read_to_end(&mut b)?;
        Some(serde_json::from_slice::<Value>(&b)?)
    } else {
        None
    };
    let mut results = vec![];
    for n in 0..archive.len() {
        let mut entry = archive.by_index(n).map_err(|e| message(e.to_string()))?;
        let extension = Path::new(entry.name())
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !["wav", "mp3"].contains(&extension.as_str()) || entry.is_dir() {
            continue;
        }
        if results.len() >= 128 || entry.size() > 256 * 1024 * 1024 {
            return Err(message("Archive exceeds loop-bank capacity"));
        }
        let filename = entry.name().to_owned();
        let staging = assets.join(format!("{}.{}", music_core::types::id(), extension));
        std::io::copy(&mut entry, &mut std::fs::File::create(&staging)?)?;
        let result = one(&assets, &staging, project.as_ref());
        let _ = std::fs::remove_file(staging);
        let mut result = result?;
        result["name"] = json!(Path::new(&filename)
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Loop"));
        results.push(result);
    }
    if results.is_empty() {
        return Err(message("Archive contains no WAV or MP3 audio"));
    }
    Ok(json!({"results":results}))
}
fn one(assets: &Path, path: &Path, project: Option<&Value>) -> Result<Value> {
    let audio = music_core::loops::import_audio(path)?;
    let target = assets.join(format!("{}.wav", music_core::types::id()));
    audio.write(&target)?;
    let bpm = project.and_then(|p| p["bpm"].as_f64());
    let beats = bpm.map(|b| audio.samples.len() as f64 / 48000.0 * b / 60.0);
    Ok(
        json!({"path":target,"original":target,"name":path.file_stem().and_then(|s|s.to_str()).unwrap_or("Loop"),"frames":audio.samples.len(),"bpm":bpm,"beats":beats,"key":project.and_then(|p|p["key"].as_str()),"gain":1,"pan":0}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn pro_stems_restore_music_metadata_without_reapplying_mix() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("source.wav");
        music_core::audio::Audio {
            samples: vec![[0.25, -0.5]; 4800],
        }
        .write(&wav)
        .unwrap();
        let path = dir.path().join("stems.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("project.json", options).unwrap();
        zip.write_all(br#"{"bpm":100,"key":"D minor","tracks":[{"volume":0.1,"pan":1}]}"#)
            .unwrap();
        zip.start_file("../drums.wav", options).unwrap();
        zip.write_all(&std::fs::read(wav).unwrap()).unwrap();
        zip.finish().unwrap();
        let result = import(dir.path(), &path).unwrap();
        let stem = &result["results"][0];
        assert_eq!(stem["bpm"].as_f64(), Some(100.0));
        assert_eq!(stem["key"], "D minor");
        assert_eq!(stem["gain"], 1);
        assert_eq!(stem["pan"], 0);
        let imported = PathBuf::from(stem["path"].as_str().unwrap());
        assert!(imported.starts_with(dir.path().join("assets")));
        assert_eq!(
            music_core::audio::Audio::read(&imported).unwrap().samples[0],
            [0.25, -0.5]
        );
        assert!(!dir.path().join("drums.wav").exists());
    }
    #[test]
    fn plain_library_zip_keeps_unknown_music_metadata_explicit() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("source.wav");
        music_core::audio::Audio::silence(4800).write(&wav).unwrap();
        let path = dir.path().join("library.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        zip.start_file("folder/loop.wav", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&std::fs::read(wav).unwrap()).unwrap();
        zip.finish().unwrap();
        let result = import(dir.path(), &path).unwrap();
        assert!(result["results"][0]["bpm"].is_null());
        assert_eq!(result["results"][0]["name"], "loop");
    }
}
