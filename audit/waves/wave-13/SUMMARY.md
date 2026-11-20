# Wave 13 SUMMARY — apps/cli open-path trace + apps/desktop-tauri GUI trace

Agents: W13-A01 (subagent, 7 CLI files), W13-A02 (subagent, 12 GUI/audio files incl. audio-output).

## The headline the wave settles
**The MP4 open path is REAL end to end at HEAD (CLI), and the GUI path is statically complete to the canvas — but audio is NULL-ONLY in every shipped surface, and runtime GUI pixels are UNVERIFIED.**

## Executed proof (orchestrator runs, 2026-11-16, binary from HEAD 727ad6b2)
- `printf 'advance 512\ntime\nquit\n' | lumen open sample-h264-aac.mp4` → `end-of-media`, `Playing → Ended`, `time 3.041814s (100.0%)`, EXIT=0 (`audit/runs/baseline/mp4-smoke-open-advance.txt`).
- argv→decoder chain (W13-A01, every step cited): main.rs:19 → run.rs:132/193 → dispatch.rs:81-87 → host.rs:725 `open_source` → demux_host.rs:444-458 (probe → "mp4") → containers/mod.rs:67 → containers/mp4.rs:100 (Mp4Demuxer) → :190 sample_entry avc1→"h264" (:440) → :237 `find_child_box(entry, b"avcC")` → TrackFact.extra_data → pipeline.rs:1685 `decoder_for` → pipeline_host.rs:328 `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)` → configure failures surfaced typed (host.rs:818-821). avcC harvest is real; the executed decode is its proof.
- GUI chain (W13-A02, cited): UI CustomEvent `lumen:open-media` (PlayerScreen.tsx:471) → `session.open` (open-path.ts:58-69) → `invoke("lumen_dispatch")` (tauri-transport.ts:143) → the SAME command layer (app.rs:142-152; dispatch.rs:211-213) → pacer tick 30,625 µs real engine timing (shell.rs:80-81) → I420→RGBA in-engine (pipeline_host.rs:1299/:1432) → FrameTap mailbox (frame_tap.rs:106-118) → newest-frame-per-tick raw bytes (app.rs:221) → `ctx.putImageData` (MediaSurface.tsx:225-227) on the `<canvas>` (:174).
- Audio: `builtin_backends() = [NullBackend, UnsupportedBackend]` (audio-output/src/lib.rs:152-157); the engine opens NullBackend in **discard mode** (pipeline_host.rs:1555-1556); cpal compile-time closed (`AUDIO_ADR005_ADMITTED = false`, cpal.rs:43); worklog.md:2294-2296 admits "audio output remains the null device engine-wide". **A user hears nothing in every shipped surface.**

## Files audited (19)
CLI (W13-A01): main.rs REAL · run.rs REAL · host.rs REAL (engine_info() FALSE-as-description) · pipeline_host.rs REAL · bench.rs SHALLOW · demux_host.rs REAL · containers/mod.rs REAL.
GUI (W13-A02): apps/desktop-tauri app.rs/shell.rs/dispatch.rs/frame_tap.rs/diagnostics.rs + conf REAL; ui MediaSurface.tsx REAL; frame-channel.ts REAL; tauri-transport.ts REAL; PlayerScreen.tsx REAL (stale doc block); audio-output lib/null/cpal/stub REAL (null-only). Full list + per-file verdicts: `audit/files/apps/**` and `audit/runs/W13-A0*.log.md`.

## Findings totals
W13-A01: S1:1 S2:4 S3:8 S4:6+ · W13-A02: S1:1 S2:4 S3:9 S4:12 → **S1:2 S2:8 S3:17 S4:18+ (45+)**.

## Worst findings (citations sed-verified by the orchestrator)
1. **[S1] host.rs:1452-1482** — `engine_info()` returns a static vec (`decoders: mjpeg, pcm_s16le`, `active: false`) contradicting the executed h264+aac decode in the same binary; `sessions` unpopulatable by construction; and **command_surface.rs:125-133 pins the lie as the expected test answer** (same pin-the-bug pattern as the MP4 elst test).
2. **[S1] audio-output** — null-only, discard-mode audio in every shipped surface; "HEAR" is FALSE by code (see above citations).
3. **[S2] bench.rs:300-334** — decode-bench's hard-coded mjpeg/pcm allowlist prints "no decoder in this build (first vertical: mjpeg + pcm)" for codecs the binary demonstrably decodes; bench.rs:427-446 mints `extra_data: None` so even fixing the allowlist leaves 11+ families unseeded.
4. **[S2] pipeline_host.rs:1314-1341** — window-full hold stashes the **unconverted** I420 frame (self-admitted defect: early-window frames freeze the surface).
5. **[S2] GUI proof gap** — GUI frame evidence fixtures are AVI/MJPEG+PCM only, never H.264+AAC; the SEE path's last meter is mock-tested (jsdom has no rasterizer); tauri-transport.ts:92-98 carries an unresolved `[VERIFY]` marker for the runtime event shape.
6. **[S2] containers/mp4.rs:27-32** — doc claims no h264 decoder dispatch exists; executed behavior proves the opposite.
7. **[S3] host.rs:1128/1172-1178** — fabricated `seekable: true` and `buffered=[0..duration]` snapshot fields; help.rs:57 seek help line false (real MP4 seek fails typed: `plugin.demux.reanchor.unsupported`, live-verified).
8. **[S3] PlayerScreen.tsx:36-42** — stale comment + screen-reader note claim "the DECODE/FRAMES half of the media surface is engine-wired nowhere" while the file mounts the w31.4 canvas two blocks below.
9. **[S3] frame ship ceiling** — one-frame-per-tick (≤32.65 fps) newest-wins decimation, uncounted.
10. **[S3] run.rs:738** — raw `Debug` dumps for post-first-vertical events including the seek-failure terminal.

## Anti-laziness gate
Orchestrator re-checked 5 citations (audio-output lib.rs:152-157, cpal.rs:43, null.rs:437, PlayerScreen.tsx:36-42, pipeline_host.rs:1299-1300) — all verbatim-faithful. PASS. audit/verify/fabrications.md still empty.

## What a user actually gets at HEAD (the wave's answer)
CLI: real open/demux/decode/play of MP4 H.264+AAC, silent (null audio), text events only, seek broken on MP4 (typed refusal), engine-info/decode-bench self-descriptions stale. GUI: the same real engine behind a tauri shell that ships RGBA frames to a canvas at ≤32.65 fps newest-wins — statically proven to the webview boundary, runtime-pixels UNVERIFIED in this sandbox (no display; recorded), zero sound. The docs/premise describing "MP4 cannot be opened" reflect an older tree; at HEAD the MP4 story is: picture probably (runtime-unproven), sound definitively no.

## Agents' blocked items
- W13-A02: runtime GUI execution impossible in sandbox (recorded as UNVERIFIED per finding); OS-signal crash coverage assessment limited to panic hook reading.
- W13-A01: none.
