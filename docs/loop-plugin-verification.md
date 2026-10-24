# LoopForge verification — 0.2.1

Manufacturer: Alphatester x KRGSH. Technical bundle names and parameter IDs retained.

## UI alignment update — 0.2.1

Macro titles and values, navigation tabs and action buttons now share centered alignment. Input values stay left-aligned. Single-line text is measured and ellipsized at complete UTF-8 boundaries, with logarithmic fitting for long prompts. Compact buttons retain a usable text rectangle instead of losing their height to padding.

Keyboard pages draw neutral partial white-key backing at black-note edges. Sound names and peak waveforms occupy the unobscured part of white keys; loaded black keys now display their sound name and waveform too. Cached stereo peak bins replace isolated sample reads, with display-only scaling and stronger contrast. Note labels, context marks and playback state have separate space. All playable hits retain the same 16 MIDI notes per page.

The current four-sound user bank was saved separately before reopening 0.2.1. Native visual checks covered PLAY, Coverart, EDIT/groups and GENERATE at 100%, plus 75% scaling and pages with black first/last notes. Local standalone/VST3/CLAP builds, three DSP suites, VST3 validator 47/47, CLAP scan/render/recall and package checks passed. Audio processing and parameter IDs are unchanged. macOS window/menu captions now read LoopForge.

The preceding 0.2.0 commit passed the complete Windows x64, macOS x64 and macOS arm64 CI matrix ([run 40048476597](https://github.com/frankhildebrandt/music-generator/actions/runs/40048476597)). CI for this update runs separately; practical DAW acceptance limits below still apply.

## Current change

- Native macOS arm64 standalone, VST3 and CLAP built successfully.
- DSP tests cover queued group changes, phase at the actual boundary, immediate changes with sample offsets, different loop lengths, simultaneous independent groups and host seeks into longer targets.
- Embedded bank round trip includes shared idea/revision, groups and macro mappings; a generated version-one fixture remains readable with unchanged audio.
- Existing realtime allocation audit, tempo/export and 100-cycle timing tests pass.
- Rust edit tests include stereo DC removal before normalization, exact frame count and immutable input. Shared core and service regression suites pass.
- Native UI checked with a real existing bank: stereo editor, template application, group creation/addition, keyboard stripes, waveform/coverart switch during playback.
- Artwork import, macro mapping, native bank save and validators passed; see the final UI record below. Platform CI and practical Windows/Intel host acceptance are separate checks.
- Windows CI exposed a parallel MSVC import-library collision between VST3 and CLAP; per-format import and PDB directories fix the collision without serializing the build.

## Earlier acceptance record (0.1.2)

# Loop plugin verification — 2026-10-24

Development version 0.1.2, manufacturer Alphatester x KRGSH. Checked on macOS Apple Silicon with REAPER 7.82. This is an engineering record, not a completed release acceptance certificate.

## Automated checks performed

- Native VST3 SDK validator: 47 passed, zero failed.
- Native CLAP contract host: scan, instrument initialization, 17 stereo buses, 1,024 fixed parameter IDs, MIDI event offset 37, output routing isolation, non-silent rendering, embedded state save and reopen in a fresh instance. No helper/model process is needed for these checks.
- DSP: embedded bank/audio roundtrip, truncated bank rejection, Float32 WAV roundtrip, independent target tempo and sample-rate frame counts.
- Real-time instrumentation: 32 voices plus voice stealing, tempo changes, transposition and boundary automation, zero calls to C++ new/new[] during processing. The DSP path contains no file access or blocking mutexes. This instrumentation does not intercept every C-library allocator.
- Timing: Gate/Toggle/One-shot, MIDI offsets, beat quantization, transport jump and 100 exact source cycles without cumulative drift.
- Rust shared-core tests: stereo seam blend, exact frame counts, immutable originals, normalization of silence and non-silence, finite spectral reduction with reduced noise energy, invalid edit ranges, target-context rejection. Existing desktop tests pass after extracting shared modules.
- Service import fixtures: library WAV ZIP and Pro stem ZIP, metadata, neutral gain/pan, ZIP path isolation.
- Real ACE-Step Base inference: text2music drums, followed by Lego bass with the generated drums as selected context; 50 inference steps, A minor, 120 BPM, requested four bars, 384,000 frames per result. Existing models reused without downloading another copy. Exact musical interpretation and instrument isolation require human listening.

## REAPER checks performed

Both formats scan/load on Apple Silicon. CLAP displays the new PLAY interface, imports app WAVs, displays assigned MIDI notes and renders a persistent WAV at host sample rate. VST3 import, playback controls, WAV render and project save were checked. A CLAP project with embedded loop audio was saved separately. Host latency reporting is visible. The new interface was checked at 100%, 75% and 50% scaling. The rendered WAV exists and the native drag API is connected; a successful drop into the arrangement has not yet been confirmed with the UI automation driver. Save WAV as remains available for a normal file import.

## Remaining release acceptance

- Confirm native WAV drag-out in an ordinary REAPER session.
- Practical Windows x64 and Intel-Mac DAW acceptance; the preceding plugin-only CI run passed Windows x64, macOS arm64 and Intel after the atomic checkout fix. The standalone/XL update needs a fresh matrix run.
- Full manual REAPER matrix for all MIDI modes, automation, extra output routing, offline DAW bounce, tempo/transport changes and restoration on a second machine with no source files/models.
- Human listening for generated tonality, stem separation, seamless transitions and noise-reduction quality across diverse material.
- Stress/soak testing of cancellation, parallel app/plugin model installation, service failure recovery, multiple plugin instances, large banks, malformed archives and long sessions.
- Public macOS Developer-ID signing/notarization and Windows signing. Local bundles use ad-hoc signatures.

## Current limits

128 fixed slots, one voice per slot, 32 concurrent voices, 512 MiB per embedded bank, 600 seconds per audio asset and bounded ZIP/import sizes. Outputs are assigned exclusively to one bus. Pending jobs are not resumed after restarting the service. Tonality changes between major and minor require regeneration. Highpass/lowpass are live player parameters; spectral noise reduction is an offline edit with an explicitly selected profile.

Profiles, model selection and generated/exported files persist outside the plugin bundles. Export cleanup is manual because DAW projects may reference these files. Playback of embedded audio remains independent of IPC/model availability.

## App model discovery update — 2026-10-24

The model-folder picker was removed. The service reads the app's existing data profiles and model-folder setting, prefers existing downloads over empty profiles, and uses the app's default model directory for missing downloads. Stale plugin-local folder settings no longer override the shared app directory. Four resolver tests and the IPC smoke test pass, including updating the app model setting without restarting the service. On the local Mac, discovery finds the existing VAE and textencoder in `~/Library/Application Support/de.alphatester.music-generator/models`; Base is still missing there. Download/repair remains an explicit Check models action.

## Standalone, musical length and XL update — 2026-10-24

The same instrument now builds as a native macOS app and Windows executable using iPlug2's RtAudio/RtMidi host. Standalone audio is output-only stereo; portable bus assignments remain stored and are monitored through the master. Its manual BPM controls playback and export. The macOS package was launched independently of REAPER; the existing generated drum bank was saved before updating it. Windows and Intel practical standalone audio/MIDI checks remain open.

Generated material includes two extra bars, with a minimum of ten seconds. A bar-aligned region is checked for long leading/trailing silence before acceptance; missing usable material fails instead of padding with silence. Explicit Fit loop repair may repeat a shorter complete phrase; musical duration and originals remain preserved. Synthetic regressions cover eight bars, silent endings, stereo-only activity, preserved internal rests, fractional frame bar lengths, and model DC offsets that would otherwise hide a silent ending. Native export tests cover eight bars at 120 and 137 BPM without inserted silent blocks.

A real 8-bar/120-BPM test export contained roughly two seconds of trailing near-silence. The packaged service's Fit loop returned exactly 768,000 frames at 48 kHz, using four complete bars repeated twice. Last-second RMS increased from 0.0000032 to 0.0594. A separate portable test bank retains original and repaired audio. This validates removal of the unwanted tail; human listening remains required for artistic phrasing and seam quality.

New text2music and Lego requests explicitly select `acestep-v15-xl-sft-Q8_0.gguf`. A shared list controls status, download, verification and AppData profile scoring: XL, Qwen textencoder, VAE. A request-contract regression checks both task types and ensures Base is not demanded by model setup. Historical Base inference checks above do not constitute XL inference acceptance.

Local checks: all shared-core/service and desktop regression tests, Clippy, three native DSP suites, VST3 validator 47/47, CLAP contract host, package checksums and ad-hoc bundle signatures pass. The initial desktop test run was sandbox-blocked from binding local test HTTP servers; it passed with local socket access enabled.

## Split generation workspace and progress

GENERATE now has independent settings and take-list scrolling. It immediately allocates visible draft cards for each requested variant, renders real waveforms only after audio arrives, and shows engine DiT step counts scoped to the current log offset. The service publishes each completed variant before later variants finish. Worker-side waveform reduction and WAV rendering keep preparation away from the audio thread. Cards support persistent WAV drag-out, internal drag-to-key assignment, explicit assignment and Play/Stop. Completed takes remain available across subsequent jobs in the current editor session; banks persist accepted loops.

The progress parser regression checks current-generation offsets and rejects invalid/zero step totals. Cancellation/failure stops draft indicators and retains completed takes and assigned bank audio. Additional macOS visual checks are recorded after the local package run.

## Playheads and instrument selection

Instrument selection uses a native dropdown with the twelve supported instrument IDs and a single current-selection checkmark. The audio player publishes normalized per-slot positions through atomics once per block. The UI reads them for take preview, assigned takes, the trim editor and the PLAY position marker. Positions account for stretch output latency, wrap with the musical cycle and disappear when the voice stops. A timing regression checks phase advancement and stopped visibility; existing no-allocation tests cover the additional publication path.

Actual XL text2music at 155 BPM produced an exact eight-bar loop in the standalone package. Its draft appeared immediately, the finished waveform rendered, Play/Stop toggled and internal waveform drag assigned the result to note 38 while preserving the prior note-36 audio. XL Lego with selected note-38 context returned variants incrementally, with real 50-step progress visible while further variants were still running. Native WAV drop to REAPER remains subject to the manual acceptance item above.

## Final UI and package checks

The 0.2.0 UI imported PNG artwork and mapped M1 to High Pass. Saving through the native bank dialog produced a version-two bank containing all three assigned sounds, group membership, macro target 4, 13,855 embedded artwork bytes, idea revision 2 and workspace settings. Artwork fitting was corrected after this visual test to preserve the full image and its aspect ratio. The native VST3 validator passed 47/47 and the CLAP host passed its scan/render/recall checks. Package verification found 480 files with matching checksums and valid local signatures.

Windows compilation now succeeds through all three plugin formats. The next CI run exposed the platform's smaller default thread stack in the enlarged DSP test fixtures. Player fixtures now live on the heap; the audio preroll buffer is also preallocated as part of the player instead of occupying 256 KiB on the callback stack. This preserves the no-allocation audio callback contract. Final Windows execution and practical Windows/Intel DAW acceptance remain separate checks.

A real XL generation with the shared Midnight Electro idea completed successfully with exactly eight bars at 155 BPM (594,581 frames at 48 kHz). The returned result preserved idea revision 3. A subsequent edit preserved the frame count, removed DC and reached the −1 dBFS normalization target. The standalone editor uses the selected slot's source tempo and cycle length when applying edits, independently of the next generation's settings. A pending group target is re-quantized after a host seek and still passes through exclusive group activation; this edge case is covered by the timing suite.
