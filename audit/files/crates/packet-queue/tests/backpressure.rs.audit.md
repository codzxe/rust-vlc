# AUDIT: crates/packet-queue/tests/backpressure.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 489 / 13 test fns + 5 helpers |
| File verdict | REAL (strong behavioral suite; one structural coverage gap) |
| Reachable from shipped entry point? | N/A (test) — plain `#[test]`s, no ignores, no cfg gates; part of W00's 1751-passing baseline. Execution UNVERIFIED here (no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | Read; cross-check against queue.rs (my file #3); sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

The queue crate's acceptance suite, and it is the real thing: real threads, real condvar parking, per-axis cap proofs (packets/bytes/duration each pinned to `LimitAxis`), FIFO across overflow cycles, stats accounting, close semantics, watermark cycling, and a lock-freedom-while-parked proof. These tests would fail if the queue's core contracts broke — they are the evidence behind my queue.rs REAL verdict. Two gaps: the suite never parks more than ONE producer, so the crate's "MPMC-capable" claim is tested only in SPMC shape and the notify_one starvation defect I filed against queue.rs (F-001 there) is structurally invisible to this suite; and the park-admission sleeps are timing-based (mitigated by generous 5 s timeouts).

## 2. What actually works (verified)

- Per-axis proofs: packet axis (72-118, `LimitAxis::Packets` + untouched item + stats.full_rejections==1 + FIFO 1,2,3,4), byte axis (123-157), duration axis (161-184, resident duration 900 ns asserted). Each would fail if `binding_axis` mislabeled the refusing cap.
- Oversized-item livelock invariant (194-225): an item heavier than the whole budget admits alone into the empty queue, accumulation behind it is refused, drain re-opens admission — the 6.FIX-pcm regression guard, with the historical failure documented in the comment (188-192).
- Parking proofs with real threads: wake-on-drain (230-257), bounded deadline (262-283, elapsed ≥120 ms and <5 s around a 150 ms deadline, full_rejections counted), immediate return with room (287-296), close-wakes-parked-producer (301-324), parked-consumer-wakes-on-push (329-350).
- Close semantics (356-382): producers refused with item in hand (both try and parking paths), resident data drainable, then `Closed` — never silent destruction.
- Lock discipline proof (444-489): while a producer is parked, `try_push` answers `Full` and `try_pop` completes within 500 ms — a parked producer demonstrably holds no lock; FIFO of the parked producer's item verified after wake.
- `join_or_propagate` (62-66) resumes panics on the main thread — a producer panic fails the test instead of vanishing.

## 3. Findings

### F-001 [S3] "MPMC-capable" is never tested with more than one producer — the starvation defect is invisible to this suite
Where: crates/packet-queue/tests/backpressure.rs:230-257, 301-324, 444-489 (all multi-thread tests) vs queue.rs F-001
Code (238-245):
```rust
    let producer = thread::Builder::new()
        .name("ac-parked-producer".into())
        .spawn(move || {
            let started = Instant::now();
            let result = parked_queue.push_wait(stamped(1), None);
            let _ = tx.send((result, started.elapsed()));
        })
```
What is wrong: Every parking test spawns exactly one producer and (at most) one consumer. The crate doc's "MPMC-capable" (lib.rs:38) and the `notify_one` wakeup discipline (queue.rs:503, 524) are exercised only in the single-parked-producer shape. With two parked producers of different item sizes, the wake can land on the producer whose item still does not fit, which re-parks without forwarding the wakeup (queue.rs F-001) — no test in this file can detect that, because no test creates the second parked producer.
Why it matters: The suite provides false confidence precisely where the queue's API is weakest. The passing suite is being used (via lib.rs docs) to substantiate a threading model that is both untested in its MPMC shape and unused in production.
What correct looks like: A test parking N producers with heterogeneous `size` behind a full queue, draining one small slot, and asserting ALL producers eventually complete (with a deadline), which fails under the current notify_one discipline.
Confidence: HIGH — Verification: read (test inventory + queue.rs analysis).
### F-002 [S4] Park-admission and timing bounds rely on wall-clock sleeps
Where: crates/packet-queue/tests/backpressure.rs:248, 316, 342, 462
Code (247-248):
```rust
    // Let the producer reach the condvar, then drain one slot.
    thread::sleep(Duration::from_millis(100));
```
What is wrong: "Let the producer park" is enforced by sleeping 100 ms. Under a pathologically loaded CI runner, 100 ms may not suffice for thread scheduling; the test then pops before the producer parks, the producer's later `push_wait` sees space immediately, and the test still passes — the wake-on-space path silently degenerates to the immediate path. The asserts (completion within 5 s) remain valid, so it is a weakened-proof risk, not a flake risk.
Why it matters: The parking proof can silently stop proving parking. Margins are generous (100 ms vs sub-ms park), so in practice this holds; hygiene.
What correct looks like: A pre-park handshake (producer signals "about to park" via channel before calling push_wait, or poll `stats()`/a test-only handle), or accept and document the sleep.
Confidence: MEDIUM — Verification: read (reasoning; no run).

## 4. Per-function table

| Test | Lines | Verdict | Notes |
|---|---|---|---|
| ac_overflow_backpressure_proof_packets_axis | 72-118 | REAL | Exact per-axis + FIFO + stats. |
| ac_overflow_backpressure_proof_byte_axis | 123-157 | REAL | Byte axis pinned. |
| duration_cap_refuses_pushes | 161-184 | REAL | Duration axis pinned. |
| oversized_item_admits_alone_and_bounds_accumulation | 194-225 | REAL | Livelock regression guard (6.FIX-pcm). |
| push_wait_parks_until_consumer_drains | 230-257 | REAL | Real thread; F-002 sleep note. |
| push_wait_deadline_bounds_the_park | 262-283 | REAL | Bounded park with elapsed window. |
| push_wait_returns_immediately_with_room | 287-296 | REAL | Generous 500 ms bound. |
| close_wakes_parked_producer | 301-324 | REAL | Typed Closed abort, item intact. |
| parked_consumer_wakes_on_push | 329-350 | REAL | Consumer-side condvar. |
| close_refuses_producers_and_drains_consumers | 356-382 | REAL | Full close contract. |
| watermark_flags_track_the_backpressure_cycle | 387-410 | REAL | Q4 flags + usage. |
| stats_count_backpressure_events | 414-436 | REAL | full_rejections = 2 Full + 1 Deadline. |
| parked_producer_does_not_hold_the_lock | 444-489 | REAL | Lock-freedom under park; strongest threading test here. |

Helpers: `Blob`/`Weigh` impl (24-39, real per-axis weights), `blob`/`stamped`/`limits` (41-60), `join_or_propagate` (62-66, panic propagation).

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 13 | YES (with F-001/F-002 scope caveats) | Each asserts exact outcomes/identities/counts; mutation examples: swap `notify_one`→remove → parked tests hang→timeout fail; mislabel binding axis → axis tests fail; drop FIFO → order asserts fail; count nothing → stats tests fail. |

No mocks-of-subject (subject is the real queue; `Blob` is a legitimate test payload implementing the real `Weigh` seam). No ignores. No circular goldens.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The parking variant proves the wake-on-space path, and the deadline variant proves the park is bounded" (5-7) | module doc | REAL | Tests 230-296. |
| (Indirect) "MPMC-capable" via lib.rs | packet-queue lib.rs:38 | SHALLOW | Never tested with >1 producer (F-001). |
| Q3 "never spins" | doc 03 §4.4 via module docs | REAL-UNPROVEN | No spin observed by construction (wait_while); no test measures CPU during park — not falsifiable by this suite beyond the completion bounds. |

## 7. Dead / duplicated / unreachable code

None. Note: this suite is the ONLY consumer of `push_wait`/`pop_wait`/`PushAborted` in the entire repo (production-dead per queue.rs F-002) — the suite tests real code that no shipped binary reaches.

## 8. Dependencies & flags

dev-deps: testkit (Must), media-types. std threads/mpsc. No feature gates; runs in default profile.

## 9. Open questions for the next agent

- Whoever fixes queue.rs F-001: add the multi-producer heterogeneous-size test described in F-001 here.
- W20 verification: this file's suite passing is cited in my queue.rs audit as the behavioral evidence — a re-run of `cargo test -p packet-queue` would confirm (UNVERIFIED here).
