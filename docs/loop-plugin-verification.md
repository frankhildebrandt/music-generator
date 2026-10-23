# Loop plugin verification — 2026-10-23

Development version 0.1.0, manufacturer Alphatester x KRGSH. Checked on macOS Apple Silicon with REAPER 7.82. This is an engineering record, not a completed release acceptance certificate.

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
- Native Windows x64 and Intel-Mac builds and DAW acceptance; CI definitions are provided but have not run in this local session.
- Full manual REAPER matrix for all MIDI modes, automation, extra output routing, offline DAW bounce, tempo/transport changes and restoration on a second machine with no source files/models.
- Human listening for generated tonality, stem separation, seamless transitions and noise-reduction quality across diverse material.
- Stress/soak testing of cancellation, parallel app/plugin model installation, service failure recovery, multiple plugin instances, large banks, malformed archives and long sessions.
- Public macOS Developer-ID signing/notarization and Windows signing. Local bundles use ad-hoc signatures.

## Current limits

128 fixed slots, one voice per slot, 32 concurrent voices, 512 MiB per embedded bank, 600 seconds per audio asset and bounded ZIP/import sizes. Outputs are assigned exclusively to one bus. Pending jobs are not resumed after restarting the service. Tonality changes between major and minor require regeneration. Highpass/lowpass are live player parameters; spectral noise reduction is an offline edit with an explicitly selected profile.

Profiles, model selection and generated/exported files persist outside the plugin bundles. Export cleanup is manual because DAW projects may reference these files. Playback of embedded audio remains independent of IPC/model availability.
