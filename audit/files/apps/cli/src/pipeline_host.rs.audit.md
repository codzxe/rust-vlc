# apps/cli/src/pipeline_host.rs — W13-A01 audit

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A01 |
| Date | 2026-11-16 |
| Lines | 1990 (≈45 functions incl. converter + tests) |
| File verdict | **REAL** — the decoder provider, consumer, color converter and null-audio session are all real and executed; one admitted live defect (F-001) and one inaccurate error message (F-002) |
| Reachable from shipped entry point | YES — `open` → `run::repl_mode` → `lumen_dispatch` → `host.open_session` (host.rs:711) → `HostDecoderProvider` construction (host.rs:769) → engine `Pipeline::wire` → `provider.decoder_for` (playback-engine/src/pipeline.rs:1685) |
| Findings | F-001 [S2], F-002 [S3], F-003 [S4] |

## 1 One-paragraph verdict

This is the real composition root of the decode/present path and the executed evidence proves it end to end: the h264+aac MP4 opened through the shipped binary resolved its video track to `H264SwDecoder` via the seeded-config seam (avcC blob harvested by the mp4 host), decoded both limbs to `Playing → Ended` at 100.0%, and presented through the I420→RGBA converter's BT.709-limited arm. The decoder dispatch table (13 families) is honest and typed; `configure()` failures propagate (not swallowed) into the engine's fatal path; the `DrainGatedDecoder` gate on the null ring is the reason end-of-media fires at the true media end. The two things that do not hold up: the window-full hold stashes the UNCONVERTED frame — a live, self-admitted rendering defect that freezes the surface on early window frames — and the provider's "landed:" error message under-reports its own dispatch table by omitting vp9 and dnxhd.

## 2 What actually works (verified)

- **The h264 seeding path, cited end to end** (the hostile question, answered): argv `open …/sample-h264-aac.mp4` → main.rs:19 → run.rs:132 `repl_mode` → run.rs:193 `parse::open_command` → dispatch.rs:81-87 `host.open_session` → host.rs:725 `open_source` (demux_host.rs:362) → demux_host.rs:458 `crate::containers::open("mp4", …)` (containers/mod.rs:67) → containers/mp4.rs:100-101 `Mp4Demuxer::new` + `start()` → containers/mp4.rs:190 `demuxer.sample_entry_bytes(track.id)` (real demuxer accessor, crates/demux/src/mp4/mod.rs:669) → containers/mp4.rs:237 `"h264" => find_child_box(entry, b"avcC")` (avcC record payload without box header — the ffmpeg extradata shape) into `TrackFact.extra_data` → host.rs:769 `HostDecoderProvider::new(Arc<MediaFacts>)` → host.rs:775-781 `SessionConfig` carries the provider → playback-engine pipeline.rs:1685 `provider.decoder_for(track, &config)` → pipeline_host.rs:328:
  ```rust
  "h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),
  ```
  → pipeline_host.rs:207-213 `seeded()`: `seeded_config` (184-192) injects `fact.extra_data` (the avcC blob) into the engine-minted `DecoderConfig`, `decoder.configure(&seeded)?`, wrapped in `ReseedingDecoder`. Executed proof: the orchestrator's MP4 smoke (audit/runs/baseline/mp4-smoke-open-advance.txt) decoded both limbs to `Playing → Ended`, `ok: time 3.041814s (100.0%)`, and container_conformance.rs:426 asserts the same lane emits NO `cli.codec_unavailable`. The avcC harvest is REAL (my own python box-walk of the same file found the 75-sample video trak / 131-sample audio trak the pipeline plays; bytes=45682 matches the BENCH line).
- **configure() failure is surfaced, not swallowed** (the hostile question, answered): pipeline_host.rs:209 `decoder.configure(&seeded)?` propagates the codec's typed `EngineError` out of `decoder_for`; playback-engine/src/pipeline.rs:1685 `let decoder = provider.decoder_for(track, &config)?;` — inside `obtain_decoder`, whose doc (pipeline.rs:1725) states "the caller treats the failure as fatal per the wire() contract"; the failure becomes the session `Error` state → host.rs:818-821 pumps the open to non-Opening, takes `last_fatal` (host.rs:497-499 records `EngineEventKind::Fatal`), tears down, and returns the typed error as the open's process answer. Nothing swallows.
- **DrainGatedDecoder gating** (the hostile question, answered): pipeline_host.rs:490 — `DecodeOutcome::Drained if self.audio_backlog() >= RENDER_QUANTUM => Ok(DecodeOutcome::NeedMoreInput)`. The gate is the null device's ring content via the consumer probe: `audio_backlog()` (461-472) → `AudioLimb::buffered_frames()` (546-548) → `NullSink::buffered_frames()`; no consumer / no audio limb answers 0 (no gating — video-only or `--no-audio`). Sub-quantum residue (< RENDER_QUANTUM = 1470 frames) is released so `Ended` can fire, with the host's `drain_tail_at_end` (host.rs:616-651) rendering the residue — the two halves of the 6.FIX-pcm handshake. Executed proof: end position 3.041814s = 100.0%, not a stranded 96.5%.
- **Color converter**: the four (BT.601/709 × full/limited) Q16 tables convert I420→RGBA under the frame's own tag; BT.2020 refuses typed (1438-1444). Unit-pinned (1690-1789: studio-swing black/white/gray exact on both matrices; h264's BT.709-limited arm is the one the MP4 smoke exercised — conformance asserts no `cli.i420_color_unsupported`).
- **ReseedingDecoder** re-seeds the open-path blob across the engine's header-less generation reconfigures — unit-pinned (1922-1989: reseeded replay + explicit-blob precedence + fact-less passthrough).
- **Null audio session + audio-master clock** construction (1544-1577): real; the aac track's geometry pair (mp4a entry declaration, containers/mp4.rs:200-203) sized the limb for the executed run (live `#1 audio aac 1ch 44100Hz`).

## 3 Findings

### F-001 [S2] The window-full hold stashes the UNCONVERTED frame — early-window frames die at the renderer
Where: pipeline_host.rs:1314-1341
```rust
        PushOutcome::WindowFull { .. } if state.video_hold.len() < MAX_VIDEO_HOLD => {
            // [KNOWN DEFECT, registered as the W1-06 follow-up card
            // (Lead decision required): this arm stashes the
            // UNCONVERTED original (`frame.clone()`), so every
            // hold-retried I420 frame is refused at the renderer
            // (`render.upload.format-unsupported`) and the surface
            // freezes on an early window frame.
```
with the stash at 1340: `state.video_hold.push_back(frame.clone());`
Wrong: the code pushes the raw I420 frame while `retry_video_hold` (993-1013) pushes it straight into `scheduler.push` (via `scheduler_push_for_hold`, 809-816, "no conversion: the hold stores already-converted frames" — a doc comment the code contradicts). Every hold-retried I420 frame is then refused at upload. The comment block itself goes further and names the second load-bearing defect (the render loop drains the whole ring per pump, so one `advance` tick plays the entire 2.000 s fixture), and states the fix is blocked on a Lead pacing/backpressure decision (D-5: no test weakened).
Correct: stash the converted frame (the `uploadable` value computed at 1297-1310 is already in scope and dropped on this arm); the pacing/backpressure half needs the registered Lead decision.
Confidence: high — the defect is admitted verbatim in the source; the contradicted doc comment is adjacent; behavior matches the description's mechanism. Not re-observed in my own runs (the fixtures never filled the 16-slot window under `advance 512`).
Severity S2: a live rendering defect on a reachable path (any decode burst that outruns the 16-entry scheduler window), registered but unfixed at HEAD.

### F-002 [S3] The "landed:" capability list in the codec-unavailable error omits vp9 and dnxhd
Where: pipeline_host.rs:354-361
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
Wrong: the same match statement wires `"vp9" => Vp9Decoder::new()` (295-299) and `"dnxhd" => DnxDecoder::new()` (308-312), so the error's own inventory under-reports its dispatch table by two families. A user with a hevc/mpeg2 file is told the build landed 12 codec families when it landed 14 arms. The message's entire purpose is the honest capability inventory (doc 05 §5.2 "typed, not silent") — an inventory that is itself wrong.
Correct: `…landed: mjpeg, pcm, flac, mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, vp9, dnxhd, h264)` — or generate the list from the arms so it cannot drift.
Confidence: high — both arms sed-verified at 295-312; message sed-verified at 354-361.
Severity S3: it is the error a script branches on, and it misdescribes the build.

### F-003 [S4] Dead const arm for QUEUE_CAPACITY
Where: pipeline_host.rs:121-125 — `None => loop {}` with `#[allow(clippy::empty_loop)]`, self-documented as "provably dead const arm (policy.rs precedent)". Honest, harmless. S4, recorded only.

## 4 Per-function table

| Function (line) | Role | Verdict |
|---|---|---|
| `host_error` (131-138) | mint typed engine error with sentence in chain root | REAL |
| `HostDecoderProvider::new` (159-164) | build over harvested facts | REAL |
| `attach_consumer` (169-171) | attach drain-gate probe (host.rs:773 calls it) | REAL |
| `seeded_config` (184-192) | inject fact `extra_data` into engine-minted config | REAL — the avcC/esds/cookie seam, executed |
| `seeded` (201-214) | build + configure + ReseedingDecoder wrap | REAL — `configure(&seeded)?` surfaces |
| `decoder_for` (218-363) | THE dispatch: mjpeg, flac, mp3/mp1, mp2, vorbis, opus, aac/aac-lc, alac, adpcm_, av1/av01, vp9, dnxhd, h264/avc1 (328), pcm_ (+DrainGatedDecoder), else typed refusal | REAL — 13 families + families; message drift F-002 |
| `ReseedingDecoder::new` (405-407) | wrap + retain blob | REAL |
| `ReseedingDecoder::effective` (412-420) | re-seed header-less incoming configs | REAL — unit-pinned |
| `ReseedingDecoder::{configure,decode,flush}` (424-441) | forward with effective config | REAL |
| `DrainGatedDecoder::audio_backlog` (461-472) | ring content via consumer probe; 0 when limb-less | REAL |
| `DrainGatedDecoder::{configure,decode}` (476-486) | passthrough | REAL |
| `DrainGatedDecoder::flush` (488-519) | gate Drained on backlog ≥ quantum; residue release | REAL — executed (100.0% end) |
| `AudioLimb::clock_source` (539-541) | sink counter source | REAL |
| `AudioLimb::buffered_frames` (546-548) | device backlog | REAL |
| `AudioLimb::written_frames` (554-556) | cumulative write counter | REAL — seek arithmetic |
| `AudioLimb::observe_drift` (580-591) | per-quantum drift observation (counters only) | REAL — unarmed in production open (regime note 567-576 honest) |
| `AudioLimb::reset_drift` / `arm_drift_governor` / `drift` (596-615) | regime restart / late arming / current correction | REAL (harness) |
| `AudioLimb::stage` (626-633) | process + mix + write ring | REAL — executed |
| `AudioLimb::drift_stats` (638-640) | governor counters | REAL |
| `PresentTap::tap_present` (659-667) | w31.1 desktop mirror trait | REAL — CLI constructs none (byte-identical claim holds in-tree) |
| `VideoLimb::tick_once` (735-742) | scheduler tick → enqueue/drain | REAL |
| `VideoLimb::enqueue_and_drain` (744-790) | queue push, stale drop, latched presenter drain, saturated-add presented | REAL |
| `VideoLimb::render_request` (792-805) | direct render on Full arm (never drops the request) | REAL |
| `VideoLimb::scheduler_push_for_hold` (809-816) | hold-retry push mapping | REAL — doc line contradicted by F-001 |
| `VideoLimb::seek_boundary` (827-829) | queue generation publish | REAL |
| `VideoLimb::scheduler_stats`/`last_presented_ns`/`staging_stats`/`queue_stats`/`published_generation` (833-869) | observability + F3 latch | REAL — latch is the position truth for audio-less sessions |
| `VideoLimb::screenshot_png` (876-886) | readback + encode | REAL |
| `LatchingPresenter::present` (713-725) | render + latch + tap (success path only) | REAL |
| `ConsumerState::video`/`audio` (958-965) | limb accessors | REAL |
| `ConsumerState::tick_video` (977-987) | tick + hold retry, tap beside limb | REAL |
| `ConsumerState::retry_video_hold` (993-1013) | FIFO hold drain into window | REAL — feeds F-001 |
| `ConsumerState::note_video_error` (1016-1020) | first-error-only record | REAL |
| `ConsumerState::observe_audio_drift`/`audio_drift_stats` (1025-1039) | drift plumbing | REAL |
| `ConsumerState::seek_boundary` (1062-1076) | hold retain-pre-mint + queue publish + drift reset + audio floor | REAL |
| `ConsumerState::audio_stale_prefix` (1097-1114) | write-order exact old-prefix | REAL — 6.FIX-ogg-reanchor |
| `CliFrameConsumer::new` (1132-1154) | audio pre-wired, video lazy | REAL |
| `CliFrameConsumer::lock` (1159-1164) | poison-recovering lock | REAL |
| `set_present_tap`/`take_video_error`/`last_presented_ns` (1170-1192) | tap install / error take / latch | REAL |
| `CliFrameConsumer::accept` (1196-1207) | FrameConsumer seam: audio/video routing, subtitle/data accepted-and-dropped | REAL — executed |
| `accept_audio` (1211-1242) | floor gate + generation bookkeeping + stage | REAL |
| `accept_video` (1245-1350) | lazy VideoLimb build + format convert + scheduler push + hold | REAL with F-001 |
| `convert_i420_to_rgba` (1432-1514) | 4-matrix Q16 conversion, BT.2020 typed refusal, extent/truncation validation | REAL — unit-pinned + executed |
| `clamp_u8` (1516-1518) | clamp | REAL |
| `open_audio_session` (1544-1577) | NullBackend stream + AudioPipeline + audio-master clock | REAL — executed |
| `wall_clock` (1586-1592) | system-monotonic fallback clock | REAL — honest `[PROPOSAL]` |
| tests (1594-1990) | converter vectors, reseeding pin, packet shape | REAL — in the 1001-pass suite |

## 5 Test honesty

- In-file unit tests (1594-1990) are strong: JFIF vectors, the four landed color pairs, odd sizes + truncated-plane typed corruption, the limited-range flip pin, and the ReseedingDecoder semantics pin (reseed, precedence, passthrough) — all part of the 1001-pass suite.
- apps/cli/tests/container_conformance.rs:426 `mp4_h264_family_lane` proves this file's h264 + BT.709-limited + DrainGated composition through the REAL binary (asserts absence of `cli.codec_unavailable` AND `cli.i420_color_unsupported`).
- apps/cli/tests/pipeline_accounting.rs drives `headless_accounting` over this file's consumer counters — in the suite.
- NOT covered by any test: the window-full hold path with an I420 frame (F-001's exact mechanism) — consistent with it being an admitted, unpinned defect.

## 6 Doc claims

- docs/feature-matrix.toml:428 (codec-video-h264-software evidence): "crates/codecs/src/video/h264/sw/** (codec-h264-sw, rust_h264 =0.6.1 …); engine e2e apps/cli mp4_h264_family_lane (mp4_h264_aac.mp4 plays to Ended, exit 0)" — CONSISTENT: this file is the provider arm that row's e2e depends on.
- docs/feature-matrix.toml:152: same e2e claim — CONSISTENT.
- The module doc (1-46) describes the mjpeg/pcm-first-vertical shape and BT.601-full-only conversion — STALE by one wave (the converter now carries four pairs per the 1418-1431 doc block and the 2026-11-15 extension note at 1427-1431 is accurate; the header was not updated to match). S4 doc drift, recorded.
- containers/mp4.rs:27-32 doc says "No software h264/hevc decoder is dispatched in this build: the blob is the forward-honest config, and `decoder_for` refuses the codec typed" — FALSE as of the 2026-11-15 h264 landing (pipeline_host.rs:328 dispatches it; the executed smoke decodes). That lie is filed under the demux_host/containers audit trail (the text lives in containers/mp4.rs).

## 7 Dead/duplicated/unreachable

- `QUEUE_CAPACITY`'s `None => loop {}` arm (F-003) — provably dead, documented.
- The `EnqueueOutcome::Full` arm of `enqueue_and_drain` (752-757) — "unreachable in this wiring" per its own comment; honored honestly by presenting directly rather than dropping.
- `DriftGovernor` arming path (`arm_drift_governor`) — never called by the production open (regime note is explicit and honest); harness-only, not dead code.
- `present_tap` — `None` in the pure CLI by design (byte-identical claim); the desktop drives it.

## 8 Dependencies & flags

`codec-h264-sw` IS in the shipped default feature set: crates/codecs/Cargo.toml:47-53 (`default = [ … "codec-h264-sw" … ]`), and apps/cli/Cargo.toml (`codecs = { path = "../../crates/codecs" }`, no feature restriction) resolves it. `H264SwDecoder` is compiled unconditionally for this build — crates/codecs/src/video/h264/mod.rs:55/60 gates it with `#[cfg(feature = "codec-h264-sw")]`, which is ON. The remaining externals (rav1d, rusty_vp9, filmcraft-dnx, symphonia-*, ruopus, claxon) ride their own default-on features; the vp9/dnxhd arms of this file therefore also compile in the shipped binary — making F-002's omission doubly wrong (they ARE in "this build").

## 9 Open questions

- F-001's complete fix (converted stash + render-loop pacing/backpressure) is registered as a Lead decision (W1-06 follow-up); no test pins the current broken behavior — was D-5's "no test weakened" claim audited anywhere? UNVERIFIED (outside my file set).
- `audio_backlog()` reads the ring through the CONSUMER's mutex from the decoder path (poll thread == pump thread, single-writer claim) — no contention in-model; a real audio backend on another thread would contend. Design-level, not a defect today.
