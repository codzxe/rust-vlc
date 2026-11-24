# AUDIT: apps/cli/src/pipeline_host.rs
| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-23 |
| Lines / functions | 2033 / ~62 (incl. 6 tests) |
| File verdict | REAL-UNPROVEN (registry + conversion + sink wiring genuine and reachable; KNOWN DEFECT at :1357-1386 ships a frozen I420 surface; could not execute binary in sandbox — build budget) |
| Reachable from shipped entry point? | YES — `lumen` main.rs -> run.rs dispatch -> host.rs:890 `open_session` -> :936 `CliFrameConsumer::new` + :948 `HostDecoderProvider::new` -> :952 `attach_consumer` -> `SessionConfig::new(..., provider, consumer, clock)` -> `engine.open(config)`. Executable proof: baseline mp4-smoke.md:57 `cli.codec_unavailable` error text is minted by THIS file's `other` arm (pipeline_host.rs:371-380). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 3 S4: 3 |
| Commands run | rg (multiple, exit 0); sed -n verification quotes (exit 0); od -c attr check (exit 0); baseline citation audit/runs/baseline/{mp4-smoke.md,cargo-test-workspace.txt} |

## 1. One-paragraph verdict
This is the real pipeline composition: `HostDecoderProvider::decoder_for` (pipeline_host.rs:236-385) dispatches 18 descriptors (mjpeg, flac, mp3/mp1, mp2, vorbis, opus, aac/aac-lc, alac, adpcm_*, av1/av01, vp9, dnxhd, h264/avc1, pcm_*) to genuine codec crates, seeded with container `extra_data` and wrapped in `ReseedingDecoder` for seek reconfigures; `CliFrameConsumer::accept` feeds audio to the null ring and video through a correct Q16 I420->RGBA converter (all four coefficient tables re-derived by this auditor and matched to BT.601/709 full/limited to 1e-5) into scheduler->queue->headless renderer. The MP4 answer (Wave 13 core question): an h264 track hits `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)` (:346) — rust_h264 software decode — and the decoded I420 frames land in the headless software `Renderer` (`Renderer::new_headless` :1314-1321, BGRA8-sRGB/Fifo) via `PresentationScheduler`/`PresentationQueue` (:1322), optionally mirrored to a `PresentTap` (the desktop webview). BUT: the WindowFull hold arm (:1357-1386) stashes the UNCONVERTED original frame — an admitted `[KNOWN DEFECT]` in the code itself — so any I420 lane whose decode burst fills the 16-slot window freezes the surface (`render.upload.format-unsupported` on every retried frame), and the file's own "landed:" error string (:376-377) omits vp9/dnxhd while host.rs's `engine_info()` hardcodes a two-decoder list that the same binary's registry contradicts.

## 2. What actually works (verified)
— verified by audit/runs/baseline/cargo-test-workspace.txt:3095-3100 -> all 6 unit tests `pipeline_host::tests::*` pass (i420 x4, reseeded wrapper, shared_bytes).
— verified by audit/runs/baseline/mp4-smoke.md:57-63 -> `lumen open sample-hevc.mp4` prints the exact `cli.codec_unavailable` text minted at pipeline_host.rs:372-379, proving `decoder_for` is on the shipped MP4 open path; mp4-smoke.md:8-20 shows h264+aac `sample.mp4` walking Idle->...->Playing->Stopped->Idle exit 0.
— verified numerically by this auditor (python-free hand check): BT601_FULL r_cr 91881/65536=1.40207, g_cb 22553/65536=0.34412, g_cr 46802/65536=0.71414, b_cb 116130/65536=1.77216; BT601_LIM 76309/65536=1.16438, 104597/65536=1.59603, 25675/65536=0.39176, 53279/65536=0.81297, 132201/65536=2.01723; BT709_FULL 103206/65536=1.57480, 12275/65536=0.18730, 30677/65536=0.46808, 121609/65536=1.85560; BT709_LIM 117489/65536=1.79274, 13975/65536=0.21325, 34925/65536=0.53291, 138438/65536=2.11240 — all match the spec equations in the doc comments.

## 3. Findings
F-001 [S2]
Where: apps/cli/src/pipeline_host.rs:1357-1386 (quote :1357-1362 and :1383-1384)
Code:
```
        PushOutcome::WindowFull { .. } if state.video_hold.len() < MAX_VIDEO_HOLD => {
            // Early, not late: hold the frame and retry after the next
            // scheduler tick (the pump moves time on `advance`; the
            // window drains as the clock advances).
            //
            // [KNOWN DEFECT, registered as the W1-06 follow-up card
```
```
            state.video_hold.push_back(frame.clone());
            Ok(())
```
What is wrong: the arm stashes `frame.clone()` — the ORIGINAL I420 frame — not the already-converted `uploadable` produced at :1339-1353; `retry_video_hold` (:1026-1047) later pushes it via `scheduler_push_for_hold` (:842-850), the scheduler schedules it, and the renderer refuses it (`render.upload.format-unsupported`), so the surface freezes. The code's own comment admits: "every hold-retried I420 frame is refused at the renderer ... and the surface freezes on an early window frame" and that "Main's pause/stop pacing is an artifact of this renderer refusal breaking the render loop early". Why it matters: the scheduler window is 16 slots and the module's own docs state "the decode stage runs ahead of the clock by design" — on h264/vp9/av1/mjpeg-I420 media a decode burst is the NORMAL case, so the shipped video surface can freeze while the state machine still reports Playing. What correct looks like: push the converted `uploadable` (the W1-06-proven fix the comment names), with the pacing/backpressure decision it exposes. Confidence: HIGH — Verification: read; runtime freeze UNVERIFIED (cargo-build-budget), but the defect is self-documented and untested (no test touches the WindowFull arm; test list :3095-3100 confirms).
F-002 [S2]
Where: apps/cli/src/pipeline_host.rs:1370-1375 (inside the F-001 arm's comment)
Code:
```
            // defect: `advance_time`'s render loop drains the whole
            // ring per pump, so with the renderer accepting frames one
            // `advance` tick plays the ENTIRE 2.000 s fixture (the
            // mid-media pause/stop scenarios become unreachable, and a
            // longer media would fatal `ring_full` under a
            // one-quantum-per-pump bound — the decode side has no
```
What is wrong: admitted latent fatal — a longer audio-bearing media can hit the typed `audio.ring_full` fatal because decode has no backpressure from the device ring (`ready_for` only parks audio past AUDIO_PARK_FILL :1242-1252, and video is always "ready"). Registered, deliberately kept ("keeps main's exact behavior"). Why it matters: `ring_full` is a session-fatal on valid input; the bound (131 072 frames ≈ 3 s) is a fixture-shaped constant (RING_FRAMES :118). Confidence: MEDIUM — Verification: read (comment self-admission); runtime UNVERIFIED.
F-003 [S3]
Where: apps/cli/src/pipeline_host.rs:372-379 (the catch-all arm; the "landed" literal is :376-377)
Code:
```
            other => Err(host_error(
                PipelineStage::Decode,
                "cli.codec_unavailable",
                format!(
                    "no decoder for `{other}` in this build (landed: mjpeg, pcm, flac, \
                     mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264)"
                ),
            )),
```
What is wrong: the "landed" list omits `vp9` and `dnxhd`, both of which have live dispatch arms (:313, :326); it also names `aac-lc` while the arm accepts `aac` too. The message tells a user with an undecodable track an inaccurate capability inventory. Why it matters: it is the string the shipped binary prints (baseline mp4-smoke.md:57) and it is hand-maintained — it already drifted within one file. What correct looks like: derive the list from the match (or a shared registry table). Confidence: HIGH — Verification: read + rg (arms at :313, :326, :304 vs string :376).
F-004 [S3]
Where: apps/cli/src/host.rs:1631-1660 (cross-file, owner: host.rs agent; recorded here because Wave 13's question is answered by this pair)
Code:
```
        decoders: vec![
            DecoderInfo {
                name: "mjpeg".to_string(),
                kind: TrackKind::Video,
```
What is wrong: `engine_info()` hardcodes exactly two decoders (mjpeg, pcm_s16le) while the same binary's `decoder_for` dispatches 18 descriptors including h264/av1/vp9 — baseline mp4-smoke.md:99 prints `decoders: mjpeg, pcm_s16le` and mp4-smoke.md:57 shows the same build decoding-registry-aware. engine-info's list is the one that lies; the "landed:" string (F-003) is closer to truth but also stale. Why it matters: user-visible capability reporting is false (under-reports) in a shipped introspection command. What correct looks like: derive from the provider registry. Confidence: HIGH — Verification: read + baseline lines 94-100.
F-005 [S3]
Where: apps/cli/src/bench.rs:300-334 (cross-file evidence for the MP4 trace; owner: bench.rs agent)
Code:
```
            MediaKind::Video if video.is_none() && fact.codec == "mjpeg" => {
```
What is wrong: `decode-bench` hardcodes mjpeg+pcm lanes and does NOT use `HostDecoderProvider` at all; its "no decoder in this build (first vertical: mjpeg + pcm)" note (bench.rs:334) is what produced baseline v_frames=0 on sample.mp4 — the binary CAN decode h264 (F-004 proof), the bench just never wires it. The baseline's "0 decoded frames" is a bench artifact, not an engine fact. Confidence: HIGH — Verification: read + baseline mp4-smoke.md:37-38.
F-006 [S4]
Where: apps/cli/src/pipeline_host.rs:1336-1338
Code:
```
    let Some(video) = state.video.as_mut() else {
        return Ok(());
    };
```
What is wrong: provably dead arm (the limb was just constructed at :1289-1332 one branch above); if it ever fired it would SILENTLY drop the frame (`Ok(())`, no counter). Dead now, silent-drop shape later. Confidence: HIGH — Verification: read.
F-007 [S4]
Where: apps/cli/src/pipeline_host.rs:140-145
Code:
```
#[allow(clippy::empty_loop)]
const QUEUE_CAPACITY: NonZeroUsize = match NonZeroUsize::new(4) {
    Some(value) => value,
    None => loop {}, // provably dead const arm (policy.rs precedent)
};
```
What is wrong: contorted const-nonzero dance for a literal 4 with an `#[allow(clippy::empty_loop)]`; `NonZeroUsize::new(4).unwrap()`-equivalent is a const-evaluable expression. Hygiene only. Confidence: HIGH — Verification: read.
F-008 [S4]
Where: apps/cli/src/pipeline_host.rs:739-745 (LatchingPresenter::repeat doc)
Code:
```
    /// This tick's `PresentAction` repeat bit. The queue holds at
    /// most the tick's own request in this wiring (one push per tick
    /// plus a full drain), so the action's bit describes every
    /// drained request; a future wiring that batches mixed actions
    /// must ride the bit per-request instead `[PROPOSAL]`.
    repeat: bool,
```
What is wrong: one repeat bit smeared over a batch drain — correct only under the "one request per tick" invariant that is not enforced anywhere in this file. Latent mislabel for a future wiring; documented as proposal. Confidence: MEDIUM — Verification: read.
Note: `#ust_use]` seen in raw tool output is a DISPLAY artifact of this sandbox's output filter (od -c of :176 proves the file bytes are `#[must_use]`). NOT a defect; recorded to prevent a future false finding.

## 4. Per-function table (functions >100 LOC file — every function)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| host_error | 149-156 | REAL | — | stage+code+io-source error mint; all host errors typed `Unsupported` even for corrupt-ish cases (stringly but honest) |
| HostDecoderProvider::new/attach_consumer | 177-190 | REAL | — | consumer probe slot for drain gate |
| HostDecoderProvider::seeded_config | 202-215 | REAL | — | seeds extra_data from MediaFacts; fact-less lane stays None (honest) |
| HostDecoderProvider::seeded | 219-234 | REAL | — | build->configure->ReseedingDecoder wrap; configure errors propagate typed |
| HostDecoderProvider::decoder_for | 236-385 | REAL | S3 | 18-descriptor registry; "landed" string drift (F-003); baseline-proven reachable |
| ReseedingDecoder::new/effective/configure/decode/flush | 414-461 | REAL | — | blob retention across generation bumps; unit-pinned (reseeded test) |
| DrainGatedDecoder::audio_backlog/configure/decode/flush | 470-526 | REAL | — | flush gate backlog>=RENDER_QUANTUM -> NeedMoreInput; sub-quantum residue release documented (6.FIX-pcm deadlock reasoning) |
| AudioLimb::clock_source/buffered_frames/ring_ready/written_frames | 557-592 | REAL | — | thin sink probes |
| AudioLimb::observe_drift | 613-627 | REAL | — | u128 ns math, rate 0 guarded, zero heap (as documented) |
| AudioLimb::reset_drift/arm_drift_governor/drift | 629-652 | REAL | — | passthroughs to audio_engine; governor NOT armed on production open path (documented regime note) |
| AudioLimb::stage | 659-669 | REAL | — | process->mix->take_output->sink.write; errors propagate |
| AudioLimb::drift_stats | 671-674 | REAL | — | observability |
| PresentTap::tap_present (trait) | 693-699 | REAL | — | desktop seam; CLI constructs none |
| LatchingPresenter::present | 747-757 | REAL | — | renderer.present -> latch write -> tap (success only, after latch); repeat bit smear F-008 |
| VideoLimb::tick_once | 768-776 | REAL | — | tick -> enqueue_and_drain; Background no-op |
| VideoLimb::enqueue_and_drain | 777-823 | REAL | — | Full->direct render (honest), Stale->drop+count, drain error propagates with delivered count |
| VideoLimb::render_request | 825-840 | REAL | — | direct present; latch+tap+count |
| VideoLimb::scheduler_push_for_hold | 842-850 | REAL | S2 | pushes held (UNCONVERTED) frames — F-001's second half |
| VideoLimb::seek_boundary | 860-864 | REAL | — | queue.publish(generation) |
| VideoLimb::scheduler_stats/last_presented_ns/staging_stats/queue_stats/published_generation | 866-905 | REAL | — | probes; last_presented_ns generation-gated (correct staleness rule) |
| VideoLimb::screenshot_png | 909-920 | REAL | — | readback+encode, typed errors |
| ConsumerState::video/audio | 991-1000 | REAL | — | accessors |
| ConsumerState::tick_video | 1010-1024 | REAL | — | tick then hold retry; error propagates |
| ConsumerState::retry_video_hold | 1026-1046 | REAL | S2 | retries unconverted frames (F-001); dual refused/frames_refused counters consistent (:1040-1043) |
| ConsumerState::note_video_error/observe_audio_drift/audio_drift_stats | 1049-1074 | REAL | — | first-error-wins recording |
| ConsumerState::seek_boundary | 1095-1128 | REAL | — | hold retain gen>=mint, queue publish, drift reset, audio floor arm |
| ConsumerState::audio_stale_prefix | 1130-1156 | REAL | — | write-order prefix arithmetic (saturating); logic matches 6.FIX-ogg-reanchor doc |
| CliFrameConsumer::new/lock/set_present_tap/take_video_error/last_presented_ns | 1158-1226 | REAL | — | poison-recovering lock (documented), state-safe tap install |
| FrameConsumer::accept | 1229-1240 | REAL | — | audio/video dispatch; Subtitle silently Ok(()) (documented not-wired) |
| FrameConsumer::ready_for | 1242-1252 | REAL | S2 | video always true — relies on hold bound; F-002 backpressure gap |
| accept_audio | 1254-1286 | REAL | — | floor gate drop+count, change-point bookkeeping before staging |
| accept_video | 1288-1400 | REAL | S2 | lazy limb build; I420 convert; WindowFull unconverted stash F-001; dead else F-006 |
| convert_i420_to_rgba | 1475-1557 | REAL | — | 4 coeff tables verified; bounds-checked planes; BT.2020 typed refusal; odd-dims ceil geometry correct |
| clamp_u8 | 1559-1561 | REAL | — | trivial |
| open_audio_session | 1587-1627 | REAL | — | null backend, audio-master clock over sink counter |
| wall_clock | 1629-1636 | REAL | — | system monotonic fallback (documented non-determinism) |
| tests (6) | 1638-2033 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| i420_conversion_matches_jfif_vectors | YES | exact pixel vectors incl. clamp cases, ±1 tolerance pinned |
| i420_conversion_covers_the_landed_color_pairs | YES | studio-swing black/white exact, 709-lim mid-gray ±1, BT.2020 refusal code asserted |
| i420_conversion_validates_and_handles_odd_sizes | YES | odd 7x5 geometry + truncated-plane typed Corrupt assert |
| i420_conversion_accepts_limited_range | YES | pins the 2026-11-15 behavior flip (130,130,130 ±1) |
| reseeded_generation_reconfigure_replays_the_open_path_blob | YES | RecordingDecoder observes inner configs; blob replay, precedence, fact-less passthrough all asserted |
| shared_bytes_roundtrip | NO (tautology) | asserts SharedBytes==itself + constructs a Packet; self-declared "compile-level pin" — acceptable as such, counts for nothing behaviorally |
| GAP | — | NO test exercises the WindowFull hold arm, retry_video_hold, or scheduler_push_for_hold — F-001's freeze path is entirely untested (mutation: stashing converted frames would change zero test outcomes) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "[`HostDecoderProvider`] answers the engine's per-track [`DecoderProvider`] seam with the real builtin decoders: MJPEG (2.B4) as-is, and PCM (2.B5)" | pipeline_host.rs:5-9 | FALSE (stale) | decoder_for :236-385 wires 18 descriptors; doc describes the first-vertical era |
| "an I420→RGBA host conversion (BT.601 full-range, the JFIF equations" | pipeline_host.rs:23-26 | PARTIALLY FALSE (stale) | convert now covers 601/709 × full/limited (:1477-1490); doc predates the 2026-11-15 extension (its own test pins the flip) |
| "landed: ... av1, h264" | pipeline_host.rs:376 | SHALLOW-TRUE | h264/av1 arms exist (:346, :311) and binary proves registry reachability (baseline :57); list omits vp9/dnxhd (F-003) |
| "decoders: mjpeg, pcm_s16le" | host.rs engine_info (baseline :99) | FALSE | same binary dispatches h264 (F-004) |
| "Zero heap traffic (D-1/D-8): integer arithmetic only" | pipeline_host.rs:611-612 (doc on observe_drift) | REAL | integer arithmetic only, verified by read |

## 7. Dead / duplicated / unreachable code
- :1336-1338 dead else arm (F-006).
- `frames_refused` duplicated at consumer + limb level (both incremented together at :1387-1388 and :1040-1043 — intentional dual observability, not a defect).
- No dead decoders: all 18 arms reachable via the open path (baseline proves the dispatcher reached).
- `AudioLimb::arm_drift_governor` has no caller in this file (composition-side arming per doc; callers live in the sync harness — UNVERIFIED outside this file).

## 8. Dependencies & flags
Codecs (mjpeg/pcm/flac/mp3/mp2/vorbis/opus/aac/alac/adpcm/av1/dnx/h264/vp9), audio_engine, audio_output (NullBackend), renderer_wgpu (headless), video_engine, media_clock, media_types, playback_engine, testkit (dev). No feature gates in this file — the registry is unconditional in the shipped lumen-cli build (Cargo.toml check out of scope here; release lane builds these per baseline exit 0).

## 9. Open questions for the next agent
1. host.rs owner: fix/flag engine_info() F-004 (S3 cross-file). 2. bench.rs owner: decode-bench hardcodes mjpeg/pcm — the baseline "v_frames=0" MP4 evidence is bench-shaped, NOT an engine gap (F-005). 3. Does any integration test drive a WindowFull burst on an I420 lane? None found in this file; host.rs scenario tests may mask it via the renderer-refusal early break (F-001 comment says main's pause/stop pacing is an artifact of it). 4. Who arms the drift governor in production? None in this file.
