use crate::error::{message, Result};
use serde_json::{json, Value};

pub async fn check() -> Result<Value> {
    let client = reqwest::Client::builder()
        .user_agent("music-generator-desktop")
        .timeout(std::time::Duration::from_secs(20))
        .build()?;
    let response = client
        .get("https://api.github.com/repos/frankhildebrandt/music-generator/releases/latest")
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(json!({"available":false,"current":env!("CARGO_PKG_VERSION")}));
    }
    let release: Value = response.error_for_status()?.json().await?;
    let latest = release["tag_name"]
        .as_str()
        .ok_or_else(|| message("Release has no version"))?
        .trim_start_matches('v');
    let version = semver::Version::parse(latest).map_err(|e| message(e.to_string()))?;
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    let suffix = asset_suffix(
        std::env::consts::OS,
        std::env::consts::ARCH,
        env!("MUSIC_GENERATOR_LINUX_BACKEND"),
    );
    let asset = release["assets"].as_array().and_then(|assets| {
        assets
            .iter()
            .find(|a| a["name"].as_str().is_some_and(|n| n.ends_with(suffix)))
    });
    let url = asset
        .and_then(|a| a["browser_download_url"].as_str())
        .filter(|u| {
            u.starts_with("https://github.com/frankhildebrandt/music-generator/releases/download/")
        });
    Ok(
        json!({"available":version>current && url.is_some(),"current":current.to_string(),"latest":latest,"url":url,"notes":release["body"]}),
    )
}

fn asset_suffix(os: &str, arch: &str, linux_backend: &str) -> &'static str {
    if os == "macos" {
        if arch == "x86_64" {
            "macos-x64.zip"
        } else {
            "macos-arm64.zip"
        }
    } else if os == "windows" {
        "windows-x64.zip"
    } else if linux_backend == "cuda" {
        "linux-x64-cuda.AppImage"
    } else {
        "linux-x64.AppImage"
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn updates_select_the_correct_architecture() {
        assert_eq!(
            super::asset_suffix("macos", "x86_64", "vulkan"),
            "macos-x64.zip"
        );
        assert_eq!(
            super::asset_suffix("macos", "aarch64", "cuda"),
            "macos-arm64.zip"
        );
        assert_eq!(
            super::asset_suffix("windows", "x86_64", "cuda"),
            "windows-x64.zip"
        );
        assert_eq!(
            super::asset_suffix("linux", "x86_64", "vulkan"),
            "linux-x64.AppImage"
        );
    }

    #[test]
    fn linux_updates_keep_the_installed_backend_when_both_assets_exist() {
        let names = [
            "Music-Generator-0.3.0-linux-x64.AppImage",
            "Music-Generator-0.3.0-linux-x64-cuda.AppImage",
        ];
        for backend in ["vulkan", "cuda"] {
            let suffix = super::asset_suffix("linux", "x86_64", backend);
            let matching: Vec<_> = names.iter().filter(|name| name.ends_with(suffix)).collect();
            assert_eq!(matching.len(), 1);
            assert_eq!(matching[0].contains("-cuda.AppImage"), backend == "cuda");
        }
    }
}
