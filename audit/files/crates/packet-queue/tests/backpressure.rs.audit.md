# AUDIT: crates/packet-queue/tests/backpressure.rs
| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-17 |
| Lines / functions | 489 / 18 (13 #[test], 5 helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES for the code under test: `BoundedQueue` push/pop/close/watermark/stats is the production queue used by crates/playback-engine/src/pipeline.rs (engine sessions of apps/cli + apps/web). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | baseline cargo-test-workspace.txt:3852-3872 (13 passed, 0 failed, 0.30s); sed quote verification. |

## 1. One-paragraph verdict
A focused, honest AC suite for card 2.B1: all three backpressure axes (packets, bytes, duration) are pinned with exact typed outcomes (`Full { binding: LimitAxis::… }`, item handed back untouched, `full_rejections` counted), the livelock invariant from a real past bug (6.FIX-pcm: wav 64 KiB packets vs the audio lane's 1 s duration cap) is regression-pinned, and five genuinely-threaded tests cover the condvar protocol (park-on-full, wake-on-space, deadline abort, close broadcast for both sides, and the lock-not-held-while-parked proof). Assertions are exact wherever deterministic. One test-quality gap: `parked_producer_does_not_hold_the_lock` joins a deadline-less parked producer, so a lost-wakeup bug would HANG the test binary instead of failing loudly — the one place this crate's otherwise-uniform "never a hung gate" discipline (concurrency.rs:592-597) is violated.

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt:3852-3872` -> "running 13 tests … test result: ok. 13 passed; 0 failed; 0 ignored … 0.30s".
— verified by sed: the three axis tests assert `binding == LimitAxis::Packets/Bytes/Duration` respectively with the item's identity preserved (lines 88-97, 144-150, 176-183).

## 3. Findings
F-001 [S3]
Where: crates/packet-queue/tests/backpressure.rs:444-457,480
Code:
```
            // Parks (nobody drains until the proof's own pop below).
            let _ = parked_queue.push_wait(stamped(1), None);
```
and
```
    // The pop freed the slot: the parked producer wakes and completes,
    // and its item is the next to pop (FIFO).
    join_or_propagate(producer);
```
What is wrong: The producer parks with NO deadline (`push_wait(…, None)`); if the queue's wake-on-space were broken, `join_or_propagate(producer)` blocks forever — the test binary hangs until an external timeout instead of failing with a message. Every other parking test in this crate carries a deadline or a bounded `recv_timeout`; this one relies on correctness to terminate. Compare concurrency.rs:592-597 ("No-deadlock is asserted the only sound way for condvar code: every park carries a deadline … never a hung gate"). Why it matters: a regression in the condvar wake path surfaces as an unkillable hang (CI stall), not a diagnostic failure. What correct looks like: park with a generous deadline (e.g. 30 s) and assert the abort never fires. Confidence: HIGH — Verification: read.
(Note: the 100 ms "reach the condvar" sleeps in tests 5/8/9/13 are NOT correctness-critical — if the thread has not parked yet, the triggering operation still produces the same terminal state for it — so they are not flakiness findings.)

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Blob + Weigh impl | 24-39 | REAL | — | Per-axis controllable weights. |
| blob / stamped / limits | 41-60 | REAL | — | Builders. |
| join_or_propagate | 62-66 | REAL | — | Panic propagation. |
| ac_overflow_backpressure_proof_packets_axis | 71-118 | REAL | — | Typed Full, axis pinned, FIFO across cycle, stats pinned. |
| ac_overflow_backpressure_proof_byte_axis | 123-157 | REAL | — | Byte axis before packet axis reached. |
| duration_cap_refuses_pushes | 161-184 | REAL | — | Duration axis; usage.duration pinned (900). |
| oversized_item_admits_alone_and_bounds_accumulation | 194-225 | REAL | — | Livelock invariant (6.FIX-pcm regression): empty queue admits oversized; accumulation still bounded. |
| push_wait_parks_until_consumer_drains | 230-257 | REAL | — | Threaded; 5 s recv bound; wake-on-space. |
| push_wait_deadline_bounds_the_park | 262-283 | REAL | — | Deadline abort, item back, elapsed bounds (≥120 ms, <5 s). |
| push_wait_returns_immediately_with_room | 287-296 | REAL | — | <500 ms immediate path. |
| close_wakes_parked_producer | 301-324 | REAL | — | Typed Closed abort with item. |
| parked_consumer_wakes_on_push | 329-350 | REAL | — | Consumer side of the condvar protocol. |
| close_refuses_producers_and_drains_consumers | 356-382 | REAL | — | Resident data survives close; Closed for both APIs; is_closed. |
| watermark_flags_track_the_backpressure_cycle | 387-410 | REAL | — | High/low marks through fill/drain (Q4 input). |
| stats_count_backpressure_events | 414-436 | REAL | — | full_rejections = 2 typed + 1 deadline. |
| parked_producer_does_not_hold_the_lock | 444-489 | REAL | S3 | F-001: unbounded join on a deadline-less park; lock-freedom proof otherwise sound (try_push/try_pop under 500 ms while parked). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 3 axis tests + oversized | YES | Exact typed-outcome and usage equality asserts. |
| push_wait_parks_until_consumer_drains | YES | 5 s recv bound; wrong result asserted. |
| push_wait_deadline_bounds_the_park | YES | Deadline abort shape + timing bounds. |
| push_wait_returns_immediately_with_room | YES | Timing bound + success. |
| close_wakes_parked_producer / parked_consumer_wakes_on_push | YES | Typed outcomes, 5 s bounds. |
| close_refuses_producers_and_drains_consumers | YES | Full ordering of refuse/drain/Closed. |
| watermark_flags / stats_count | YES | Exact flag and counter values. |
| parked_producer_does_not_hold_the_lock | PARTIAL | Lock-freedom asserts yes; but a lost-wakeup bug hangs rather than fails (F-001). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a queue at capacity rejects/signals the producer's push (typed Full, item handed back untouched)" | lines 3-5 | REAL | Axis tests: item identity asserted in every Full arm. |
| "the parking variant proves the wake-on-space path, and the deadline variant proves the park is bounded" | lines 5-7 | REAL | Tests 5/6 pass; bounded via deadline or recv_timeout (except F-001's join). |
| "the lock is never held while parked (doc 03 §4.4(c))" | lines 438-442 | REAL | try_push + try_pop complete <500 ms while a producer is parked. |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
packet_queue only (+ media_types, testkit). No feature gates, no unsafe, no loom.

## 9. Open questions for the next agent
- backpressure_property.rs (next file): does it close the Q1 exhaustive gap noted in concurrency.rs's audit?
- The watermark thresholds (`with_default_watermarks`) — the engine's Buffering transitions consume these; confirm the default high/low marks match doc 03 §4.4 figures in whoever audits queue_policy/queue.rs.
