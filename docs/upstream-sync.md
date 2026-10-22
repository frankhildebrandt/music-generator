# Upstream synchronization — 2026-10-22

Base: rabbitfiremediacreation/music-generator `main` at `eb19a988e31ab9072e9d0f5f02e158cf710a7e0b` (2026-10-22). This commit and its ancestors are merged into the desktop branch. Applicable Python/browser changes through `91440ee` are implemented through Rust/Tauri; the Intel-Mac follow-up is scoped below.

| Original changes | Desktop implementation |
| --- | --- |
| `0ef950d`: Automatic / CPU-only language model | SQLite `lm_device`, settings selector in twelve languages, separate CPU LM with accelerated synthesis, changes applied to the next operation |
| `d49f4d7`: explicit CPU device on newer Windows engines | Bounded/cancellable CLI capability probe; add `--device CPU` when supported and retain `GGML_BACKEND=CPU` for older pinned engines |
| `91440ee`: WAV / MP3 downloads and ZIP | Native dialogs, 192 kbit/s bundled converter, original WAV retained for playback, source-stamped cache and deletion cleanup, localized errors |
| `0cc2851`: allow slow first engine startup | Two-minute readiness budget per engine, shared across port retries, with cancellation and owned-process cleanup |
| Ubuntu/Arch/Windows/macOS installer fixes and usage reports | Desktop bundles continue replacing installers. Updated PDFs and READMEs remain explicitly historical; the new Ubuntu script is archived in `docs/legacy/` |
| `eb19a98`: Intel-Mac CPU synthesis, installer GPU-test bypass and Python log-based memory hint | Initial merge `30929e2` kept the Python backend and root installer/start scripts deleted, preserving the updated original README/PDFs and guide sources. Experimental desktop Intel support follows separately in 0.2.4 (below); the Python log-tail error hint is not introduced |

The `eb19a98` conflict resolution changes documentation only relative to desktop commit `0021f3c`. Application, engine, manifests and build configuration remain unchanged. The published `v0.2.3-test.1` artifacts still correspond to `0021f3c`; this documentation merge does not replace them.

## Intel desktop follow-up — 0.2.4

User-requested Intel support adds a separate native x64 DMG/ZIP targeting macOS 13.3+, with the same sandbox inheritance and native window controls. The pinned C++ engine is built without Metal, with portable CPU variants and no host-native instruction assumption. One CPU server handles language and synthesis, including old profiles requesting split LM; the assistant validates downloads and engine readiness without a GPU benchmark. Settings/assistant explain CPU performance in all twelve languages. Update download selection distinguishes x64 and arm64. GitHub Actions adds a native `macos-15-intel` job and package checks for every Mach-O architecture/deployment target, missing Metal backend, signatures, actual sandbox denial, health/shutdown and MP3/ZIP export. Engine recipe/cache hashes include the architecture policy. Older-system GUI/audio and real model inference remain manual acceptance items; the target version is not a claim of a macOS 13.3 runtime test. Existing 0.2.3 release files are not replaced.

Existing settings migrate through defaults (`auto`, `wav`); the completed first-run flag remains authoritative. Automatic hardware-test fallback is separate from the explicit CPU preference, so selecting Auto does not erase a known CPU fallback. Model management preserves the manual device choice. The app still contains no Python runtime or frontend webserver; Node.js remains a build dependency only.

MP3 conversion uses the existing process supervisor, so closing/crashing the app terminates its own converter. Source audio is copied into temporary container storage before invoking the converter; the parent alone accesses external song/export locations. On macOS the converter receives the same two sandbox-inheritance entitlements as the engine/supervisor. ZIP/WAV exports and the existing audio playback path remain available independently of model startup.

Local checks: TypeScript/Vite, ESLint, six frontend tests, 35 Rust integration tests, Rustfmt and Clippy with warnings as errors. Windows omits the three Unix process/CLI tests (32 Rust tests). Native bundle verification now includes a real PCM-WAV-to-MP3 conversion; on macOS the packaged Rust export command, MP3 ZIP content, original WAV preservation, CPU split process and cache deletion are exercised inside App Sandbox for both ZIP and DMG. A timed-out device probe reports an error instead of silently treating a newer engine as an older one that honors only the backend environment variable.

Full GUI/inference on clean Windows/Linux machines, physical microphone and screenreader checks remain manual acceptance items as listed in `desktop.md`. The historical Python PDFs are not evidence for desktop behavior.
