# Music Generator Desktop

## Start

Download the bundle for your platform from https://github.com/frankhildebrandt/music-generator/releases:

| Platform | Bundle | Requirements |
| --- | --- | --- |
| macOS | `Music-Generator-VERSION-macos-arm64.zip` | Apple Silicon, macOS 26+ |
| Windows | `Music-Generator-VERSION-windows-x64.zip` | Windows 10/11 x64, current GPU driver |
| Linux | `Music-Generator-VERSION-linux-x64.AppImage` | x64; Ubuntu 22.04 or compatible newer distribution, current GPU driver |

Extract the complete ZIP and open Music Generator. Keep the Windows EXE, `engine` and `WebView2` directories together. On Linux run `chmod +x Music-Generator-*.AppImage` once and open the AppImage. If FUSE is unavailable, run `./Music-Generator-VERSION-linux-x64.AppImage --appimage-extract`, then `./squashfs-root/AppRun`.

These first builds are unsigned test builds. macOS Gatekeeper and Windows SmartScreen can display warnings. There is no installer or requirement to install Python, Node.js, Git, compilers or package managers. GPU drivers are supplied by the operating system/vendor, not by this app. Windows ships WebView2 app-locally. On Windows 10 the app applies Microsoft's required read/execute permissions to that runtime directory; application data is not included in this permission change. Extract onto a local writable drive, not a network share.

## First-run assistant

1. Choose the interface language. The app keeps all twelve existing languages.
2. Optionally select an old Music Generator project directory to copy its library and songs. Import requires an empty library. The source database is opened read-only and backed up through SQLite, including committed WAL data; original songs are preserved. There is no library merge.
3. Review RAM/GPU information, language model recommendation and folders. A different model directory can be chosen, including an old installation's model folder. Existing model files must pass SHA-256 verification. A damaged existing final model is preserved: remove that damaged file manually or select a fresh folder, then retry.
4. Choose 0.6B, 1.7B or 4B. XL is optional and downloads in addition to the standard sound model. Download and free-space figures are estimates; the downloader checks actual response size and disk space again before writing.
5. Download and run the hardware test. Downloads can be cancelled and resumed. The app does not activate incomplete or unverified files. If the accelerated language model fails its audio-code test, a separate CPU language-model process is tested and used.

The first setup needs several GB and internet access. Model URLs, immutable revisions and SHA-256 checksums are in `models/manifest.json`. Generation, recording, analysis and playback are local after setup. Update checks only contact GitHub when requested.

## Data and updates

- macOS: `~/Library/Application Support/de.freicon.music-generator/`
- Windows: `%LOCALAPPDATA%\de.freicon.music-generator\`
- Linux: `$XDG_DATA_HOME/de.freicon.music-generator/` (normally `~/.local/share/…`)

The database is `music.db`. Models and songs default to subdirectories there; their locations are selectable. Logs are in `logs/desktop.log`, `logs/ace-server.log`, and, in split mode, `logs/ace-lm.log`.

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

The preparation script downloads the pinned Mac/Windows engine archives, checks their hashes, preserves runtime libraries/licenses and records final file hashes. Mac library search paths are relative. Linux builds the pinned engine revision with Vulkan and CPU variants; the CI image installs build dependencies, users do not. Windows includes app-local Visual C++ runtime DLLs from the build tools' redistributable directory and a Fixed Version WebView2 runtime pinned in `models/webview2.json`. Updating that manifest requires downloading the official Microsoft CAB and verifying its checksum. Ship a new bundle to update the fixed browser runtime.

The `Desktop bundles` GitHub Actions workflow builds each platform natively and uploads artifacts. A version tag or manual dispatch triggers it; it does not automatically publish a GitHub Release. Attach the tested artifacts and their SHA-256 files to a release tagged `vVERSION` for in-app update discovery. The version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` must match.

For signed macOS releases configure the standard Tauri secrets `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`. Keep certificates and passwords out of source control. Windows signing can be added to the portable packaging step once a certificate is available; current Windows builds are unsigned.

## Release acceptance

Automated tests cover serial queue behavior, stable seeds/variants, cancellation against late results, retry/restart, folders/favorites, lookup beyond 500 visible rows, imports with WAL and import rollback, path validation, HTTP-Range continuation/restart, checksum rejection, disk-space rejection, engine JSON/multipart results, timeout/cancellation and supervisor shutdown.

Before calling a platform release supported, test the actual bundle on a clean system without development tools: setup, at least one real generated song, file analysis, microphone permission/recording, waveform playback, WAV/TXT/ZIP exports, keyboard and screenreader operation, close during a job, reopen, offline operation and moving/replacing the program bundle. Linux needs additional current Ubuntu and Arch smoke tests. Windows needs a test with no preinstalled WebView2/VC++ runtime. CI compilation alone is not equivalent to these checks.
