AUDIT: apps/cli/src/pipeline_host.rs
====================================

| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-17 |
| Lines / functions | 2033 / 71 (incl. 11 test fns, 4 const tables) |
| File verdict | REAL-UNPROVEN (static trace only; no build tools — behavior evidence from W00 baseline runs) |
| Reachable from shipped entry point? | YES — `lumen` main.rs:19 `run::run` → `host::CliHost` (host.rs:926-948 constructs `CliFrameConsumer::new`, `HostDecoderProvider::new`) → `playback_engine` poll loop calls `DecoderProvider::decoder_for` (line 236) and `FrameConsumer::accept` (line 1229). Also reached by `bench.rs:270` open path (provider NOT used there — see F-006 cross-ref). |
| Findings | S0:0 S1:0 S2:2 S3:3 S4:1 |

Commands run (static analysis; no cargo/node available per adaptations):
- Read file complete (2033 lines, 5 chunks)
- `rg -n "HostDecoderProvider|CliFrameConsumer::new|open_audio_session|wall_clock" apps/cli/src` → host.rs:926-948 + 15 container-host test usages
- `rg -n "first vertical: mjpeg \+ pcm" apps/cli/src` → bench.rs:334 (NOT this file)
- `rg -n "pump_wallclock" apps/cli` → host.rs:523 (def), host.rs:1892-1933 (tests only — no production caller)
- `sed -n '341,379p;1357,1385p' apps/cli/src/pipeline_host.rs` (quote verification, passed)
- W00 baseline facts cited: `audit/runs/baseline/mp4-smoke3-stdout.txt` (open h264+aac.mp4 → Playing, time 0.000000s), `advance-test.txt` (MJPEG AVI: time 0.000000s until `advance`, then Ended/100%), `mp4-smoke-bench.txt` (bench refuses h264/aac).

## 1. One-paragraph verdict

This file is the decoder registry shim and the audio/video frame sink for the CLI pipeline, and it is the one place the repo's "no decoder" story falls apart: it dispatches **h264/avc1 and aac/aac-lc to real decoders** (lines 288, 346) — the W00 baseline confirms an MP4 session driven by `advance` reaches Ended/100%, i.e. these arms decode. The decode-bench's "no decoder in this build (first vertical: mjpeg + pcm)" message comes from `bench.rs:334`, which hardcodes its own mjpeg+pcm limb list and never consults this provider — two disjoint codec-selection tables in one binary. Beyond that, the file is genuinely implemented (no stubs), but it ships one self-admitted KNOWN DEFECT (the video hold stashes unconverted I420 frames → renderer refusal → frozen surface) that is deliberately kept because fixing it would expose a second, load-bearing pacing defect (one `advance` pump drains the entire 3 s ring).

## 2. What actually works (verified)

— verified by static trace + W00 baseline: `pipeline_host.rs:346` (`"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)`) and `:288` (`"aac" | "aac-lc" => self.seeded(track, config, AacDecoder::new)`) are reachable from `lumen open`, seeded with `avcC`/ASC blobs harvested by `containers/mp4.rs:237/:232`; W00 `mp4-smoke3-stdout.txt` shows the h264+aac session opening, reaching `Playing` with 2 tracks, and the task-supplied W00 fact "`advance N` ticks DO drive playback to Ended/100%" proves the decode arms run. UNVERIFIED by direct execution (no cargo).
— verified by `sed -n '1684,1886p'` read: I420→RGBA unit tests assert real pixel vectors (JFIF ±1, studio-swing 16/235, truncation refusal) — these would fail if the coefficient tables were wrong.

## 3. Findings

### F-001 [S2] — video hold stashes the UNCONVERTED frame; hold-retried I420 frames freeze the surface
Where: apps/cli/src/pipeline_host.rs:1357-1384
Code:
```rust
        PushOutcome::WindowFull { .. } if state.video_hold.len() < MAX_VIDEO_HOLD => {
            // ...
            // [KNOWN DEFECT, registered as the W1-06 follow-up card
            // (Lead decision required): this arm stashes the
            // UNCONVERTED original (`frame.clone()`), so every
            // hold-retried I420 frame is refused at the renderer
            state.video_hold.push_back(frame.clone());
```
What is wrong: the window-full hold pushes the raw I420 `frame.clone()` (line 1383), but the conversion to RGBA happens at line 1340-1353 *before* the push and the converted `uploadable` is dropped. `retry_video_hold` → `scheduler_push_for_hold` (line 842) re-pushes the unconverted frame; the renderer accepts RGBA only, so the first hold-retried I420 present fails with `render.upload.format-unsupported` and the surface freezes.
Why it matters: any decode burst that overflows the 16-slot scheduler window on an I420 lane (every h264/vp9/av1 session that decodes faster than the clock — which is the *designed* headless regime per line 948-951) hits this. The file itself rates it a KNOWN DEFECT.
What correct looks like: stash the converted `uploadable` (the code comment says the outcome hands it back) — and then fix the ring-drain pacing defect it masks (F-002).
Confidence: HIGH — Verification: read (self-admitted in source; cross-checked `scheduler_push_for_hold` at :842-849 and the RGBA-only renderer config at :1310-1321).

### F-002 [S2] — one pump drains the whole ring: no decode-side backpressure from the device ring
Where: apps/cli/src/pipeline_host.rs:1366-1381 (admission), code in apps/cli/src/host.rs:703-740 (`advance_time`)
Code:
```rust
            // second, load-bearing defect: `advance_time`'s render
            // loop drains the whole ring per pump, so with the
            // renderer accepting frames one `advance` tick plays the
            // ENTIRE 2.000 s fixture (the mid-media pause/stop
            // scenarios become unreachable, and a
            // longer media would fatal `ring_full` under a
```
What is wrong: `CliHost::advance_time` renders the entire null-device ring contents in one pump (host.rs:715 `while device.buffered_frames() >= RENDER_QUANTUM`), so one `advance` tick plays up to 3 s of media. The only reason sessions don't blow the ring is F-001 breaking the render loop early — the two defects mask each other, and the file says the complete fix "needs the Lead's pacing/backpressure decision".
Why it matters: it makes `advance` semantics quantum-unbound (deterministic scripts only work because the fixture is 2 s), and a longer media fatals `audio.ring_full` per the comment. `AUDIO_PARK_FILL` (line 135) parks *decode* but the render side is the unbounded half.
What correct looks like: wall-bounded render per pump — which already exists as `pump_wallclock_at` (host.rs:531) but has zero production callers (see log trace).
Confidence: HIGH — Verification: read (comment admission; cross-read host.rs:703-740, host.rs:523-536).

### F-003 [S3] — MP4 video sessions schedule on a guessed 30 fps cadence
Where: apps/cli/src/pipeline_host.rs:1296-1299
Code:
```rust
        let frame_ns = facts
            .fact_for(track)
            .and_then(|fact| fact.frame_ns)
            .unwrap_or(33_333_333);
```
What is wrong: `containers/mp4.rs:217` deliberately sets `frame_ns: None` ("VFR family — no fixed frame duration … a guessed cadence would be a lie", mp4.rs:56-59), and this fallback guesses one anyway (33 333 333 ns = 30 fps) to build `SchedulerCfg::for_frame_duration`.
Why it matters: every MP4 (the project's litmus container) gets a wrong scheduler window configuration for ≠30 fps content (e.g. 24/25/60 fps), while the container host explicitly refuses to lie. Contradictory honesty policies between two files of the same open path.
What correct looks like: derive the window from the first frames' actual PTS deltas (the demuxer emits PTS), or refuse VFR explicitly like the BT.2020 arm does.
Confidence: HIGH — Verification: read (both files).

### F-004 [S3] — refusal message under-reports the dispatched codec set
Where: apps/cli/src/pipeline_host.rs:372-379
Code:
```rust
            other => Err(host_error(
                PipelineStage::Decode,
                "cli.codec_unavailable",
                format!(
                    "no decoder for `{other}` in this build (landed: mjpeg, pcm, flac, \
                     mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264)"
                ),
            )),
```
What is wrong: the message omits `vp9` (line 313-317) and `dnxhd` (line 326-330) which are dispatched above it.
Why it matters: this is the error a user sees for e.g. `hevc` — a wrong capability inventory in the diagnostic itself; the same class of lie as bench.rs's "first vertical: mjpeg + pcm" (see log trace), smaller blast radius.
What correct looks like: generate the list from the match arms (or fix the string).
Confidence: HIGH — Verification: read.

### F-005 [S3] — subtitle frames silently swallowed but counted as accepted
Where: apps/cli/src/pipeline_host.rs:1228-1239
Code:
```rust
    fn accept(&self, frame: Frame, track: TrackId) -> Result<(), EngineError> {
        let mut state = self.lock();
        state.frames_accepted = state.frames_accepted.saturating_add(1);
        match frame {
            Frame::Audio(audio) => accept_audio(&mut state, &audio),
            Frame::Video(video) => {
                accept_video(&mut state, &video, track, &self.facts, &self.clock)
            }
            // Subtitle/data limbs are not wired in the first vertical.
            Frame::Subtitle(_) => Ok(()),
```
What is wrong: `frames_accepted` is incremented before the dispatch (line 1231), so silently-dropped subtitle frames (and stale-audio drops from `accept_audio:1271-1273`) inflate the "frames accepted" counter that `tests/pipeline_accounting.rs` and the AC accounting read.
Why it matters: the counter is the pipeline's honesty observable; counting dropped frames as accepted skews it. The subtitle drop itself is an undocumented-in-help silent no-op for any container that wires a subtitle limb.
What correct looks like: count per-disposition (accepted/stale-dropped/unsupported-limb).
Confidence: MEDIUM — Verification: read (accounting consumers not re-verified; tests/pipeline_accounting.rs is another agent's file).
### F-006 [S4] — module doc overstates determinism ("zero wall-clock reads") for the video-only path
Where: apps/cli/src/pipeline_host.rs:40-44 vs :1622-1635
Code:
```rust
//! deterministically with zero wall-clock reads on the audio-bearing
//! path. `[PROPOSAL: the render quantum (1470 frames ≈ 33.33 ms at
```
and
```rust
/// The no-audio fallback clock: system monotonic (wall time). The
```
What is wrong: the determinism claim is scoped "audio-bearing" but the adjacent `wall_clock()` (line 1629) builds a system-monotonic clock for every audio-less session, whose `now()` extrapolates wall elapsed (the file admits this at line 715-718). Doc precision debt only.
Why it matters: a reader auditing determinism will miss that video-only sessions (e.g. `--no-audio` opens) are wall-clock driven.
What correct looks like: state the exception in the determinism section itself.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `host_error` | 149-156 | REAL | — | error mint helper; message rides io::Error::other root. No panic paths. |
| `HostDecoderProvider::new` | 177-182 | REAL | — | trivial ctor. |
| `HostDecoderProvider::attach_consumer` | 187-189 | REAL | — | must be called pre-wire; host.rs:948-955 does it. |
| `HostDecoderProvider::seeded_config` | 202-210 | REAL | — | clones DecoderConfig per call (per-track, cold path — fine). |
| `HostDecoderProvider::seeded` | 219-232 | REAL | — | configure gate delegated to codec; wraps in ReseedingDecoder. |
| `HostDecoderProvider::decoder_for` | 236-381 | REAL-UNPROVEN | S3 (msg) | the codec table: mjpeg/flac/mp3/mp2/vorbis/opus/aac/alac/adpcm_*/av1/vp9/dnxhd/h264/pcm_*; h264+aac arms proven by W00 advance-to-Ended fact; refusal message wrong (F-004). |
| `ReseedingDecoder::new` | 423-425 | REAL | — | trivial. |
| `ReseedingDecoder::effective` | 430-438 | REAL | — | incoming blob wins over retained (tested at 1999-2013). |
| `ReseedingDecoder::configure` | 442-447 | REAL | — | pure delegation with re-seed. |
| `ReseedingDecoder::decode` | 449-455 | REAL | — | pure delegation. |
| `ReseedingDecoder::flush` | 457-459 | REAL | — | pure delegation. |
| `DrainGatedDecoder::audio_backlog` | 479-490 | REAL | — | lock() recovers poison (documented). |
| `DrainGatedDecoder::configure` | 494-496 | REAL | — | delegation. |
| `DrainGatedDecoder::decode` | 498-504 | REAL | — | delegation. |
| `DrainGatedDecoder::flush` | 506-537 | REAL | — | backlog>=quantum ⇒ NeedMoreInput (end-of-media waits for play-out); sub-quantum release rationale documented with measured incident (32 000 frames @ 8 kHz stranded 1 130). UNVERIFIED at runtime. |
| `AudioLimb::clock_source` | 556-559 | REAL | — | delegation. |
| `AudioLimb::buffered_frames` | 564-566 | REAL | — | delegation. |
| `AudioLimb::ring_ready` | 579-581 | REAL | — | park gate at 32 768 frames. |
| `AudioLimb::written_frames` | 587-589 | REAL | — | delegation. |
| `AudioLimb::observe_drift` | 613-624 | REAL-UNPROVEN | — | u128 math, rate_hz==0 guarded; `as u64` truncation needs >584 y — benign. Governor never armed by the CLI open path (admitted at 606-609). |
| `AudioLimb::reset_drift` | 629-631 | REAL | — | delegation. |
| `AudioLimb::arm_drift_governor` | 639-641 | REAL | — | no production caller in apps/cli (harness-only) — UNREACHABLE from shipped entry points. |
| `AudioLimb::drift` | 646-648 | REAL | — | delegation. |
| `AudioLimb::stage` | 659-666 | REAL | — | process(0,…) then mix then write; per-frame. |
| `AudioLimb::drift_stats` | 671-673 | REAL | — | delegation. |
| `PresentTap::tap_present` (trait) | 692-700 | REAL | — | seam; CLI passes None; desktop installs it. |
| `VideoLimb::tick_once` | 768-775 | REAL | — | tick + enqueue/drain. |
| `VideoLimb::enqueue_and_drain` | 777-823 | REAL | — | Full⇒direct render; Stale⇒documented drop (`let _ = request;` :797); error path saturating-adds delivered count. |
| `VideoLimb::render_request` | 825-838 | REAL | — | latch + presented++. |
| `VideoLimb::scheduler_push_for_hold` | 842-849 | REAL | S2 | re-pushes held frames — F-001's re-entry point. |
| `VideoLimb::seek_boundary` | 860-862 | REAL | — | queue publish. |
| `VideoLimb::scheduler_stats` | 866-868 | REAL | — | delegation. |
| `VideoLimb::last_presented_ns` | 878-881 | REAL | — | generation-checked latch. |
| `VideoLimb::staging_stats` | 885-887 | REAL | — | delegation. |
| `VideoLimb::queue_stats` | 893-895 | REAL | — | delegation. |
| `VideoLimb::published_generation` | 900-902 | REAL | — | delegation. |
| `VideoLimb::screenshot_png` | 909-919 | REAL | — | readback + encode_rgba. |
| `ConsumerState::video` | 991-993 | REAL | — | accessor. |
| `ConsumerState::audio` | 997-999 | REAL | — | accessor. |
| `ConsumerState::tick_video` | 1010-1020 | REAL | — | split borrows for tap. |
| `ConsumerState::retry_video_hold` | 1026-1046 | REAL | — | O(hold) per tick, hold ≤64. |
| `ConsumerState::note_video_error` | 1049-1053 | REAL | — | first-error-wins. |
| `ConsumerState::observe_audio_drift` | 1058-1062 | REAL | — | no-op without limb. |
| `ConsumerState::audio_drift_stats` | 1068-1072 | REAL | — | default when limb-less. |
| `ConsumerState::seek_boundary` | 1095-1109 | REAL | — | retain gen>=mint, publish, reset drift, arm floor. |
| `ConsumerState::audio_stale_prefix` | 1130-1147 | REAL-UNPROVEN | — | write-order arithmetic; subtle (two-case change-point). Mutation-checked only by reading; UNVERIFIED at runtime. |
| `CliFrameConsumer::new` | 1165-1187 | REAL | — | video limb lazy. |
| `CliFrameConsumer::lock` | 1192-1197 | REAL | — | poison recovery. |
| `CliFrameConsumer::set_present_tap` | 1203-1205 | REAL | — | desktop-only. |
| `CliFrameConsumer::take_video_error` | 1209-1211 | REAL | — | take(). |
| `CliFrameConsumer::last_presented_ns` | 1220-1225 | REAL | — | delegation. |
| `CliFrameConsumer::accept` (FrameConsumer) | 1229-1240 | REAL | S3 | F-005 counting/subtitle drop. |
| `CliFrameConsumer::ready_for` (FrameConsumer) | 1242-1250 | REAL | — | audio parked at 32 768; video always ready. |
| `accept_audio` | 1254-1285 | REAL | — | floor gate → stale_dropped++; change-point bookkeeping before staging. |
| `accept_video` | 1288-1393 | REAL | S2/S3 | lazy limb build (F-003 cadence guess), I420 convert, NV12/YUYV typed refusal (:1343-1352), hold arm = F-001. |
| `convert_i420_to_rgba` | 1475-1557 | REAL | — | 4 matrix/range pairs, BT.2020 typed refusal, truncation check, box upsample; per-pixel integer math, one alloc per frame. |
| `clamp_u8` | 1559-1561 | REAL | — | trivial. |
| `open_audio_session` | 1587-1620 | REAL | — | null stream + pipeline + audio_master clock. |
| `wall_clock` | 1629-1635 | REAL | — | system monotonic for audio-less sessions (see F-006). |
| tests `i420_frame`/`pixel` | 1646-1680 | REAL | — | helpers. |
| `i420_conversion_matches_jfif_vectors` | 1684-1726 | REAL | — | real vectors, ±1 tolerance, would fail on wrong coefficients. |
| `i420_conversion_covers_the_landed_color_pairs` | 1732-1832 | REAL | — | studio-swing 16/235→0/255 exact, BT.2020 refusal asserted. |
| `i420_conversion_validates_and_handles_odd_sizes` | 1836-1864 | REAL | — | 7×5 ceil geometry + truncation refusal. |
| `i420_conversion_accepts_limited_range` | 1871-1886 | REAL | — | behavior-flip pin. |
| `shared_bytes_roundtrip` | 1890-1904 | SHALLOW | S4 | asserts a 3-byte roundtrip + builds an unused `_packet_shape` — compile pin only, tests nothing behavioral. |
| `RecordingDecoder` impl + `recorded` | 1910-1944 | REAL | — | test double for reseed. |
| `test_config` | 1946-1955 | REAL | — | helper. |
| `reseeded_generation_reconfigure_replays_the_open_path_blob` | 1964-2032 | REAL | — | would fail if reseed logic broke (asserts blob on both configures, precedence, fact-less passthrough). |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| i420_conversion_matches_jfif_vectors | YES | exact pixel assertions incl. clamp arms. |
| i420_conversion_covers_the_landed_color_pairs | YES | 16/235 exact mapping + refusal code assert. |
| i420_conversion_validates_and_handles_odd_sizes | YES | truncation must yield Corrupt{Truncated{96,10}}. |
| i420_conversion_accepts_limited_range | YES | 130±1 pixel check. |
| shared_bytes_roundtrip | NO | tautology: asserts what `from_vec` just stored (S4). |
| reseeded_generation_reconfigure_replays_the_open_path_blob | YES | asserts inner decoder saw re-seeded blob + precedence. |
| wall_pump_renders_only_what_the_wall_owes (host.rs:1888) | YES (covers this file's AudioLimb ring arithmetic) | exact quantum counts at 50 ms/1 s/2 s synthetic instants. |

Not one test in this file covers: F-001 (hold-arm conversion), F-003 (VFR cadence guess), the h264/aac dispatch arms (those live in containers/*/tests via HostDecoderProvider — e.g. mp4.rs:487), or `DrainGatedDecoder::flush` gating.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no decoder in this build (first vertical: mjpeg + pcm)" | bench.rs:334 (module doc claim about the build) | FALSE | pipeline_host.rs:346 dispatches h264; :288 aac; W00 baseline: MP4 advance→Ended/100%. bench.rs hardcodes its own list (bench.rs:300-313). |
| "Requires the container's avcC record in extra_data for ISO-BMFF tracks" | pipeline_host.rs:337-345 comment | REAL-UNPROVEN | mp4.rs:237 harvests avcC payload; configure gate is the codec's (codecs crate, other wave). |
| "No software h264/hevc decoder is dispatched in this build" | containers/mp4.rs:29-32 module doc | FALSE (stale) | pipeline_host.rs:346 dispatches H264SwDecoder since the 2026-11-15 landing (its own comment). hevc is indeed absent. |
| h264 differential "bit-exact vs ffmpeg 7.1.5 on six lanes" | pipeline_host.rs:335-337 comment | UNVERIFIED | crates/codecs/tests/h264_sw_differential.rs is another wave's file; no run possible. |

## 7. Dead / duplicated / unreachable code

- `AudioLimb::arm_drift_governor` (639-641) and `DRIFT_TARGET_FILL_NS` (101): no caller in apps/cli production paths (rg: only doc references + the S8 harness) — UNREACHABLE from shipped entry points.
- `PresentTap`/`set_present_tap`: reachable only from apps/desktop (documented, additive — CLI passes None; acceptable seam, not dead).
- `child_box_full` duplication etc. is in mp4.rs (other agent).

## 8. Dependencies & flags

- File compiles into the `lumen-cli` crate; no feature gates inside this file. Crate features audited in apps/cli/Cargo.toml.audit.md. Uses `codecs::{aac, h264::H264SwDecoder, ...}` — whether `H264SwDecoder` is behind a codecs-crate feature must be checked there (cross-file; see Cargo.toml audit: no default-off gate found for the decoders used here; `codecs` is a plain dependency).

## 9. Open questions for the next agent

1. `crates/codecs/src/video/h264.rs` — does `H264SwDecoder::configure` accept the bare avcC payload shape mp4.rs:237 harvests (vs full box)? The pipeline arms are wired; a shape mismatch would refuse typed at open and would look exactly like "MP4 falls out at decode".
2. `apps/cli/src/host.rs:703-740` (`advance_time`) — the whole-ring drain (F-002) lives there; this file only admits it.
3. `crates/demux/src/mp4/mod.rs:534-539` — `unpack_language` adds `b'a'` (0x61) instead of 0x60 to each 5-bit value: `und` (packed 0x55C4) decodes as **"voe"** — exactly the language W00's mp4-smoke3 showed on both tracks. Off-by-one corrupts every MP4 language and defeats the `if tag == "und"` filter. (Found during this file's trace; belongs to the demux wave.)
4. `tests/pipeline_accounting.rs` — does it read `frames_accepted` in a way F-005 skews?
