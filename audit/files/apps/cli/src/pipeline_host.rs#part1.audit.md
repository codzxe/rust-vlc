# AUDIT: apps/cli/src/pipeline_host.rs#part1 (lines 1–1148)
| Field | Value |
|---|---|
| Auditor | W13-A06 |
| Date | 2026-11-21 |
| Lines / functions | 1148 (of 2033; part2 = 1150–2033, A07) / 61 items |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → run.rs → host.rs open path → `HostDecoderProvider::new`/`attach_consumer` (host.rs:948,936) + `CliFrameConsumer` pump hooks (host.rs:600–631); container test modules construct it directly) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "^(pub \|)(async )?(fn\|struct…" pipeline_host.rs` (function inventory); `rg HostDecoderProvider\|CliFrameConsumer\|open_audio_session apps/cli/src` (reachability → host.rs:926–948 + 12 container test mods); `rg ready_for\|packets_per_limb crates/playback-engine/src` + Read pipeline.rs:2300–2463 (park-gate is per-packet inside the 16-packet loop → AUDIO_PARK_FILL proof holds); `rg format-unsupported\|I420 crates/renderer-wgpu/src` (renderer refuses I420: staging.rs:275, renderer.rs:240/316); `rg arm_drift_governor apps/` (test-only caller); `rg W1-06 docs/worklog` (registered defect card); `sed -n` verbatim quote checks |

Chunk boundary note: part1 = lines 1–1148 (module docs, constants, `host_error`, `HostDecoderProvider`, `ReseedingDecoder`, `DrainGatedDecoder`, `AudioLimb`, `PresentTap`, `VideoLimb`, `LatchingPresenter`, `MAX_VIDEO_HOLD`/`HoldRetry`, `ConsumerState` through `audio_stale_prefix`). Lines 1150–2033 (`CliFrameConsumer`, `accept_audio`, `accept_video`, I420→RGBA conversion, `AudioSession`, `open_audio_session`, `wall_clock`, tests) are part2 (A07). Cross-boundary facts used here were read, not assumed.

## 1. One-paragraph verdict
This is the CLI's real pipeline composition root and it is genuinely wired: the decoder dispatch answers 15 codec families, the audio limb stages into the null sink, the video limb drives scheduler→queue→renderer, and the production open path (host.rs:926–948) constructs all of it. The hostile trace — argv → open → `HostDecoderProvider::decoder_for` → `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)` (line 346) — shows the 4:2:0 MP4 lane is fully reachable, and the 4:4:4 refusal surfaces from the decoder itself (baseline `mp4-smoke-open-session.txt`: `error [decode.unsupported] … h264.sw.unsupported`), not from a gap here. Two real defects: a load-bearing doc comment that states the opposite of the code (the hold-retry path), and a capability message that under-reports the dispatch table. The registered W1-06 freeze defect lives in part2's line range but is triggered through part1's retry function — cross-referenced below so it cannot be lost.

## 2. What actually works (verified)
- AUDIO_PARK_FILL sizing proof (lines 120–134) verified against the engine: `pipeline.rs:2343-2348` consults `consumer.ready_for(MediaKind::Audio)` inside the per-packet loop (`for _ in 0..budget.packets_per_limb`, pipeline.rs:2317), before each pop — so worst case is park fill 32 768 + one 64 KiB 8-bit-mono packet burst 65 536 = 98 304 < RING_FRAMES 131 072. The proof's premise ("the gate runs BEFORE a packet pop") is true. — verified by reading both sites.
- `ReseedingDecoder` re-seeding logic (lines 414–460): incoming `None` + retained `Some(blob)` → reseed; caller's own blob wins; passthrough otherwise. Consistent with `seeded()` (219–232) which retains exactly the blob the open path used. — verified by reading.
- Reachability: `HostDecoderProvider`/`CliFrameConsumer` constructed on the production open path (host.rs:936–948: `open_audio_session` → `CliFrameConsumer::new` → `HostDecoderProvider::new`), and from 12 container test modules (wav.rs:190, ts.rs:598/973, mp4.rs:487, adts.rs:218/406/603, ogg.rs ×6, ps.rs:603/855, es.rs:659/678, ivf.rs:431/661, mkv.rs:417, aiff.rs:179). — verified by `rg`.
- 4:2:0 MP4 lane: dispatch (line 346) + avcC seeding via `seeded_config` (202–210) is the code path the corrected baseline (`INDEX.md` CORRECTION v2, `mp4-smoke-open-session-420.txt` Buffering → Playing → Stopped → Idle) exercises. — verified by citing baseline + tracing dispatch.

## 3. Findings

### F-001 [S3]
Where: apps/cli/src/pipeline_host.rs:840-849
Code:
```rust
    /// Push one held frame into the scheduler (the hold-retry path —
    /// no conversion: the hold stores already-converted frames).
    fn scheduler_push_for_hold(&mut self, frame: VideoFrame) -> HoldRetry {
        match self.scheduler.push(frame) {
```
What is wrong: The doc comment asserts "the hold stores already-converted frames" as a present-tense invariant. The hold does the opposite: part2 line 1383 (`state.video_hold.push_back(frame.clone());`) stashes the UNCONVERTED original — the code's own `[KNOWN DEFECT … W1-06 follow-up card]` note (part2 lines 1362–1382) states "this arm stashes the UNCONVERTED original (`frame.clone()`), so every hold-retried I420 frame is refused at the renderer (`render.upload.format-unsupported`) and the surface freezes". The renderer refusal is real: `crates/renderer-wgpu/src/staging.rs:275` (`if !matches!(format, PixelFormat::Rgba8 | PixelFormat::Bgra8)`) refuses I420, confirmed by renderer.rs:316 test.
Why it matters: A reader auditing the retry path in part1 is told the one invariant that makes `scheduler_push_for_hold` correct ("no conversion" is safe). It is false; this function is the delivery path of a registered surface-freeze defect (S2-class, registered at docs/worklog/archive-2026-10.md:7461, deliberately kept per D-5 pending the Lead's pacing decision). Any I420 media whose decode burst outruns the 16-frame scheduler window (the documented default decode-ahead regime, lines 947–951) freezes the surface here.
What correct looks like: The comment must state the actual behavior ("the hold currently stashes the unconverted original — registered W1-06 defect; retried I420 frames are refused by the RGBA-only renderer") or the hold must stash the converted frame (the fix the register says is blocked on the pacing decision).
Confidence: HIGH — Verification: read (code at 1383 + renderer staging.rs:275 + worklog card) / baseline-cited (no MP4 render-level run exists; see Open questions).

### F-002 [S3]
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
```
What is wrong: The capability list is hard-coded and contradicts the dispatch table 60 lines above it: `vp9` (lines 313–317) and `dnxhd` (lines 326–330) are dispatched but absent from the message; the message says `aac-lc` while the arm (line 288) matches both `aac` and `aac-lc`; `mp1` (line 261) and `av01` (line 304) aliases are also unlisted.
Why it matters: This is the user-facing answer to "why won't my file play" and it misreports the build's capability — the same defect family as the hard-coded `lumen engine-info` decoder list already on the baseline's standing-defects register (INDEX.md CORRECTION v2: engine-info "under-reports the real registry"). A user with a VP9 file is told VP9 isn't landed when the code above would have decoded it.
What correct looks like: Derive the message from the same table that dispatches (or at minimum keep the literal in sync); better, a `const LANDED: &[&str]` the match arms reference.
Confidence: HIGH — Verification: read (dispatch arms 241–346 vs message 376–377).

### F-003 [S4]
Where: apps/cli/src/pipeline_host.rs:633-648
Code:
```rust
    pub fn arm_drift_governor(&mut self, target_ns: u64) {
        self.pipeline.arm_drift_governor(target_ns);
    }
```
What is wrong: The drift-governor arming path is dead in every shipped composition: the only caller of `arm_drift_governor` in the repo is a test (`apps/cli/tests/sync_scenarios.rs:93`), and `DRIFT_TARGET_FILL_NS` (line 101) is referenced only by that test. The production pump calls `observe_audio_drift` (host.rs:618, 735) every quantum, which samples latency and discards it — `DriftCorrection::ZERO` forever.
Why it matters: Per-quantum no-op work plus ~90 lines of governor plumbing (633–648, 591–631) that no shipped entry point activates; A/V drift correction effectively does not ship. Mitigating: the code says so honestly (lines 607–609 "the production open path leaves the limb unarmed").
What correct looks like: Either the pump arms the governor once 3.B3 pacing is steady-state (the comment's own condition), or the dead path is flagged `#[cfg(test)]`-adjacent / removed until then.
Confidence: HIGH — Verification: `rg arm_drift_governor apps/` → single test caller; read of host.rs call sites.

### F-004 [S4]
Where: apps/cli/src/pipeline_host.rs:149-156
Code:
```rust
fn host_error(stage: PipelineStage, code: &'static str, message: impl Into<String>) -> EngineError {
    EngineError::new(
        stage,
        ErrorKind::Unsupported { format: None },
```
What is wrong: Every host-minted error is typed `ErrorKind::Unsupported` regardless of semantics — `cli.pcm_facts_absent` (353), `cli.codec_unavailable` (372), `cli.audio_limb_absent` (part2:1259, Mix stage), `cli.pixel_format_unconverted` (part2:1344, Present stage) all surface as "unsupported".
Why it matters: Error taxonomy lies to the wire layer: a composition bug ("an audio frame arrived with no audio limb wired") is reported to the user as an unsupported format. Limited blast radius (the dotted code disambiguates for grepping), hence S4.
What correct looks like: Map internal wiring gaps to a `Bug`/`Internal` kind (or at least pass the format name for the genuinely-unsupported cases).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-47 | REAL | — | Accurate map of the composition; the "hold stores already-converted frames" claim lives at 841 (F-001), not here |
| `RENDER_QUANTUM` | 90-92 | REAL | — | 1470 ≈ 33.33 ms @44.1 kHz; consumed by host.rs pump |
| `DRIFT_TARGET_FILL_NS` | 94-101 | REAL (test-only use) | S4 | F-003: referenced only by sync_scenarios.rs:93 |
| `RING_FRAMES` | 103-118 | REAL | — | 131 072; sizing invariant verified (see §2) |
| `AUDIO_PARK_FILL` | 120-135 | REAL | — | Proof premise verified against pipeline.rs:2343 |
| `QUEUE_CAPACITY` | 137-143 | REAL | — | `loop {}` arm provably dead; `#[allow(clippy::empty_loop)]` documented precedent |
| `host_error` | 145-156 | REAL | S4 | F-004: everything is `Unsupported` |
| `HostDecoderProvider` (struct) | 162-172 | REAL | — | facts + optional consumer probe |
| `HostDecoderProvider::new` | 174-182 | REAL | — | trivial, correct |
| `attach_consumer` | 184-189 | REAL | — | pre-wiring contract; host.rs:936-948 honors it |
| `seeded_config` | 191-210 | REAL | — | clones config per call (cold path, fine) |
| `seeded` | 212-232 | REAL | — | configure-gate stays the codec's; wraps in ReseedingDecoder |
| `decoder_for` | 235-381 | REAL | S3 | 15 families dispatched; F-002 message drift; `"mp1"`→Mp3Decoder is sound (mp3 header.rs handles Layer I); HEVC correctly falls to refusal (baseline hevc run) |
| `ReseedingDecoder` (struct) | 384-418 | REAL | — | — |
| `ReseedingDecoder::new` | 420-425 | REAL | — | trivial |
| `ReseedingDecoder::effective` | 427-438 | REAL | — | precedence: caller blob > retained; correct |
| `ReseedingDecoder::configure` | 441-447 | REAL | — | reseed on header-less reconfigure |
| `ReseedingDecoder::decode` | 449-455 | REAL | — | passthrough |
| `ReseedingDecoder::flush` | 457-460 | REAL | — | passthrough |
| `DrainGatedDecoder` (struct) | 462-473 | REAL | — | PCM-lane only (see decoder_for 347-371) |
| `DrainGatedDecoder::audio_backlog` | 475-491 | REAL | — | locks consumer state; no-limb → 0; brief critical section, no deadlock (pump/poll alternate on same thread) |
| `DrainGatedDecoder::configure` | 493-496 | REAL | — | passthrough |
| `DrainGatedDecoder::decode` | 498-504 | REAL | — | passthrough |
| `DrainGatedDecoder::flush` | 506-537 | REAL | — | `Drained`→`NeedMoreInput` while backlog ≥ quantum; sub-quantum residue release documented with the measured 6.FIX-pcm case; bounded per tick by engine (pipeline.rs:2452) |
| `AudioLimb` (struct) | 544-552 | REAL | — | — |
| `AudioLimb::clock_source` | 555-559 | REAL | — | sink counter; used by open_audio_session clock |
| `AudioLimb::buffered_frames` | 561-566 | REAL | — | drain-gate truth |
| `AudioLimb::ring_ready` | 568-581 | REAL | — | `<= AUDIO_PARK_FILL` matches doc ("holds MORE than this" parks); fixes the settle-loop ring_full fatal |
| `AudioLimb::written_frames` | 583-589 | REAL | — | seek-prefix arithmetic input |
| `AudioLimb::observe_drift` | 591-624 | REAL | S4 | u128 arithmetic safe; governor never armed in prod (F-003) so this is a sampled-and-discarded no-op |
| `AudioLimb::reset_drift` | 626-631 | REAL | — | called at seek_boundary |
| `AudioLimb::arm_drift_governor` | 633-648 | REAL | S4 | F-003: test-only caller |
| `AudioLimb::drift` | 643-648 | REAL | — | observability |
| `AudioLimb::stage` | 650-666 | REAL | — | track 0 hard-coded — consistent with `PipelineSpec { tracks: 1 }` (part2:1600-1604); rate/channel mismatch → typed refusal (pipeline.rs:288-293) |
| `AudioLimb::drift_stats` | 668-673 | REAL | — | — |
| `PresentTap` (trait) | 676-700 | REAL | — | desktop-only implementor (frame_tap); CLI constructs none — verified by rg (only desktop/src references) |
| `VideoLimb` (struct) | 702-724 | REAL | — | — |
| `LatchingPresenter` (struct) | 726-744 | REAL | — | repeat-bit caveat honestly flagged `[PROPOSAL]` |
| `Presenter::present` (LatchingPresenter) | 746-758 | REAL | — | latch written only on success; tap failure does not unwrite latch |
| `VideoLimb::tick_once` | 760-775 | REAL | — | one action per tick |
| `VideoLimb::enqueue_and_drain` | 777-823 | REAL | — | Full arm → direct render (honest); Stale → drop untapped; delivered-count on error path preserved |
| `VideoLimb::render_request` | 825-838 | REAL | — | duplicates LatchingPresenter::present body (bypass path) — minor duplication |
| `VideoLimb::scheduler_push_for_hold` | 840-849 | REAL | S3 | F-001: false invariant comment; pushes unconverted frames into an RGBA-only renderer |
| `VideoLimb::seek_boundary` | 851-862 | REAL | — | queue publish only; scheduler self-purges |
| `VideoLimb::scheduler_stats` | 864-868 | REAL | — | — |
| `VideoLimb::last_presented_ns` | 870-881 | REAL | — | generation-gated latch; correct staleness rule |
| `VideoLimb::staging_stats` | 883-887 | REAL | — | — |
| `VideoLimb::queue_stats` | 889-895 | REAL | — | — |
| `VideoLimb::published_generation` | 897-902 | REAL | — | — |
| `VideoLimb::screenshot_png` | 904-919 | REAL | — | typed error mapping |
| `MAX_VIDEO_HOLD` | 922-927 | REAL | — | 64; `[PROPOSAL]` honest |
| `HoldRetry` (enum) | 929-939 | REAL | — | — |
| `ConsumerState` (struct) | 941-986 | REAL | — | single-mutex model documented |
| `ConsumerState::video` | 989-993 | REAL | — | — |
| `ConsumerState::audio` | 995-999 | REAL | — | — |
| `ConsumerState::tick_video` | 1001-1020 | REAL | — | error propagates before hold retry (pump breaks anyway) |
| `ConsumerState::retry_video_hold` | 1022-1046 | REAL | S3 | clone-per-iteration is Arc-cheap (FrameStorage::Cpu(SharedBytes)); feeds F-001 path |
| `ConsumerState::note_video_error` | 1048-1053 | REAL | — | first-error-only, documented |
| `ConsumerState::observe_audio_drift` | 1055-1062 | REAL | S4 | no-op in prod (F-003) |
| `ConsumerState::audio_drift_stats` | 1064-1072 | REAL | — | — |
| `ConsumerState::seek_boundary` | 1074-1109 | REAL | — | hold retain by generation; floor armed after caller's prefix flush (contract — see open questions) |
| `ConsumerState::audio_stale_prefix` | 1111-1147 | REAL | — | write-order arithmetic; change-point bookkeeping consistent with accept_audio (part2:1275-1283) |

## 5. Test honesty
No tests inside part1's line range (tests start at 1638, part2). `apps/cli/tests/sync_scenarios.rs` exercises the drift machinery — it would fail if the governor were broken, but it tests a path no shipped composition arms (F-003).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "audio: … straight mix → the null sink's ring" (no device) | module docs 19-22 | REAL | code 650-666 + baseline "audible output: none — null device" (INDEX.md standing defects) |
| "video: I420→RGBA host conversion … RGBA-native frames pass through unconverted" | module docs 23-29 | REAL for the direct path; FALSE for the hold path | accept_video converts (part2:1340-1342) but hold stashes unconverted (part2:1383) — F-001 |
| "The CLI binary constructs no tap, so its behavior stays byte-identical" | module docs 29-31 + 679-681 | REAL-UNPROVEN | rg shows no CLI construction site; byte-identity not measured (no run available) |
| "131 072 frames … absorbs a full rebuffer cycle with >2× margin" | RING_FRAMES doc 103-117 | REAL | arithmetic + engine gate verified (§2) |
| "the gate runs BEFORE a packet pop" | AUDIO_PARK_FILL doc 126-127 | REAL | pipeline.rs:2343-2348 inside per-packet loop |

## 7. Dead / duplicated / unreachable code
- `arm_drift_governor`/`DRIFT_TARGET_FILL_NS`/`drift_stats`/`drift`/`observe_drift`: production-dead, test-live (F-003).
- `render_request` duplicates `LatchingPresenter::present` (justified bypass, noted).
- `PresentTap` trait: no CLI implementor (by design; desktop implements).

## 8. Dependencies & flags
No feature gates in this file. Imports pull codecs (h264 sw, aac, adpcm, alac, flac, mp2, mp3, opus, vorbis, av1, dnx, vp9), audio-engine, audio-output (null), renderer-wgpu, video-engine, media-clock, playback-engine — all default-on per baseline build (build-release-workspace.log.txt exit 0). No license conflicts introduced (cargo-deny licenses OK, INDEX.md #25).

## 9. Open questions for the next agent
1. (A07, part2) The `[KNOWN DEFECT]` at 1362–1382: does any baseline run show the freeze? The AVI and 4:2:0 MP4 open transcripts show no `event: error` — either the hold never engaged in those runs (pump pacing) or `last_fatal` (host.rs:839) is not printed as an event. Needs a run with present-count telemetry: REQUEST-ORCH-RUN candidate — see log.
2. (host.rs agent) Contract check: `seek_boundary` (1095) must be called AFTER the caller computes `audio_stale_prefix` (1130) — verify order in host.rs's seek path.
3. (host.rs agent) Does `surface_video_error`'s `last_fatal` reach the wire as an `event: error` line? If not, present-path failures are invisible in transcripts.
4. (bench.rs agent) decoder_for handles h264 — decode-bench prints "no decoder in this build" from its own fall-through (INDEX.md standing defect), unrelated to this dispatch.
