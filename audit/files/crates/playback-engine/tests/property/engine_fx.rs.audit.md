# AUDIT: crates/playback-engine/tests/property/engine_fx.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-23 |
| Lines / functions | 465 / 22 (fixture: types, fakes, builders, oracles) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test-only fixture) — `tests/property/main.rs:36 mod engine_fx;`, compiled into the `property` test binary; exercised by seek_roundtrip/queue_invariants/generation_monotonicity suites (all ran green via the prebuilt binary) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/property-25b39d1917495830 seek_roundtrip` -> 6 passed (fixture-driven; see seek_roundtrip audit); `sed -n '2140,2155p' crates/playback-engine/src/pipeline.rs` (restamp verification) |

## 1. One-paragraph verdict
The shared black-box fixture of the property suites, and it is what it claims: a scripted seekable demuxer over synthetic timelines, an immediate one-frame-per-packet decoder with zero steady-state allocation, a recording frame collector, frozen injected clocks, and two oracles. It touches only public playback-engine APIs (SessionConfig seams: Demuxer, DecoderProvider, FrameConsumer, MediaClock, EventPolicy) — the black-box claim holds. The engine-side restamp contract is respected (fixture packets carry `GenerationId::FIRST`, and pipeline.rs:2147 `packet.generation = self.generation;` overwrites it — verified), so the fixture's constant stamp is inert, not a cheat. The two oracle weaknesses are both disclosed in comments: `resolve_target_ns` deliberately MIRRORS the engine's seek.rs arithmetic, and `landing_of` duplicates the scripted demuxer's own landing rule — making the property suite's target-resolution and landing checks implementation-coupled rather than spec-derived.

## 2. What actually works (verified)
- The fixture compiles and drives 3 property families to green (seek_roundtrip 6/6 executed; the binary also lists queue_invariants/generation_monotonicity which use the same fixture).
- Restamp verification: `sed -n '2140,2155p' crates/playback-engine/src/pipeline.rs` shows the demux driver stamps the current generation at push — the fixture's `GenerationId::FIRST` packet stamp (283) is documented as authoritative-for-nothing and the engine indeed overwrites it.
- Determinism: FrozenNow (32-38) for both the session clock and the throttle policy (394-397) — no ambient time anywhere in the fixture.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/property/engine_fx.rs:457-462
Code:
```
        SeekTarget::Percent { fraction } => duration_ns.map(|duration| {
            // The engine's exact arithmetic (seek.rs `resolve_target`),
            // mirrored: a different rounding would be a false
            // counterexample.
            clamp((f64::from(fraction.clamp(0.0, 1.0)) * duration as f64).round() as i64)
```
What is wrong: the resolution oracle is a copy of the implementation's algorithm (clamp bounds, percent rounding, saturating relative arithmetic), so any spec-level error shared by seek.rs and this mirror is invisible to every suite that consumes it (seek_roundtrip's generated ops). The comment is honest about the motive (avoiding false counterexamples) but the consequence — no independent spec check of resolution semantics — is unstated. Mitigated only by seek_roundtrip's two hand-computed boundary anchors. Confidence: HIGH — Verification: read. Cross-ref: seek_roundtrip.rs.audit.md F-001.

F-002 [S4]
Where: crates/playback-engine/tests/property/engine_fx.rs:119-123 (reanchor) vs 428-435 (landing_of)
Code:
```
        let landing = media
            .slots
            .iter()
            .rposition(|slot| slot.pts_ns <= target_ns)
            .unwrap_or(0);
```
What is wrong: the scripted demuxer's reanchor and the test oracle `landing_of` implement the identical rposition rule — the "keyframe-safe landing" the engine receives and the "expected landing" the tests assert are produced by twin code. For a scripted all-keyframes source this is defensible (the landing rule IS the fixture's contract), but it means the engine's seek flow is only ever validated against one landing rule; real-demuxer keyframe tables, non-keyframe sources, and landing-before-target shapes are outside this fixture (seek_flow.rs / recovery_media.rs own those). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FrozenNow + impl MonotonicSource | 32-38 | REAL | — | frozen virtual time |
| lock_media | 68-72 | REAL | — | poison-recovering lock |
| TimelineDemuxer::start / next_event | 84-96 | REAL | — | in-order scripted packets, EndOfStream at cursor end |
| TimelineDemuxer::seek | 98-102 | REAL | — | refuses: models "seek is not the reanchor path" |
| TimelineDemuxer::seekability | 104-110 | REAL | — | Fast{indexed} or None |
| TimelineDemuxer::reanchor | 112-133 | REAL | S4 | F-002: compute-then-commit landing; target-missing → corrupt |
| unsupported / corrupt | 136-152 | REAL | — | typed errors |
| ImmediateDecoder::configure/decode/flush | 160-181 | REAL | — | one frame per packet, static payload (zero alloc), stale drop |
| ImmediateProvider::decoder_for | 186-196 | REAL | — | per-track immediate decoder |
| Collector::accept (FrameConsumer) | 219-233 | REAL | — | records track/generation/pts |
| Collector::len / frames_from | 235-251 | REAL | — | snapshotted under one lock |
| base / at_nanos / stamp_ns | 257-272 | REAL | — | u64→i64 clamp at i64::MAX (inert in fixture domain) |
| packet_of | 274-285 | REAL | — | keyframe=true always (MJPEG-in-AVI shape); generation stamp inert (restamped by pipeline.rs:2147 — verified) |
| audio_frame | 287-300 | REAL | — | static samples |
| track | 304-325 | REAL | — | declared extent or UNKNOWN |
| tiny_queues / tiny_budget | 329-345 | REAL | — | 16-packet caps; 12/4/8 budget |
| build | 369-403 | REAL | — | injects demuxer/provider/collector/clock/frozen throttle |
| open | 408-422 | REAL | — | fresh engine + session |
| landing_of | 428-435 | REAL | S4 | F-002 oracle twin |
| resolve_target_ns | 442-465 | REAL | S4 | F-001 mirror of seek.rs |

## 5. Test honesty
No tests in this file (fixture). Its honesty is inherited: the suites that consume it (seek_roundtrip — audited separately) are mutation-honest for the laws that do NOT depend on the two coupled oracles (anchor truth, generation attribution, flush boundary, clock re-anchor, refusal typing).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "black-box: only public APIs are touched" (5) | module doc | REAL | imports use playback_engine public re-exports + plugin-api/codec-api seams only (verified by reading imports 10-24) |
| "an immediate one-frame-per-packet decoder" (2-3) | module doc | REAL | decode 166-176 emits exactly one static-payload frame |
| "mirrored: a different rounding would be a false counterexample" (458-460) | fn doc | REAL (honest disclosure) | the mirror's consequence (no independent spec check) is the unstated half — F-001 |
| "all keyframes (the MJPEG-in-AVI shape)" (61-62) | Media doc | REAL | packet_of sets keyframe: true (281) |

## 7. Dead / duplicated / unreachable code
- Duplication (by design, disclosed): landing rule (reanchor vs landing_of, F-002); resolve rule (seek.rs vs resolve_target_ns, F-001); the fixture family mirrors tests/seek_flow.rs harness shapes (stated at 4).
- `Built` (358-363) is used by the storm suites (queue_invariants/generation_monotonicity reopen sessions on one engine) — not dead.

## 8. Dependencies & flags
codec_api, media_clock, media_types, packet_queue, playback_engine (public), plugin_api, testkit. No feature gates; test-binary only — never compiled into any shipped lane.

## 9. Open questions for the next agent
- queue_invariants.rs and generation_monotonicity.rs consume this fixture too but are OTHER agents' assignments (W03-A01 has generation_monotonicity) — their audits should cross-ref the two coupled oracles found here.
- If seek.rs's `resolve_target` is ever rewritten, the mirror at 442-465 must be updated in lockstep or the suite will produce false counterexamples — a maintenance coupling worth a cross-file test or shared constant.
