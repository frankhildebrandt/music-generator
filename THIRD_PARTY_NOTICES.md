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

## HeartMuLa native provider

- audio.cpp, Copyright 2026 ShugoAI LLC, Apache License 2.0: https://github.com/0xShug0/audio.cpp (pinned source and archive hashes in `models/heartmula-engine.json`). The packaged HeartMuLa CLI includes its upstream license and collected third-party notices; macOS additionally links this project's security-scoped bookmark bridge.
- HeartMuLa-3B-happy-new-year and HeartCodec by the HeartMuLa contributors, Apache License 2.0: https://github.com/HeartMuLa/heartlib. Native GGUF packaging by audio-cpp: https://huggingface.co/audio-cpp/audio.cpp-gguf. Weights download separately through the verified model catalog; Python reference code is not part of the runtime.
- ggml, MIT license: https://github.com/ggml-org/ggml. Additional native runtime dependency licenses are retained under the packaged HeartMuLa runtime's `licenses` directory.
- Windows app-local Vulkan loader by Khronos/LunarG/Valve, MIT/Apache-2.0: https://vulkan.lunarg.com/. The pinned runtime-components ZIP and SHA-256 are in `models/heartmula-engine.json`; `LICENSE-Vulkan-Runtime.txt` is shipped beside the DLL. No runtime installer is executed.
- Linux app-local Vulkan loader: https://github.com/KhronosGroup/Vulkan-Loader. Its license is retained in `scripts/desktop/licenses/Vulkan-Loader.txt` and beside the packaged loader. CI builds against the pinned Linux Vulkan SDK; final loader bytes are included in the runtime checksums.

## Shared local title model

- llama.cpp v0.7.0 (MIT): https://github.com/ggml-org/llama.cpp. The statically linked CPU-only `music-title` helper uses the pinned source revision and archive checksum in `models/title-model.json`. Its runtime checksums and build recipe are in `engine/title/TITLE_RUNTIME.json`; the llama.cpp/ggml and nlohmann/json MIT license texts accompany the helper.
- Qwen3-0.6B-GGUF (Apache-2.0): https://huggingface.co/Qwen/Qwen3-0.6B-GGUF. Model weights are downloaded separately to the selected model folder, never fetched by the inference helper. The exact Q8 model revision, byte size and SHA-256 are pinned in `models/title-model.json`. Upstream license: https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/blob/main/LICENSE.
