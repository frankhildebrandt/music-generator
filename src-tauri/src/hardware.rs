use serde::Serialize;
use std::process::Command;
use sysinfo::{MemoryRefreshKind, System};

#[derive(Debug, Clone, Serialize)]
pub struct Hardware {
    pub ram_bytes: u64,
    pub gpu: String,
    pub vram_bytes: Option<u64>,
    pub recommended_lm: String,
    pub vae_chunk: Option<u32>,
    pub platform: &'static str,
}

pub fn recommend(ram: u64, vram: Option<u64>) -> (&'static str, Option<u32>) {
    let gb = 1024 * 1024 * 1024;
    let ram_gb = ram / gb;
    let ram_size = if ram_gb < 12 {
        0
    } else if ram_gb < 24 {
        1
    } else {
        2
    };
    let gpu_size = vram
        .map(|v| {
            if v / gb < 6 {
                0
            } else if v / gb < 8 {
                1
            } else {
                2
            }
        })
        .unwrap_or(2);
    let size = ["0.6B", "1.7B", "4B"][ram_size.min(gpu_size)];
    let chunk = if cfg!(target_os = "macos") {
        if ram_gb < 12 {
            Some(256)
        } else if ram_gb < 16 {
            Some(512)
        } else {
            None
        }
    } else if let Some(v) = vram {
        if v / gb < 8 {
            Some(128)
        } else if v / gb < 12 {
            Some(256)
        } else if v / gb < 16 {
            Some(512)
        } else {
            None
        }
    } else {
        Some(256)
    };
    (size, chunk)
}

pub fn inspect() -> Hardware {
    let mut sys = System::new();
    sys.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    let ram = sys.total_memory();
    let (gpu, vram) = adapter();
    let (lm, chunk) = recommend(ram, vram);
    Hardware {
        ram_bytes: ram,
        gpu,
        vram_bytes: vram,
        recommended_lm: lm.into(),
        vae_chunk: chunk,
        platform: std::env::consts::OS,
    }
}

fn output(program: &str, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let output = cmd.output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn adapter() -> (String, Option<u64>) {
    if cfg!(target_os = "macos") {
        let chip = output("/usr/sbin/sysctl", &["-n", "machdep.cpu.brand_string"])
            .unwrap_or_else(|| "Apple Silicon".into());
        return (format!("{chip} · Metal / CPU"), None);
    }
    if let Some(text) = output(
        "nvidia-smi",
        &[
            "--query-gpu=name,memory.total",
            "--format=csv,noheader,nounits",
        ],
    ) {
        if let Some((name, memory)) = text.lines().next().and_then(|l| l.rsplit_once(',')) {
            return (
                format!("{} · CUDA / Vulkan / CPU", name.trim()),
                memory.trim().parse::<u64>().ok().map(|m| m * 1024 * 1024),
            );
        }
    }
    #[cfg(windows)]
    {
        if let Some(name) = output(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "(Get-CimInstance Win32_VideoController).Name -join ', '",
            ],
        ) {
            return (format!("{name} · Vulkan / CPU"), None);
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                if let Ok(bytes) =
                    std::fs::read_to_string(entry.path().join("device/mem_info_vram_total"))
                {
                    return ("AMD · Vulkan / CPU".into(), bytes.trim().parse().ok());
                }
            }
        }
    }
    ("Vulkan if available; CPU fallback".into(), None)
}
