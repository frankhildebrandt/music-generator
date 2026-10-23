# Desktop verification — 2026-10-21

Release 0.2.6 follow-up (2026-10-23, original main through `ad6c98d`): nullable SQLite `started_at` is added without rewriting stored parameter bytes or existing song timestamps. Queue claims set it atomically for each attempt; retries and restart recovery receive a new start when claimed. Import uses explicit column names, preserves existing starts and upgrades only the staged copy of an old source library. Fifteen frontend tests include generation-time display without queue waiting, old-song fallback and invalid timestamps; 47 Rust tests include migration, queue retry/resume and imports with/without start times. Existing saved-value and timeout regressions pass. TypeScript/Vite, ESLint, Node build-policy, Rustfmt and Clippy also pass locally. Published 0.2.5 bundles remain unchanged; this source prepares a new 0.2.6 test release.

Timeout-message follow-up (2026-10-23, `b5b1670`): Rust reports the timed-out engine phase, rounded elapsed minutes and guidance to raise Timeout in Settings. The engine is still cancelled on timeout, while user cancellation retains its own error. A fresh synthesis memory log cannot replace the timeout guidance. Tests cover minute formatting/serialization, actual HTTP job cancellation and full stored parameters after error persistence, SQLite restart and retry. Frontend cases cover old/new German and English timeout messages, unrelated failures, and settings guidance in all twelve translations. No schema, stored generation parameters, request defaults or timeout setting/default changes. Published 0.2.5 assets remain unchanged.

Local checks passed: TypeScript/Vite, ESLint, twelve frontend tests, one Node build-policy test, 44 Rust tests, Rustfmt and Clippy with warnings as errors. All 60 timeout/length/sampling strings match current upstream exactly; the existing old/custom/new reuse and SQLite restart/error/retry regressions still pass.

The settings path applies validated poll/timeout durations to the cached client rather than replacing it, so saving a higher timeout preserves synthesis-log diagnostics. Device changes retain their existing engine-restart behavior.

0.2.5 ten-minute follow-up (2026-10-23, `0e64bc7`): the duration slider now reaches 600 seconds with a localized warning above five minutes. Rust limits LM overproduction to requested duration × 5 and at most 3000 codes before synthesis, leaving saved parameters unchanged. New tests cover code bounds and a real LM → synthesis mock contract, plus fresh-log memory explanations for failed jobs, HTTP errors and missing audio while ignoring earlier errors. Unit tests cover missing/truncated logs and length-error precedence. stdout/stderr share the same owned-engine log on all desktop platforms. Local TypeScript/Vite, ESLint, ten frontend tests, one Node build-policy test, 43 Rust tests, Rustfmt and Clippy passed; all 48 localized length/sampling strings match the original main. Real ten-minute inference on Intel or 8 GiB systems remains untested.

Upstream defaults follow-up (2026-10-23, `3374648`): fresh forms use 80 iterations and variance 1.2; saved song parameters and legacy model defaults remain unchanged. Ten frontend tests include explicit old/custom/new song reuse, omitted/sentinel fallback, fresh HTML values and all twelve localized tooltips while retaining desktop MP3/device messages. A new SQLite regression verifies complete parameter objects survive restart and a real error → retry → queued execution cycle alongside newly queued 80/1.2 songs. Backend request defaults, schema and storage code are unchanged. The previously published 0.2.4 test-release binaries are not replaced by this merge.

Intel desktop follow-up (0.2.4, 2026-10-22):

- Separate native x64 DMG/ZIP with a macOS 13.3 deployment target, CPU-only pinned engine, portable runtime-selected CPU variants, sandbox inheritance and native window controls. A `macos-15-intel` CI job builds/tests the Intel packages alongside the existing three platform jobs.
- Local checks passed: TypeScript/Vite, ESLint, six frontend tests, one Node build-policy test, 37 Rust tests, Rustfmt and Clippy with warnings as errors. The new Rust cases verify that Intel uses one CPU server even with saved split/CPU preferences, and that update assets are selected by architecture.
- Bundle verification additionally checks every Mach-O architecture/deployment target, the app's minimum-OS plist value and absence of a Metal backend in Intel packages. The packaged sandbox probe checks a shared CPU LM/synthesis server on Intel, while retaining the separate-CPU-LM assertion on Apple Silicon. Existing actual sandbox enforcement and WAV/MP3/ZIP checks apply to both architectures.
- [Native bundle verification run 39850148957](https://github.com/frankhildebrandt/music-generator/actions/runs/39850148957), source `355746fc126098b61dab89dafba596320c1f819e`: Intel macOS 15 passed all 37 Rust tests, frontend/build-policy tests, Clippy and the final ZIP and DMG probes. Both Intel distributions passed Mach-O architecture/deployment-target checks, plist minimum version 13.3, no Metal backend, strict signatures, actual main/engine sandbox denial, shared CPU server health/shutdown, MP3 conversion/ZIP and cache deletion. The other three platform packages passed their native verification too. The [Intel DMG/ZIP artifact](https://github.com/frankhildebrandt/music-generator/actions/runs/39850148957/artifacts/12459467664) includes both SHA-256 files. The downloaded Intel files passed SHA-256 locally, ZIP archive integrity and `hdiutil verify`.
- This host is Apple Silicon; no local Intel hardware inference or macOS 13.3 GUI/audio acceptance is claimed. CPU setup checks engine startup/health and skips the GPU correctness benchmark; it does not perform music inference. Older Intel machines can take many minutes per song. Existing 0.2.3 release artifacts are unchanged.

Upstream synchronization (0.2.3, 2026-10-22): see [upstream-sync.md](upstream-sync.md) for the original commits, Rust/Tauri adaptations and new verification coverage. Local checks pass: 35 Rust tests, six frontend tests, TypeScript/Vite, ESLint, Rustfmt and Clippy. Earlier sections below describe the corresponding historical test versions.

Persistent first-run completion (0.2.2):

- Startup now trusts the existing SQLite `setup_complete` flag. It neither scans model files nor waits for the engine. Existing completed installations need no migration or repeated hardware test.
- Generation/analysis starts the engine on demand and validates model availability/integrity then. Pending jobs resume through the existing worker. The health command treats a configured, idle app as healthy rather than incorrectly showing it as offline.
- Completion is committed only after successful folder scoping and a final cancellation check. Failures during later model use do not erase a previously completed setup or prevent access to the library.
- A real SQLite restart regression verifies that unfinished setup remains unfinished, completed setup restores readiness without models or engine resources, no engine is started, and missing models are reported on first use while completion stays saved.
- Local TypeScript/Vite, ESLint, three frontend tests, 30 Rust integration tests, Rustfmt and Clippy passed. No new end-to-end GUI startup timing is claimed here.

Setup verification optimization (0.2.1):

- On the same M5 Pro, checking the same five real models (12.51 GiB, including 4B and XL) with an empty verification cache took 26.351 seconds before and 6.444 seconds after enabling ARM SHA-256 acceleration and increasing the read buffer. This is about 4.1× faster (76% less time). The OS file cache was not cleared; these figures measure model verification, not engine startup or the language-model self-test.
- Reusing the existing 0.2.0 verification cache took 0.813 ms after the change. Successful downloads and checks now populate the same persistent cache immediately, including after promoting a `.part` file. Subsequent setup attempts and launches no longer hash those files again while their canonical path, checksum, length and modification time match.
- Verification progress now reports actual bytes, including during startup, and engine startup is displayed separately. Startup phase timings are written to the desktop log. Hardware detection refreshes RAM only instead of collecting process and CPU statistics.
- Local TypeScript/Vite, ESLint, three frontend tests, 29 Rust integration tests, Rustfmt and Clippy passed. Added regressions cover persistent reuse, changed content/checksums, a damaged cache, cancellation during verification and reuse of downloaded files without another checksum pass.
- Portable-bundle verification selects the current application version explicitly, so an older package in `release/` cannot accidentally satisfy the check.

Local environment: macOS 26.6.2, Apple M5 Pro, 64 GiB RAM; application version 0.2.0.

App Sandbox and DMG follow-up:

- [Successful native Mac CI job](https://github.com/frankhildebrandt/music-generator/actions/runs/39598450971/job/119216542103), source commit `d2299c8`: ZIP and DMG both pass their sandbox and packaged-engine checks.
- The [complete native CI run](https://github.com/frankhildebrandt/music-generator/actions/runs/39598450971) is green for macOS, Windows and Linux. The new import case brings Rust integration coverage to 25 tests on macOS/Linux and 24 on Windows, plus the three frontend tests on every platform.
- Packaged Mac app enables App Sandbox; supervisor and engine use the two inheritance entitlements. There are no blanket filesystem exceptions.
- Local ZIP and read-only DMG checks passed: SHA-256, strict/deep signature verification, engine resource hashes, Applications shortcut, copying the app out of the DMG and ejecting the image.
- Actual OS enforcement passed in both distributions: main app cannot read an unselected file or write into its directory; the inherited engine cannot read that file either. Container storage, loopback health and supervisor shutdown work.
- TypeScript/Vite, ESLint, three frontend tests, 25 Rust integration tests and Clippy pass locally. The added import case preserves an earlier unsandboxed desktop profile and its model location.
- The sandboxed GUI imported the existing three-song desktop library through the native folder dialog. The originals remain in the previous profile; models are read from the authorized external folder without copying their large files.
- The 4B Metal hardware self-test passed. Restarting the app then restored the folder grants and reached the studio without selecting folders again; a real short music generation succeeded after that restart with models outside the sandbox container.
- The downloaded CI DMG passed SHA-256 and `hdiutil verify`; the corresponding ZIP passed SHA-256 and archive integrity checks. Developer ID signing and notarization are not claimed for these ad-hoc test builds.

Passed automated checks:

- TypeScript checking, Vite production build, ESLint.
- 3 frontend command-contract tests.
- 24 Rust integration tests covering downloads, engine contracts, process supervision, library and legacy import.
- Rust formatting and Clippy with warnings treated as errors.
- Native macOS release build with bundled engine, engine checksum verification, ZIP and SHA-256 generation.

Native GitHub Actions verification:

- [Successful run 39573303203](https://github.com/frankhildebrandt/music-generator/actions/runs/39573303203), source commit `82ddf1f`, all three native jobs green.
- macOS 26 / Apple Silicon: ZIP containing the `.app`, native window traffic lights and bundled Metal/CPU engine; valid ad-hoc bundle signature.
- Windows Server 2022 / x64: portable ZIP containing the EXE, CUDA/Vulkan/CPU engine, app-local Visual C++ DLLs and pinned Fixed Version WebView2 runtime.
- Ubuntu 22.04 / x64: AppImage containing the Vulkan/CPU engine and media framework. A pinned Vulkan SDK supplies the shader compiler. Final packaging with checksummed appimagetool 1.9.1 preserves the original verified engine files after linuxdeploy's ELF processing.
- Each job passed TypeScript/Vite, ESLint, three frontend tests, Rust formatting, integration tests and Clippy with warnings treated as errors. Windows runs 23 Rust tests; macOS/Linux also run the Unix process-supervisor test, for 24.
- Each final artifact passed SHA-256 verification, extraction into a relocated directory with spaces, all bundled engine-file checksums, packaged supervisor/engine startup in CPU mode, HTTP health and shutdown on control-pipe closure. The smoke test uses a tiny metadata-only GGUF fixture; it does not perform inference.
- All three downloaded artifacts passed SHA-256 checks locally. Windows and macOS ZIPs also passed ZIP-integrity checks; the downloaded Mac app's ad-hoc signature passed strict/deep verification.

Passed in the actual `.app` bundle:

- First-run assistant, hardware detection, checksummed downloads of the base models, optional XL and 4B language model.
- Accelerated 4B language-model self-test: 50 audio codes for the ten-second test.
- A real ten-second instrumental generated with standard SFT and the 4B LM; finished in approximately sixteen seconds.
- Waveform display and playback with advancing play time.
- Native WAV save dialog; exported file SHA-256 matches the source audio.
- Local analysis of the exported WAV; caption, tempo, key and style populated the form.
- App close terminates its engine/supervisor; reopening preserves the library and reuses models.
- Launch from a copied/moved `.app` directory preserves the library and reaches the studio without downloading models again.
- Final ZIP integrity, SHA-256 verification and the ad-hoc macOS bundle signature pass.
- Responsive layout at 480 px width, approximately 1013×560, and maximized; full client width, stacked layout when narrow, independent panes when wide, accessible library after scrolling.
- Native macOS close/full-screen/minimize controls are present and accessible; custom right-side controls are hidden on Mac. Restoring old saved window state no longer removes the native decorations.

Not verified on this host:

- Full Windows/Linux GUI and inference acceptance on clean machines without development tools, including current Ubuntu and Arch. Native CI builds and packaged-engine smoke tests have passed, but do not replace those checks.
- GPU/driver combinations other than this Mac; CPU fallback is covered by contract/self-test logic, not a hardware run on this host.
- Physical microphone recording, actual VoiceOver/NVDA/JAWS operation and notarization/Developer ID signing.

The existing browser-installation PDFs are historical and are not evidence for the desktop app. Follow the release acceptance checklist in desktop.md before publishing a supported cross-platform release.
