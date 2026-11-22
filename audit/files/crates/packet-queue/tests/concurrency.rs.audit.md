# AUDIT: crates/packet-queue/tests/concurrency.rs

| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-22 |
| Lines / functions | 884 / 7 `#[test]` + workers/helpers (Blob, limits, join_or_propagate, record_id, XorShift) |
| File verdict | REAL (serious MPMC/generation-storm/deadlock coverage) with one hung-test hole contradicting its own doc claim |
| Reachable from shipped entry point? | N/A (test). Subject `BoundedQueue` is the shipped pipeline's queue stage. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | Read (full file, 2 chunks); Grep `#[test]` count = 7, no `#[ignore]`/cfg/env gates; cross-read queue.rs park/close semantics for the join-order analysis; sed verification of quoted ranges. No cargo — W00 baseline cited. |

## 1. One-paragraph verdict

Seven ungated threaded tests (all ran in the W00 baseline's 1751)
stressing the real `BoundedQueue` from every concurrency angle: MPMC
exactly-once delivery with BTreeSet dedup and counter conservation
(:76-170); a genuinely racy generation storm whose producers
stamp-then-sleep-1 ms-then-push so flushes land inside the window,
with a per-consumer monotonic floor asserting the Q2 observation law
(no item observed after a marker that supersedes it) AND
race-actually-happened asserts (`flushed > 0`, `stale_rejected > 0`,
:309-316); a best-effort usage sampler honestly labeled as such
(:319-321); a parked-producer paced-consumer test; a close-broadcast
test that would catch a `notify_one` bug stranding 3 of 4 parked
producers (:503-587); a burst storm and a deadlock triad where every
park carries a deadline so no schedule can hang (:627-884). One real
defect: the parked-MPMC test parks producers with `push_wait(None)` —
NO deadline — while its "global wall-clock budget" lives only inside
the consumer thread; the main thread joins the producers FIRST, so a
lost producer wakeup hangs the test on `join()` before any assert can
fire, directly contradicting the test's own doc claim ("turns a lost
wakeup into a loud failure instead of a hung test"). Plus the
now-third copy of the XorShift harness.

## 2. What actually works (verified)

— 7 ungated `#[test]`s → all in W00's `cargo test --workspace` run (1751 pass).
— Exactly-once oracles are real: length + BTreeSet-unique length + length==unique (:155-162, :487-490, :719-727), cross-checked against the queue's own counters (pushed/popped/flushed/stale_rejected/full_rejections) — a lost or duplicated item fails at least two independent asserts.
— `flush_storm_conserves_every_item_and_holds_q2` (:179-317) is the strongest: the 1 ms stamp-to-push window makes the flush race substantial (not incidental); one consumer observes the whole marker chain, so `pushed == popped + marker_dropped` is exact; `marker_dropped == stats.flushed` and `stats.popped == popped` pin the queue's internal accounting against the consumer's observations; the floor check (:239-250) asserts the Q2 law along the observation order — this is the only test in the repo that partially covers the gap I flagged in property.rs (stale items after a marker); and the storm refuses to pass vacuously (:309-316 — if no flush ever dropped or no push was ever stale-rejected, the test FAILS as "did not race").
— `close_wakes_every_parked_producer` (:503-587): 4 parked producers on a full queue, close must wake ALL of them with the typed `Closed` abort carrying the item back intact, within 2 s (a `notify_one`-vs-`notify_all` bug strands 3 → `recv_timeout(5 s).must()` fails), and the 8 resident items still drain afterwards with `pushed == 8` (aborts entered nothing).
— Burst storm + deadlock triad (:627-884): every park is deadline-bounded (10 s storm budget / 3 ms triad deadlines), so termination is structural — the stated convention (:592-597) is actually honored here; budget asserts in-thread plus a final elapsed check; conservation and Q1 re-verified at rest per seed.
— Panic propagation: `join_or_propagate` resumes unwinds (:44-50), and the sampler's asserts live inside its thread, joined through the same path — a Q1 observation anywhere fails the test.
— Mutation reasoning (UNVERIFIED, no cargo): drop an item in push → exact-once fails; duplicate on wake → unique-len fails; lost wakeup in burst/triad → deadline abort → `unreachable!`; `notify_one` on close → 3 recv timeouts; counter leak → conservation asserts.

## 3. Findings

### F-001 [S3] The parked-MPMC test can hang exactly where its doc claims it cannot — the budget assert is in the wrong thread and the join order blocks it

Where: crates/packet-queue/tests/concurrency.rs:411-412, 432, 453-459, 477-478
Code:
```rust
/// is delivered exactly once, the producers complete without spinning
/// (their progress is bounded by the consumer's pace, not a retry
/// loop), and the counters conserve. The global wall-clock budget
/// turns a lost wakeup into a loud failure instead of a hung test.
```
```rust
                        match queue.push_wait(item, None) {
```
```rust
                assert!(
                    Instant::now() < budget,
                    "consumer stalled at {drained}/{TOTAL} — a producer lost its wakeup"
                );
```
```rust
    join_or_propagate(producer_handles);
    join_or_propagate(vec![consumer]);
```
What is wrong: the producers park with `push_wait(item, None)` — no deadline (the only park in this file without one; the burst storm and triad explicitly carry deadlines for exactly this reason, :592-597). The 30 s budget assert lives inside the CONSUMER thread (:453-459). If a producer loses its wakeup, the consumer stalls, its budget assert fires at 30 s, the consumer panics — but the main thread is blocked at `join_or_propagate(producer_handles)` (:477) on the very producer that never woke. `join()` on a live parked thread does not return; the consumer's panic message is never observed; the test process hangs until the harness/CI timeout kills it.
Why it matters: the doc comment promises "a loud failure instead of a hung test" — for the producer-lost-wakeup case (the exact scenario the sentence names) the test delivers a hung test instead. A future queue regression in the `space` condvar would turn this suite into a CI timeout instead of a diagnostic.
What correct looks like: give the producers a generous deadline (`Some(Instant::now() + 30s)` — an abort then fails loudly with the item id), or join the consumer first, or wrap the whole schedule in a watchdog thread.
Confidence: HIGH — Verification: read (join order at :477-478; `push_wait(item, None)` at :432; budget assert only at :456-459).

### F-002 [S4] Third copy of the XorShift harness

Where: crates/packet-queue/tests/concurrency.rs:600-616
Code:
```rust
/// Deterministic xorshift* (the `limits.rs` convention).
struct XorShift(u64);
```
What is wrong: the same hand-rolled xorshift now exists in this file, in tests/property.rs (:61-80, itself "duplicated from playback-engine's tests/property/rng.rs"), and presumably in limits.rs — three-plus copies across the crate's test binaries, each drift-prone, while the shared `testkit` dev-dependency (imported at :18) is the natural home. `below` retains the modulo bias (irrelevant for test pacing, noted for completeness).
Why it matters: §3.5 duplication debt; a fix to the harness (e.g. a seeding bug) must be applied N times.
What correct looks like: one `testkit::prop::XorShift`.
Confidence: HIGH — Verification: read of both files (property.rs:61-80 is character-identical logic).

### F-003 [S4] Gap-before-new-items ordering remains unasserted here too

Where: crates/packet-queue/tests/concurrency.rs:242-262
Code:
```rust
                    PopOutcome::Item(stamped) => {
                        // Q2 along the observation order: after a
                        // marker published G, no item stamped < G.
                        assert!(
                            stamped.generation >= floor,
```
What is wrong: the floor check asserts the no-STALE-item-after-marker half of the consumer ordering law, but not the other half documented at queue.rs:327 ("Consumers observe the drop via `PopOutcome::Discontinuity` before any new item") — a queue that delivered a NEW item (stamped ≥ floor) before surfacing its pending marker would pass this test too. Cross-reference: property.rs F-001 found the same blind spot; nothing in the repo asserts the marker-first ordering.
Why it matters: the marker is the pipeline's signal to drop decoder state; a queue that let new items jump the marker would desynchronize consumers silently.
What correct looks like: in the storm consumer, track that no `Item` is accepted between a flush's return and the marker's observation (needs a flusher-consumer handshake or a model of pending markers, as property.rs's `Model.pending_gap` already has — it just never checks it on the Item path).
Confidence: HIGH — Verification: read (this file's consumer + queue.rs:325-331 contract).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Blob`/`impl Weigh` | 20-33 | REAL | — | fixed 4 B / 2 ns weights |
| `limits` | 35-42 | REAL | — | builder |
| `join_or_propagate` | 44-50 | REAL | — | resume_unwind — panics surface |
| `ConsumerEvent`/`record_id` | 52-70 | REAL | — | poison-recovering sink |
| test `mpmc_no_loss_no_duplication` | 75-170 | REAL | — | 4×250 / 3 consumers; exact-once + counters; producers retry-on-Full with yield (busy but bounded by consumers) |
| test `flush_storm_conserves_every_item_and_holds_q2` | 178-317 | REAL | S4 | the file's strongest; see F-003 for the unasserted ordering half; race-happened asserts real |
| test `usage_sampler_never_observes_caps_exceeded` | 322-404 | REAL | — | honestly best-effort (:319-321); sampler panics propagate via join |
| test `parked_mpmc_producers_complete_under_paced_consumption` | 413-496 | REAL | S3 | F-001: unbounded park + budget in the wrong thread + producer-first join order |
| test `close_wakes_every_parked_producer` | 503-587 | REAL | — | broadcast proof; latency bound; data-survives-close proof |
| `XorShift` | 600-616 | REAL | S4 | F-002: third copy |
| test `burst_storm_parks_drains_and_delivers_exactly_once` | 627-742 | REAL | — | 4 seeds; deadline-bounded parks; `full_rejections == 0` pins the healthy park/wake shape |
| test `deadlock_triad_terminates_under_seeded_interleavings` | 750-884 | REAL | — | 5 seeds; flush+close control plane; conservation + Q1 at rest |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| mpmc_no_loss_no_duplication | YES | exact-once + unique-set + counters |
| flush_storm_conserves_every_item_and_holds_q2 | YES | conservation against consumer observations, Q2 floor, race-happened gates |
| usage_sampler_never_observes_caps_exceeded | PARTIALLY (by design, admitted) | best-effort sampling; only observed violations fail; any violation observed IS real |
| parked_mpmc_producers_complete_under_paced_consumption | YES when healthy; HANGS on the lost-wakeup regression it names | F-001 |
| close_wakes_every_parked_producer | YES | all-must-wake + item-back + latency + drain |
| burst_storm_parks_drains_and_delivers_exactly_once | YES | deadline aborts panic; budget; exact-once; full_rejections == 0 |
| deadlock_triad_terminates_under_seeded_interleavings | YES | every park deadline-bounded; budget; conservation at rest |

All seven ran in the W00 baseline (ungated). Combined wall cost: the sleeps/deadlines total in the tens of seconds — absorbed by the baseline run.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no loss, no duplication, exact conservation under generation storms" | header :1-4 | REAL | :155-169, :296-316 |
| "a usage sampler that must never observe the caps exceeded" | header :3-4 | REAL (best-effort, self-labeled) | :319-321 admits the sampling blind spot |
| "The global wall-clock budget turns a lost wakeup into a loud failure instead of a hung test" | :411-412 | FALSE for the producer-lost-wakeup case | F-001: producers park unbounded; main joins producers before the consumer whose budget would fire |
| "every park carries a deadline … never a hung gate" | :592-597 | REAL for burst/triad; NOT for parked_mpmc | :655, :779, :813 vs :432 |

## 7. Dead / duplicated / unreachable code

- `XorShift` — triplicated harness (F-002).
- No dead asserts found; every `unreachable!` in this file guards a genuinely impossible-in-design outcome and would fire on the corresponding bug.

## 8. Dependencies & flags

No feature flags, no env knobs, no gates — the whole file is part of the default test run (W00 baseline). Dev-deps: media_types, packet_queue, testkit.

## 9. Open questions for the next agent

- F-001's hang shape: if the CI ever shows a packet-queue test timeout (rather than a failure), `parked_mpmc_producers_complete_under_paced_consumption` is the prime suspect.
- The marker-before-new-items ordering (queue.rs:327) is now confirmed unasserted in BOTH packet-queue property suites (property.rs F-001, this file F-003) — a claim-ledger row should record that the documented contract has no test.
