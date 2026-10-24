# Native HeartMuLa provider (0.2.9 development)

HeartMuLa uses a second packaged C++ runtime, **audio.cpp**, alongside the existing ACE-Step engine. Rust owns catalog selection, verified/resumable downloads, settings, queue snapshots, process supervision, cancellation, timeouts, WAV validation and library insertion. The frontend remains local Tauri UI. No Python/Node interpreter, package manager, converter, ffmpeg process, Hub client or local webserver is required for HeartMuLa generation on the user's machine.

## Pinned runtime and models

- Runtime: [audio.cpp](https://github.com/0xShug0/audio.cpp), release `v0.9.2-drama3-base-small`, source commit `507495611b22dc4590e0e5609e05e8096384e33f`, Apache-2.0. `models/heartmula-engine.json` pins the source archive and Windows portable Vulkan archive SHA-256.
- macOS: native source build with only HeartMuLa enabled; Metal/CPU on Apple Silicon and portable CPU on Intel. The existing Foundation bookmark bridge is linked into the CLI and the CLI receives inherited sandbox entitlements. The model's file grant is transferred through the supervisor's `--model` argument just as ACE-Step uses `--models`. No broad filesystem exceptions are added.
- Linux: source build on the Ubuntu-22.04 baseline, Vulkan/CPU, portable CPU flags. Windows: pinned upstream portable Vulkan/CPU package with its DLLs and app-local MSVC runtime. ACE-Step's existing CUDA/Vulkan/CPU support is unchanged. This initial HeartMuLa adapter does not add a separate CUDA package.
- Model repository: [audio-cpp/audio.cpp-gguf](https://huggingface.co/audio-cpp/audio.cpp-gguf/tree/ef1ab2598f6d4d101486e4c5b9e605c687f20642/HeartMuLa-GGUF), revision `ef1ab2598f6d4d101486e4c5b9e605c687f20642`. Q8_0: 7,659,762,592 bytes; F16: 11,204,759,552 bytes. Each GGUF includes HeartMuLa-3B-happy-new-year, HeartCodec, configuration and tokenizer assets. No separate ACE-Step LM, text encoder or VAE is required. Do not substitute GGUFs from unrelated converters: their tensor names/layouts can differ.

All model URLs and checksums are embedded in the versioned catalog. Model selection accepts catalog identifiers, never arbitrary download URLs or paths. Per-asset URLs allow providers to use different pinned repositories while sharing the streaming downloader and integrity cache. Runtime preparation verifies archive hashes, records all shipped files and licenses, and release builds reject absent/mismatched HeartMuLa resources. Both runtimes retain separate directories so their ggml libraries cannot collide.

## UI and request behavior

Choose HeartMuLa in the first-run assistant or the studio/settings ModelPicker. New installations can download HeartMuLa directly without first downloading ACE-Step weights. The picker offers Q8_0 and F16 and identifies the integrated language model/codec. Download, verification, cancel, retry and offline reuse follow the same Rust path as ACE-Step. Switching providers remembers each provider's previous model selection.

The RAM estimate adds 16 GiB of runtime/OS headroom to the full packed model size. Both options therefore require roughly a 32-GiB machine under the conservative filter. This is a resource estimate, not proof for every GPU/driver or duration. Long songs need additional time; Intel CPU inference can be very slow. The explicit CPU setting applies to the entire HeartMuLa pipeline.

HeartMuLa accepts written lyrics and comma-separated style tags. Instrumental requests send an instrumental section plus instrumental/no-vocals tags. Empty vocal lyrics are rejected before enqueueing; automatic lyric writing remains an ACE-Step feature. Tempo, key, meter and language are appended as style hints, not promised hard constraints. This adapter caps one song at 240 seconds, matching the standard generation path; ACE-Step retains its 600-second control. Requests with longer HeartMuLa durations receive a clear error without modifying the form or saved song.

Separate HeartMuLa controls persist in each job's `heartmula_options`: temperature (default 1), CFG (1.5), top-k (50), codec solver steps (10), and codec guidance (1.25). These do not reinterpret or overwrite ACE-Step's `inference_steps`/`lm_temperature`. Reusing a song preserves both sets of values. The recommendation button targets the active provider's control. Existing songs/parameter JSON are not migrated or rewritten.

Audio import/recording analysis remains available through ACE-Step; HeartMuLa's native music model is not an audio transcriptor. The UI disables those controls for HeartMuLa and explains the provider change needed. Existing waveform/playback and WAV/MP3/lyrics/ZIP exports operate on the resulting local WAV normally.

## Execution and failure handling

Each HeartMuLa job uses an owned CLI/supervisor pair, with absolute model and output paths and argument vectors rather than shell execution. Generation is offline. One job at a time runs under the existing operation lock; switching from ACE-Step releases its resident engine first. The CLI exits after each song, releasing model/codec memory. This favors clear lifetime/cancellation over persistent warm sessions.

Output goes into a unique work folder in the app data container. On cancellation, timeout or error the owned process is stopped and the work folder removed. Only a successful process producing a complete nonempty RIFF/WAV enters the library through the existing atomic song-file path. Errors use fresh log output, not another job's diagnostics. Queue interruption/retry keeps the provider/model/options snapshot. The song records `heartmula_audio_cpp` as its engine.

## Verification scope

Tests cover provider-specific dependencies/URLs, invalid mixed-provider selections, separate sampling parameters, immutable queue/song values, per-provider profile persistence, direct HeartMuLa first-run planning, duration/lyrics/range validation and malformed/truncated WAV rejection. Existing tests continue covering downloader interruption/ranges/checksums, offline reuse, queue cancellation/restart and export preservation.

A real native Q8_0/HeartCodec Metal run on Apple M5 Pro generated 4.08 seconds of stereo 48-kHz 16-bit PCM WAV (783,404 bytes, nonzero signal). This proves native inference and decode for that tested setup, not lyric fidelity or acoustic parity with the Python reference. F16, longer outputs, other GPUs and complete Windows/Linux/Intel inference acceptance need platform-specific real-model tests. Native CI additionally verifies packaged runtime startup and macOS sandbox enforcement. CI status and final in-app generation evidence are recorded in the PR after completion.
