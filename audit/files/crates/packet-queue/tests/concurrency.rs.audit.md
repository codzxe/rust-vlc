# AUDIT: crates/packet-queue/tests/concurrency.rs
| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-17 |
| Lines / functions | 884 / 12 (7 #[test], 5 helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES for the code under test: `BoundedQueue` is the production queue (crates/playback-engine/src/pipeline.rs imports it; engine sessions are opened by apps/cli + apps/web hosts). The tests exercise the shipped queue directly. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | rg baseline (cargo-test-workspace.txt:3879-3893: 7 passed, 0 failed, 0.24s); sed quote verification; rg BoundedQueue production usage (pipeline.rs); sed queue.rs:185-205 (pushed counts accepted only). |

## 1. One-paragraph verdict
Hostile question answered: these tests are ACTUALLY CONCURRENT, not sequential simulations. All 7 tests spawn real OS threads (`thread::Builder::new().spawn`) — 4 producers + 3 consumers (mpmc), 2 producers + 1 consumer + a flusher (flush storm), a sampler + churn threads, 3 parking producers + 1 paced consumer, 4 parked producers woken by a close broadcast, and two seeded 6-thread storms (burst, deadlock triad) run over 4-5 seeds each. Producers park on the queue's condvars via `push_wait`, consumers via `pop_wait`, and — critically — `flush_storm_conserves_every_item_and_holds_Q2` FAILS if the race did not happen (`stats.flushed > 0` and `stats.stale_rejected > 0` are asserted), so the suite can be starved into a loud false-failure but never a vacuous pass. Conservation is cross-checked two ways (consumer-observed items + marker-dropped counts vs the queue's own counters). All 7 pass in baseline in 0.24s. Weaknesses are honest and documented: Q1 cap enforcement is only sampled (best-effort), and race coverage is sleep-paced wall-clock timing.

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt:3879-3893` -> "running 7 tests … test result: ok. 7 passed; 0 failed; 0 ignored … finished in 0.24s".
— verified by sed queue.rs:185-205 -> `fn accept(…) { self.items.push_back(…); … self.stats.pushed = self.stats.pushed.saturating_add(1); }` — `pushed` counts only ACCEPTED items, so the triad conservation assert (pushed == popped + resident + flushed) is sound.
— verified by rg `BoundedQueue` over crates/*/src -> production consumer is crates/playback-engine/src/pipeline.rs.

## 3. Findings
F-001 [S3]
Where: crates/packet-queue/tests/concurrency.rs:319-345
Code:
```
/// A concurrent usage sampler must never observe the caps exceeded —
/// best-effort by nature (a violation that slips between samples is
/// invisible), but any violation it DOES observe is a real Q1 break.
#[test]
fn usage_sampler_never_observes_caps_exceeded() {
```
What is wrong: Q1 (caps never exceeded) is NOT proven under concurrency — it is sampled at ~1 ms intervals by one thread. A transient overshoot between samples is invisible. The file admits this in the doc comment (honest), but the module header (line 3-4: "a usage sampler that must never observe the caps exceeded") presents a sampler as the Q1 proof; the exhaustive Q1 argument must live in the queue's single-lock design (queue.rs State under one Mutex), which these tests do not and cannot pin. Why it matters: a future refactor to fine-grained locks could break Q1 without this suite noticing. What correct looks like: state the single-lock invariant as the Q1 argument in the queue docs and keep the sampler as a tripwire only. Confidence: HIGH — Verification: read.
F-002 [S3]
Where: crates/packet-queue/tests/concurrency.rs:204-205,272-274,543
Code:
```
                        let stamp = queue.generation();
                        thread::sleep(Duration::from_millis(1));
```
and
```
    for bump in 1..=FLUSHES {
        thread::sleep(Duration::from_millis(4));
        let _ = queue.flush(GenerationId::new(bump));
```
What is wrong: Race coverage depends on wall-clock sleeps (1 ms stamp-to-push windows, 4 ms flush spacing, a 100 ms pre-close parking wait, 10 s/5 s storm budgets). On a heavily loaded 2-core box the required interleavings may fail to materialize, making these tests FLAKY in the loud direction (they assert the race happened), and budget asserts (`elapsed < STORM_BUDGET`) can spuriously fire under load. Not observed in baseline (0.24s total), but the failure mode exists. Never a false pass. Confidence: MEDIUM — Verification: read; no failure observed in baseline.
F-003 [S4]
Where: crates/packet-queue/tests/concurrency.rs:871-882
Code:
```
        // Whatever the interleaving, the invariants held at rest: Q1
        // and the conservation law (pushed == popped + resident +
        // flushed + stale_rejected; deadline-aborted pushes returned
```
What is wrong: The comment states the conservation law INCLUDES `stale_rejected`; the assert below it correctly omits it (stale/deadline rejections never entered, so they are not in `pushed` — verified queue.rs:198). Comment contradicts the code it describes; a reader "fixing" the assert to match the comment would weaken it. Confidence: HIGH — Verification: read + sed queue.rs.
F-004 [S4]
Where: crates/packet-queue/tests/concurrency.rs:613-615
Code:
```
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
```
What is wrong: Modulo reduction biases the seeded RNG. Pacing-only (consumer sleep/yield choices); no invariant depends on uniformity. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Blob + Weigh impl | 20-33 | REAL | — | 4 bytes, 2 ns duration weights. |
| limits | 35-42 | REAL | — | QueueLimits builder. |
| join_or_propagate | 44-50 | REAL | — | Panics propagate across thread joins. |
| ConsumerEvent / record_id | 52-70 | REAL | — | Poison-recovering observation sink. |
| mpmc_no_loss_no_duplication | 75-170 | REAL | — | 4 prod × 250 + 3 consumers; BTreeSet uniqueness + counter cross-check. |
| flush_storm_conserves_every_item_and_holds_Q2 | 178-317 | REAL | S3 | Race asserted to have happened; Q2 floor along observation order; conservation pushed == popped + dropped. |
| usage_sampler_never_observes_caps_exceeded | 322-404 | REAL | S3 | F-001: sampled Q1 only; churn 1000 items; counters conserved. |
| parked_mpmc_producers_complete_under_paced_consumption | 413-496 | REAL | — | push_wait(None) parking, 30 s lost-wakeup budget, exactly-once. |
| close_wakes_every_parked_producer | 503-587 | REAL | — | Broadcast semantics: all 4 abort with item handed back; resident data drains after close; <2 s wake. |
| XorShift (next/below) | 600-616 | REAL | S4 | F-004 modulo bias. |
| burst_storm_parks_drains_and_delivers_exactly_once | 627-742 | REAL | — | 4 seeds × 4 producers parking with deadline; asserts full_rejections == 0, budget, exactly-once, id-range endpoints. |
| deadlock_triad_terminates_under_seeded_interleavings | 750-884 | REAL | S4 | 5 seeds × {2 prod, 2 cons, flusher, closer}; every park has a 3 ms deadline; budget assert; at-rest Q1 + conservation. F-003 comment mismatch. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mpmc_no_loss_no_duplication | YES | Length + uniqueness + duplication asserts, queue counters must match. |
| flush_storm_conserves_every_item_and_holds_Q2 | YES | Conservation equality (not ≥), marker/flush equality, Q2 in-thread assert, race-occurred asserts. |
| usage_sampler… | PARTIAL | Only observed violations trip; between-sample violations invisible (documented). |
| parked_mpmc… | YES | Exactly-once + 30 s lost-wakeup budget would fire on a parked-forever bug. |
| close_wakes_every_parked_producer | YES | All 4 must abort with Closed + item; wake latency <2 s; a notify_one bug strands 3. |
| burst_storm… | YES | Any lost wakeup aborts the parked push (unreachable! fires); overrun fails budget; full_rejections == 0 pinned. |
| deadlock_triad… | YES | Budget + per-thread deadline asserts; at-rest conservation equality. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "MPMC stress — no loss, no duplication, exact conservation under generation storms" | lines 1-4 | REAL | Tests 1-2 pass in baseline; uniqueness + conservation equalities. |
| "a usage sampler that must never observe the caps exceeded" | lines 3-4 | SHALLOW (as Q1 proof) | F-001: sampling is best-effort, admitted in the test's own doc comment. |
| "every park carries a deadline … never a hung gate" | lines 592-597 | REAL | burst/triad use Some(deadline); parked_mpmc uses None but a 30 s consumer budget asserts progress. |

## 7. Dead / duplicated / unreachable code
None. All helpers used; `ConsumerEvent::Dropped` used in flush storm only (fine).

## 8. Dependencies & flags
No feature gates. Depends on packet_queue (BoundedQueue, PushOutcome, PopOutcome, PushAborted, Stamped, Weigh, QueueLimits), media_types, testkit::Must. No loom/crossbeam — raw std threads + the queue's own condvars, which is the right level for this crate.

## 9. Open questions for the next agent
- Q1's exhaustive argument should be documented in queue.rs (single-lock State invariant). Verify whoever audits crates/packet-queue/src/queue.rs checks that resident_bytes/resident_duration are only mutated under the one lock.
- The backpressure suite (backpressure.rs / backpressure_property.rs, next in my list) — do they add the exhaustive Q1 reasoning the sampler lacks?
