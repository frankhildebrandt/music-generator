use crate::{
    engine::Engine,
    error::{message, Result},
    types::Settings,
};
use std::path::Path;
use tokio_util::sync::CancellationToken;

// Check actual OS enforcement and inherited engine restrictions, not only the signing metadata.
pub fn run(forbidden: &Path) -> Result<()> {
    if std::fs::read(forbidden).is_ok() {
        return Err(message("App Sandbox did not deny an unselected file"));
    }
    let write = forbidden.with_extension("write-check");
    if std::fs::write(&write, b"sandbox probe").is_ok() {
        let _ = std::fs::remove_file(write);
        return Err(message(
            "App Sandbox did not deny an unselected directory write",
        ));
    }
    let root = super::data_root().join(format!("sandbox-probe-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("models"))?;
    let mut metadata = vec![0u8; 96];
    metadata[..4].copy_from_slice(b"GGUF");
    metadata[4..8].copy_from_slice(&3u32.to_le_bytes());
    metadata[16..24].copy_from_slice(&1u64.to_le_bytes());
    metadata[24..32].copy_from_slice(&20u64.to_le_bytes());
    metadata[32..52].copy_from_slice(b"general.architecture");
    metadata[52..56].copy_from_slice(&8u32.to_le_bytes());
    metadata[56..64].copy_from_slice(&10u64.to_le_bytes());
    metadata[64..74].copy_from_slice(b"acestep-lm");
    std::fs::write(root.join("models/smoke.gguf"), metadata)?;
    std::env::set_var("MUSIC_SANDBOX_DENIED_PATH", forbidden);
    std::env::set_var("GGML_BACKEND", "CPU");
    let exe = std::env::current_exe()?;
    let resources = exe.parent().unwrap().parent().unwrap().join("Resources");
    let settings = Settings {
        models_dir: root.join("models").to_string_lossy().into(),
        ..Settings::default()
    };
    let result = tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let engine = Engine::start(&resources, &root.join("logs"), &settings, &CancellationToken::new()).await?;
        drop(engine);
        let log = std::fs::read_to_string(root.join("logs/ace-server.log"))?;
        if !log.contains("[Desktop Sandbox] Unselected file access denied") { return Err(message("Engine sandbox enforcement was not confirmed")); }
        println!("App Sandbox: unselected read/write denied; inherited engine read denied; container storage, loopback health and supervisor shutdown passed.");
        Ok(())
    });
    let _ = std::fs::remove_dir_all(root);
    result
}
