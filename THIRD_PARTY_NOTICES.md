# Third-party components

Music Generator is distributed under the MIT license; see LICENSE.

- acestep.cpp by ServeurpersoCom: https://github.com/ServeurpersoCom/acestep.cpp (MIT). Upstream licenses are preserved. Platform source revisions and Windows archive checksums are in models/manifest.json; final bundled-file checksums are in engine/VERSION.desktop.json. The macOS source build includes the project's small Objective-C++ sandbox bookmark bridge; its SHA-256 and build-recipe hash are recorded in that provenance file.
- GGML and its CPU/GPU backends: https://github.com/ggml-org/ggml (MIT). See the engine's bundled license files.
- wavesurfer.js by katspaugh and contributors: https://github.com/katspaugh/wavesurfer.js (BSD-3-Clause). Its full license is in `vendor/wavesurfer.LICENSE` in the bundled frontend assets and `static/vendor/wavesurfer.LICENSE` in the source tree.
- Tauri: https://github.com/tauri-apps/tauri (MIT or Apache-2.0). JavaScript packages retain license files in their source distributions; Rust dependencies and versions are recorded in Cargo.lock.
- Microsoft WebView2 Fixed Version Runtime, Windows only: https://developer.microsoft.com/microsoft-edge/webview2/ — distributed under Microsoft's WebView2 Runtime terms. Its runtime license files remain inside the WebView2 directory. The exact download and SHA-256 are in models/webview2.json. It is not installed globally.
- Microsoft Visual C++ runtime DLLs, Windows only: redistributed app-locally from the Visual Studio build tools' redistributable directory under Microsoft's redistribution terms.

The GGUF models are downloaded separately from the pinned Hugging Face revision in models/manifest.json. Model cards and license information: https://huggingface.co/Serveurperso/ACE-Step-1.5-GGUF. Models are not included in the application download.

Linux AppImage media support includes the GStreamer base/good plugins supplied by the build distribution. Additional proprietary codecs and GStreamer ugly plugins are not deliberately bundled.

## Native loop instrument

- iPlug2 (zlib), including its bundled WDL, IGraphics and graphics dependencies: https://github.com/iPlug2/iPlug2. Full license notices are shipped in the plugin package.
- CLAP and clap-helpers (MIT): https://github.com/free-audio/clap and https://github.com/free-audio/clap-helpers.
- VST SDK 3.8 (MIT): https://github.com/steinbergmedia/vst3sdk. VST is a trademark of Steinberg Media Technologies GmbH.
- Signalsmith Stretch and Linear (MIT): https://github.com/Signalsmith-Audio/signalsmith-stretch and https://github.com/Signalsmith-Audio/linear.
- nlohmann JSON (MIT): https://github.com/nlohmann/json. Copyright Niels Lohmann and contributors; full license is included in the header and plugin package.
- Roboto font (Apache-2.0): plugin/resources/Roboto-LICENSE.txt.
- Rust helper dependencies, including Symphonia (MPL-2.0), retain their source licenses; exact versions are in crates/loops-service/Cargo.lock and crates/music-core/Cargo.lock. Symphonia source: https://github.com/pdeljanov/Symphonia; this project makes no modifications to it.

Pinned source revisions and graphics archive checksums are in scripts/plugin/dependencies.json. The framework adapter has a documented local VST3 sample-offset patch in scripts/plugin/iplug-automation.patch. Output Arcade is a reference for the player layout only; no Output audio or graphics are distributed.
