# AUDIT: crates/packet-queue/tests/generation.rs

| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-22 |
| Lines / functions | 419 / 9 `#[test]` + helpers (Blob, stamped, limits, join_or_propagate) |
| File verdict | REAL — a tight, exact-value acceptance suite for the generation semantics; zero findings above S4 |
| Reachable from shipped entry point? | N/A (test). Subject: the queue's generation/staleness surface — live production code. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read (full file); Grep `#[test]` = 9, no `#[ignore]`/cfg/env gates; cross-ref queue.rs flush/marker semantics for the zero-drop-marker confirmation. No cargo — W00 baseline cited. |

## 1. One-paragraph verdict

Card 2.B1 AC-b's generation stale-drop proofs, all nine of them
exact-value: the wholesale drop is verified field-by-field
(`Discontinuity{new_generation, dropped, dropped_bytes,
dropped_duration}` compared with `assert_eq!` against a literal),
late old-generation pushes come back typed and untouched with
`current` echoed, the parked-producer and parked-consumer races use
`recv_timeout(...).must()` so a lost wakeup FAILS the test instead of
hanging it (the correct shape — contrast concurrency.rs F-001),
unobserved markers merge with exact totals, a regressing flush drops
but never unpublishes, a zero-drop flush still signals (the very
behavior that makes property.rs's drain loop dead — confirmed here at
:317-332), staleness dominates backpressure on both push paths, and a
real `media_types::Packet` flows through `Stamped::from_payload` with
its own generation. The one defect is a comment overstatement: the
parked-consumer test claims the marker is observed "before any
new-generation item" but never pushes a new-generation item — the
ordering half of that sentence is untested (the same repo-wide gap I
flagged in property.rs/concurrency.rs/limits.rs).

## 2. What actually works (verified)

— 9 ungated `#[test]`s → all in W00's `cargo test --workspace` (1751 pass).
— Exact-field oracles: the flush report compared against a struct literal (:96-104), the merged marker against exact totals (:272-280), final stats against exact counters (:144-148), usage against `QueueUsage::EMPTY` / a literal (:85-92).
— The race tests cannot hang: both wait on `rx.recv_timeout(5 s).must()` (:200, :235) — a lost wakeup is a timeout panic, and `join_or_propagate` runs only after the result is in. This is the deadline-free-park done right (the producer parks with `None`, but the OBSERVER is bounded).
— `flush_on_empty_queue_publishes_and_signals` (:317-332) pins the zero-drop marker: `observed.dropped == 0` and the pop still returns `Discontinuity` — independently confirming my property.rs F-001 root cause (a 0-drop flush mints a pending marker).
— `stale_check_precedes_the_capacity_check` (:337-364): full-AND-stale resolves to `Stale` on both `try_push` and `push_wait`, with `usage().packets == 2` proving neither abort path touched the queue.
— `packet_stamps_flow_through_generation_bumps` (:369-419): the composite with a real `Packet` payload and `Stamped::from_payload` — the demux-side shape, not just the Blob abstraction.
— Mutation reasoning (UNVERIFIED, no cargo): silent drop (no marker) → :111-114/:324-331 fail; marker under-count → :272-280; regression of published gen → :302-305; stale push accepted → :119-126/:347-353; stale park (parking before the boundary check) → :161-171's elapsed bound or the abort-shape match; lost consumer wakeup → :235's timeout.

## 3. Findings

### F-001 [S4] The parked-consumer test's comment claims an ordering it does not test

Where: crates/packet-queue/tests/generation.rs:216-217
Code:
```rust
/// A parked consumer wakes on a flush and observes the discontinuity
/// before any new-generation item (the consumer side of §4.4(f)).
```
What is wrong: the test body (:219-245) parks a consumer on an EMPTY queue, flushes, and asserts the wake delivers `Discontinuity` with `dropped == 0`. No new-generation item is ever pushed, so "before any new-generation item" is never exercised — the test proves wake-with-marker, not marker-before-items. That ordering is the queue's documented contract (queue.rs:327) and remains unasserted repo-wide (property.rs F-001, concurrency.rs F-003, limits.rs table note — all mine, this file).
Why it matters: §3.4 names-vs-body; a reader credits the ordering law with a test it does not have.
What correct looks like: push a second-generation item after the flush and assert the parked consumer's FIRST observation is the marker, then the item.
Confidence: HIGH — Verification: read (whole test; no push of a second-generation item).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Blob`/`stamped`/`limits`/`join_or_propagate` | 25-66 | REAL | — | fixed 10 B / 100 ns weights; local-by-design documented (:23-24) |
| `ac_stale_generation_drop_proof` | 71-149 | REAL | — | the AC, field-exact; stats conservation at the end |
| `stale_push_wait_aborts_immediately` | 153-173 | REAL | — | boundary-before-park; elapsed bound 500 ms |
| `parked_producer_wakes_on_flush_with_stale_abort` | 179-214 | REAL | — | bounded observer (recv_timeout.must) — cannot hang; 100 ms pre-sleep is not load-bearing (a late park hits the boundary check and still aborts typed) |
| `parked_consumer_wakes_on_flush_with_discontinuity` | 216-245 | REAL | S4 | F-001 comment overstatement |
| `unobserved_flush_markers_merge` | 249-282 | REAL | — | exact merged totals |
| `flush_never_regresses_the_published_generation` | 287-312 | REAL | — | high-water pinned on both the return and the marker |
| `flush_on_empty_queue_publishes_and_signals` | 317-332 | REAL | — | zero-drop marker pinned (property.rs F-001's root cause confirmed) |
| `stale_check_precedes_the_capacity_check` | 337-364 | REAL | — | both push paths; queue untouched |
| `packet_stamps_flow_through_generation_bumps` | 369-419 | REAL | — | real Packet payload composite |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 9 | YES | exact literals for every outcome; bounded observers for both races; see the mutation list in §2 |

All nine ran in the W00 baseline (ungated).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "all gen-N packets are dropped wholesale with the typed signal (Discontinuity, never silent)" | header :3-5 | REAL | :95-114, :249-282, :317-332 |
| "a producer parked on a full queue wakes and observes PushAborted::Stale within a bounded number of wakeups" | header :6-8 | REAL | :179-214 (bounded by the 5 s observer + 2 s latency assert) |
| "observes the discontinuity before any new-generation item" | :216-217 | SHALLOW | F-001: the ordering is not tested |

## 7. Dead / duplicated / unreachable code

- None. The `Blob` duplication across packet-queue test files is documented and deliberate ("kept local so each proof file reads alone", :23-24) — a defensible trade, though testkit would still be one home.

## 8. Dependencies & flags

No feature flags, no env knobs. Two tests use `thread::sleep(100 ms)` to encourage parking — not correctness-load-bearing (both outcomes are typed aborts either way). Part of the default test run (W00 baseline).

## 9. Open questions for the next agent

- The marker-before-items ordering now has four audits pointing at the same untested contract — the claim ledger should carry one row: "consumer marker ordering (queue.rs:327): documented, untested repo-wide."
