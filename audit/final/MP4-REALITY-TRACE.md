# MP4 reality trace — exact points of failure and non-failure (CLI + GUI)

Evidence date: 2026-11-16. Binary: `target/release/lumen` built from HEAD `727ad6b2`
(w31.6) with `cargo build --release --locked -p lumen-cli -p desktop`. Sample: 3.000s
H.264 320x240@25 + AAC, 45,682 bytes (`audit/runs/baseline/sample-h264-aac.mp4`).

## A. The litmus, verbatim (CLI)

```
$ printf 'advance 512\ntime\nquit\n' | ./target/release/lumen open audit/runs/baseline/sample-h264-aac.mp4
ok: accepted session=1 gen=0
event: state-changed Idle → Opening
event: tracks 2 ready
event: state-changed Opening → Buffering
event: TimingDurationKnown { duration: Timestamp(3000000) }
event: MediaInfoReady { ... tracks: h264 (Video, is_selected: true), aac (Audio, is_selected: true) ... seekable: true }
event: PlaybackEndOfTrack { track_kind: Video, id: TrackId(0) }
event: buffering 1.00
event: state-changed Buffering → Playing
event: end-of-media
event: state-changed Playing → Ended
state: Ended (position 3.041814s)
ok: time 3.041814s (100.0%), 1 buffered range(s)
event: state-changed Ended → Idle
EXIT=0
```

**The MP4 opens, demuxes, decodes (H.264 video limb + AAC audio limb), plays to natural end
at 100.0% and exits 0 — on this Linux host, in this binary.** The owner-reported "an MP4
video cannot be opened properly" does not reproduce in the CLI at HEAD on Linux.

## B. Where it actually fails / does not fail (all points executed or cited)

| # | Point on the path | Status at HEAD | Evidence |
|---|---|---|---|
| 1 | Container probe → mp4 | REAL (executed) | demux_host.rs:444-458; containers/mod.rs:67 |
| 2 | Box walk, moov/trak/stbl parse | REAL (executed; S1 edge defects exist — unclamped stts/ctts loops) | W05-A01: mod.rs:1253-1266/:1314-1332 |
| 3 | avc1 → "h264" + avcC harvest into extra_data | REAL (executed) | containers/mp4.rs:190/:237; :440; W13-A01 chain |
| 4 | aac track identification | REAL (executed) | open trace: `codec: Some("aac")` |
| 5 | H.264 software decode (rust_h264 behind codec-api) | REAL (executed; conformance lane in the 1001-pass suite) | pipeline_host.rs:328; container_conformance.rs:426 |
| 6 | AAC-LC decode | REAL (executed to null sink) | pipeline_host.rs audio limbs; open trace Ended 100.0% |
| 7 | **Audio the user can HEAR** | **FAILS by design** — NullBackend discard mode is the only backend; cpal admission closed (`AUDIO_ADR005_ADMITTED=false`) | audio-output lib.rs:152-157; pipeline_host.rs:1555-1556; cpal.rs:43; worklog.md:2294-2296 |
| 8 | Seek on MP4 | FAILS typed (`plugin.demux.reanchor.unsupported`), and the help line describing seek semantics is itself false | W13-A01 live-verified; help.rs:57 |
| 9 | Video the user can SEE (CLI) | N/A — the CLI is a headless text host by design | run.rs |
| 10 | Video the user can SEE (GUI, lumen-tauri) | Statically complete: engine I420→RGBA (pipeline_host.rs:1299/:1432) → FrameTap mailbox (frame_tap.rs:106-118) → newest-frame-per-tick raw ship (app.rs:221) → canvas putImageData (MediaSurface.tsx:225-227 on the canvas at :174). **Runtime pixels UNVERIFIED** — this audit sandbox has no display; the last meter is mock-tested (jsdom, no rasterizer); the GUI frame-proof fixtures are AVI/MJPEG only, never H.264+AAC | W13-A02 full chain |
| 11 | Self-description (engine-info, decode-bench) | **Both lie by omission**: static 2-decoder vec (host.rs:1452-1482, pinned by a test at command_surface.rs:125-133) and a stale mjpeg/pcm bench allowlist (bench.rs:300-334) that prints "no decoder in this build" for codecs the binary demonstrably decodes | W13-A01 |
| 12 | Windows-specific path | UNVERIFIED — no Windows host in this sandbox; w30/w31 commits claim the Windows GUI works ("Windows actually works (w30: the path fix…)", commit 53a106fb); not executed here | commit 53a106fb message |

## C. What stands between this and "an MP4 plays with picture and sound on Windows"

1. **Audio output** (blocker, certain): no non-null backend exists; the cpal lane is stashed incomplete (`git stash list`: "w31-audio-cpal-partial"). Without it, no shipped surface can emit sound on any OS.
2. **Runtime GUI display proof** (blocker for the claim, not necessarily the code): the SEE path has never been observed running on real pixels by anyone whose evidence is in the tree; GUI frame fixtures are MJPEG-era; the tauri-transport event shape carries an unresolved `[VERIFY]` marker (tauri-transport.ts:92-98).
3. **Seek on MP4** (major defect): typed refusal today; backward seek has a cursor bug (W05-A01) if it were enabled.
4. **Doc/self-description refresh** (not a runtime blocker): every status surface (engine-info, decode-bench, PARITY-REPORT, KNOWN-LIMITS canvas row) is at least one wave stale.
5. **Demux hardening** (pre-release fixes): stts/ctts unclamped loops (S1, CPU DoS), track-id space split on chapter traks (S1), elst units bug pinned by a test (S2).

## D. Verdicts

- CLI MP4 open/decode/play: **REAL** (executed).
- GUI MP4 display: **REAL-UNPROVEN** (statically traced end to end; runtime unobserved).
- Audible playback anywhere: **STUB** (null-only, by admission).
- MP4 seek: **STUB** (typed refusal) on a demuxer that otherwise plays the file.
