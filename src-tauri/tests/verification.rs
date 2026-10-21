use music_generator::verification::{ModelVerifier, Verification};
use tokio_util::sync::CancellationToken;

const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

#[tokio::test]
async fn verified_models_are_reused_after_reopening_the_cache() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("model.gguf");
    std::fs::write(&file, b"abc").unwrap();
    let cancel = CancellationToken::new();
    let mut verifier = ModelVerifier::open(root.path());
    assert_eq!(
        verifier
            .verify(&file, ABC, &cancel, |_, _| {})
            .await
            .unwrap(),
        Verification::Hashed
    );
    let mut reopened = ModelVerifier::open(root.path());
    assert_eq!(
        reopened
            .verify(&file, ABC, &cancel, |_, _| panic!(
                "unchanged models must not be read again"
            ))
            .await
            .unwrap(),
        Verification::Cached
    );
}

#[tokio::test]
async fn changed_models_and_manifest_checksums_are_rechecked() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("model.gguf");
    std::fs::write(&file, b"abc").unwrap();
    let cancel = CancellationToken::new();
    let mut verifier = ModelVerifier::open(root.path());
    assert_eq!(
        verifier
            .verify(&file, ABC, &cancel, |_, _| {})
            .await
            .unwrap(),
        Verification::Hashed
    );
    assert_eq!(
        verifier
            .verify(&file, &"0".repeat(64), &cancel, |_, _| {})
            .await
            .unwrap(),
        Verification::Invalid
    );
    assert_eq!(
        verifier
            .verify(&file, ABC, &cancel, |_, _| {})
            .await
            .unwrap(),
        Verification::Hashed
    );
    std::fs::write(&file, b"abd").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(2)),
        )
        .unwrap();
    assert_eq!(
        ModelVerifier::open(root.path())
            .verify(&file, ABC, &cancel, |_, _| {})
            .await
            .unwrap(),
        Verification::Invalid
    );
}

#[tokio::test]
async fn cancellation_during_verification_does_not_cache_an_unfinished_check() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("model.gguf");
    std::fs::write(&file, b"abc").unwrap();
    let cancel = CancellationToken::new();
    let result = ModelVerifier::open(root.path())
        .verify(&file, ABC, &cancel, |done, total| {
            if done == total {
                cancel.cancel();
            }
        })
        .await;
    assert!(matches!(
        result,
        Err(music_generator::error::Error::Cancelled)
    ));
    assert_eq!(
        ModelVerifier::open(root.path())
            .verify(&file, ABC, &CancellationToken::new(), |_, _| {})
            .await
            .unwrap(),
        Verification::Hashed
    );
}

#[tokio::test]
async fn damaged_cache_is_rebuilt_with_visible_verification_progress() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("model.gguf");
    std::fs::write(&file, b"abc").unwrap();
    std::fs::write(root.path().join("verified-models.json"), b"broken").unwrap();
    let progress = std::sync::Mutex::new(Vec::new());
    assert_eq!(
        ModelVerifier::open(root.path())
            .verify(&file, ABC, &CancellationToken::new(), |done, total| {
                progress.lock().unwrap().push((done, total));
            })
            .await
            .unwrap(),
        Verification::Hashed
    );
    let progress = progress.into_inner().unwrap();
    assert_eq!(progress.first(), Some(&(0, 3)));
    assert_eq!(progress.last(), Some(&(3, 3)));
}
