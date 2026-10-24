//! Versioned provider/asset catalog. Only adapters implemented by this app can run.
use crate::{
    error::{message, Result},
    hardware::Hardware,
    manifest::Model,
    types::Settings,
    verification::{ModelVerifier, Verification},
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub status: String,
    pub adapter: Option<String>,
    pub source: String,
    #[serde(default)]
    pub integrated_language_model: bool,
    #[serde(default)]
    pub supports_analysis: bool,
    #[serde(default)]
    pub max_duration_seconds: u32,
    #[serde(default)]
    pub memory_reserve_bytes: Option<u64>,
    #[serde(default)]
    pub default_synthesis: String,
    #[serde(default)]
    pub default_language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub file: String,
    pub provider: String,
    pub role: String,
    pub family: String,
    pub precision: String,
    pub estimated_bytes: u64,
    pub sha256: String,
    pub recommended_steps: Option<u32>,
    #[serde(default)]
    pub url: Option<String>,
}

impl Asset {
    pub fn url(&self, catalog: &Catalog) -> String {
        self.url
            .clone()
            .unwrap_or_else(|| format!("{}/{}", catalog.base_url, self.file))
    }
    pub fn download(&self) -> Model {
        Model {
            file: self.file.clone(),
            estimated_bytes: self.estimated_bytes,
            sha256: self.sha256.clone(),
            kind: self.role.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub version: u32,
    pub revision: String,
    pub base_url: String,
    pub providers: Vec<Provider>,
    pub models: Vec<Asset>,
}

pub fn catalog() -> Catalog {
    serde_json::from_str(include_str!("../../models/catalog.json")).expect("embedded model catalog")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub provider: String,
    pub synthesis: String,
    pub language: String,
}

impl Selection {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            provider: settings.model_provider.clone(),
            synthesis: if settings.synthesis_model.is_empty() {
                if settings.dit == "xl" {
                    "acestep-v15-xl-sft-Q8_0.gguf"
                } else {
                    "acestep-v15-sft-Q8_0.gguf"
                }
                .into()
            } else {
                settings.synthesis_model.clone()
            },
            language: settings.lm_model.clone(),
        }
    }

    pub fn apply(&self, settings: &mut Settings) -> Result<()> {
        required(self)?;
        let previous = Self::from_settings(settings);
        if required(&previous).is_ok() {
            settings
                .model_profiles
                .insert(previous.provider.clone(), previous);
        }
        settings
            .model_profiles
            .insert(self.provider.clone(), self.clone());
        settings.model_provider = self.provider.clone();
        settings.synthesis_model = self.synthesis.clone();
        settings.lm_model = self.language.clone();
        if self.provider == "ace-step" {
            settings.dit = if self.synthesis.contains("-xl-") {
                "xl"
            } else {
                "standard"
            }
            .into();
        }
        Ok(())
    }
}

/// Resolve identifiers through the pinned catalog, never through user-provided paths/URLs.
pub fn required(selection: &Selection) -> Result<Vec<Asset>> {
    catalog().required(selection)
}

impl Catalog {
    pub fn required(&self, selection: &Selection) -> Result<Vec<Asset>> {
        // Each available provider must have an explicitly implemented native adapter.
        let adapter = match selection.provider.as_str() {
            "ace-step" => "acestep.cpp",
            "heartmula" => "audio.cpp",
            _ => "",
        };
        if adapter.is_empty()
            || !self.providers.iter().any(|p| {
                p.id == selection.provider
                    && p.status == "available"
                    && p.adapter.as_deref() == Some(adapter)
            })
        {
            return Err(message(
                "This model provider has no supported engine adapter yet",
            ));
        }
        let find = |file: &str, role: &str| {
            self.models
                .iter()
                .find(|m| m.file == file && m.role == role && m.provider == selection.provider)
                .cloned()
                .ok_or_else(|| message("Unknown model or incorrect model role"))
        };
        let mut models = vec![find(&selection.synthesis, "synthesis")?];
        if selection.provider == "heartmula" {
            if !selection.language.is_empty() {
                return Err(message(
                    "HeartMuLa includes its own language model and HeartCodec",
                ));
            }
        } else {
            models.push(find(&selection.language, "language")?);
        }
        models.extend(
            self.models
                .iter()
                .filter(|m| m.provider == selection.provider && m.role == "dependency")
                .cloned(),
        );
        Ok(models)
    }
}

/// Called under AppState.operation so downloads, verification and profile writes
/// cannot race setup, analysis, another activation or an engine job.
pub async fn install_profile(
    catalog: &Catalog,
    db: &crate::db::Database,
    selection: &Selection,
    cancel: &CancellationToken,
    progress: impl Fn(crate::download::DownloadProgress),
) -> Result<()> {
    let mut settings = db.settings()?;
    let models = catalog.required(selection)?;
    let root = Path::new(&settings.models_dir);
    let missing: u64 = models
        .iter()
        .filter(|m| !root.join(&m.file).is_file())
        .map(|m| {
            m.estimated_bytes.saturating_sub(
                root.join(format!("{}.part", m.file))
                    .metadata()
                    .map(|m| m.len())
                    .unwrap_or(0),
            )
        })
        .sum();
    if missing > 0 {
        crate::download::ensure_space(root, missing.saturating_add(512 * 1024 * 1024))?;
    }
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .read_timeout(std::time::Duration::from_secs(60))
        .build()?;
    let mut verifier = ModelVerifier::open(&db.root);
    for model in models {
        crate::download::download_model(
            &client,
            &model.url(catalog),
            root,
            &model.download(),
            &mut verifier,
            cancel,
            &progress,
        )
        .await?;
    }
    if cancel.is_cancelled() {
        return Err(crate::error::Error::Cancelled);
    }
    selection.apply(&mut settings)?;
    db.save_settings(&settings)
}

/// Conservative weight footprint plus runtime/OS headroom; duration can require more.
/// A small dedicated GPU does not make a model incompatible: the engine supports CPU.
pub fn memory_bytes(models: &[Asset]) -> u64 {
    let weights: u64 = models.iter().map(|m| m.estimated_bytes).sum();
    let reserve = if models.iter().any(|m| m.provider == "heartmula") {
        16
    } else if models
        .iter()
        .any(|m| m.role == "synthesis" && m.file.contains("-xl-"))
    {
        4
    } else {
        3
    };
    weights.saturating_add(reserve * 1024 * 1024 * 1024)
}

pub fn compatible(selection: &Selection, hardware: &Hardware) -> Result<bool> {
    Ok(memory_bytes(&required(selection)?) <= hardware.ram_bytes)
}

pub fn cached(selection: &Selection, folder: &Path, verifier: &ModelVerifier) -> Result<bool> {
    Ok(required(selection)?
        .iter()
        .all(|m| verifier.is_verified(&folder.join(&m.file), &m.sha256)))
}

pub async fn verify(
    selection: &Selection,
    folder: &Path,
    verifier: &mut ModelVerifier,
    cancel: &CancellationToken,
) -> Result<bool> {
    for model in required(selection)? {
        let path = folder.join(&model.file);
        if !path.is_file()
            || verifier
                .verify(&path, &model.sha256, cancel, |_, _| {})
                .await?
                == Verification::Invalid
        {
            return Ok(false);
        }
    }
    Ok(true)
}
