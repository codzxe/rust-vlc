# AUDIT: crates/packet-queue/tests/backpressure_property.rs
| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-17 |
| Lines / functions | 422 / 9 (2 #[test], 7 helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES for the code under test (`BoundedQueue::new(limits, marks)`, `Watermark::evaluate`, the full push/pop/flush/close vocabulary — the production queue in playback-engine/src/pipeline.rs). The thresholds mirrored here are the production `QueuePlan::thresholds` (pipeline.rs:400-420). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | baseline cargo-test-workspace.txt (backpressure_property block: 2 passed, 0.01s); sed pipeline.rs:390-420 to compare the mirror; sed quote verification. |

## 1. One-paragraph verdict
A genuine deterministic property suite: 8 fixed seeds (+ env-gated `LUMEN_PROP_SEEDS`) × 4,000 adversarial ops exercising the FULL backpressure vocabulary — fresh/stale-stamped try-pushes, expired-deadline parks (the sleep-free deterministic form), try-pops, parking pops, advancing/stuttering/REGRESSING flushes, close, and burst mode — with a real ORACLE: on every `Full`, the test recomputes which axis should bind from usage + item weights and asserts the queue's `binding` agrees (axis precedence included). Q1 (caps), the conservation identity (pushed == popped + resident + flushed), and the hysteresis laws are checked after EVERY operation, which closes the single-threaded half of the Q1 gap left by concurrency.rs's sampler. The second test pins the B2 gap-2 fix (packets-axis-only low is reachable for every payload mix, including the two wedge shapes). Both tests pass in 0.01s. Findings are hygiene only.

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt` (backpressure_property block) -> "running 2 tests … test result: ok. 2 passed; 0 failed; 0 ignored … finished in 0.01s".
— verified by sed pipeline.rs:400-420 vs test lines 64-76: the mirror is currently EXACT (same high = cap − cap/4, same low = packets/4 floored at 1, bytes/duration at caps).
— verified by sed: the Full-oracle at lines 212-231 recomputes all three axes with u128 arithmetic and asserts precedence agreement.

## 3. Findings
F-001 [S4]
Where: crates/packet-queue/tests/backpressure_property.rs:59-76 vs crates/playback-engine/src/pipeline.rs:400-420
Code:
```
/// The engine-shaped thresholds of card 3.B3 (`QueuePlan::thresholds`
/// in playback-engine, mirrored here so the queue crate's property
/// suite stays engine-agnostic): high = cap − cap/4 on every axis;
```
What is wrong: `engine_thresholds` is a hand-copied mirror of the production `QueuePlan::thresholds`. If the engine's marks change, this property suite keeps passing against the STALE shape while production behavior moves on. playback-engine's own queue_policy tests pin the production copy, but nothing cross-checks the two against each other. Confidence: HIGH — Verification: read + sed comparison.
F-002 [S4]
Where: crates/packet-queue/tests/backpressure_property.rs:376-384
Code:
```
        let usage = queue.usage();
        let mark = Watermark::evaluate(usage, &marks);
        if usage.packets <= stall_line {
            assert!(
                mark.low,
```
What is wrong: The wedge-mix-B check asserts `low` only in the packets ≤ stall_line direction; the complementary direction (packets > stall_line ⇒ NOT low) is unasserted, so a watermark that stuck `low` on forever would pass this block. (The 50-cycle loop below partially compensates via check_hysteresis's low implication, but the non-low direction is still only implied, never pinned.) Confidence: MEDIUM — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Blob + Weigh impl | 23-38 | REAL | — | Per-axis weights. |
| XorShift (next/below) | 42-57 | REAL | — | Seeded, seed printed on failure; modulo bias (pacing/oracle input only — the oracle recomputes exactly, so bias cannot create a false pass). |
| engine_thresholds | 64-76 | REAL | S4 | F-001 mirror drift risk. |
| check_q1 | 78-98 | REAL | — | All three caps, every step. |
| check_conservation | 106-114 | REAL | — | Per-step identity; rejections correctly excluded. |
| check_hysteresis | 119-139 | REAL | — | low ⇒ packets ≤ stall line; high ⇒ some axis at resume line. |
| seeds | 142-152 | REAL | — | 8 fixed + env sweep (C5: no proptest, documented). |
| backpressure_vocabulary_holds_q1_conservation_and_hysteresis | 162-318 | REAL | — | 8 seeds × 4,000 ops; Full-binding oracle; burst mode; regressing flushes; expired-deadline parks; terminal conservation. |
| engine_shaped_low_is_reachable_for_every_mix | 325-422 | REAL | S4 | Wedge mixes A/B + 50 flood/drain cycles with all three checks; F-002 one-directional assert. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| backpressure_vocabulary… | YES | Oracle disagrees on wrong binding axis; Q1/conservation/hysteresis after every op; unexpected Closed asserted. |
| engine_shaped_low_is_reachable… | YES (positive direction) | Wedge mixes must reach low; flood cycles pin high at the resume line; F-002 caveat on the negative direction. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "property tests with synthetic producers/consumers — no deadlock, bounded memory under burst" (doc 13 §6 / doc 03 §8 AC) | lines 1-5 | REAL for bounded memory (Q1 every op incl. bursts); the NO-DEADLOCK half is NOT proven here (single-threaded by design) — it lives in concurrency.rs's threaded triad/burst budgets. | File header itself says "single-threaded, no proptest"; concurrency.rs:592-697. |
| "Every failure prints its seed" | lines 9-10 | REAL | `format!("seed {seed}")` context strings throughout. |

## 7. Dead / duplicated / unreachable code
The `|| in_burst && r < 100` half of the guard at line 193 is dead (the scrutinee is 0 when in_burst, so `r < 40` already holds) — harmless documentation of intent. F-001's mirror is the one real duplication.

## 8. Dependencies & flags
packet_queue + media_types + testkit only. Reads `LUMEN_PROP_SEEDS` env var (widening sweep) — test-only, documented. No feature gates, no unsafe, no threads (by design).

## 9. Open questions for the next agent
- Add a cross-crate drift pin: one test in playback-engine asserting `QueuePlan::thresholds(limits)` equals a packet-queue-exposed canonical constructor (or golden values), so F-001's mirror cannot rot.
- concurrency.rs audit asked whether this file closes the Q1 gap: single-threaded exhaustive — YES; under concurrency — still sampled-only.
