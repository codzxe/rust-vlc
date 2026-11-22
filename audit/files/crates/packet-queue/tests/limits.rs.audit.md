# AUDIT: crates/packet-queue/tests/limits.rs

| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-22 |
| Lines / functions | 478 / 3 `#[test]` + XorShift + Model oracle + `check_q1` + `MustOrElse` helper |
| File verdict | REAL — the original deterministic Q1/Q2 model driver; strong oracles throughout |
| Reachable from shipped entry point? | N/A (test). Subject `BoundedQueue`/`QueueLimits` is the shipped pipeline's queue stage. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read (full file); Grep `#[test]` = 3, no `#[ignore]`/cfg/env gates; cross-ref my property.rs/concurrency.rs audits for the XorShift lineage and the marker-ordering gap. No cargo — W00 baseline cited. |

## 1. One-paragraph verdict

The deterministic core of doc 03 §4.4(f)'s "model-check a small state
machine": 8 seeds × 3,000 seeded interleavings (push with a 7/10
current + 2/10 future + 1/10 stale stamp mix, pop, flush with
advance/stutter/regress targets, invariant sampling) against an exact
weight-tracking model — Q1 checked after EVERY op (not just the 5%
sample), the `Full` verdict's binding axis must equal the oracle's
first-violated-cap in u128, every flush's gap (count, bytes, duration,
new generation) must equal the model exactly, Q2 asserted on every
pop, and the run closes with exact marker accounting
(`markers_observed == total_flushed`), an exact drain, and the
conservation identity. Plus two focused units: the unknown-duration
wedge test (three phases, exact typed refusals per phase) and the
unbounded-impossible contract (constructor caps positive; degenerate
configs refused with the exact typed errors). All three tests ran in
the W00 baseline. One hygiene finding: this is one of FOUR copies of
the XorShift harness across the repo's test binaries. Notably, this
file's marker handling is stronger than property.rs's: markers are
consumed and accounted, and a lost marker fails the final identity —
though the gap-before-items ORDERING remains unasserted here too.

## 2. What actually works (verified)

— 3 ungated `#[test]`s → all in W00's `cargo test --workspace` (1751 pass).
— Per-op Q1 (:283-284 — the comment is accurate: checked after every operation, not only the sampled branch), including the usage-vs-stats identity `pushed − popped − flushed == packets` (:114-118).
— The Full-binding oracle (:186-198): the queue's refusal axis must equal `Model::fits`'s first violated cap, computed in u128 exactly like the crate's own arithmetic — an axis-priority regression fails with both values in the message.
— Flush exactness (:253-271): dropped == model packets, dropped_bytes == model bytes, dropped_duration == model duration, new_generation == the non-regressing max — all four fields pinned.
- Marker accounting closes exactly (:311-319): every flushed drop is eventually observed by the consumer or the test fails — a marker lost by the queue cannot hide (stronger than property.rs, which only checks markers it happens to pop).
— Stale handling on both sides: stale-stamped pushes must be refused with `current == published` (:199-205); popped items must be `>= published` (:224-227); the stamp generator produces all three classes (current/future/stale, :155-163) so the paths are genuinely exercised.
— `unknown_duration_items_do_not_wedge_the_duration_cap` (:343-419): three exact phases — byte-axis refusal with resident duration pinned at 0, packet-cap governance at 64 resident, and a known-duration item refused on the duration axis alone (packet/byte room explicitly available). This is the doc-named unit, done properly.
— `every_constructed_queue_is_bounded` (:424-463): the default constructors (audio/video) plus a 1/1/1 cap all yield positive caps and round-trip through `limits()`; `Unknown` and `Known(0)` durations refuse with the exact typed errors.
— Mutation reasoning (UNVERIFIED, no cargo): wrong axis priority → :188-192; marker drop-count inflation → :231-235 or the final identity; stale item popped → :224-227; duration wedge (unknown-duration items counting toward the cap) → phase 1/2 of the wedge test.

## 3. Findings

### F-001 [S4] Fourth copy of the XorShift harness (this file is the in-crate origin of two of the others)

Where: crates/packet-queue/tests/limits.rs:36-53
Code:
```rust
/// Deterministic xorshift* — the driver must be reproducible: a failed
/// seed reruns the exact interleaving.
struct XorShift(u64);
```
What is wrong: the identical generator now exists in (1) playback-engine's `tests/property/rng.rs` (the stated origin), (2) this file, (3) packet-queue `tests/property.rs` ("duplicated from playback-engine's … rng.rs"), and (4) packet-queue `tests/concurrency.rs` ("the limits.rs convention"). Four hand-maintained copies of a random-number primitive across five test binaries, each with its own `below` modulo bias, kept in sync by nothing.
Why it matters: §3.5 duplication; a seeding or bias fix must land four times, and the "reproducible seed" contract each copy promises depends on all four staying identical.
What correct looks like: `testkit::prop::XorShift` (testkit is already a dev-dependency of every one of these binaries).
Confidence: HIGH — Verification: read of all four sites (this audit set).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Blob`/`impl Weigh` | 19-34 | REAL | — | variable weights (unlike concurrency.rs's fixed) |
| `XorShift` | 36-53 | REAL | S4 | F-001: one of four copies |
| `Model` (+packets/bytes/duration/fits) | 55-91 | REAL | — | weight-tracking oracle; u128 axis arithmetic |
| `check_q1` | 93-119 | REAL | — | all three caps + the usage/stats identity |
| test `q1_q2_hold_under_adversarial_interleavings` | 125-327 | REAL | — | 8×3000 ops; per-op Q1; exact flush/marker/drain/conservation closure. Note: the Item arm (:213-228) does not assert markers surface before items — the queue's documented ordering law (queue.rs:327) stays unasserted here too (cross-ref property.rs F-001 / concurrency.rs F-003); marker NON-loss is asserted, which is stronger |
| test `unknown_duration_items_do_not_wedge_the_duration_cap` | 343-419 | REAL | — | the doc-named unit, three exact phases |
| test `every_constructed_queue_is_bounded` | 424-463 | REAL | — | typed-error refusals pinned exactly |
| `MustOrElse` | 465-478 | REAL | — | local context-carrying unwrap; documented rationale |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| q1_q2_hold_under_adversarial_interleavings | YES | per-op Q1, axis oracle, exact flush fields, Q2, marker closure, conservation — any queue-law break fails with seed+op context |
| unknown_duration_items_do_not_wedge_the_duration_cap | YES | exact outcomes per phase |
| every_constructed_queue_is_bounded | YES | exact typed errors and cap positivity |

All three ran in the W00 baseline (ungated).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Q1 (bounds hold, ever) and Q2 (no stale item pops, stale pushes drop at the boundary) under adversarial interleavings" | header :1-6 | REAL | :126-327 |
| "Q1 after EVERY operation, not just sampled ones" | :283 | REAL | :284 is outside the match — runs per iteration |
| "the duration-accounting unit the doc names explicitly" (doc 03 §4.4(f)) | :5-6 | REAL | :343-419's three phases |

## 7. Dead / duplicated / unreachable code

- F-001 (XorShift, fourth copy). No dead asserts found.

## 8. Dependencies & flags

No feature flags, no env knobs, no sleeps — fully deterministic (the reproducibility claim holds structurally: state is seed + op index only). Part of the default test run (W00 baseline).

## 9. Open questions for the next agent

- The marker-ordering law (queue.rs:327 "before any new item") is now confirmed unasserted in all three packet-queue suites — this file is the natural home for the fix (the Model already tracks everything needed; one `assert!(markers_observed_at_last_pop …)`-style guard in the Item arm would close it repo-wide).
