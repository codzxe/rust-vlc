# AUDIT: crates/packet-queue/tests/generation.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-23 |
| Lines / functions | 419 / 14 (9 `#[test]`) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test lane. Code under test (BoundedQueue, queue.rs) is shipped — see queue.rs.audit.md header. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | prebuilt `./target/debug/deps/generation-641a90c85a705c6e` -> `9 passed; 0 failed` (exit 0); baseline cargo-test-workspace.txt packet-queue section: `9 passed ... in 0.10s`. |

## 1. One-paragraph verdict
Nine exact, adversarial proofs of the generation semantics (card 2.B1 AC-b): wholesale drop with typed accounting, late-stale boundary rejection with the item handed back, parked-producer/parked-consumer wakeup races under flush (real threads, bounded observation budgets), marker merging, non-regressing publication, staleness dominating backpressure, and the real-`Packet` composite. Every assertion is exact (counts, bytes, durations, generations); I executed the binary myself: 9/9 pass. The only critiques are sleep-based race priming and deliberate local duplication of the 30-line Blob fixture.

## 2. What actually works (verified)
— verified by running the prebuilt test binary `target/debug/deps/generation-641a90c85a705c6e` -> `test result: ok. 9 passed; 0 failed; 0 ignored` (exit 0). Matches baseline (packet-queue `tests/generation.rs ... 9 passed ... 0.10s`).

## 3. Findings
F-001 [S4]
Where: crates/packet-queue/tests/generation.rs:197, 233
Code:
```
    // Let the producer park on the full queue, then flush.
    thread::sleep(Duration::from_millis(100));
    let _ = queue.flush(second);
```
What is wrong: race priming by fixed 100 ms sleep rather than a synchronized "parked" signal. On the 2-core sandbox under load the producer may not have parked when the flush fires — the test still passes (wait_while re-checks the predicate; an unparked producer sees the stale generation on its first lock), so this is not a correctness hole, but the "parked producer" premise of doc 03 §4.4(f) is only PROBABABLY exercised, not guaranteed. The 2 s elapsed bound and 5 s recv budget bound the worst case. What correct looks like: a pre-park handshake (e.g. an AtomicBool set inside a wrapper before calling push_wait) or spin-wait on queue stats. Confidence: HIGH — Verification: read + executed.
F-002 [S4]
Where: crates/packet-queue/tests/generation.rs:23-24
Code:
```
/// Minimal weighed payload with a controllable id (mirrors
/// `tests/backpressure.rs`; kept local so each proof file reads alone).
```
What is wrong: the Blob/Weigh fixture is duplicated across this file, backpressure.rs, and (a variant of) property.rs — documented and defensible (integration tests are separate crates; each reads alone), but it is three copies of the same 30 lines. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Blob + Weigh | 25-40 | REAL | S4 | duplicated fixture (F-002) |
| stamped / limits / join_or_propagate | 42-66 | REAL | — | panic propagation on join — good discipline |
| ac_stale_generation_drop_proof | 71-149 | REAL | — | THE AC: exact gap {4, 40B, 400ns}, consumer signal, late-stale typed, gen-N+1 flows, exact final stats |
| stale_push_wait_aborts_immediately | 153-173 | REAL | — | no park; <500 ms on a 5 s deadline; counted |
| parked_producer_wakes_on_flush_with_stale_abort | 179-214 | REAL | S4 | real thread, unbounded park deadline, 5 s bounded observation; sleep-primed (F-001) |
| parked_consumer_wakes_on_flush_with_discontinuity | 216-245 | REAL | S4 | gap before any item; sleep-primed (F-001) |
| unobserved_flush_markers_merge | 247-282 | REAL | — | 2+3 merged: dropped 5, 50 B, 500 ns, new_generation third |
| flush_never_regresses_the_published_generation | 284-312 | REAL | — | flush(3) after 5: drops, stays 5; consumer marker reports 5 |
| flush_on_empty_queue_publishes_and_signals | 314-332 | REAL | — | zero-drop move still observable |
| stale_check_precedes_the_capacity_check | 334-364 | REAL | — | full AND stale → Stale wins, both push variants, queue untouched |
| packet_stamps_flow_through_generation_bumps | 366-419 | REAL | — | real media-types Packet + Stamped::from_payload; exact byte/duration weights |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 9 | YES | Mutation reasoning: a silent flush (no Discontinuity) fails tests 1/4/5/7/9 (`unreachable!("expected Discontinuity")`); a lost wakeup (flush not notifying `space`) hangs the parked producer past the 5 s `recv_timeout` → `.must()` fails (test 3); stale-after-capacity ordering fails test 8; a regressing generation fails test 6; a mutating stale path (item altered) fails the `item.value` equality asserts; dropped merge counts fail test 5. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "all gen-N packets are dropped wholesale *with the typed signal* (`Discontinuity`, never silent)" | module docs 1-5 | REAL | test 1 executed |
| "a producer parked on a full queue wakes and observes `PushAborted::Stale` within a bounded number of wakeups" | module docs 6-8 | REAL | test 3 executed (real thread; premise probabilistic — F-001) |
| "never as silent data loss" (Q2, queue core) | lib.rs:29-30 | REAL | tests 4, 5, 7 executed |

## 7. Dead / duplicated / unreachable code
No dead code. Duplication: F-002.

## 8. Dependencies & flags
packet_queue, media_types (incl. real `Packet`), testkit. No unsafe, no feature gates, no #[ignore]. Threads used in exactly two tests, both with bounded observation channels and join-propagation.

## 9. Open questions for the next agent
- None for this file. (backpressure.rs / concurrency.rs owners: consider sharing the Blob fixture via a common module to kill F-002-class duplication.)
