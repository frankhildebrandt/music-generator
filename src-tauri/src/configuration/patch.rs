use crate::{
    error::{message, Result},
    types::Settings,
};
use std::path::Path;

#[derive(Default, serde::Deserialize)]
#[serde(default)]
pub struct SettingsPatch {
    pub lm_device: Option<String>,
    pub export_format: Option<String>,
    pub timeout_minutes: Option<f64>,
    pub dit: Option<String>,
    pub songs_dir: Option<String>,
}

impl SettingsPatch {
    pub(super) fn apply(
        self,
        mut settings: Settings,
        root: &Path,
        resources: &Path,
    ) -> Result<Settings> {
        if let Some(device) = self.lm_device {
            settings.lm_device = device;
        }
        if let Some(format) = self.export_format {
            settings.export_format = format;
        }
        if let Some(timeout) = self.timeout_minutes {
            settings.timeout_minutes = timeout;
        }
        if let Some(dit) = self.dit {
            settings.dit = dit;
            settings.synthesis_model.clear();
        }
        if let Some(dir) = self.songs_dir {
            settings.songs_dir = if dir.is_empty() {
                root.join("songs").to_string_lossy().into()
            } else {
                dir
            };
        }
        validate_settings(&settings)?;
        if settings.export_format == "mp3" && !crate::audio_export::codec_path(resources).is_file()
        {
            return Err(message("mp3_failed"));
        }
        if settings.synthesis_model.is_empty()
            && settings.dit == "xl"
            && !Path::new(&settings.models_dir)
                .join("acestep-v15-xl-sft-Q8_0.gguf")
                .is_file()
        {
            return Err(message("Download the XL model in model setup first"));
        }
        Ok(settings)
    }
}

pub fn validate_settings(settings: &Settings) -> Result<()> {
    if !["auto", "cpu"].contains(&settings.lm_device.as_str()) {
        return Err(message("Unknown language model device"));
    }
    if !["wav", "mp3"].contains(&settings.export_format.as_str()) {
        return Err(message("Unknown export format"));
    }
    if !settings.poll_interval.is_finite()
        || !(0.1..=60.0).contains(&settings.poll_interval)
        || !settings.timeout_minutes.is_finite()
        || !(1.0..=600.0).contains(&settings.timeout_minutes)
    {
        return Err(message("Invalid timeout or poll interval"));
    }
    if !["standard", "xl"].contains(&settings.dit.as_str()) {
        return Err(message("Unknown synthesis model"));
    }
    for path in [&settings.models_dir, &settings.songs_dir] {
        if !std::path::Path::new(path).is_absolute() {
            return Err(message("Select an absolute data folder"));
        }
    }
    Ok(())
}
