# AUDIT: crates/transcode/src/queue.rs
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-15 |
| Lines / functions | 482 / 13 non-test + 6 tests (+2 helpers) |
| File verdict | REAL (working bounded SPSC queue) with one real concurrency defect in the interrupt path |
| Reachable from shipped entry point? | NO — `transcode` crate has zero dependents (library-only; see wav_roundtrip audit) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `cargo test -p transcode --lib queue::` → 6 passed/0 failed (exit 0); sed quote checks; rg callers |

## 1. One-paragraph verdict
A real two-axis (slots + bytes) bounded queue with mutex + two condvars, saturating
accounting, FIFO, drain-on-close, and an honest oversized-packet refusal. The
parking/wait logic is correct for close and for data movement (all state
mutations + notifies share the mutex). The defect: `interrupt()` notifies both
condvars WITHOUT holding the mutex while the wake condition is an external
`AtomicBool` — a notify that lands in the predicate-eval-to-park window is lost,
so a cancelled job can leave a parked producer or consumer asleep forever. The
module doc explicitly claims "no waiter may miss the signal"; that invariant does
not hold. The claimed provenance (`packet-queue`) never makes this mistake — its
`close`/`flush` notify under the lock (queue.rs:310-314, 338-372 there).

## 2. What actually works (verified)
— verified by `cargo test -p transcode --lib queue::` → 6 passed; 0 failed (exit 0).
The backpressure test (`full_queue_parks_the_producer_backpressure_not_growth`) is
a real 2-thread proof: parked producer, sampled high-water marks ≤ both axes, FIFO
across parks, interrupt-unpark assertions.

## 3. Findings
F-001 [S2]
Where: crates/transcode/src/queue.rs:175-182 (interrupt) vs 238-240 (wait_while)
Code:
```
    pub(crate) fn interrupt(&self) {
        self.cancel.cancel();
        self.space.notify_all();
        self.items.notify_all();
    }
```
What is wrong: Lost-wakeup window. The producer parks via
`wait_while(state, |s| !s.closed && !self.cancel.is_cancelled() && !s.can_accept(bytes))`
(the consumer analogously at L260-262). The cancel flag lives in a separate
`Arc<AtomicBool>` NOT protected by the queue mutex, and `interrupt()` calls
`notify_all()` without taking that mutex. Interleaving: (1) waiter holds the
mutex, evaluates the predicate — `is_cancelled()` still false, queue full →
predicate true, about to park; (2) interrupter sets the flag and fires
`notify_all` — the waiter is not yet queued on the condvar, so the wake is lost;
(3) waiter parks; the predicate is never re-evaluated; the thread sleeps until
an unrelated push/pop/close on the same queue — which a torn-down job never
issues → `job.wait()` hangs. Contrast `close()` at L187-192 which takes the lock
first (correct), and `packet-queue`'s `close`/`flush`, which notify under the
lock. The module doc's claim (L14-18) "no waiter may miss the signal" is false
for this path; the comment at L234-237 defends the opposite direction (a wake
never misses the flag) and does not cover this window. Why it matters: prompt
cancel is this queue's stated purpose (runner.rs:225 calls it on teardown).
What correct looks like: `interrupt()` takes the state mutex (then cancels +
notifies), or the cancel flag moves under the queue's mutex. Confidence: HIGH —
Verification: read; UNVERIFIED by execution (nanosecond race not reproducible on
demand; the passing `interrupt_wakes_a_parked_consumer` test only covers the
already-parked case).

F-002 [S3]
Where: crates/transcode/src/queue.rs:147-153 vs 110-117
Code:
```
pub(crate) struct PacketStageQueue {
    state: Mutex<State>,
    space: Condvar,
    items: Condvar,
    cancel: CancelToken,
    max_bytes: usize,
}
```
What is wrong: Crate unreachable from any shipped entry point (zero dependents —
same crate-level finding as the wav_roundtrip audit), so this defect ships
nowhere; recorded for the day the Convert UI (7.E3) wires it. Confidence: HIGH —
Verification: read (rg dependency graph).

F-003 [S4]
Where: crates/transcode/src/queue.rs:152 and 115
Code:
```
    cancel: CancelToken,
    max_bytes: usize,
```
What is wrong: `max_bytes` is stored twice (queue field + `State.max_bytes`); the
pre-lock `TooLarge` check (L215) uses the outer copy, `can_accept` the inner one.
They are always equal today (both set in `new`), but nothing enforces it — a
future setter would split the truth. What correct looks like: one copy.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| unpoison | 55-60 | REAL | — | poison recovery; critical sections infallible by construction |
| State::can_accept | 120-123 | REAL | — | two-axis check, saturating add |
| State::accept | 125-131 | REAL | — | high-water marks maintained |
| State::take | 133-138 | REAL | — | FIFO pop, byte release |
| PacketStageQueue::new | 158-173 | REAL | S4 | dup max_bytes (F-003) |
| PacketStageQueue::interrupt | 178-182 | REAL | S2 | notify without lock (F-001) |
| PacketStageQueue::close | 187-192 | REAL | — | lock + closed + notify — correct pattern |
| len/is_empty (cfg(test)) | 195-204 | REAL | — | test observability |
| stats | 207-209 | REAL | — | copy under lock |
| push_wait | 213-242 | REAL | S2 | TooLarge pre-check honest; park loop correct except F-001 window |
| pop_wait | 246-264 | REAL | S2 | drain-before-closed correct; same F-001 window |
| too_large_error | 267-269 | REAL | — | maps to transcode.limits.over-budget |
| tests ×6 | 307-481 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| push_pop_roundtrip_fifo | YES | order + stats + peaks pinned |
| closed_queue_refuses_pushes_keeps_residents | YES | exact PushWait::Closed + resident count |
| byte_axis_refuses_the_oversized_packet | YES | 16 ok / 17 TooLarge + error code + park-not-refuse for fitting-after-pop |
| full_queue_parks_the_producer_backpressure_not_growth | YES | bounded sampling + FIFO across parks + interrupt results vector pinned per index |
| interrupt_wakes_a_parked_consumer | YES (partially) | covers only the ALREADY-PARKED consumer; does not (cannot cheaply) cover the F-001 eval-to-park window |
| close_drains_residents_before_closed | YES | drain order + idempotence pinned |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no waiter may miss the signal" | queue.rs:14-18 | FALSE in one interleaving | F-001: notify without mutex + external atomic flag |
| "the lock is held only for deque push/pop and saturating arithmetic" | queue.rs:8-10 | TRUE for data/close paths, and exactly why interrupt is racy | read L187-192 vs L178-182 |
| "every counter is saturating" | queue.rs:19-21 | REAL | read L127-136; saturating_add/sub throughout |
| "a packet whose payload alone exceeds the byte axis can never be accepted" | queue.rs:25-30 | REAL | L215-219 pre-lock check + test |

## 7. Dead / duplicated / unreachable code
- Crate library-only (F-002). No dead functions inside the file.

## 8. Dependencies & flags
std (VecDeque/Mutex/Condvar), media_types::Packet, crate cancel/error. No
features. `len`/`is_empty` correctly cfg(test).

## 9. Open questions for the next agent
- Does `packet-queue`'s own interrupt-equivalent (if any) take the lock? If the
  pattern there is also lockless, the bug may be a workspace-wide doctrine error.
- runner.rs teardown: is there any post-interrupt push/pop that would mask the
  lost wakeup in practice (e.g. close() always following interrupt)?
