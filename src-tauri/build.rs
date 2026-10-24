fn main() {
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        use sha2::{Digest, Sha256};
        let root = std::path::Path::new("resources/engine");
        let metadata: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("VERSION.desktop.json"))
                .expect("Engine missing: run npm run engine:prepare"),
        )
        .expect("Invalid engine provenance");
        let platform = match std::env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
            "macos" => "macos",
            "windows" => "windows",
            "linux" => "linux",
            _ => panic!("Unsupported desktop platform"),
        };
        assert_eq!(
            metadata["platform"], platform,
            "Engine platform does not match build target"
        );
        let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
        assert_eq!(
            metadata["arch"],
            if arch == "aarch64" { "arm64" } else { "x64" },
            "Engine architecture does not match build target"
        );
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../models/manifest.json"))
                .expect("Invalid model manifest");
        assert_eq!(
            metadata["revision"], manifest["engines"][platform]["revision"],
            "Engine revision does not match the pinned manifest"
        );
        let executable = if platform == "windows" {
            "ace-server.exe"
        } else {
            "ace-server"
        };
        let files = metadata["files"]
            .as_object()
            .expect("Engine file checksums missing");
        assert!(
            files.contains_key(executable),
            "Engine executable missing from manifest"
        );
        let heart: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("heartmula/HEART_RUNTIME.json"))
                .expect("HeartMuLa runtime missing: run npm run engine:prepare"),
        )
        .expect("Invalid HeartMuLa provenance");
        let pin: serde_json::Value =
            serde_json::from_str(include_str!("../models/heartmula-engine.json"))
                .expect("Invalid HeartMuLa pin");
        assert_eq!(
            heart["revision"], pin["revision"],
            "Wrong HeartMuLa runtime revision"
        );
        assert_eq!(heart["platform"], metadata["platform"]);
        assert_eq!(heart["arch"], metadata["arch"]);
        assert!(
            files.contains_key(if platform == "windows" {
                "heartmula/audiocpp_cli.exe"
            } else {
                "heartmula/audiocpp_cli"
            }),
            "HeartMuLa executable missing from resources"
        );
        println!("cargo:rerun-if-changed=../models/heartmula-engine.json");
        let title: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("title/TITLE_RUNTIME.json"))
                .expect("Title runtime missing: run npm run engine:prepare"),
        )
        .expect("Invalid title runtime provenance");
        let pin: serde_json::Value =
            serde_json::from_str(include_str!("../models/title-model.json"))
                .expect("Invalid title model pin");
        assert_eq!(title["revision"], pin["engine"]["revision"]);
        assert_eq!(title["platform"], metadata["platform"]);
        assert_eq!(title["arch"], metadata["arch"]);
        assert!(
            files.contains_key(if platform == "windows" {
                "title/music-title.exe"
            } else {
                "title/music-title"
            }),
            "Title executable missing from resources"
        );
        println!("cargo:rerun-if-changed=../models/title-model.json");
        for (name, checksum) in files {
            assert!(
                !name.split('/').any(|p| p == "..") && !std::path::Path::new(name).is_absolute(),
                "Invalid resource path"
            );
            let file = root.join(name);
            println!("cargo:rerun-if-changed={}", file.display());
            let digest = format!(
                "{:x}",
                Sha256::digest(std::fs::read(&file).expect("Missing engine resource"))
            );
            assert_eq!(
                Some(digest.as_str()),
                checksum.as_str(),
                "Engine resource was modified: {name}"
            );
        }
        println!("cargo:rerun-if-changed=resources/engine/VERSION.desktop.json");
    }
    tauri_build::build();
}
