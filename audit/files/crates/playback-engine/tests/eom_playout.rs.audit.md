# AUDIT: crates/playback-engine/tests/eom_playout.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2025-06-14 |
| Lines / functions | 316 / 12 (9 harness items + 2 tests + base helper) |
| File verdict | REAL — a genuine engine-level regression test for the F5 defect (video-only end-of-media firing at drain-completion instead of media playout); both tests executed directly from the prebuilt binary and pass |
| Reachable from shipped entry point? | N/A (test file). The behavior it pins — the clock-gated drain (`is_drained` must relate to the presentation clock for video-only sessions) — runs in every shipped session (engine drain path); video-only opens in the shipped CLI hit it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/eom_playout-e37121e17e14b67b` -> `2 passed; 0 failed ... finished in 0.00s`, exit 0 (prebuilt binary run directly; cargo rebuild blocked by disk-full) |

## 1. One-paragraph verdict
This is a textbook regression test: the module header names the measured defect (mp4 corpus 0115 reported end-of-media at 0.111 s of a 4.0 s media — 2.8% — through `advance 512`), the root cause (drain = pure state inspection, no presentation-clock relation for a video-only limb with no audio renders), and the fixture reproduces exactly the defect's stage (frozen clock, 100 × 1 ms frames that exhaust long before the 99 ms last pts). Test 1 asserts the engine STAYS Playing with queues/decoders fully drained and the clock at 0, then ends only after the clock hand passes the last frame; test 2 is the honest complement proving the gate never wedges an already-presented single-frame media. Both pass (direct binary run). Only cosmetic notes.

## 2. What actually works (verified)
- verified by running the prebuilt test binary directly: `test video_only_end_of_media_waits_for_the_clock_to_present_the_last_frame ... ok` and `test a_short_video_media_whose_first_frames_cover_the_clock_ends_at_drain ... ok`, exit 0.
- Baseline agrees (audit/runs/baseline/cargo-test-workspace.txt, eom_playout section).
- Mutation reasoning (no edit): if the drain gate regressed to pure state inspection, test 1's 400-poll drain loop would exit `Playing` (Ended at drain time) and the `assert_eq!(engine.state(), SessionState::Playing)` at line 265-270 would fail with the F5 message. If the gate over-wedged, test 2 would fail. Both directions are pinned.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/eom_playout.rs:142-157
Code:
```
    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        out.accept(Frame::Video(VideoFrame {
```
What is wrong: the fake decoder emits unconditionally and skips the stale-packet guard (`packet.is_stale(self.generation)`) that the shared `tests/common` FakeDecoder carries. Harmless here — no generation bumps occur in either test — but the harness would silently mis-behave if reused for seek/switch scenarios.
Confidence: HIGH — Verification: read + compared tests/common/mod.rs:144-157.
F-002 [S4]
Where: crates/playback-engine/tests/eom_playout.rs:228-236
Code:
```
fn has_end_of_media(engine: &mut Engine) -> bool {
    engine.take_outputs().iter().any(|output| {
```
What is wrong: `take_outputs()` discards receipts as a side effect of the event probe — fine in this two-test file (no commands submitted after open), a reuse hazard in general.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| micros_base / at_micros | 40-46 | REAL | — | µs timebase helpers |
| SharedNow + MonotonicSource | 48-56 | REAL | — | the frozen clock is the defect's stage |
| video_track | 60-75 | REAL | — | video-only, mjpeg descriptor, UNKNOWN span |
| video_script | 79-96 | REAL | — | 1 ms-spaced packets, 96-byte payloads, EOS tail |
| ScriptDemuxer (start/next_event/seek/seekability) | 98-128 | REAL | — | pop-front script; unseekable typed; poison-tolerant lock |
| VideoDecoder (configure/decode/flush) | 132-162 | REAL | S4 | F-001: no staleness guard (unused here) |
| VideoProvider | 164-176 | REAL | — | one decoder per track |
| Collector | 179-193 | REAL | — | saturating frame counter (not a Vec — no harness leak) |
| open_video_frozen | 197-226 | REAL | — | frozen throttle source; real SessionConfig |
| has_end_of_media | 228-236 | REAL | S4 | F-002: probing consumes outputs |
| video_only_end_of_media_waits_for_the_clock_to_present_the_last_frame | 238-295 | REAL | — | the F5 regression: stays Playing through full drain at clock 0, no EOM event; ends within 40 polls after the clock passes 99 ms |
| a_short_video_media_whose_first_frames_cover_the_clock_ends_at_drain | 297-316 | REAL | — | anti-wedge complement: single frame at pts 0 ends at drain |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| video_only_end_of_media_waits... | YES | the exact pre-fix behavior (EOM at drain time) trips `assert_eq!(state, Playing)` + the no-EOM-event assert; mutation direction verified by reasoning |
| a_short_video_media... | YES | an over-corrected gate (never ends without clock advance) fails the 400-poll Ended assert |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| F5 defect: eom at 0.111043 s of a 4.0 s media through `advance 512` | eom_playout.rs:4-7 | REAL (as recorded defect) | the regression fixture reproduces the stage; the worklog/issue #16 is the primary record — UNVERIFIED beyond this file (docs/perf out of my scope) |
| "Audio limbs are exempt ... DrainGatedDecoder note in the CLI host" | eom_playout.rs:13-15 | REAL (cross-ref) | apps/cli/src/pipeline_host.rs decode-park machinery exists (read during host.rs audit) |

## 7. Dead / duplicated / unreachable code
- None. Every harness item is used by the two tests.

## 8. Dependencies & flags
- Dev-deps: codec_api, media_clock, media_types, playback_engine, plugin_api, testkit. No features. The engine path under test is the shipped drain machinery.

## 9. Open questions for the next agent
- Where is the FIX itself? The clock-gated drain should live in pipeline.rs/session.rs (`is_drained` + last-frame pts gate) — that file's auditor should confirm the gate consults the clock for video-only limbs and cite this test as its regression pin.
- The F5 record says the defect was measured on the mp4 corpus via the CLI `advance` path — an apps-level E2E regression (real mp4 fixture, real CLI) does not exist here; engine-level only. If a matrix row claims "EOM playout verified end-to-end", check for an apps/cli test citing corpus 0115.
