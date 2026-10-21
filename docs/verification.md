# Desktop verification — 2026-10-21

Local environment: macOS 26.6.2, Apple M5 Pro, 64 GiB RAM; application version 0.2.0.

Passed automated checks:

- TypeScript checking, Vite production build, ESLint.
- 3 frontend command-contract tests.
- 24 Rust integration tests covering downloads, engine contracts, process supervision, library and legacy import.
- Rust formatting and Clippy with warnings treated as errors.
- Native macOS release build with bundled engine, engine checksum verification, ZIP and SHA-256 generation.

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

Not verified on this host:

- Native Windows and Linux bundles and clean-machine runtime checks. The build pipelines are supplied but have not run here.
- GPU/driver combinations other than this Mac; CPU fallback is covered by contract/self-test logic, not a hardware run on this host.
- Physical microphone recording, actual VoiceOver/NVDA/JAWS operation and notarization/Developer ID signing.

The existing browser-installation PDFs are historical and are not evidence for the desktop app. Follow the release acceptance checklist in desktop.md before publishing a supported cross-platform release.
