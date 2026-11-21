# AUDIT: apps/cli/src/pipeline_host.rs#part3 (lines 1288-2033)
| Field | Value |
|---|---|
| Auditor | W13-A05 |
| Date | 2026-11-21 |
| Lines / functions | 746 audited (of 2033; chunk boundary pinned at the natural seam: part3 = `fn accept_video` (1288) → EOF; part1 = 1-674 (A06), part2 = 675-1287 (A07)) / 19 items (8 non-test + 11 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → host.rs:890 `open_session` → host.rs:926 `open_audio_session` / host.rs:933 `wall_clock` / host.rs:936 `CliFrameConsumer::new` → engine `open` (host.rs:983) wires the consumer → playback-engine poll thread → `FrameConsumer::accept` (1228) → `accept_video` (1288) → `convert_i420_to_rgba` (1342). EXECUTED: mp4-smoke-open-session-420.txt (h264 video track, Buffering 1.00 → Playing, EXIT=0), mp4-smoke-open-session-aac.txt (audio limb + null session built, Playing), rc-smoke-engine-only.log.txt:24 (AVI MJPEG lane, command-layer-play PASS)) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 4 S4: 1 |
| Commands run | `Read` over the full 2033 lines (context) + part3 in detail; `rg -n "^(pub )?(async )?fn ..." ` (item map); `rg "pub enum PushOutcome" -A12 crates/video-engine/src/scheduler.rs` (WindowFull hands the frame back, 120-127); `rg "DEFAULT_MAX_PENDING" crates/video-engine/src/policy.rs` (=16, line 174; `for_frame_duration` uses it, 241-251); `rg "packets_per_limb" crates/playback-engine/src/pipeline.rs` (default 16, line 473); `rg "format-unsupported|uploads_format" crates/renderer-wgpu/src` (renderer.rs:239-241 accepts only Rgba8/Bgra8; error.rs:159); `rg "pub enum ColorMatrix|ColorRange" crates/media-types/src/frame.rs` (exactly Bt601/Bt709/Bt2020Ncl × Limited/Full — match at 1476 exhaustive); `rg "PixelFormat::I420|BT709_LIMITED" crates/codecs/src/video/{h264/sw.rs,mjpeg/mod.rs}` (h264 sw emits I420@BT709_LIMITED sw.rs:407/438; mjpeg emits I420@Bt601 mod.rs:18/217); `rg "open_audio_session|wall_clock|CliFrameConsumer::new" apps/cli/src` (callers: host.rs:926/933/936); `rg "take_video_error|note_video_error" -C10 apps/cli/src/host.rs` (degrade path sets last_fatal, host.rs:835-841); `rg "frame_ns" apps/cli/src/containers/*.rs` (mp4.rs:217 `frame_ns: None`); `rg "\.resize\(" apps/cli/src` (Renderer::resize never called); `rg -ln "video_hold|WindowFull|MAX_VIDEO_HOLD" apps/cli` (only pipeline_host.rs — no test coverage); `rg "pipeline_host::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (all 6 tests ok, lines 2881-2886); python3 Q16 coefficient recomputation (all 4 tables match spec equations to ±1 LSB); `sed -n` verification of every quote below. |

## 1. One-paragraph verdict
This chunk is the video half of the frame consumer plus the audio-session composition root, and it is real, reachable, and executed — the I420→RGBA converter is bounds-checked, its four Q16 coefficient tables recompute exactly against the BT.601/BT.709 spec equations, and its unit vectors ran green in the baseline (orchrun log 2881-2884), while `open_audio_session` demonstrably ran in every audio-bearing smoke session. But this chunk carries the traced answer to this wave's hostile question for the MP4 **video** half: the window-full hold arm stashes the UNCONVERTED original frame (a defect the code itself registers as the W1-06 card), so once the 16-slot scheduler window overflows — which the 16-packets-per-limb decode burst guarantees on any I420 video, i.e. every h264 4:2:0 MP4 and every MJPEG AVI — every hold-retried frame is refused by the RGBA-only renderer, the pump records a fatal video error, and the surface freezes while audio plays on to an honest EXIT=0. The baseline smoke cannot see this (its `quit` arrives instantly and no per-frame evidence is printed), and the repo has zero tests constructing a WindowFull push at any level, so the defect — admitted in prose since W1-06 — remains unfixed and un-failable.

## 2. What actually works (verified)
— `convert_i420_to_rgba` EXECUTED in the shipped binary: the AVI first-vertical lane (MJPEG → I420, codecs/src/video/mjpeg/mod.rs:18) played through the full lifecycle — rc-smoke-engine-only.log.txt:21-24 (`v_decoded=60`, command-layer-play PASS) — and the 420 MP4 reached Playing with a Video h264 track (mp4-smoke-open-session-420.txt:12); every I420 frame on both lanes passes through this function (1342) before the RGBA-only renderer (renderer.rs:239-241).
— Coefficient tables verified by independent recomputation: BT601_FULL r_cr=91881 (1.402·65536=91881.5), g_cb=22553 (22554.06), g_cr=46802 (46802.05), b_cb=116130 (116129.8); BT601_LIMITED y_scale=76309 (76309.4), r_cr=104597 (104596.9), g_cb=25675 (25674.7), g_cr=53279 (53278.7), b_cb=132201 (132201.5); BT709_FULL r_cr=103206 (103206.9), g_cb=12275 (12274.9), g_cr=30677 (30677.1), b_cb=121609 (121609.9); BT709_LIMITED r_cr=117489 (117489.3), g_cb=13975 (13975.2), g_cr=34925 (34924.7), b_cb=138438 (138438.9). All within ±1 LSB of the spec equations quoted in the const docs.
— Conversion unit tests EXECUTED and mutation-resistant: `i420_conversion_matches_jfif_vectors` / `_covers_the_landed_color_pairs` / `_validates_and_handles_odd_sizes` / `_accepts_limited_range` all ok — orchrun-test-workspace-nofailfast.log.txt:2881-2884. Mutation reasoning: swapping Cb/Cr or breaking a table flips the blue vector's `assert_eq!(b, 255)` / red vector's `assert_eq!(b, 50)`; truncating the length check breaks the `Truncated { expected: 96, found: 10 }` assert.
— `open_audio_session` EXECUTED: the AAC-only session built an audio limb over the null device (INDEX.md corrected matrix: "Buffering → Playing → Stopped (audible output: none — null device)"; mp4-smoke-open-session-aac.txt: buffering 1.00 → Playing, EXIT=0) — the ring, pipeline, and audio-master clock all constructed through this function (host.rs:926).
— `reseeded_generation_reconfigure_replays_the_open_path_blob` EXECUTED ok (orchrun log 2885) and is mutation-resistant: if `ReseedingDecoder::effective` stopped re-seeding, the second `configure` would record `None` and the `seen.iter().all(... == [7u8;16])` assert fails.

## 3. Findings

### F-001 [S2] The window-full hold stashes the UNCONVERTED frame — every I420 video's surface freezes once the 16-slot scheduler window overflows (the registered W1-06 defect; the concrete MP4 video-lane failure point)
Where: apps/cli/src/pipeline_host.rs:1357-1385 (defect line 1383)
Code:
```
        PushOutcome::WindowFull { .. } if state.video_hold.len() < MAX_VIDEO_HOLD => {
            // …
            // [KNOWN DEFECT, registered as the W1-06 follow-up card
            // (Lead decision required): this arm stashes the
            // UNCONVERTED original (`frame.clone()`), so every
            // hold-retried I420 frame is refused at the renderer
```
What is wrong: the arm discards the CONVERTED frame — `PushOutcome::WindowFull { frame, .. }` hands the pushed (already-converted) frame back untouched (video-engine/src/scheduler.rs:120-127: "The frame is handed back untouched") — and instead stashes `frame.clone()`, the pre-conversion original (pipeline_host.rs:1383). For I420 frames (every h264 4:2:0 MP4 per codecs/src/video/h264/sw.rs:407, and the AVI MJPEG lane per codecs/src/video/mjpeg/mod.rs:18), the hold-retry path (`retry_video_hold` → `scheduler_push_for_hold`, 1026-1049) then pushes I420 into the scheduler; at present time the RGBA-only renderer refuses it (`render.upload.format-unsupported` — renderer-wgpu/src/renderer.rs:239-241 `uploads_format` accepts only Rgba8/Bgra8, error.rs:159), `tick_once` propagates the error, and the pump records it via `note_video_error` → `last_fatal` (host.rs:835-841) — video degraded, audio continues. The mechanism is load-bearing and unavoidable: the scheduler window is 16 (`DEFAULT_MAX_PENDING`, video-engine/src/policy.rs:174, used by `SchedulerCfg::for_frame_duration` at policy.rs:249), the engine's decode burst is 16 packets per limb per poll (`TickBudget::default()`, playback-engine/src/pipeline.rs:473), and during `Buffering` the session clock is frozen while decode continues (this file's own RING_FRAMES comment, 108-112) — so any I420 video longer than 16 frames overflows the window and parks frames in the hold. The 90-frame sample-420.mp4 parks ~64 frames in the hold (MAX_VIDEO_HOLD=64) and refuses the rest at 1386-1389.
Why it matters: this is the exact point where the owner's MP4's VIDEO half falls out: 4:2:0 h264 decodes fine, converts fine, schedules fine — and then the surface freezes on the first hold-retried present, while the session still walks Playing → Stopped → Idle with EXIT=0 (mp4-smoke-open-session-420.txt:12-15 — which cannot show the freeze: `quit` is piped instantly and no per-frame/present evidence is printed). The same defect hits the advertised AVI first-vertical lane. Not S0/S1: playback state machinery and audio are unaffected, no crash, and the defect is admitted in-code with a registered follow-up card (W1-06) whose full fix (pacing/backpressure) is a Lead decision; not S3: it is wrong user-visible behavior in the primary realistic case (any I420 video > 16 frames).
What correct looks like: bind `PushOutcome::WindowFull { frame, .. }` and stash the handed-back CONVERTED frame (`state.video_hold.push_back(converted)`), paired with the pacing/backpressure decision the comment names (one-quantum-per-pump bound or engine-level decode backpressure) so the ring-full fatal the naive fix exposes cannot fire.
Confidence: HIGH — Verification: read (all five links in the chain traced: stash 1383 → retry 1026-1049 → scheduler hand-back scheduler.rs:120-127 → renderer refusal renderer.rs:239-241 + error.rs:159 → degrade host.rs:835-841); runtime engagement NOT directly evidenced by any baseline artifact (no per-frame logs; smoke's instant `quit`) — UNVERIFIED as an observed freeze, but the in-code comment states "W1-06 proved it".

### F-002 [S3] `scheduler_push_for_hold`'s contract comment is false for the I420 lane — "the hold stores already-converted frames"
Where: apps/cli/src/pipeline_host.rs:840-842 (contract; violated by 1383 in this part)
Code:
```
    /// Push one held frame into the scheduler (the hold-retry path —
    /// no conversion: the hold stores already-converted frames).
    fn scheduler_push_for_hold(&mut self, frame: VideoFrame) -> HoldRetry {
```
What is wrong: the doc comment (part2's range, but the violation is committed by this part's stash at 1383) asserts an invariant the code does not uphold: for I420 frames the hold stores the UNCONVERTED original (F-001). Any future reader trusting this comment while touching the hold/retry path will reason from a false invariant — exactly the kind of comment that makes the W1-06 fix harder than it needs to be.
Why it matters: comments are load-bearing for maintainers; this one contradicts the KNOWN DEFECT comment 540 lines below it in the same file. The file argues with itself.
What correct looks like: either the comment states the actual (defective) state with a pointer to the W1-06 card, or the stash is fixed and the comment becomes true.
Confidence: HIGH — Verification: read of both sites (sed-verified above).

### F-003 [S3] No mid-stream resolution-change handling: the renderer surface extent is fixed at the FIRST frame and `Renderer::resize` is never called
Where: apps/cli/src/pipeline_host.rs:1310-1321
Code:
```
        let extent = Extent2D {
            width: frame.size.0,
            height: frame.size.1,
        };
        let renderer = Renderer::new_headless(
            extent,
```
What is wrong: the VideoLimb is built once, from the first decoded frame's size, and never resized — `rg "\.resize\(" apps/cli/src` finds no `Renderer::resize` caller (the only hit is an unrelated `wav.resize` in host.rs:1797). A stream whose picture size changes mid-playback (valid H.264: SPS change; the realistic adaptive/live case) is handled only by `compute_placement`'s letterbox math (renderer-wgpu/src/params.rs:210+), i.e. the new resolution is scaled/cropped into the stale surface extent, silently, with no typed signal and no `PresentationScheduler` reconfiguration either.
Why it matters: wrong-but-quiet presentation geometry on a valid input class; the screenshot path (`screenshot_png`, 909-919) reports the stale extent as truth.
What correct looks like: detect `frame.size != renderer.extent()` at accept time and call `Renderer::resize` (renderer.rs:132) — the seam exists and is unused by this host.
Confidence: HIGH for the code fact (read + grep); MEDIUM for real-world frequency (no fixture in the repo changes resolution mid-stream — UNVERIFIED whether any decoder lane can emit it).

### F-004 [S3] `convert_i420_to_rgba` allocates 4·W·H per frame with no host-side extent cap — file-controlled dimensions drive an unbounded allocation
Where: apps/cli/src/pipeline_host.rs:1531
Code:
```
    let mut rgba = Vec::with_capacity(width * height * 4);
```
What is wrong: `width`/`height` derive from the decoded frame (`frame.size`, u32 pair — 1489), i.e. ultimately from the container/SPS. There is no extent ceiling anywhere on this path (the h264 sw decoder's `emit_frame` likewise allocates `total = 1.5·W·H` uncapped, sw.rs:391-401), so a hostile SPS declaring extreme dimensions makes this line attempt a multi-GiB allocation → abort. On 64-bit the multiplication itself cannot overflow (u32·u32 ≤ 2^64), so the failure mode is the allocator's, not a wrap.
Why it matters: defense-in-depth on the present-stage path; the first OOM point is upstream (decoder slot), but this line adds a second, larger (4× vs 1.5×), per-frame transient allocation with no bound of its own.
What correct looks like: a host-side extent ceiling (the `SchedulerCfg`/renderer already have their own shape gates) refusing typed above e.g. 8192×8192, mirroring the typed-refusal posture used everywhere else in this file.
Confidence: HIGH — Verification: read + grep for any cap (`rg "MAX|cap|limit" crates/codecs/src/video/h264/sw.rs` — none on dims); no crash reproduced (would need a hostile SPS — not run; sandbox forbids builds).

### F-005 [S3] The hold-retry path has zero test coverage anywhere in the repo — the registered defect is also un-failable
Where: apps/cli/src/pipeline_host.rs:1357-1385 (untested arm); evidence `rg -ln "video_hold|WindowFull|MAX_VIDEO_HOLD" apps/cli` → only pipeline_host.rs itself
Code:
```
        PushOutcome::WindowFull { .. } | PushOutcome::UnschedulablePts { .. } => {
            video.refused = video.refused.saturating_add(1);
            state.frames_refused = state.frames_refused.saturating_add(1);
            Ok(())
        }
```
What is wrong: no test in apps/cli (unit or integration) constructs a `PushOutcome::WindowFull` or touches `video_hold` — the identifiers appear only in this file. The in-code defect note says the integration scenarios "keep their bug-masked shape" (1380-1382), i.e. the suite was shaped around the defect rather than pinning either behavior. Consequently the W1-06 fix (stashing the converted frame) can land or regress with identical test outcomes — the defect is invisible to CI by construction.
Why it matters: a registered, load-bearing defect with no failing test is a defect that can silently return; "no test was weakened" is technically true and practically worthless when no test ever covered the arm.
What correct looks like: a consumer-level test: a fake clock + a VideoLimb-less ConsumerState path or a seam-extracted scheduler, driving >16 frames, asserting the hold drains and the frames that reach the presenter are Rgba8.
Confidence: HIGH — Verification: rg across apps/cli (src + tests) — only self-references.

### F-006 [S4] `shared_bytes_roundtrip` tests media-types, not this module
Where: apps/cli/src/pipeline_host.rs:1890-1904
Code:
```
    /// A media-types packet field sanity check for the imports this
    /// module's public surface touches (compile-level pin).
    #[test]
    fn shared_bytes_roundtrip() {
        let bytes = SharedBytes::from_vec(vec![1, 2, 3]);
        assert_eq!(bytes.as_slice(), &[1, 2, 3]);
```
What is wrong: the assertions exercise `SharedBytes` (media-types' own unit territory) and the `_packet_shape` binding is dead; nothing in pipeline_host is under test. The name is honest ("compile-level pin") but the test would pass with every function in this file broken.
Why it matters: suite noise presented as coverage; 6 tests report for this file, only 5 constrain its behavior.
What correct looks like: delete it or pin something this module owns (e.g. `host_error`'s stage/code wiring).
Confidence: HIGH — Verification: read; baseline orchrun log 2886 shows it passing alongside real tests.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `accept_video` | 1288-1393 | REAL | S2 (F-001) | Lazy VideoLimb construction from first frame (extent never resized — F-003); format routing 1340-1352 honest (Nv12/Yuyv422 typed refusal at Present); WindowFull hold arm stashes unconverted original (F-001); MP4 lane always uses the 33_333_333 fallback cadence (mp4.rs:217 `frame_ns: None` — documented VFR posture; scheduler DropPolicy/lead for every MP4 is a 30 fps guess, no observability distinguishes it); defensive `let Some(video) … else return Ok(())` at 1336-1338 provably dead (just constructed); saturating counters throughout; no panics (no unwrap/index beyond bounds-checked planes). |
| `struct YuvToRgb` | 1403-1412 | REAL | — | Data-only (6 Q16 fields); documented equations match usage at 1541-1544. |
| `const BT601_FULL` | 1417-1424 | REAL | — | Coefficients recomputed: all within ±1 LSB of JFIF equations (see §2). |
| `const BT601_LIMITED` | 1429-1436 | REAL | — | Recomputed: matches BT.601-7 §2.5.3 studio swing (±1 LSB). |
| `const BT709_FULL` | 1429-1436 → 1440-1447 | REAL | — | Recomputed: matches BT.709 full-range (±1 LSB). |
| `const BT709_LIMITED` | 1452-1459 | REAL | — | Recomputed: matches BT.709-6 §3 studio swing (±1 LSB). |
| `convert_i420_to_rgba` | 1475-1557 | REAL | S3 (F-004) | Exhaustive (matrix,range) match — enum verified to be exactly 3×2 variants (frame.rs:131-146), Bt2020Ncl typed refusal 1481-1487; zero-extent typed corrupt 1490-1498; ceil chroma geometry 1499-1502; truncation typed 1514-1525; OpaqueHandle typed 1505-1511; per-pixel Q16 math overflow-safe (max ≈34.4M < i32::MAX); nearest co-located chroma upsample documented as v0 convention; unbounded 4·W·H allocation (F-004); odd sizes covered by test (7×5). |
| `clamp_u8` | 1559-1561 | REAL | — | Trivial and correct (`i32::clamp(0,255) as u8` — no truncation possible after clamp). |
| `struct AudioSession` | 1569-1578 | REAL | — | Data holder; consumed by host.rs:921-928. |
| `open_audio_session` | 1587-1620 | REAL | — | EXECUTED (aac-only smoke: null device played). Hardcodes `tracks: 1` in PipelineSpec — matches the open path's single-audio-limb composition (host.rs:914-918 `find` first audio geometry); multi-audio-track media would need engine-side selection (UNVERIFIED how the engine wires multiple audio limbs — open question, engine wave territory). Audio-master clock anchored at seek target. Typed errors propagated, none swallowed. |
| `wall_clock` | 1629-1635 | REAL-UNPROVEN | — | Reachable (host.rs:933, audio-less open); no baseline transcript isolates a video-only session, so no pasted output proves execution — genuine but unexecuted in evidence. |
| `tests::i420_frame` | 1646-1666 | REAL | — | Helper; builds well-formed I420 with tagged BT.601-full color. |
| `tests::pixel` | 1668-1680 | REAL | — | Helper; OpaqueHandle arm returns empty slice (safe). |
| `tests::i420_conversion_matches_jfif_vectors` | 1684-1726 | REAL | — | Mutation-resistant (§2); executed ok (orchrun 2883). |
| `tests::i420_conversion_covers_the_landed_color_pairs` | 1732-1832 | REAL | — | Pins all 4 pairs + BT.2020 refusal code; executed ok (orchrun 2882). |
| `tests::i420_conversion_validates_and_handles_odd_sizes` | 1836-1864 | REAL | — | Odd 7×5 + typed Truncated{96,10}; executed ok (orchrun 2884). |
| `tests::i420_conversion_accepts_limited_range` | 1871-1886 | REAL | — | Behavior-flip pin (2026-11-15); executed ok (orchrun 2881). |
| `tests::shared_bytes_roundtrip` | 1890-1904 | SHALLOW | S4 (F-006) | Tests media-types; no subject behavior. |
| `tests::RecordingDecoder` + `recorded` + `test_config` | 1910-1955 | REAL | — | Test scaffolding; poison-recovery without unwrap is deliberate (comments honest). |
| `tests::reseeded_generation_reconfigure_replays_the_open_path_blob` | 1964-2032 | REAL | — | Subject is part1's `ReseedingDecoder` (414-460) — cross-part coverage; mutation-resistant; executed ok (orchrun 2885). |

## 5. Test honesty (tests in this chunk)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| i420_conversion_matches_jfif_vectors | YES | Swapped Cb/Cr, wrong table, or broken rounding flips the red/blue vector asserts (1698-1725). |
| i420_conversion_covers_the_landed_color_pairs | YES | Studio-swing black/white exactness (1770, 1785-1789) pins y_scale/y_offset; BT.2020 refusal code pinned (1831). |
| i420_conversion_validates_and_handles_odd_sizes | YES | Odd-size chroma geometry (1843-1844) and Truncated{expected:96,found:10} (1853-1861) pin the length check and ceil math. |
| i420_conversion_accepts_limited_range | YES | Y=128→130±1 pins the studio-swing scale (1882-1885). |
| shared_bytes_roundtrip | NO | Asserts media-types behavior only; every function in this file could be deleted and it still passes (F-006). |
| reseeded_generation_reconfigure_replays_the_open_path_blob | YES | Header-less second configure must carry the retained blob (1992-1997); precedence and fact-less passthrough pinned (2008-2031). |
| (hold path — no test) | — | Zero coverage: WindowFull/hold identifiers exist only in this file (F-005). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "video: an I420→RGBA host conversion (BT.601 full-range … RGBA-native frames pass through unconverted)" | module docs, this file 23-28 | PARTIALLY FALSE (stale) | The converter now covers 601/709 × full/limited (1475-1488, tests 1732-1832); the module doc still says "BT.601 full-range" only. Doc drift, not code drift. |
| "the hold stores already-converted frames" | this file 841 | FALSE | Contradicted by 1383 (F-002); the file's own KNOWN DEFECT note says so. |
| "4:2:0 H.264+AAC MP4 reaches Playing and exits 0" | wave-13 prompt / INDEX corrected matrix | REAL | mp4-smoke-open-session-420.txt:12-15 — and this chunk shows why the surface can still be frozen (F-001): no present-path evidence exists in any baseline artifact. |
| "audio goes to a null discard sink" | wave-13 prompt | REAL | open_audio_session (1587-1620) → NullBackend ring; INDEX: "audible output: none — null device". |

## 7. Dead / duplicated / unreachable code
- `accept_video`'s `let Some(video) = state.video.as_mut() else { return Ok(()); }` (1336-1338) is provably dead (the limb was just constructed at 1323-1334) — harmless defensive style, noted, not counted.
- Nothing else dead in the chunk; all public items (`AudioSession`, `open_audio_session`, `wall_clock`) have live callers (host.rs:921-933).

## 8. Dependencies & flags
- Consumed crates (all in-bounds per boundaries.toml for lumen-cli): audio-engine, audio-output, codec-api, codecs, media-clock, media-types, playback-engine, renderer-wgpu, video-engine, testkit (dev). No feature gates inside this chunk (`#[cfg(test)]` only). No unsafe. Saturating arithmetic on all counters (presented/refused/frames_accepted/stale drops).
- `QUEUE_CAPACITY` (part1, 139-143) used at 1322 — NonZeroUsize const-match with dead `loop{}` arm, `#[allow(clippy::empty_loop)]` — part1's territory, noted for A06.

## 9. Open questions for the next agent
- A07 (part2): the `video_hold` doc contradiction (F-002) lives in your range at 841-842 — please cross-reference F-001 when auditing `retry_video_hold` (1026-1049).
- Engine wave / W13-A06 (host.rs part1): does any baseline artifact pin `wall_clock` execution (video-only open)? Also: how does playback-engine wire limbs for MULTI-audio-track media — one limb per track into the single `AudioLimb` (mix) or first-track-only (host.rs:914-918 `find` suggests the latter)?
- Whoever owns the integration scenarios: does any scenario outlive `quit` long enough to present a hold-retried frame? If not, the W1-06 defect has never been observed end-to-end in CI (F-005) — REQUEST-ORCH-RUN candidate: a scripted `open sample-420.mp4` session that sleeps >3 s before `quit` and then dumps `screenshot` + queue/scheduler stats (`lumen` interactive commands permitting), to turn F-001's read-verified mechanism into an executed observation.
