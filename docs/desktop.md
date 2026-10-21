# Music Generator Desktop

## Start

Download the bundle for your platform from https://github.com/frankhildebrandt/music-generator/releases:

| Platform | Bundle | Requirements |
| --- | --- | --- |
| macOS | `Music-Generator-VERSION-macos-arm64.dmg` or `.zip` | Apple Silicon, macOS 26+ |
| Windows | `Music-Generator-VERSION-windows-x64.zip` | Windows 10/11 x64, current GPU driver |
| Linux | `Music-Generator-VERSION-linux-x64.AppImage` | x64; Ubuntu 22.04 or compatible newer distribution, current GPU driver |

Extract the complete ZIP and open Music Generator. Keep the Windows EXE, `engine` and `WebView2` directories together. On Linux run `chmod +x Music-Generator-*.AppImage` once and open the AppImage. If FUSE is unavailable, run `./Music-Generator-VERSION-linux-x64.AppImage --appimage-extract`, then `./squashfs-root/AppRun`.

On macOS you can instead open the DMG and drag Music Generator onto its Applications shortcut (or copy it into another folder). Eject the disk image and open the copied app. The DMG contains the same sealed `.app` as the ZIP and does not run an installer. Both downloads include a SHA-256 file. CI verifies the read-only disk image, copies its app into a relocated folder, ejects the image and tests the copied app's signature, engine files, engine startup and shutdown.

macOS uses its native red/yellow/green window controls on the left. Windows and Linux use the custom minimize/maximize/close controls on the right. The title bar is draggable, and the layout fills the window at every supported size.

These first builds are unsigned test builds. macOS Gatekeeper and Windows SmartScreen can display warnings. There is no installer or requirement to install Python, Node.js, Git, compilers or package managers. GPU drivers are supplied by the operating system/vendor, not by this app. Windows ships WebView2 app-locally. On Windows 10 the app applies Microsoft's required read/execute permissions to that runtime directory; application data is not included in this permission change. Extract onto a local writable drive, not a network share.

## First-run assistant

1. Choose the interface language. The app keeps all twelve existing languages.
2. Optionally select an old Music Generator project directory to copy its library and songs. Import requires an empty library. The source database is opened read-only and backed up through SQLite, including committed WAL data; original songs are preserved. There is no library merge.
3. Review RAM/GPU information, language model recommendation and folders. A different model directory can be chosen, including an old installation's model folder. Existing model files must pass SHA-256 verification. A damaged existing final model is preserved: remove that damaged file manually or select a fresh folder, then retry.
4. Choose 0.6B, 1.7B or 4B. XL is optional and downloads in addition to the standard sound model. Download and free-space figures are estimates; the downloader checks actual response size and disk space again before writing.
5. Download and run the hardware test. Downloads can be cancelled and resumed. The app does not activate incomplete or unverified files. If the accelerated language model fails its audio-code test, a separate CPU language-model process is tested and used.

The first setup needs several GB and internet access. Model URLs, immutable revisions and SHA-256 checksums are in `models/manifest.json`. Generation, recording, analysis and playback are local after setup. Update checks only contact GitHub when requested.

The assistant saves its completion status in SQLite only after downloads, integrity checks, the hardware test and folder access have succeeded. Later launches use that saved result and open the studio directly, without a model scan or waiting for the engine. Existing completed installations use this behavior without repeating setup. The engine starts when generation or analysis needs it; pending jobs also resume in the background. Models are checked at that point, using their saved checksum results where possible. Missing or invalid models report an error directing you to model management; the library remains accessible. Repeat setup and hardware testing explicitly through Settings → Models and hardware test.

## Data and updates

- macOS: `~/Library/Containers/de.alphatester.music-generator/Data/Library/Application Support/de.alphatester.music-generator/`
- Windows: `%LOCALAPPDATA%\de.alphatester.music-generator\`
- Linux: `$XDG_DATA_HOME/de.alphatester.music-generator/` (normally `~/.local/share/…`)

The database is `music.db`. Models and songs default to subdirectories there; their locations are selectable. Logs are in `logs/desktop.log`, `logs/ace-server.log`, and, in split mode, `logs/ace-lm.log`.

Packaged macOS builds enable Apple's App Sandbox. Network client/server entitlements permit model downloads, update checks and the internal loopback engine connection; microphone and user-selected file access are enabled separately. There are no blanket home-directory or temporary-exception grants. Pick external model/song folders through Browse so macOS can authorize them. Security-scoped bookmarks restore those grants after restarting. The engine receives a transferable bookmark for its model folder and runs under the inherited sandbox through a dedicated supervisor. Both child executables carry only the sandbox and inheritance entitlements.

The current ad-hoc test signatures have no stable Developer ID identity. Replacing the app with a different test build can require selecting external folders again; model and song files remain intact. Folder grants survive restarting the same build, which has been tested. Stable signing and notarization remain a separate release step.

Earlier unsandboxed desktop test builds used `~/Library/Application Support/de.alphatester.music-generator/`. To retain that library, select that directory using the first-run import button; it copies the database and songs into an empty sandbox library and reuses verified models after authorization. Existing files remain in place. Legacy browser-project imports still use the same button.

The app does not write into the program bundle. Replace the complete app/bundle for updates; models and library remain available. Setup → Version checks releases in this fork and opens the platform's new bundle. The app does not perform Git pulls or replace its own running files. Setup → Models and hardware test can download another model or repeat the test. Changing the song folder copies files first and switches the saved path only after successful copying; queued/running songs must finish or be cancelled before moving.

Closing while work is pending asks for confirmation. Running jobs and pending jobs are resumed on the next start. Cancelling a song explicitly leaves it cancelled until Retry. A control-pipe supervisor terminates the owned engine when the GUI exits, including an unexpected GUI crash. Other programs/processes are never stopped based on a port number.

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

The preparation script builds the pinned Mac/Linux engine source revisions and checks the pinned Windows archive hash. The Mac revision adds the small `engine-sandbox.mm` bookmark bridge; its patch and build-recipe hashes are recorded alongside runtime-file hashes. Mac CPU code targets portable Apple Silicon and uses relative library paths. The app, supervisor and engine are signed from the inside out with separate parent/child entitlements. Linux builds Vulkan and CPU variants; the CI image installs build dependencies, users do not. Windows includes app-local Visual C++ runtime DLLs from the build tools' redistributable directory and a Fixed Version WebView2 runtime pinned in `models/webview2.json`. Updating that manifest requires downloading the official Microsoft CAB and verifying its checksum. Ship a new bundle to update the fixed browser runtime.

The `Desktop bundles` GitHub Actions workflow builds each platform natively and uploads artifacts. A version tag, manual dispatch or push to `codex/desktop-bundles` triggers it; it does not automatically publish a GitHub Release. Linux uses a checksummed Vulkan SDK 1.4.309.0 for its shader compiler, since Ubuntu 22.04 does not provide `glslc` in its standard package archive.

After packaging, `node scripts/desktop/verify-bundle.mjs` checks the artifact checksum, extracts it into a different directory containing spaces, verifies all engine resource hashes and starts the packaged supervisor/engine in CPU mode. A tiny tensor-free GGUF fixture permits an HTTP health check without model downloads or inference. The test also checks shutdown when the supervisor control pipe closes, plus the Windows runtime files. macOS additionally checks signing entitlements and actual OS denial of unselected file reads/writes by the main app and a read by the inherited engine. It repeats those checks after copying the app from a read-only DMG and ejecting it (`--dmg`). These checks do not generate music.

Attach the tested artifacts and their SHA-256 files to a release tagged `vVERSION` for in-app update discovery. The version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` must match.

For signed macOS releases configure the standard Tauri secrets `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`. Keep certificates and passwords out of source control. Windows signing can be added to the portable packaging step once a certificate is available; current Windows builds are unsigned.

## Release acceptance

Automated tests cover serial queue behavior, stable seeds/variants, cancellation against late results, retry/restart, folders/favorites, lookup beyond 500 visible rows, imports with WAL and import rollback, path validation, HTTP-Range continuation/restart, checksum rejection, disk-space rejection, engine JSON/multipart results, timeout/cancellation and supervisor shutdown.

Before calling a platform release supported, test the actual bundle on a clean system without development tools: setup, at least one real generated song, file analysis, microphone permission/recording, waveform playback, WAV/TXT/ZIP exports, keyboard and screenreader operation, close during a job, reopen, offline operation and moving/replacing the program bundle. Linux needs additional current Ubuntu and Arch smoke tests. Windows needs a test with no preinstalled WebView2/VC++ runtime. CI compilation alone is not equivalent to these checks.
