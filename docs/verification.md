# Desktop verification — 2026-10-21

Local environment: macOS 26.6.2, Apple M5 Pro, 64 GiB RAM; application version 0.2.0.

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
