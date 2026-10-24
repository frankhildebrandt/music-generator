# Realtime generation progress

The desktop implementation uses the bundled engine's existing `GET /logs` SSE stream and Tauri events. It does not need a Python server, a browser-facing `/api/events` endpoint, or an additional local HTTP listener.

## Data flow

1. Rust submits an LM or synthesis job and receives its engine job ID.
2. During that operation, it consumes `/logs` alongside the existing `/job` status/result requests. The stream includes a replayed ring buffer. Progress is accepted only after `[Server] Job <id> created …` for the current operation; earlier jobs and lines after completion are ignored.
3. Known log messages become model-loading, LM, DiT, and VAE phases. `[DiT] Step N/M` supplies a measured fraction. LM step counts have no known total; LM, loading and VAE therefore use an indeterminate bar. The DiT fraction measures diffusion only, not whole-song completion.
4. The worker publishes `generation-progress` with the library song ID, a monotonic sequence, phase, step, total and elapsed seconds. It also keeps the latest snapshot in `queue.progress` and persists a readable message for fallback. Startup and audio-saving phases come from the worker.
5. The studio changes only the active row on progress. `library-changed` refreshes library metadata and announces completed/failed songs. Listener registration precedes the first snapshot; overlapping refreshes are serialized, with a trailing refresh if an event arrives in flight. Snapshots cannot replace newer progress events.

The stream protocol and step messages were checked against the pinned macOS/Linux engine (`b7ba6d9`) and the Windows server (`6013134`). The parser intentionally recognizes a small set of messages rather than displaying arbitrary engine logs.

## Recovery and lifetime

- Rust retries a closed or unavailable log stream every three seconds while the job is active. Each connection must see the current job's creation marker again. If that marker has aged out of the engine's 512-line replay buffer, the last measured progress and elapsed-time updates remain available; a later job opens its own stream.
- Status polling remains authoritative for completion, errors, cancellation and result retrieval because `/logs` is a diagnostic stream without a durable completion contract. Missing, malformed or unsupported logs never fail the generation.
- Cancellation, timeout, completion and errors drop the stream future with the operation; no detached reader survives into the next job. LM and synthesis use their respective server URLs, including split CPU-LM mode.
- The frontend does not poll the library while both event listeners and snapshots work. Failed listener registration enables 1.5-second snapshot polling and three-second registration retries. Failed snapshots are retried until recovery. Window focus requests a fresh snapshot; page teardown removes listeners and timers. The independent health check remains unchanged.
- Completion clears the cached job progress. A cancelled or terminal row ignores late progress. Ending a drag also refreshes changes whose render was deferred during the gesture.

## Verification

Automated coverage includes fragmented UTF-8/CRLF SSE lines, bounded buffering, old/wrong job rejection, invalid step counts, LM steps without totals, DiT/VAE phases, a real HTTP stream disconnect/reconnect, engines without `/logs`, completion/result handling, listener recovery, no library polling during healthy event delivery, overlapping snapshots and teardown. A timeout during an active SSE stream must retain the typed timeout error, settings guidance and engine-job cancellation. Existing engine diagnostics, audio-code duration limits and queue timing tests continue to apply.

Run:

```sh
npm run build
npm run lint
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
```

A local macOS smoke test also ran the real pinned engine with the existing 4B LM and SFT model: a ten-second instrumental request with eight DiT steps produced 44 progress updates spanning LM, loading, synthesis, all eight DiT steps and VAE, plus a valid 1,843,244-byte PCM WAV. The test used a temporary engine copy signed for standalone execution; the shipped sandbox entitlements were unchanged.

Native Windows/Linux inference and screenreader acceptance remain platform checks; log parsing and transport recovery run locally against mock HTTP engines.
