# AUDIT: crates/playback-engine/tests/property/queue_invariants.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 347 / 3 tests + Observer + drivers |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | N/A — property test binary (tests/property/). W00 baseline: 1751/2/25; this binary's individual result UNVERIFIED (no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | `cat` full file (exit 0); cross-ref engine_fx fixture (read in full for the next audit). No cargo. |

## 1. One-paragraph verdict

A genuinely adversarial engine-level property suite: 64 seeds × 96 ops of pause/play/seek storms over a 1200-packet timeline on a 16-packet queue plan, with four laws asserted after **every single poll** (monotone lifetime counters; cumulative no-fabrication bound `frames ≤ polls × 12`; delivered-generation ≤ wire-generation and never rewinding; bounded settle to `Ended` within 4000 polls), plus a deterministic exactly-once drain test that explicitly refuses to pass if backpressure was never exercised (`full_rejections_packets == 0` fails the test — the rare test that demands the bound bite) and a past-end fabrication probe. The `run` driver rebuilds the whole case from (seed, ops), so the harness's shrink-bisection purity requirement is honored. Two hygiene notes: the doc header's "every packet delivered exactly once" is scoped to the no-seek drain test only (seeks in the storm legitimately re-deliver); and the assertions use `unreachable!(msg)` as the failure vehicle — functional, but a plain `panic!`/`assert` would carry clearer intent.

## 2. What actually works (verified)

- The four per-poll laws (Observer::observe, 94-173): counter monotonicity across four stats fields; the cumulative fabrication bound with the documented soundness argument for why a per-window bound would be wrong (133-138); generation flooring (≤ wire, never rewinding). All read-verified against the engine's stats surface (session.rs EngineStats).
- The backpressure-must-bite assert (311-316): a test that fails when the cap never refuses is the correct shape for proving a bound exists — most suites in the wild assert only "no overflow".
- Exactly-once drain (289-323): `frames == PACKETS`, zero stale drops, zero preroll discards (no mints happened in the no-seek run — consistent with the seekless timeline).
- Determinism for shrinking: `run(seed, ops)` reconstructs fixture + observer from scratch (189-199); ops regeneration is seed-pure (canonical path regenerates 96 ops from the same seed, 190-196).

## 3. Findings

### F-001 [S4] Header overstates the exactly-once law's scope
Where: crates/playback-engine/tests/property/queue_invariants.rs:7-11
Code (verbatim):
```rust
//! - **bounded work per poll** — with a 16-packet queue plan and a
//!   1200-packet timeline (N ≫ cap), every poll is a bounded slice
//!   and the session still drains to `Ended` with **every** packet
//!   delivered exactly once (backpressure delays, it never drops:
//!   doc 03 Q3 "fresh data is never dropped");
```
What is wrong: exactly-once is asserted only in the seek-free drain test (304-309). The storm suite asserts bounded/fabrication-free delivery, not exactly-once — a seek backward re-delivers timeline frames by design, and the storm's law set correctly does not forbid that. The header sentence, read alone, claims more than the suite proves.
Confidence: HIGH — Verification: read.
Note: the doc for the drain test itself (281-288) is precise about scope — the overreach is only the module header.

### F-002 [S4] `unreachable!` as the assertion vehicle
Where: 276, 301, 305, 312, 318 — e.g. `unreachable!("{failure}");`
What is wrong: `unreachable!` is semantically "this arm cannot occur"; here it is the failure path of a property law. It works (panic + message, including the shrink/rerun text from `run_case`), but it inverts the reader's model of which path is expected. `assert!(...)`/`panic!` would be honest. Cosmetic.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `audio_timeline` | 38-50 | REAL | — | 1200 slots, 1 ms apart |
| `generate_ops` | 61-72 | REAL | — | 40% poll / 10% pause / 10% play / 40% seek |
| `Observer::new`/`observe` | 85-173 | REAL | — | the four per-poll laws; sound cumulative-bound argument |
| `poll_and_observe` | 179-187 | REAL | — | poll + drain + laws, every poll of the lifetime |
| `run` | 189-264 | REAL | — | rebuild-from-seed (shrink-pure); prime→storm→settle with a 4000-poll liveness bound; auto-resume of a paused tail (documented as driver law) |
| `queue_plan_bounds_hold_under_playback_storms` | 270-279 | REAL | — | full seed bank via run_case |
| `long_media_drains_exactly_once_with_backpressure_exercised` | 289-323 | REAL | — | exactly-once + must-bite + no-stale asserts |
| `exhausted_media_delivers_nothing_more` | 328-347 | REAL | — | past-end fabrication probe (64 extra polls, count frozen) |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| queue_plan_bounds_hold_under_playback_storms | YES (reasoned) — for queue-cap violations, counter rewinds, fabricated frames, future/rewound generations, and settle deadlocks (4000-poll bound with state in the message) | the laws are exact inequalities checked per poll; the seek/pause/play mix mints generations and flushes |
| long_media_drains_exactly_once_with_backpressure_exercised | YES (reasoned) | exact frame count; refuses zero-rejection runs; refuses nonzero stale/preroll counters |
| exhausted_media_delivers_nothing_more | YES (reasoned) | count must be frozen across 64 post-end polls |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "backpressure delays, it never drops" (doc 03 Q3) | header 10-11 | REAL-UNPROVEN | drain test pins exactly-once for the seekless case; run result UNVERIFIED |
| "the cap genuinely refused pushes; a queue that never fills proves nothing" | 12-15 | REAL | the zero-rejection refusal assert exists (311-316) |
| exactly-once under storms | header 7-11 | SHALLOW wording | F-001 — only the drain test is exactly-once |

## 7. Dead / duplicated / unreachable code

None. The fixture (`engine_fx`) is shared and audited next.

## 8. Dependencies & flags

playback-engine + media-types + the shared rng harness. Env: LUMEN_PROP_SEEDS/LUMEN_PROP_SHRINK via `seeds()`/`run_case`.

## 9. Open questions for the next agent

- The fabrication bound assumes `budget.demux_events = 12` (line 36, "the fixture's tiny_budget") — engine_fx must keep the two in sync; if the fixture's budget changes, the constant lies (checked in my engine_fx audit).
