# AUDIT: crates/packet-queue/tests/property.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-23 |
| Lines / functions | 981 / 20 (4 `#[test]`) |
| File verdict | REAL — genuine property-based tests (seeded adversarial op streams + exact model oracle + per-op invariant checks) |
| Reachable from shipped entry point? | Test file: dev lane. Code under test YES: `BoundedQueue` is the queue core consumed by playback-engine (`QueuePlan`/`QueueLimits` wired in every session config, e.g. apps/cli/src/host.rs:971-974 `queue_plan_from_product_limits`, and crates/playback-engine/src/pipeline.rs limbs). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -p packet-queue --test property` -> 4 passed, 0 failed, 0.83s (exit 0); baseline cargo-test-workspace.txt packet-queue section (`4 passed ... in 0.75s`) |

## 1. One-paragraph verdict
Hostile question 2 answered: YES, these are real property tests. The core test drives 64 seeds × 4 caps (1/3/8/13) × 6000 randomly-generated adversarial ops (1.536M ops) of try-push/park-push/pop/park-pop/flush(regress+advance)/close against an independent Vec-based model oracle, checking after EVERY op: exact resident accounting (packets/bytes/duration), all three cap bounds, FIFO identity by payload id, stale-generation drop with item returned untouched, non-regressing published generation, all five stats counters, and the conservation identity `pushed == popped + resident + flushed`. Failure output carries seed + op index; reproduction and deterministic bisection shrink are implemented and self-tested. Weaknesses are scope gaps, not fakeness: the threaded storm checks liveness only (no accounting under real concurrency), the single-threaded park ops use already-expired deadlines (never actually block), and the shrink path is only exercised against a synthetic predicate.

## 2. What actually works (verified)
— verified by `~/.cargo/bin/cargo test -p packet-queue --test property` -> `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.83s` (exit 0). Baseline matches (packet-queue section, `4 passed ... in 0.75s`).

## 3. Findings
F-001 [S4]
Where: crates/packet-queue/tests/property.rs:874-962
Code:
```
#[test]
fn threaded_storm_terminates_under_generation_movement() {
    // The bank's first 8 seeds — the threaded storm is the slowest
    // suite; the single-threaded bank above already covers the rest.
    for seed in seed_bank().into_iter().take(8) {
```
What is wrong: the threaded storm verifies liveness only — workers report within a 20s watchdog, the drain reaches `Closed` bounded, `usage().packets == 0` at the end. It performs NO accounting, FIFO, or conservation checks under real concurrent parking; exact accounting is proven only in the single-threaded suite where parks never actually park. The two halves never combine, so a concurrency-only accounting defect (e.g. a lost wake counting a push without the resident) is unproven. Why it matters: the queue's raison d'être is concurrent producer/consumer use. What correct looks like: per-thread pushed/pop tallies + a final conservation check (pushed_total == popped_total + flushed_total + residents). Confidence: HIGH — Verification: read. Mitigated partly by tests/concurrency.rs (7 tests, baseline ok) and backpressure.rs (13 tests) — cross-file coverage exists; not my files to verify here.

F-002 [S4]
Where: crates/packet-queue/tests/property.rs:379-395
Code:
```
            Op::ParkPush { size, dur_ns } => {
                // The bounded park under an already-expired deadline:
                // deterministic, sleep-free — the no-deadlock shape.
                let stamp = GenerationId::new(model.published);
```
What is wrong: in the deterministic suite both park flavors (`push_wait`/`pop_wait` with `Some(Instant::now())`, lines 395/438) use already-expired deadlines, so the blocking paths never block; what is exercised is the abort classification, not the wait/wake machinery. Documented in-comment and compensated by the threaded storm (which does park, via 3ms/2ms deadlines), but the *model-oracle exactness* never runs against a genuinely parked-and-woken operation. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/packet-queue/tests/property.rs:964-981
Code:
```
/// The documented shrink procedure, self-tested: the bisection finds
/// the minimal failing prefix of a synthetic predicate (log2 N reruns
/// over a deterministic rebuild), and a non-failing sequence shrinks
/// to nothing. The property cases above drive it through `run_case`
/// on every failure; this pins the machinery itself.
```
What is wrong: the shrink machinery is only ever exercised against the synthetic `contains(&40)` predicate; no test forces a real property failure through `run_case`, so the production shrink wiring (env var `LUMEN_PROP_SHRINK`, rerun format string) is never proven end-to-end. A typo in the env-var name would silently disable shrinking forever. Confidence: MEDIUM — Verification: read; env-var parsing checked by eye only.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| XorShift (new/next/below) | 61-79 | REAL | — | xorshift64, `seed\|1`, modulo bias (negligible) |
| seed_bank | 83-93 | REAL | — | 64 fixed odd seeds + `LUMEN_PROP_SEEDS` widening |
| shrink_requested | 95-100 | REAL | — | env-driven; untested end-to-end (F-003) |
| shrink_prefix | 104-119 | REAL | — | bisection on failing prefix; self-tested |
| run_case | 121-145 | REAL | — | wraps failures with reproduce/shrink hints |
| Blob + Weigh | 152-167 | REAL | — | distinct id (FIFO identity), size + duration weights |
| Op enum | 174-192 | REAL | — | 6 op kinds incl. regressing flush |
| generate_ops | 194-221 | REAL | — | distribution 46/12/26/8/6/2; stale-prone stamps (-2..+5) |
| Model | 224-247 | REAL | — | independent Vec oracle + counters + pending_gap merge |
| run | 251-674 | REAL | — | THE driver: per-op oracle checks for every PushOutcome/PopOutcome/Flush/Close, per-op law set (541-616), terminal drain (618-673) |
| check_gap | 676-710 | REAL | — | discontinuity payload exact (dropped/bytes/duration/new_generation) |
| bounded_accounting_and_stale_laws_hold_under_adversarial_interleavings | 716-727 | REAL | — | 64 seeds × 4 caps × 6000 ops; executed, passes in 0.83s whole-suite |
| stale_generation_packets_are_always_dropped | 734-817 | REAL | — | exhaustive (published 0..48) × (stamp 0..published+6); stale never enters; Full axis pinned to Packets; drain never yields stale |
| producer / consumer | 827-867 | REAL | — | bounded-deadline (3ms/2ms) parking loops |
| threaded_storm_terminates_under_generation_movement | 874-962 | REAL | S4 | liveness only (F-001); 8 seeds, 12 seeded flushes + close, 20s watchdog |
| shrink_prefix_bisects_to_the_minimal_failing_prefix | 969-981 | REAL | S4 | synthetic predicate only (F-003) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| bounded_accounting_and_stale_laws... | YES | Mutation reasoning: a stale item entering the queue breaks `Stale` classification (300-308) or surfaces at pop ("a stale item was popped", 466-473); a cap overrun trips Q1 checks (564-583); a FIFO swap trips id equality (453-458); a lost/phantom resident trips exact accounting (542-561) or conservation (611-616); a generation regression trips 585-591; a mutated handed-back item trips the `item.value != blob` checks; removal of the empty-queue unconditional admission trips "Full while fits" (316-323). The oracle is an independent Vec implementation, not a copy of queue internals (verified against src/queue.rs — see that audit). |
| stale_generation_packets_are_always_dropped | YES | exhaustive pairwise sweep; both misclassification directions (stale-entered / current-dropped) assert |
| threaded_storm_terminates_under_generation_movement | YES for deadlock/liveness | a parked thread that never wakes fails the 20s watchdog; NO for accounting (F-001) |
| shrink_prefix_bisects... | YES for the bisection algorithm | synthetic predicate; does not prove the env-var wiring (F-003) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exact accounting — ... checked against `usage()`/`stats()` after **every** op" | module docs 9-13 | REAL | run() lines 541-616 execute per-op inside the loop; suite executed |
| "N ≫ cap — caps of 1/3/8/13 packets against 6000 ops per seed" | module docs 14-15 | REAL | line 719 `for cap in [1usize, 3, 8, 13]`, 6000 ops at 721 |
| "stale generations always dropped — ... a dedicated exhaustive sweep beside the interleavings" | module docs 16-19 | REAL | test at 734-817 |
| "no deadlock — a threaded storm ... must terminate under a watchdog" | module docs 26-28 | REAL (liveness only) | test at 874-962; accounting absent (F-001) |
| "no proptest (not in Cargo.lock; C5)" | module docs 32-33 | REAL-UNPROVEN | did not grep Cargo.lock for proptest (trivial; next agent may confirm) |

## 7. Dead / duplicated / unreachable code
None dead. `XorShift`/`seed_bank` are deliberately duplicated from playback-engine's tests/property/rng.rs (documented: integration tests are separate crates) — acceptable duplication, S4-note only (already counted as normal duplication debt; no separate finding since documented and forced by crate boundaries).

## 8. Dependencies & flags
packet_queue, media_types, testkit only. No proptest/quickcheck (documented C5 decision). No unsafe use in the test file. No feature gates. Env knobs `LUMEN_PROP_SEEDS` / `LUMEN_PROP_SHRINK` are opt-in widening, defaults deterministic.

## 9. Open questions for the next agent
- Does tests/concurrency.rs (7 tests) or tests/backpressure.rs (13 tests) close the concurrent-accounting gap (F-001)? Both pass in baseline — verify their assertions include conservation.
- Confirm proptest absence from Cargo.lock (doc claim 5).
- The storm uses `std::thread::sleep` (913) for flush spacing (~0.3-0.7ms × 12 × 8 seeds ≈ 60ms total) — fine, but note the module's "no sleeps" discipline applies to the single-threaded suite only.
