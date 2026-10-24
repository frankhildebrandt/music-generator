# Music Generator Desktop

## Start

Download the bundle for your platform from https://github.com/frankhildebrandt/music-generator/releases:

| Platform | Bundle | Requirements |
| --- | --- | --- |
| macOS | `Music-Generator-VERSION-macos-arm64.dmg` or `.zip` | Apple Silicon, macOS 26+ |
| macOS Intel (experimental) | `Music-Generator-VERSION-macos-x64.dmg` or `.zip` | Intel x64, macOS 13.3+, at least 8 GiB RAM; CPU only |
| Windows | `Music-Generator-VERSION-windows-x64.zip` | Windows 10/11 x64, current GPU driver |
| Linux | `Music-Generator-VERSION-linux-x64.AppImage` | x64; Ubuntu 22.04 or compatible newer distribution, current GPU driver |

Extract the complete ZIP and open Music Generator. Keep the Windows EXE, `engine` and `WebView2` directories together. On Linux run `chmod +x Music-Generator-*.AppImage` once and open the AppImage. If FUSE is unavailable, run `./Music-Generator-VERSION-linux-x64.AppImage --appimage-extract`, then `./squashfs-root/AppRun`.

On macOS you can instead open the DMG and drag Music Generator onto its Applications shortcut (or copy it into another folder). Eject the disk image and open the copied app. The DMG contains the same sealed `.app` as the ZIP and does not run an installer. Both downloads include a SHA-256 file. CI verifies the read-only disk image, copies its app into a relocated folder, ejects the image and tests the copied app's signature, engine files, engine startup and shutdown.

macOS uses its native red/yellow/green window controls on the left. Windows and Linux use the custom minimize/maximize/close controls on the right. The title bar is draggable, and the layout fills the window at every supported size.

Intel Macs use a separate native x64 bundle; no Rosetta, Python, Node.js or local compilation is needed. Both language-model inference and synthesis run on the CPU, even when an old profile requests a separate CPU language model. One engine process handles both tasks to avoid duplicating model memory. The Intel engine excludes Metal and includes runtime-selected portable CPU variants rather than instructions tied to the build host. macOS 13.3 is the deployment target (including Accelerate's newer LAPACK API); native CI runs on macOS 15. Actual generation, GUI/audio and minimum-OS acceptance remain manual checks: this is experimental support, not a claim of tests on every Intel Mac or on macOS 13.3.

Expect many minutes per song on older Intel CPUs. Start with a ten-second song, the 0.6B language model and standard sound model; XL remains optional and needs more memory/time. Setup verifies model checksums and engine startup/health, skips the inapplicable GPU benchmark and saves completion normally. It does not claim to have generated a song during setup. CPU performance guidance appears in the assistant and settings in all twelve languages.

These first builds are unsigned test builds. macOS Gatekeeper and Windows SmartScreen can display warnings. There is no installer or requirement to install Python, Node.js, Git, compilers or package managers. GPU drivers are supplied by the operating system/vendor, not by this app. Windows ships WebView2 app-locally. On Windows 10 the app applies Microsoft's required read/execute permissions to that runtime directory; application data is not included in this permission change. Extract onto a local writable drive, not a network share.

## First-run assistant

1. Choose the interface language. The app keeps all twelve existing languages.
2. Optionally select an old Music Generator project directory to copy its library and songs. Import requires an empty library. The source database is opened read-only and backed up through SQLite, including committed WAL data; original songs are preserved. There is no library merge.
3. Review RAM/GPU information, language model recommendation and folders. A different model directory can be chosen, including an old installation's model folder. Existing model files must pass SHA-256 verification. A damaged existing final model is preserved: remove that damaged file manually or select a fresh folder, then retry.
4. Choose 0.6B, 1.7B or 4B. XL is optional and downloads in addition to the standard sound model. Download and free-space figures are estimates; the downloader checks actual response size and disk space again before writing.
5. Download and run the hardware test. Downloads can be cancelled and resumed. The app does not activate incomplete or unverified files. If the accelerated language model fails its audio-code test, a separate CPU language-model process is tested and used.

The first setup needs several GB and internet access. Model URLs, immutable revisions and SHA-256 checksums are in `models/manifest.json`. Generation, recording, analysis and playback are local after setup. Update checks only contact GitHub when requested.

The assistant saves its completion status in SQLite only after downloads, integrity checks, the hardware test and folder access have succeeded. Later launches use that saved result and open the studio directly, without a model scan or waiting for the engine. Existing completed installations use this behavior without repeating setup. The engine starts when generation or analysis needs it; pending jobs also resume in the background. Models are checked at that point, using their saved checksum results where possible. Missing or invalid models report an error directing you to model management; the library remains accessible. Repeat setup and hardware testing explicitly through Settings → Models and hardware test.

Settings → Language model runs on offers Automatic and CPU only. The choice is saved in SQLite and survives setup, restarts and bundle replacement. CPU only keeps synthesis on the accelerated engine and starts a separate CPU language-model process; Automatic still respects a CPU fallback detected by the hardware test. Changes take effect at the next generation/analysis without an app restart. Newer engines receive `--device CPU` when supported; older engines retain `GGML_BACKEND=CPU`. Engine readiness allows up to two minutes, with cancellation and process supervision intact.

Settings → Export selects lossless WAV or MP3 at 192 kbit/s for both individual songs and ZIPs. Playback and the library keep their original audio. MP3 uses the bundled `mp3-codec` through the same owned-process supervisor. The parent copies source audio into temporary container storage before conversion; no external folder grants are needed by the converter. Converted copies are cached by source size/modification time and deleted with their song. Incomplete conversions never replace originals or existing export destinations. If conversion fails, choose WAV and inspect `logs/export.log`. The native save dialog uses the selected extension. All twelve languages include the upstream labels and conversion errors, with desktop-specific device-change instructions.

## Data and updates

Songs can be 10 seconds to 10 minutes long. Above five minutes the form shows a localized time/memory warning. The language model may occasionally return extra audio codes; the outgoing synthesis payload is capped at the requested duration × 5 and at 3000 codes (600 seconds). This does not rewrite saved song parameters. Synthesis failures inspect only new lines in the combined stdout/stderr engine log; memory exhaustion and the model's maximum-length error get specific explanations. Earlier jobs' memory errors are ignored. Ten-minute generation remains untested on Intel Macs and 8 GiB machines; start with a short song on those systems.

Since 0.2.7, new generations normalize to the actual audio peak instead of the engine's percentile clipping. Rust sends `peak_clip: 0` to both generation stages and reapplies it after LM enrichment. Existing library audio and stored parameters are retained. This addresses that normalization step; it is not a claim that every possible audible distortion has been eliminated.

- macOS: `~/Library/Containers/de.alphatester.music-generator/Data/Library/Application Support/de.alphatester.music-generator/`
- Windows: `%LOCALAPPDATA%\de.alphatester.music-generator\`
- Linux: `$XDG_DATA_HOME/de.alphatester.music-generator/` (normally `~/.local/share/…`)

The database is `music.db`. Models and songs default to subdirectories there; their locations are selectable. Logs are in `logs/desktop.log`, `logs/ace-server.log`, and, in split mode, `logs/ace-lm.log`.

Packaged macOS builds enable Apple's App Sandbox. Network client/server entitlements permit model downloads, update checks and the internal loopback engine connection; microphone and user-selected file access are enabled separately. There are no blanket home-directory or temporary-exception grants. Pick external model/song folders through Browse so macOS can authorize them. Security-scoped bookmarks restore those grants after restarting. The engine receives a transferable bookmark for its model folder and runs under the inherited sandbox through a dedicated supervisor. Both child executables carry only the sandbox and inheritance entitlements.

The current ad-hoc test signatures have no stable Developer ID identity. Replacing the app with a different test build can require selecting external folders again; model and song files remain intact. Folder grants survive restarting the same build, which has been tested. Stable signing and notarization remain a separate release step.

To import an existing library, use **Import existing library** in an empty installation and select the directory containing `music.db` through the native folder dialog.

Import copies SQLite/WAL data and songs, keeping original song parameters and source files. It does not merge populated libraries or change the previous installation. Existing models can be reused after checksum verification; authorize external model/song folders again through the native Browse dialog when required. The new app cannot silently inherit another identifier's sandbox permissions. Legacy browser-project imports still use the same button.

The studio and Settings include a [ModelPicker](model-picker.md) for 52 ACE-Step synthesis weights and eight language weights, with a RAM-budget filter and verified resumable downloads. [HeartMuLa](heartmula.md) adds two native integrated model/codec choices with its own parameters, downloads and local generation. New jobs retain a model snapshot; existing songs keep their saved values.

The app does not write into the program bundle. Replace the complete app/bundle for updates; models and library remain available. Setup → Version checks releases in this fork and opens the platform's new bundle. The app does not perform Git pulls or replace its own running files. Setup → Models and hardware test can download another model or repeat the test. Changing the song folder copies files first and switches the saved path only after successful copying; queued/running songs must finish or be cancelled before moving.

Closing while work is pending asks for confirmation. Running jobs and pending jobs are resumed on the next start. Cancelling a song explicitly leaves it cancelled until Retry. A control-pipe supervisor terminates the owned engine when the GUI exits, including an unexpected GUI crash. Other programs/processes are never stopped based on a port number.

If an engine job exceeds its time limit, the app cancels that job and reports the phase and elapsed minutes. For long songs, raise Timeout in Settings (gear); the existing default remains 30 minutes per engine operation. The song-list summary gives this guidance in all twelve languages, including for older stored German/English timeout messages. Retry retains the song's saved generation parameters; updating the app does not change them or overwrite your timeout setting.

Since 0.2.6, the creation-time display measures each attempt from queue claim to completion, excluding waiting behind earlier variants. An additive nullable `started_at` column stores the start; old completed songs retain their previous display using the existing creation timestamp. Retry and restart recovery record a fresh start when the job is claimed. Existing generation parameters and timestamps are preserved during migration. Import supports both schema versions without changing the original library.

## Build

Development needs Node.js 24+, stable Rust, and Tauri's native build prerequisites. End users do not need these tools.

```sh
npm ci
npm run engine:prepare
npm run dev
```

The development app embeds built frontend assets, so it works without a development webserver. Restart `npm run dev` after changing frontend sources. For production bundles:

```sh
npm run lint
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run bundle
```

`release/` contains the platform bundle and its SHA-256 file. `npm run bundle` verifies/prepares the engine first. The Rust release build also refuses missing, mismatched or modified engine resources. `npm run build` builds only the frontend.

The preparation script builds the pinned Mac/Linux engine source revisions and checks the pinned Windows archive hash. The Mac revision adds the small `engine-sandbox.mm` bookmark bridge; its patch and build-recipe hashes are recorded alongside runtime-file hashes. Mac CPU code targets portable Apple Silicon or Intel variants and uses relative library paths. The app, supervisor and engine are signed from the inside out with separate parent/child entitlements. Linux builds Vulkan and CPU variants; the CI image installs build dependencies, users do not. Windows includes app-local Visual C++ runtime DLLs from the build tools' redistributable directory and a Fixed Version WebView2 runtime pinned in `models/webview2.json`. Updating that manifest requires downloading the official Microsoft CAB and verifying its checksum. Ship a new bundle to update the fixed browser runtime.

The `Desktop bundles` GitHub Actions workflow builds each platform natively and uploads artifacts. A version tag, manual dispatch or push to `codex/desktop-bundles` triggers it; it does not automatically publish a GitHub Release. Linux uses a checksummed Vulkan SDK 1.4.309.0 for its shader compiler, since Ubuntu 22.04 does not provide `glslc` in its standard package archive.

After packaging, `node scripts/desktop/verify-bundle.mjs` checks the artifact checksum, extracts it into a different directory containing spaces, verifies all engine resource hashes and starts the packaged supervisor/engine in CPU mode. A tiny tensor-free GGUF fixture permits an HTTP health check without model downloads or inference. The test also checks shutdown when the supervisor control pipe closes, plus the Windows runtime files. macOS additionally checks signing entitlements and actual OS denial of unselected file reads/writes by the main app and a read by the inherited engine. It repeats those checks after copying the app from a read-only DMG and ejecting it (`--dmg`). These checks do not generate music.

Attach the tested artifacts and their SHA-256 files to a release tagged `vVERSION` for in-app update discovery. The version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` must match.

For signed macOS releases configure the standard Tauri secrets `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`. Keep certificates and passwords out of source control. Windows signing can be added to the portable packaging step once a certificate is available; current Windows builds are unsigned.

## Release acceptance

Automated tests cover serial queue behavior, stable seeds/variants, cancellation against late results, retry/restart, folders/favorites, lookup beyond 500 visible rows, imports with WAL and import rollback, path validation, HTTP-Range continuation/restart, checksum rejection, disk-space rejection, engine JSON/multipart results, timeout/cancellation and supervisor shutdown.

Before calling a platform release supported, test the actual bundle on a clean system without development tools: setup, at least one real generated song, file analysis, microphone permission/recording, waveform playback, WAV/TXT/ZIP exports, keyboard and screenreader operation, close during a job, reopen, offline operation and moving/replacing the program bundle. Linux needs additional current Ubuntu and Arch smoke tests. Windows needs a test with no preinstalled WebView2/VC++ runtime. CI compilation alone is not equivalent to these checks.
