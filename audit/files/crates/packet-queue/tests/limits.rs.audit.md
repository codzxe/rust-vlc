# AUDIT: crates/packet-queue/tests/limits.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2025-06-27 |
| Lines / functions | 478 / 8 (4 test fns/helpers + Model oracle + XorShift driver + MustOrElse) |
| File verdict | REAL (model-based property test of the production queue; oracle double-checks every outcome) |
| Reachable from shipped entry point? | The code under test is the shipped queue: playback-engine/src/pipeline.rs:552 `pub queue: Arc<BoundedQueue<Packet>>`, :1114 `BoundedQueue::new(limits, thresholds)` — the CLI pipeline's per-limb queues. The test binary is CI-only. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `sed -n`/`rg` verification of queue API and engine usage (exit 0); baseline cargo-test-workspace.txt:3907-3912 — all 3 tests pass |

## 1. One-paragraph verdict
This is the strongest test file audited in my assignment: a deterministic, seeded, reproducible model-check of the real production queue. An independent oracle (`Model`) recomputes every capacity verdict (packets/bytes/duration axes, i128-safe at line 77-90) and the driver asserts agreement on EVERY outcome — accepted pushes must fit, `Full { binding }` must name the exact axis the oracle says bound it, stale-stamped pushes must be refused, pops must be FIFO with no loss/dup, flush must drop exactly all residents with exact bytes/duration accounting, the published generation must never regress, discontinuity markers must account for every flushed item, and stats must conserve `pushed == popped + flushed`. I could not find a defect; the checklist below lists everything hunted and how.

## 2. What actually works (verified)
— Baseline: cargo-test-workspace.txt:3907-3912 — `every_constructed_queue_is_bounded`, `unknown_duration_items_do_not_wedge_the_duration_cap`, `q1_q2_hold_under_adversarial_interleavings` all ok.
— The oracle is genuinely independent arithmetic (lines 77-90), not a re-call of the code under test.
— Adversarial pressure is real: caps 4 packets / 40 bytes / 400 ns against pushes of 1-15 B and 0-149 ns — every axis bites within 3-4 pushes; 8 seeds × 3000 ops including future-stamped (+1..+3) and stale-stamped pushes, advancing/stuttering/regressing flush targets.

## 3. Findings
None. Zero-findings justification — exactly what was checked and how:
- **Tautology hunt**: every assertion compares the queue against the independent oracle or exact expected values; no assertion compares a value to itself. The marker-accounting inequality (line 232) is bounded above by total flushes and closes to equality at line 316-319.
- **Underflow/overflow hunt**: `total_flushed - markers_observed` (line 232) cannot underflow — markers_observed only grows from gap.dropped values already counted in total_flushed (each marker follows its flush); `stats().pushed - popped - flushed` (line 116) would panic on inconsistency, which is the assertion working. XorShift is seeded `seed | 1` (line 143) so the zero-state lock is impossible; `below(n)` is never called with n=0 (call sites: 100, 15, 10, 3).
- **Swallowed-error hunt**: `Closed` outcomes are asserted unreachable with a stated reason ("nothing closes this queue" — close semantics are another suite's scope: tests/concurrency.rs per header line 123-124); no `.ok()`, no `let _ =` discards.
- **Model-fidelity hunt**: flush semantics (drop ALL residents, publish max(target, current)) are enforced, not assumed — a queue that filtered-by-generation instead of wholesale-flushing would diverge from `model.items.clear()` and fail the Empty/drain assertions; future-stamped items surviving a flush would likewise fail.
- **Phase-3 arithmetic re-derived by hand**: after 64 pushes and 1 pop, resident = 63 packets/63 bytes/0 ns; the 11-ns timed push gives 64 ≤ 64 packets, 64 ≤ 1000 bytes, 11 > 10 ns → only the Duration axis can refuse — the assertion at line 411 is exact and non-vacuous.
- **Watermark coverage**: intentionally out of scope (this file is Q1/Q2 only, per header; watermarks are exercised via playback-engine's QueuePlan/priming suites) — honest scoping, not a gap mislabeled.
- The `MustOrElse` local trait (lines 467-478) is test-local convenience, `unreachable!` on None — appropriate in a test.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Blob + Weigh impl | 19-34 | REAL | — | test payload with size/duration |
| XorShift::next/below | 38-53 | REAL | — | reproducible driver; `seed \| 1` guards zero |
| Model::packets/bytes/duration/fits | 64-91 | REAL | — | independent oracle, u128 arithmetic |
| check_q1 | 93-119 | REAL | — | bounds + usage/stats cross-check |
| test q1_q2_hold_under_adversarial_interleavings | 125-327 | REAL | — | 8 seeds × 3000 ops; FIFO/Q2/marker/conservation all enforced |
| test unknown_duration_items_do_not_wedge_the_duration_cap | 343-419 | REAL | — | 3-phase exact unit, arithmetic verified by hand |
| test every_constructed_queue_is_bounded | 424-463 | REAL | — | defaults positive; typed errors for Unknown/Zero duration |
| MustOrElse | 467-478 | REAL | — | test helper |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| q1_q2_hold_under_adversarial_interleavings | YES | any capacity miscalculation, axis mislabel, FIFO violation, stale pop, generation regression, marker loss, or stats leak fails within one op with seed+op context |
| unknown_duration_items_do_not_wedge_the_duration_cap | YES | exact axis verdicts in all 3 phases |
| every_constructed_queue_is_bounded | YES | typed-error equality on degenerate configs |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Q1 (bounds hold, ever) and Q2 (no stale item pops, stale pushes drop at the boundary)" | header:1-3 | REAL | check_q1 after every op (line 284); stale-push/pop assertions (176-178, 224-227) |
| "doc 03 §4.4(f)'s model-check a small state machine" | header:121-124 | REAL | seeded model driver with independent oracle |
| "unknown-duration items ... must not wedge the max_duration computation" (doc 03 §4.4 f unit) | lines:329-342 | REAL | 3-phase test passes in baseline |

## 7. Dead / duplicated / unreachable code
None. All helpers are used; no `#[ignore]`; no feature gates.

## 8. Dependencies & flags
packet_queue (the code under test), media-types, testkit. Runs in the default lane (baseline :3907-3912). No fakes — the production `BoundedQueue` is exercised directly.

## 9. Open questions for the next agent
- Watermark (high/low) transitions and the `close`/`Closed` surface are NOT this file's scope — confirm tests/concurrency.rs and the packet-queue lib tests cover them before anyone cites this file as "the queue is fully proven".
