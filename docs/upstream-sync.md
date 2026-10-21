# Upstream synchronization — 2026-10-22

Base: rabbitfiremediacreation/music-generator `main` at `91440ee0d3bebcb38ddf96430886d0d8cb0c18fc` (2026-10-21). This commit and its ancestors are merged into the desktop branch; the Python/browser changes are implemented through Rust/Tauri.

| Original changes | Desktop implementation |
| --- | --- |
| `0ef950d`: Automatic / CPU-only language model | SQLite `lm_device`, settings selector in twelve languages, separate CPU LM with accelerated synthesis, changes applied to the next operation |
| `d49f4d7`: explicit CPU device on newer Windows engines | Bounded/cancellable CLI capability probe; add `--device CPU` when supported and retain `GGML_BACKEND=CPU` for older pinned engines |
| `91440ee`: WAV / MP3 downloads and ZIP | Native dialogs, 192 kbit/s bundled converter, original WAV retained for playback, source-stamped cache and deletion cleanup, localized errors |
| `0cc2851`: allow slow first engine startup | Two-minute readiness budget per engine, shared across port retries, with cancellation and owned-process cleanup |
| Ubuntu/Arch/Windows/macOS installer fixes and usage reports | Desktop bundles continue replacing installers. Updated PDFs and READMEs remain explicitly historical; the new Ubuntu script is archived in `docs/legacy/` |

Existing settings migrate through defaults (`auto`, `wav`); the completed first-run flag remains authoritative. Automatic hardware-test fallback is separate from the explicit CPU preference, so selecting Auto does not erase a known CPU fallback. Model management preserves the manual device choice. The app still contains no Python runtime or frontend webserver; Node.js remains a build dependency only.

MP3 conversion uses the existing process supervisor, so closing/crashing the app terminates its own converter. Source audio is copied into temporary container storage before invoking the converter; the parent alone accesses external song/export locations. On macOS the converter receives the same two sandbox-inheritance entitlements as the engine/supervisor. ZIP/WAV exports and the existing audio playback path remain available independently of model startup.

Local checks: TypeScript/Vite, ESLint, six frontend tests, 35 Rust integration tests, Rustfmt and Clippy with warnings as errors. Windows omits the three Unix process/CLI tests (32 Rust tests). Native bundle verification now includes a real PCM-WAV-to-MP3 conversion; on macOS the packaged Rust export command, MP3 ZIP content, original WAV preservation, CPU split process and cache deletion are exercised inside App Sandbox for both ZIP and DMG. A timed-out device probe reports an error instead of silently treating a newer engine as an older one that honors only the backend environment variable.

Full GUI/inference on clean Windows/Linux machines, physical microphone and screenreader checks remain manual acceptance items as listed in `desktop.md`. The historical Python PDFs are not evidence for desktop behavior.
