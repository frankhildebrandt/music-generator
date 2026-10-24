# ModelPicker

The studio's **Choose models** button displays the current synthesis/language pair. The same picker is available in Settings. Historical test releases have been removed; see [repository history](repository-history.md).

Choose a provider, synthesis model and language model, then select **Use selection**, **Verify and use**, or **Download and use**. Missing base files are included automatically. The default filter shows combinations within the estimated physical RAM budget; turn it off to inspect the complete catalog. An existing choice stays visible even when it exceeds the budget. A newly selected combination above the estimate cannot be activated. This is resource guidance rather than an inference certification: longer songs, dedicated GPU memory, driver support and runtime buffers can impose further limits. Intel Macs use CPU inference and retain their performance warning.

The estimate sums both selected model weights, the fixed text encoder and VAE, plus 3 GiB of OS/runtime headroom (4 GiB for XL). It does not mistake unknown VRAM for an unsupported GPU. The engine's existing CPU language-model preference/fallback remains available in Settings. A small GPU may require a smaller synthesis model; the picker does not promise automatic CPU synthesis fallback on every GPU configuration.

## Pinned ACE-Step catalog

`models/catalog.json` is a versioned, offline catalog based on the [GGUF repository](https://huggingface.co/Serveurperso/ACE-Step-1.5-GGUF/tree/666ac70204440867d8c01ba4b119cc79c95b370a), revision `666ac70204440867d8c01ba4b119cc79c95b370a`. Every asset has its exact published byte size and LFS SHA-256. The native adapter uses the existing pinned [acestep.cpp](https://github.com/ServeurpersoCom/acestep.cpp) engines; their revisions are unchanged.

| Role | Variants |
| --- | --- |
| Standard synthesis | base, sft, turbo, turbo-continuous, turbo-shift1, turbo-shift3 in BF16/Q8_0/Q6_K/Q5_K_M/Q4_K_M; sftturbo50 in BF16/Q8_0 (32 files) |
| XL synthesis | xl-base, xl-sft, xl-sftturbo50, xl-turbo in all five precisions (20 files) |
| Language | 0.6B and 1.7B in BF16/Q8_0; 4B in BF16/Q8_0/Q6_K/Q5_K_M (8 files) |
| Fixed dependencies | Qwen3-Embedding-0.6B-Q8_0 and vae-BF16 (2 files) |

There are 60 selectable generative weights and two fixed dependencies. No nonexistent Q4 language model is invented. BF16 text encoding is deliberately not selectable: both bundled server revisions use `text_enc[0]`, with no per-request text-encoder selector. Downloading both encoders would make selection ambiguous. The optional BF16 encoder in the source repository is not a separate music-generation model. Imported folders containing additional text encoders still follow the engine's own registry order; use a model folder with only the supported Q8 encoder for reproducible text encoding.

Turbo families offer eight recommended iterations; SFT/Base retain the app's 80-iteration default. Switching models does not alter sampling controls. **Apply recommended steps** is an explicit action for the currently selected model and updates only the new-song form. The range supports individual values from 1 to 200, including saved eight-step songs.

## Downloads and persistence

The existing Rust streaming downloader provides checked HTTP ranges, `.part` resumption, disk-space checks, SHA-256 verification and promotion only after success. Progress includes filename, verification phase, bytes and MiB/s. The picker stays open on errors and supports cancel/retry. Downloads and activation run under the existing operation lock, so they cannot race an engine operation or wizard. Only a complete verified profile replaces the saved default; failure/cancel leaves the previous profile and completed setup intact. The engine is stopped before activation and scans the expanded model folder on its next lazy start. Cached proofs avoid rehashing unchanged files when browsing or reusing installed models; verified offline selection needs no network or extra model-sized disk space.

New jobs store `model_selection` (provider, synthesis filename, language filename) in their parameter JSON. Worker execution and retries resolve that snapshot rather than a later global default. Reusing a new song restores its profile without changing that song or its sampling values. Analysis also sends the selected model filenames in the engine's multipart `request` field alongside the original audio. Existing settings fall back to their original Standard/XL and language model. Existing database rows and audio are not rewritten; older jobs without a snapshot retain their previous settings-based behavior.

The first-run assistant remains a small starter setup with Q8 language models and Standard SFT, plus an optional XL download. Running it again deliberately establishes that starter profile (preserving a previous XL choice when XL remains checked); use the ModelPicker for the wider catalog afterwards.

## Shared automatic-title model

The picker also shows the Qwen3-0.6B Q8 text model, its approximately 640 MB footprint, verification state and a separate download/verify button. This common auxiliary asset is independent of music-provider profiles. Downloading it never switches the provider or changes existing songs or sampling controls. The assistant includes it with the starter music models; existing installations can fetch it here or lazily for their first blank-title job. It uses the shared resumable, checksum-verified downloader and cancellation control. See [automatic titles](desktop.md#automatic-song-titles) for lyric priority, offline behavior and error handling.

## Verification

Automated tests cover the 52/8/2 catalog inventory, starter-manifest hash agreement, invalid paths/roles/providers, complete-profile RAM boundaries, persisted selection and immutable queue/retry parameters, checksum failure/cancellation leaving defaults unchanged, successful activation and offline reuse of verified files, plus selected filenames and original audio in analysis multipart requests. Existing downloader tests cover ignored/incorrect Range responses, interruption/resumption, checksum rejection and insufficient disk space.

A browser UI check with simulated Tauri responses passed at 1440×960 and 500×800: RAM filtering, all 52/8 choices, memory rejection, network error/retry, cancellation, successful selection, unchanged sampling until the explicit recommendation and the submitted job snapshot. This checks UI behavior, not real model downloads or native inference. Real inference with every new quantization/family and clean-system GUI/accessibility acceptance on all platforms remain manual checks.
