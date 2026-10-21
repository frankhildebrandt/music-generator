# Pro Mode

Pro is an audio arrangement workspace inside the desktop app. The titlebar's
Easy / Pro switch keeps the existing song generator available. Switching modes
pauses the other player. Pro projects autosave to the same SQLite database as
the library; immutable take audio lives in `songs/pro-PROJECT_ID/TAKE_ID.wav`.

## Working with a project

1. Switch to Pro and create a project. Set its length, BPM, key and meter.
   BPM changes affect the grid and future generations; existing audio is not
   time-stretched or transposed.
2. Add instrument tracks, import PCM/float WAV, or copy a finished Easy song
   from the library. Importing a mixed song preserves that mix as one track.
3. Use the shared transport, timeline ruler, zoom, loop selection, track volume,
   stereo balance, Mute and Solo. Several tracks may be soloed; Mute wins.
4. Move clips horizontally or trim their edges with the mouse. Snap uses the
   beat grid. Select a clip to split it at the playhead, trim it to the numeric
   selection, duplicate it, or remove it. Multiple overlapping clips are mixed.
5. Enable **KI** on the tracks the model should hear. This is independent of
   Mute/Solo. The entire target track is always excluded from its own context.
6. Download the optional Base model, enter a description, and generate a track
   or a selection within one clip. The existing take stays active. New takes
   appear in the inspector; **Anhören / Audition** explicitly activates one in
   the arrangement. Undo restores the previous selection, including clip edits.
7. Export the audible mix as WAV, or all tracks as an aligned WAV ZIP. Stem
   exports include muted tracks and retain their channel gain and balance.

The inspector preserves prompts per track. Undo/Redo keeps the last 50 edits
across restarts. Removing a clip or track does not delete its take files, so
history remains recoverable.

## Generation contracts

- The additional model is `acestep-v15-base-Q8_0.gguf`, 2,549,527,936 bytes,
  pinned to the existing Hugging Face manifest revision. Downloads are resumed
  and SHA-256 verified through the existing downloader. Easy continues using
  its existing SFT model selection.
- Jobs capture the arrangement, take IDs, channel settings, prompt and resolved
  seed at enqueue time. Later edits do not change that input. Jobs use the same
  serial desktop worker as Easy generation. Results never auto-activate.
- The context renderer includes the selected KI tracks, excludes the target
  track and applies one overall attenuation if necessary for headroom. Lego
  produces the new instrument; with no audible context, text2music produces
  the initial isolated instrument prompt. Both use direct DiT without the LM.
- Region regeneration currently generates a full-duration candidate with the
  backing context, then splices only the selected window into a new copy of the
  original take. A maximum 15 ms blend lies **inside** the selection. All source
  samples outside it remain exactly unchanged. This uses more inference time
  than a short standalone clip but avoids copying backing instruments outside
  a native Lego region. The old take is never overwritten.
- Explicitly cancelled jobs stay cancelled after restart. Interrupted running
  jobs return to the queue. A late result cannot replace a cancelled job.
- Project edits use optimistic revision checks, atomic SQLite transactions and
  atomic audio-file replacement. Library folder moves and desktop-profile
  imports copy project assets along with the existing Easy songs.

## Audio and current boundaries

Timeline positions use integer frames at 48 kHz. Web Audio schedules all clips
on one audio clock and schedules loop repetitions ahead of time. Mixer changes
use short gain ramps; native context/export rendering uses the same gain and
stereo-balance convention. A peak display indicates output above 0 dBFS; reduce
track levels when needed. Exports are float32 WAV, preserving headroom.

The first implementation accepts mono/stereo WAV with PCM16/24/32 or float32 at
8–192 kHz. Non-48-kHz imports use linear resampling. For production work, prefer
48-kHz source files. Projects support up to 32 tracks and ten minutes, subject
to a decoded-audio budget of twelve stereo track-minutes (about 264 MiB before
other buffers). MIDI, plugins, automatic stretching and additional codecs are
future work. Pro has German and English labels; other Easy UI languages fall
back to English in Pro.

## Verification and isolated preview

Automated tests cover project persistence and history, stale-write rejection,
clip bounds, Mute/Solo, source alignment, context exclusion, immutable job
snapshots, cancellation/restart, profile import, folder movement, and aligned
stem exports. HTTP tests cover direct synthesis with multipart source audio.

The hardware smoke example uses the **same** generation function as the desktop
worker, in an empty profile, and checks actual full-track and region inference:

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example pro_engine_smoke -- \
  /absolute/empty-profile /absolute/models /absolute/resources /absolute/stems
```

The stems folder must contain `01-drums.wav`, `02-bass.wav`, and `03-chords.wav`.
The example rejects existing databases. It produces `mix.wav`, `stems.zip` and
the preserved/generated takes. The local M5 Pro run passed real full-track and
6–10-second region generation, including bit-identical samples outside that
region. Desktop playback and mixer were inspected on macOS. Windows/Linux and
signed release-bundle GUI testing remain separate platform validation.

For a seeded development UI profile:

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example pro_preview -- \
  /absolute/empty-preview-profile /absolute/models /absolute/stems
```

Debug builds accept `MUSIC_GENERATOR_PROFILE` for an isolated database and
`MUSIC_GENERATOR_ENGINE_RESOURCES` for separate development engine resources.
Release builds ignore both. macOS release engine executables carry inherited
App Sandbox entitlements and require a sandboxed parent. An unsandboxed debug
preview must use separately prepared debug engine files; keep signed release
resources unchanged.
