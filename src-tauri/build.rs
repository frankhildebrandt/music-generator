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
