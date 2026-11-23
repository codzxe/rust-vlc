# AUDIT: crates/packet-queue/tests/zero_alloc_queue.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-23 |
| Lines / functions | 149 / 4 (2 discipline helpers + 1 type + 1 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only file, but it fences the PRODUCTION queue: `packet_queue::BoundedQueue` is the engine's per-limb queue (crates/playback-engine/src/pipeline.rs:552 `pub queue: Arc<BoundedQueue<Packet>>`), compiled into the release lane via lumen-cli. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Direct run `./target/debug/deps/zero_alloc_queue-e04b54f1b29424de` -> 1 passed / 0 failed (exit 0, 0.13 s); baseline audit/runs/baseline/cargo-test-workspace.txt:3926; sed verifications. |

## 1. One-paragraph verdict
A genuine zero-allocation fence, not a decoration: the process-global counting allocator guards a 100,000-iteration loop over the production `BoundedQueue` exercising the engine tick's exact queue surface (try_push, push_wait-with-deadline, try_pop, watermark, stats, usage) on a half-full queue, and the assert is a hard `(alloc_calls, total_bytes) == (0, 0)`. The foreign-noise discipline (settle pause + one deterministic replay, both-dirty fails) is a sound D-5 hardening: a real defect reproduces on replay and dies; transient libtest thread-spawn noise does not. My direct run passes in 0.13 s. Two hygiene-level gaps: the header sells this as the "D-1 anchor for the threaded demux future (the parked producer's wake path ... exercised here)" while the body admits the park is SKIPPED (room always exists) — the condvar timed-wait path is never inside the measured window, only the deadline/lock fast path; and the replay mechanism can mask a once-per-process lazy allocation in `usage()` specifically, because the warm-up calls watermark+stats but NOT usage before the guard opens.

## 2. What actually works (verified)
— `./target/debug/deps/zero_alloc_queue-e04b54f1b29424de` -> `test result: ok. 1 passed; 0 failed` (exit 0). Verified by read: (1) the guard opens AFTER queue construction, pre-fill, and the warm-up surface (116), so construction/pre-fill allocations cannot pollute the window; (2) the loop is net-zero per round (+2 pushes, −2 pops) with the queue held at its primed half-fill, so caps never bind and the accept fast path is what runs — matching the engine's single-writer tick shape; (3) `push_wait` with a 50 ms deadline while room exists asserts `is_ok` (132) — the deadline arithmetic and lock are really executed; (4) the outer assert (144-148) is on the delta returned by clean_replay, which is either the clean first window or the clean second — a dirty second fails inside clean_replay with both windows' numbers in the message. Mutation reasoning: any per-iteration heap allocation in the queue's push/pop/watermark/stats/usage path makes both replays dirty and fails loudly.

## 3. Findings
F-001 [S4]
Where: crates/packet-queue/tests/zero_alloc_queue.rs:4-8 vs 76-80
Code:
```
//! the regression fence for 3.B3's changes (the engine's
//! backpressure additions must not introduce heap churn into the
//! queue's hot path) and the D-1 anchor for the threaded demux future
//! (2.E1: the parked producer's wake path is the same mutex/condvar
//! arithmetic exercised here).
```
vs the body's own admission:
```
/// The steady-state push/pop cycle (100k iterations over a half-full
/// queue, with watermark and stats queries interleaved — the engine
/// tick's exact queue surface) allocates zero bytes. The parking
/// variants ride the same path with a live deadline (no sleep: room
/// exists, so the park is skipped).
```
What is wrong: the header claims the parked producer's wake path is "exercised here"; the test never parks — the queue always has room, so `push_wait` takes the accept fast path and the condvar timed-wait (the actual parking/wake machinery of the 2.E1 future) is never inside the measured window. The fence therefore proves nothing about the park path's allocation behavior; it covers only the engine's real single-writer hot path (which is try_push/pop — adequate for today's engine, inadequate for the claimed future anchor). What correct looks like: either a second measured window that actually fills the queue and parks/wakes a producer, or an honest header that scopes the proof to the fast path. Confidence: HIGH — Verification: read + ran.
Confidence: HIGH — Verification: read / ran

F-002 [S4]
Where: crates/packet-queue/tests/zero_alloc_queue.rs:101-104 (warm-up) vs 139 (loop)
Code:
```
        // Warm-up: the first watermark/stats queries and a parking push
        // with room (the condvar fast path) settle any lazy internals.
        let _ = queue.watermark();
        let _ = queue.stats();
```
and in the measured loop:
```
            let usage = queue.usage();
            let _ = usage.bytes;
```
What is wrong: the warm-up settles "any lazy internals" of watermark/stats/push_wait — but `usage()` is called only inside the measured loop, not in the warm-up. A hypothetical once-per-process lazy allocation inside `usage()` would dirty the FIRST window, and `clean_replay`'s replay (45-59) would then see a clean second window and PASS — the defect masked exactly by the noise-hardening meant to protect the proof. Today's `usage()` allocates nothing (the test passes with window 1 clean, or transiently dirty only from thread spawns), so this is a latent hole in the discipline, not a live bug. What correct looks like: warm up the full loop surface (add `let _ = queue.usage();` next to the watermark/stats warm-up). Confidence: HIGH — Verification: read (usage() absent from warm-up; window-1 cleanliness unprovable from outside — my run passed, consistent with either).
Confidence: HIGH — Verification: read / ran

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| settle_foreign_spawns | 37-39 | REAL | — | 30 ms settle; observed-noise numbers cited in comment |
| clean_replay | 45-59 | REAL | S4 | both-dirty fails, first-dirty replays — masks once-per-process allocs in un-warmed surfaces (F-002) |
| Blob + Weigh impl | 61-74 | REAL | — | fixed 4 B / 1 µs weights |
| steady_state_push_pop_cycle_allocates_nothing | 81-149 | REAL | S4 | 100k net-zero rounds, hard (0,0) assert; park path skipped (F-001); ran ok |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| steady_state_push_pop_cycle_allocates_nothing | YES for any per-iteration allocation on the exercised surface (both replays dirty → loud fail with numbers); NO for (a) the condvar park/wake path (never parked, F-001) and (b) a once-per-process allocation in usage() (replay-masked, F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the push/pop loop over a pre-filled BoundedQueue allocates nothing — measured with the testkit counting allocator" | zero_alloc_queue.rs:1-3 | REAL | hard (0,0) assert over guarded 100k rounds; my run ok |
| "the parked producer's wake path is the same mutex/condvar arithmetic exercised here" | zero_alloc_queue.rs:6-8 | MISLEADING | the park is skipped by construction (76-80); only the deadline/lock fast path runs (F-001) |
| "One #[test] per binary by design (the allocator is process-global ...)" | zero_alloc_queue.rs:10-12 | REAL | single test; serialize_tests held |
| "the section here is deterministic — no clocks inside the window, one thread" | zero_alloc_queue.rs:18-19 | PARTLY | Instant::now() deadlines ARE constructed inside the loop (117, 128-131) — clock READS happen in the window; the claim presumably means no sleeping/pacing. Wording overstates. |

## 7. Dead / duplicated / unreachable code
None. No #[ignore], no todo!/unimplemented! (rg verified). The `first` window's delta is used only for the dirty check and the error message — by design.

## 8. Dependencies & flags
packet_queue (the crate under test), media_types (GenerationId/Nanoseconds), testkit (Must + alloc_count: CountingGlobalAlloc/AllocDelta/serialize_tests). Process-global `#[global_allocator]`. No feature gates; std-only.

## 9. Open questions for the next agent
- Does `BoundedQueue::usage()` (crates/packet-queue/src) contain any lazy/one-time path the watermark warm-up would not cover? If yes, F-002 is live, not latent.
- The demux zero-alloc twin binary (referenced at 16-17) — same clean_replay discipline? If it shares the warm-up gap, the finding generalizes (that file is another agent's).
