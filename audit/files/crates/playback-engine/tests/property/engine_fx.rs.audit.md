# AUDIT: crates/playback-engine/tests/property/engine_fx.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 465 / 16 items (fixture: demuxer, decoder, provider, collector, builders, oracles) |
| File verdict | REAL-UNPROVEN (with one §3.4 honesty caveat: the seek-target oracle mirrors the engine's own math) |
| Reachable from shipped entry point? | N/A — shared test fixture for tests/property/ (queue_invariants, seek_roundtrip, timestamp_arithmetic). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | `cat` full file (exit 0); cross-ref queue_invariants.rs:36 vs tiny_budget (341). No cargo. |

## 1. One-paragraph verdict

A competently built black-box fixture: a scripted seekable demuxer (compute-then-commit reanchor, keyframe-safe landing), an immediate one-frame-per-packet decoder that honors generation staleness, a recording collector, a frozen monotonic clock (deterministic windows, no sleeps), small queue caps and tick budgets. The seek-side oracle has two halves with different epistemic value: `landing_of` (the landing model) is fixture-internal truth about where a seek should land given the scripted timeline — legitimate; `resolve_target_ns` (the target-resolution oracle) is an admitted **mirror of the engine's own seek.rs arithmetic** ("The engine's exact arithmetic … mirrored: a different rounding would be a false counterexample") — which makes the property suites structurally blind to resolution-math bugs: an engine off-by-one cloned into the oracle passes. The frozen clock also means these suites never exercise a running clock (consistent with, and reinforcing, the shipped-CLI frozen-clock finding in my session.rs audit).

## 2. What actually works (verified)

- `TimelineDemuxer` (83-134): in-order packet emission, EndOfStream at cursor end, `seek()` refusing typed (forcing the engine down the `reanchor` path — 98-102), and a compute-then-commit `reanchor` (landing ≤ target, cursor set only on success) that matches contract D4's shape.
- `ImmediateDecoder` (160-181): stale-packet short-circuit via `packet.is_stale(self.generation)`, one frame per packet carrying pts + generation — the honest minimal decoder.
- `Collector` (214-251): poison-recovering locks, snapshot-under-one-lock accessors.
- `build` (369-403): every nondeterminism source (clock, throttle) frozen; queue/budget knobs small and explicit.
- `landing_of` (428-435): independent landing oracle — same `rposition` semantics as the fixture demuxer (that is the model, and the engine is the thing under test against it).

## 3. Findings

### F-001 [S3] `resolve_target_ns` is a mirror of the engine's implementation, not an independent oracle
Where: crates/playback-engine/tests/property/engine_fx.rs:442-465
Code (457-462, verbatim):
```rust
        SeekTarget::Percent { fraction } => duration_ns.map(|duration| {
            // The engine's exact arithmetic (seek.rs `resolve_target`),
            // mirrored: a different rounding would be a false
            // counterexample.
            clamp((f64::from(fraction.clamp(0.0, 1.0)) * duration as f64).round() as i64)
        }),
```
What is wrong: the comment concedes the oracle was written by reading seek.rs, to avoid "false counterexamples". Consequence: any resolution defect in seek.rs that is faithfully mirrored (clamp bounds, saturation shape, percent rounding direction, the Chapter→None refusal) is invisible to every property suite that uses this helper — the test would agree with the bug. This is §3.4's "tests that mock the very thing they claim to test", scoped to one sub-step (the seek flow's target resolution; the landing and generation halves are independently modeled).
Why it matters: seek correctness claims backed by "property-tested seek round-trips" overstate — the round-trip verifies engine-consistency for resolution, and engine-correctness only for landing.
What correct looks like: the oracle written from doc 05 §2.4's prose ("clamping is truth") with its own arithmetic, and any engine/oracle divergence adjudicated by hand once, not erased at authorship.
Confidence: HIGH — Verification: read; seek.rs itself is another agent's file (the mirror claim is the fixture's own).
Severity: S3 — a real honesty gap in the strongest test assets the engine has.

### F-002 [S4] The frozen clock means the property suites never exercise a running clock
Where: engine_fx.rs:372-379 (`MediaClock::system_monotonic(Arc::new(FrozenNow(AtomicU64::new(0))), …)`) and 394-397 (frozen throttle)
What is wrong: nothing for determinism — but every property case runs with `now() == 0` forever: no time-ticks fire (session.rs's `is_due` never true), `video_played_out` gates against a frozen position, and pause/resume clock effects are never observed under movement. The suites prove the poll-driven machine, not the timed one. This matches the shipped CLI's frozen-clock behavior (my session.rs F-001) — the tested configuration IS the shipped one, which is honest coverage of reality but leaves the wall-clock path (desktop pacer) untested here.
Confidence: HIGH — Verification: read.

### F-003 [S4] Manual constant sync between suites: `DEMUX_EVENTS_PER_POLL` vs `tiny_budget`
Where: engine_fx.rs:341 (`demux_events: 12`) vs queue_invariants.rs:36 (`const DEMUX_EVENTS_PER_POLL: u64 = 12`)
What is wrong: the fabrication bound's correctness depends on two constants in two files agreeing; nothing asserts it. A future budget change to the fixture silently falsifies (or loosens) the bound. One `assert_eq!(DEMUX_EVENTS_PER_POLL, tiny_budget().demux_events)` in the consumer would pin it.
Confidence: HIGH — Verification: read both files.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FrozenNow` | 32-38 | REAL | — | frozen monotonic source |
| `Slot`/`Media` | 45-66 | REAL | — | adversarial dts shapes documented (pts<dts allowed) |
| `lock_media` | 68-72 | REAL | — | poison recovery |
| `TimelineDemuxer::{start,next_event,seek,seekability,reanchor}` | 83-134 | REAL | — | seek() typed refusal; compute-then-commit reanchor |
| `unsupported`/`corrupt` | 136-152 | REAL | — | typed error constructors |
| `ImmediateDecoder::{configure,decode,flush}` | 160-181 | REAL | — | stale short-circuit; 1:1 packet→frame |
| `ImmediateProvider::decoder_for` | 186-196 | REAL | — | ignores track (fine for single-codec fixture) |
| `Delivered`/`Collector::{accept,len,frames_from}` | 205-251 | REAL | — | |
| `base`/`at_nanos`/`stamp_ns` | 257-272 | REAL | — | ns saturation at i64::MAX |
| `packet_of`/`audio_frame` | 274-300 | REAL | — | fixed 64 B payload, keyframe, 1 ms duration |
| `track` | 304-325 | REAL | — | declared-extent duration; UNKNOWN span shape |
| `tiny_queues`/`tiny_budget` | 329-345 | REAL | S4 | F-003 sync |
| `build`/`open` | 369-422 | REAL | — | frozen clock + throttle |
| `landing_of` | 428-435 | REAL | — | independent landing model |
| `resolve_target_ns` | 442-465 | REAL | S3 | mirror-of-implementation oracle (F-001) |

## 5. Test honesty

This file IS the fixture; no assertions of its own. Its consumers' honesty depends on F-001 (the resolution mirror) — the per-suite audits carry that flag forward.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "black-box: only public APIs are touched" | engine_fx.rs:4-5 | REAL | fixture uses only public playback_engine/codec_api/plugin_api surfaces |
| "The engine's exact arithmetic … mirrored" | engine_fx.rs:458-460 | REAL (an admission) | F-001 |

## 7. Dead / duplicated / unreachable code

`resolve_target_ns`'s Chapter arm answers None (unresolved target family) — matches engine behavior per the comment; unused by suites I read if they never emit Chapter targets (seek_roundtrip audit follows this one).

## 8. Dependencies & flags

codec-api, media-clock, media-types, packet-queue, playback-engine, plugin-api, testkit — all workspace-internal. No features.

## 9. Open questions for the next agent

- seek.rs: does the engine's `resolve_target` clamp/round exactly as mirrored here? If it diverges, the suite fails — good; if it matches by copy, resolution bugs are shared (F-001). File: crates/playback-engine/src/seek.rs.
