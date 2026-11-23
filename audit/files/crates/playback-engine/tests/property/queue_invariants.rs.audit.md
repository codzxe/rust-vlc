# AUDIT: crates/playback-engine/tests/property/queue_invariants.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-23 |
| Lines / functions | 347 / 8 callables (3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (property test) — runs in the workspace lane: baseline cargo-test-workspace.txt:4289-4300 (all 3 tests ok) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## 1. One-paragraph verdict
A genuinely adversarial property suite, not a victory lap: 64 seeded storms of
96 pause/play/seek ops over a 1200-packet timeline squeezed through a
16-packet queue plan, with laws checked after EVERY poll — cumulative
fabrication bound (polls × demux slice), delivered-generation floor and
monotonicity, monotone lifetime counters — plus a deterministic drain law that
is deliberately anti-vacuous (it FAILS if the packets-axis rejection counter
stayed at zero: a queue that never filled proves nothing). The deterministic
half pins exact-once delivery (frames == 1200), zero stale drops with no mint,
and no fabrication past exhaustion. The harness (rng.rs) is an honest
hand-rolled proptest substitute with a documented bisection shrink and a
candid "sampler, not an exhaustive prover" scope note. One hygiene nit: storm
ops swallow command results (`let _ =`), which is defensible for a storm but
undocumented.

## 2. What actually works (verified)
- Baseline (cargo-test-workspace.txt:4289-4300):
  `queue_invariants::exhausted_media_delivers_nothing_more ... ok`,
  `queue_invariants::long_media_drains_exactly_once_with_backpressure_exercised ... ok`,
  `queue_invariants::queue_plan_bounds_hold_under_playback_storms ... ok`
  (64 seeds each).
- Budget-mirror verified: `DEMUX_EVENTS_PER_POLL = 12` (:36) matches
  `engine_fx.rs:339-341` `tiny_budget() { demux_events: 12 }` and `open()`
  applies it (engine_fx.rs:392). A drift would over-tighten the bound and
  fail loudly — fail-safe direction.
- Harness verified by reading rng.rs:81-169: SEED_BANK = first 64 odd
  integers; `LUMEN_PROP_SEEDS` widening; deterministic `shrink_prefix`
  bisection (log₂ N reruns); `run_case` rerun/shrink instructions in every
  failure message.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/property/queue_invariants.rs:218-232
Code:
```
            Op::Pause => {
                request += 1;
                let _ = fixture.handle.pause(commands::envelope::RequestId(request));
            }
            Op::Play => {
                request += 1;
                let _ = fixture.handle.play(commands::envelope::RequestId(request));
            }
```
What is wrong: storm command results are discarded (`let _ =` on all three
ops). Most rejections are legal here (pause-while-paused is a C8 confirm;
seek-from-Ended is a typed refusal — the state machine's own table tests own
legality), so the invariants-only reading is defensible — but nothing in the
file says so, and a storm whose every seek was rejected would silently
exercise less seek coverage while still passing. What correct looks like: a
one-line comment ("ops may legally reject; the laws below are
rejection-agnostic") or a counted rejection assert. Confidence: HIGH —
Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| audio_timeline | 38-50 | REAL | — | 1200 slots, 1 audio track, linear pts |
| generate_ops | 61-72 | REAL | — | 40% poll / 10% pause / 10% play / 40% seek |
| Observer::new / observe | 85-173 | REAL | — | 4 monotone counters; cumulative fabrication bound (correctly cumulative — the per-window note at :136-138 explains why); generation floor + no-rewind |
| poll_and_observe | 179-187 | REAL | — | poll, drain outbox, full law set |
| run | 189-264 | REAL | S4 | prime→Playing, storm (4 polls/op), settle ≤4000 polls with paused-resume; F-001 |
| queue_plan_bounds_hold_under_playback_storms | 270-279 | REAL | — | 64 seeds × 96 ops |
| long_media_drains_exactly_once_with_backpressure_exercised | 289-323 | REAL | — | exact-once (frames == 1200), nonzero rejections REQUIRED, zero stale/preroll |
| exhausted_media_delivers_nothing_more | 328-347 | REAL | — | 64 post-end polls fabricate nothing |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| queue_plan_bounds_hold_under_playback_storms | YES | a dropped packet breaks exact drain at settle (frames < 1200 surfaces as… see below); a fabricated frame breaks the cumulative bound; a rewound counter/ generation is an immediate error with seed+op context |
| long_media_drains_exactly_once... | YES | exact equality on 1200; vacuity explicitly refused (zero rejections = unreachable!("backpressure unproven")) |
| exhausted_media_delivers_nothing_more | YES | equality across 64 extra polls |
Note (observation, not a defect): the storm's settle path asserts
`state == Ended` within 4000 polls but the per-packet exact-once law is only
checked in the deterministic drain test — a storm that dropped one packet
mid-storm would still settle to Ended (the collector count is only bounded,
not equaled, in `observe`). The drain test carries the exact-once law for the
no-storm shape. Confidence: HIGH — Verification: read of both tests.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bounded work per poll ... every packet delivered exactly once (backpressure delays, never drops)" | queue_invariants.rs:7-11 | REAL | drain test :304-309; storm bounded slice |
| "the packets-axis full-rejection counter moves (a queue that never fills proves nothing)" | :12-15 | REAL | :311-316 unreachable! on zero |
| "nothing fabricated — collector never outgrows the cumulative demux budget" | :16-18 | REAL | :139-146 |
| "generations floor the data ... never decreases" | :19-23 | REAL | :147-169 |
| "a 64-seed bank ... is a sampler, not an exhaustive prover" | rng.rs:9-11 (scope note) | REAL (honest limitation) | read |

## 7. Dead / duplicated / unreachable code
None. `MS`/`PACKETS`/`DEMUX_EVENTS_PER_POLL` all used; Observer fields all
read.

## 8. Dependencies & flags
playback_engine public API only (Engine, EngineStats, SessionState,
SeekTarget) + commands RequestId + the property harness modules — black-box
discipline held. Env knobs: LUMEN_PROP_SEEDS / LUMEN_PROP_SHRINK (harness-wide).

## 9. Open questions for the next agent
- The storm's exact-once gap noted in §5: would a mid-storm drop be caught by
  any current test, or only by the no-storm drain? (A storm+exact-once combo
  would need the collector to know the seek-adjacent expected set — nontrivial
  after seeks legitimately re-deliver the timeline.)
- engine_fx.rs (another agent's): is the Fixture's Collector unbounded (it
  holds every delivered frame)? Fine at 1200 packets, but the pattern
  shouldn't be copied to unbounded timelines.
