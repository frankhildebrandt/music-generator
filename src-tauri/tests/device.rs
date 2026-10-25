#![cfg(unix)]
use music_generator::engine::device::cpu_arguments;
use std::os::unix::fs::PermissionsExt;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn cpu_selection_supports_both_engine_cli_generations() {
    let root = tempfile::tempdir().unwrap();
    let binary = root.path().join("engine");
    std::fs::write(
        &binary,
        "#!/bin/sh\nprintf 'Usage: engine [--device NAME]\\n' >&2\nexit 1\n",
    )
    .unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        cpu_arguments(&binary, &CancellationToken::new())
            .await
            .unwrap(),
        ["--device", "CPU"]
    );
    std::fs::write(
        &binary,
        "#!/bin/sh\nprintf 'old engine --host --port\\n'\nexit 1\n",
    )
    .unwrap();
    assert!(cpu_arguments(&binary, &CancellationToken::new())
        .await
        .unwrap()
        .is_empty());
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(cpu_arguments(&binary, &cancel).await.is_err());
}

#[tokio::test]
async fn a_hanging_device_probe_cannot_silently_select_the_gpu() {
    let root = tempfile::tempdir().unwrap();
    let binary = root.path().join("engine");
    std::fs::write(&binary, "#!/bin/sh\nexec sleep 60\n").unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
    let error = cpu_arguments(&binary, &CancellationToken::new())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("device detection timed out"));
}
