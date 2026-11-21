# AUDIT: crates/transcode/src/queue.rs
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-11-21 |
| Lines / functions | 482 / 16 production items + 8 tests |
| File verdict | REAL (correct, well-tested) — UNREACHABLE from shipped entry points; one real concurrency race against its own doc claim |
| Reachable from shipped entry point? | NO. The queue is `pub(crate)`; its only consumer is `crate::runner` (runner.rs:140-144 constructs it, pump/worker loops drive it) inside the transcode crate, which no shipped binary reaches (proof in my runner.rs#part1 F-002: zero dependents in every app/Cargo.toml; product.toml:165 `transcode = false # Phase 7`). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 0 |
| Commands run | read 1-482 in full (all items) · hand-traced push_wait/pop_wait interleavings (park windows, close-vs-cancel ordering, drain-before-closed) · cross-read runner.rs:199-229 (exit layers that mitigate F-001) + limits.rs:90-108 (axis zero/ceiling validation protects the queue's precondition) · sed -n 14,18p / 175,192p (quote verification) · baseline #3 (queue tests pass; the backpressure test is the crate's bounded-memory AC) |

## 1. One-paragraph verdict
This is the transcode crate's best file: a textbook two-condvar bounded
queue with two-axis capacity, saturating accounting, drain-before-closed
semantics, and an unusually strong in-file test (the producer-parking
backpressure proof samples bounds across a window, verifies un-park
ordering by FIFO salt, and asserts every post-interrupt push comes back
`Interrupted`). One real defect: `interrupt()` fires the cancel token
and notifies both condvars WITHOUT holding the state mutex, so a waiter
in the window between predicate evaluation and park can miss the
notification and sleep forever — directly contradicting the module doc's
"no waiter may miss the signal" claim. In the runner's composition the
race is rescued by the pump/worker exit layers' `close()` (which sets
`closed` under the mutex, so it cannot be missed), which is why this is
S2 and not S1. And like the rest of the crate, none of it is reachable
from any shipped binary.

## 2. What actually works (verified)
— Two-axis admission: can_accept (120-123) checks slots AND
saturating-bytes; the "one honest overflow" (packet alone over the byte
axis → TooLarge before any waiting, 215-219) is deadlock-correct —
parking on a packet that can never fit would hang the pump.
— Drain-before-closed: pop_wait (249-252) takes a resident packet BEFORE
checking `closed`, so a closed queue delivers its backlog then reports
Closed — pinned by close_drains_residents_before_closed (470-481,
idempotence asserted).
— close() is race-free: it sets `closed = true` under the state mutex and
notifies while holding it (187-192) — a waiter either sees `closed` in
its predicate or is already parked when the notify lands. Traced both
interleavings.
— FIFO and stats: accept/take maintain resident_bytes symmetrically
(data.len() on both sides), peak marks updated on accept only; the
roundtrip test pins pushed/popped/peak_bytes/peak_slots exactly (322-326).
— Saturating arithmetic everywhere counters mutate (module claim at
19-21 holds — every site verified: 127-128, 135-136, 233, 259).
— Pre-allocation bounded: VecDeque::with_capacity(max_slots.min(64))
(161) — a huge slot axis does not pre-allocate hugely.
— Axes cannot be zero through the public surface: JobLimits::new
rejects zero axes (limits.rs:99-101, verified), and the queue is only
constructed with validated limits (runner.rs:140-144). A direct
`PacketStageQueue::new(0, ..)` would park the producer forever, but the
constructor is pub(crate) and every call site passes validated values.
— Tests pass in baseline #3; the backpressure test would fail (or hang
into its 5s `eventually` deadline → panic) on any bound violation or
lost un-park wakeup.

## 3. Findings

### F-001 [S2]
Where: crates/transcode/src/queue.rs:175-182 (interrupt), predicate at 238-240 / 260-262, doc claim at 14-18
Code:
```rust
    pub(crate) fn interrupt(&self) {
        self.cancel.cancel();
        self.space.notify_all();
        self.items.notify_all();
    }
```
against the documented invariant (14-18):
```rust
//! * **Interrupt, not just close** — cancellation fires the shared
//!   [`CancelToken`] and calls [`PacketStageQueue::interrupt`],
//!   which notifies both condvars so every parked waiter wakes and
//!   re-checks (the reconnect executor's wake-on-generation-move
//!   doctrine: no waiter may miss the signal).
```
What is wrong: The cancel state the wait predicates read is an
AtomicBool OUTSIDE the state mutex, and interrupt() notifies without
holding that mutex. A consumer/producer inside `wait_while` holds the
mutex while evaluating the predicate (`!self.cancel.is_cancelled()`)
and parks atomically on releasing it; interrupt() can fire
token-store + notify_all entirely inside that evaluation-to-park
window. The notify finds no parked waiter (the mutex does not gate it),
the thread then parks, and — interrupt being one-shot — nothing ever
wakes it: a lost wakeup, an indefinite hang of the parked stage. The
comment at 234-237 ("interrupt() fires the token FIRST, then notifies,
so a racing wake can never observe the notification without the flag")
answers a different race (flag-vs-notify ordering) and does not cover
the notify-before-park window.
Why it matters: A hang is worse than a crash: no panic, no terminal, the
job never settles. The module's own doctrine line ("no waiter may miss
the signal") is false for interrupt(). Mitigation that keeps this at
S2: in the runner's composition, every thread's exit layer calls
`close()` (runner.rs:203-204, 220), which IS mutex-synchronized and
provably un-missable, so a parked peer is released by teardown even if
interrupt's notify is lost; standalone use of the queue (as in the
file's own interrupt test, 449-467, which relies on the `eventually`
poll making the window practically unreachable) would hang forever.
What correct looks like: interrupt() should acquire the state mutex
(even a bare `drop(unpoison(self.state.lock()))`) before notifying, so
no waiter can be between predicate and park while the notify fires; or
the cancel flag should be mirrored into the mutex-protected State.
Confidence: HIGH — Verification: read (interleaving traced against
std Condvar::wait_while semantics — predicate under lock, atomic
release-and-park; close() contrasted at 187-192). Not executed (no
cargo runs allowed; a deterministic repro needs a targeted stress
harness — see REQUEST-ORCH-RUN in the log).

### F-002 [S3]
Where: crates/transcode/src/queue.rs:147-153 (the whole type)
Code:
```rust
pub(crate) struct PacketStageQueue {
    state: Mutex<State>,
    space: Condvar,
    items: Condvar,
    cancel: CancelToken,
    max_bytes: usize,
}
```
What is wrong: Nothing in the type — the crate is dead code in every
shipped binary (transcode has zero dependents; product.toml Phase-7
gate; proof in my runner.rs#part1 F-002). 482 lines of concurrency
machinery and its proof suite are compile-and-test-only.
Why it matters: Same as the crate-level deadness already recorded
(W01-A03 F-002, runner.rs#part1 F-002): CI pays for it, users get none
of it, and its quality signals (the backpressure proof) must not be
cited as evidence of shipped streaming behavior.
What correct looks like: Phase-7 wiring or a recorded test-only status.
Confidence: HIGH — Verification: read (dependency greps in the
runner.rs#part1 audit).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-39 | REAL | S2 | Pattern provenance accurate (verified against packet-queue shape); the "no waiter may miss the signal" line is falsified by F-001. |
| `unpoison` | 49-60 | REAL | — | Poison-recovery justified: critical sections are deque ops + saturating arithmetic + notify (no panic paths). |
| `PushWait` | 62-77 | REAL | — | Four outcomes; "packet stays with the caller" holds (TooLarge pre-check, Closed/Interrupted before take). |
| `PopWait` | 79-88 | REAL | — | Item/Closed/Interrupted. |
| `QueueStats` | 90-106 | REAL | — | Six counters; parks counts episodes (increment before wait_while) — can overcount by one in a benign token-vs-park race; observability-only. |
| `State` | 108-117 | REAL | — | Data under mutex. |
| `State::can_accept` | 120-123 | REAL | — | Two-axis, saturating add. |
| `State::accept` | 125-131 | REAL | — | Peak marks updated under lock. |
| `State::take` | 133-138 | REAL | — | resident_bytes symmetric with accept (data.len()). |
| `PacketStageQueue` | 141-153 | REAL / UNREACHABLE | S3 | F-002. |
| `new` | 156-173 | REAL | — | Capacity pre-alloc capped at 64 slots. |
| `interrupt` | 175-182 | REAL | S2 | F-001 lost-wakeup race. |
| `close` | 184-192 | REAL | — | Mutex-synchronized state change + notify — provably un-missable; idempotent. |
| `len` / `is_empty` | 194-204 | REAL | — | cfg(test) observability. |
| `stats` | 206-209 | REAL | — | Copy under lock. |
| `push_wait` | 211-242 | REAL | — | TooLarge pre-check; closed/cancel checks inside the loop; wait_while predicate covers all three wake reasons. Correct. |
| `pop_wait` | 244-264 | REAL | — | Drain-before-closed ordering correct; interrupt race per F-001. |
| `too_large_error` | 266-269 | REAL | — | Static map to the typed limit error; code pinned by test (347-348). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| push_pop_roundtrip_fifo (307) | YES | Exact FIFO order + exact stats values. |
| closed_queue_refuses_pushes_keeps_residents (329) | YES | Closed semantics pinned both sides. |
| byte_axis_refuses_the_oversized_packet (339) | YES | 16/17 boundary + post-pop re-accept + typed error code. |
| full_queue_parks_the_producer_backpressure_not_growth (360) | YES | The bounded-memory AC: parks a real thread, samples bounds 20×, verifies un-park by stats, asserts all post-interrupt pushes Interrupted, FIFO across parks, final bound, token fired. A lost un-park wakeup trips the 5s `eventually` deadline → panic. |
| interrupt_wakes_a_parked_consumer (449) | YES in practice / RACY in theory | Depends on F-001's window being practically unreachable (parks stat observed via mutex, then 2ms-scale delay before interrupt); a lost wakeup hangs the join forever (test timeout, not assertion). |
| close_drains_residents_before_closed (469) | YES | Drain + idempotent Closed + push-refused. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no waiter may miss the signal" (interrupt doctrine) | queue.rs:14-18 | FALSE (narrow race) | F-001: notify without the mutex can land in the pred-eval→park window. |
| "the lock is held only for deque push/pop and saturating field arithmetic" | queue.rs:7-10 | REAL | All lock scopes verified (accept/take/stats/close; push/pop loops). |
| "a packet whose payload alone exceeds the byte axis can never be accepted" | queue.rs:23-30 | REAL | 215-219 pre-check + test 339. |
| "resident slots never exceed the slot axis, resident bytes never exceed the byte axis" | queue.rs:32-39 | REAL | can_accept + the backpressure proof test. |
| Saturating-everywhere policy | queue.rs:19-21 | REAL | Every counter mutation site listed in §2. |

## 7. Dead / duplicated / unreachable code
The entire file is unreachable from shipped entry points (F-002). No
dead items within the file: len/is_empty are cfg(test) by design;
too_large_error is called from runner.rs:404.

## 8. Dependencies & flags
media-types (Packet/SharedBytes), crate::cancel (CancelToken),
crate::error (limit_over_budget). std-only, no unsafe, no feature
gates. The mutex-poisoning policy (unpoison) is documented and sound.

## 9. Open questions for the next agent
- A06 (job.rs): the JobEventHub — does its interrupt path share this
  notify-without-lock shape? If yes, F-001 generalizes.
- A05 (runner.rs#part2): TrackHandoff::wake/close — same question; the
  handoff is the other parked surface the runner depends on.
- REQUEST-ORCH-RUN (logged): a stress test that parks a consumer,
  spins a second thread hammering interrupt() in a loop with randomized
  delays, and joins with a timeout — would empirically expose F-001's
  window (expected: rare indefinite hang).
