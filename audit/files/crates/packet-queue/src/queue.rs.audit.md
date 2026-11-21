# AUDIT: crates/packet-queue/src/queue.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2026-11-21 |
| Lines / functions | 549 / 21 production units (no in-file tests; the suite lives in tests/*.rs) |
| File verdict | REAL (core queue executed in the shipped open/tick/seek lanes; push_wait/pop_wait implemented and suite-executed but unreachable from shipped hosts — only transcode uses them) |
| Reachable from shipped entry point? | YES, EXECUTED — `BoundedQueue::new` at playback-engine pipeline.rs:1114 (`build_limb`), `try_push` at pipeline.rs:2201, `try_pop` at pipeline.rs:2352, `flush`+`close` at pipeline.rs:1157-1158 (teardown), seek_flush 1254-1255, switch_limbs 1789-1790 — all inside `Pipeline::wire`/tick/teardown, driven by every baseline open (INDEX.md AVI lifecycle, sample-420.mp4 play; 60/60 frames decoded through these queues in rc-smoke-engine-only.log.txt). NO for `push_wait`/`pop_wait`: sole production callers are crates/transcode/src/runner.rs:392,518,605,659, and no shipped host invokes transcode (`rg -n "transcode" apps/cli/src/` → zero hits; CLI SERVED table has no transcode id) — engine uses only the non-blocking pair. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full read via Read tool (549 lines); `rg -n "push_wait\|pop_wait" crates/ apps/` (callers: transcode runner + this crate's tests only); `rg -n "try_push\|try_pop\|BoundedQueue::new" crates/playback-engine/src/pipeline.rs` (the engine call sites above); `rg -n "transcode" apps/cli/src/main.rs apps/cli/src/dispatch.rs` (no CLI surface); `rg -n "struct GenerationId" -B 3 crates/media-types/src/ids.rs` (PartialOrd+Ord derived — stamp comparisons well-defined); `rg -n "trait Weigh" -A 10 crates/packet-queue/src/stamped.rs` (size_bytes/media_duration contract, unknown duration = 0); awk over orchrun no-fail-fast log lines 3534-3740 (packet_queue: lib 17, backpressure 13, backpressure_property 2, concurrency 7, generation 9, limits 3, property 4, zero_alloc_queue 1 — ALL green) |

## 1. One-paragraph verdict
This is the cleanest file I have audited in the wave: a textbook
`Mutex<VecDeque>` + two-condvar bounded queue whose doc claims I could verify
line by line — Weigh genuinely runs outside the lock (386-387, 433-434),
every in-lock operation is deque push/pop plus saturating arithmetic (making
the `unpoison` poison-recovery helper's soundness argument true as stated),
every non-`Pushed`/non-`Item` outcome hands the item back (the `#[must_use]`
annotations enforce reading them), flush publishes a
never-regressing generation and merges consumer-side discontinuity accounting
with saturating adds, and both condvars are notified on close and flush so
parked producers/consumers cannot strand. The full suite (lib + 7
integration files incl. concurrency and property tests) ran green in the
no-fail-fast baseline. The one defect is an unspecified edge, not a wrong
line: the generation check refuses only stamps **older** than the published
generation, so a **future** stamp is silently accepted — harmless under the
engine's single-writer tick model (the mint and the flush land in the same
effect batch, before the next demux push), but this crate advertises itself
as MPMC-capable infrastructure, and a second producer stamped ahead of the
published generation would slip items across a boundary no consumer can
detect.

## 2. What actually works (verified)
— Infallible-critical-section claim (module doc 4-11) — verified by
enumerating every lock-holding body: `accept` (push_back + 3 saturating
adds), `take_front` (pop_front + saturating subs), `usage`, `close`,
`flush`, the predicate closures (pure reads), `try_push`/`push_wait`/
`try_pop`/`pop_wait` bodies (checks + the above + condvar waits). `Weigh`
runs at 386-387/433-434 before `lock()`. The `unpoison` helper's premise
holds.
— Item-integrity claim ("every non-Pushed variant hands the item back
untouched", 38-40) — verified: all four `PushOutcome` variants and all three
`PushAborted` variants move `item` into the variant; the queue mutates only
on the accept path (401-405, 460-467).
— Flush semantics (317-374) — verified: residents dropped wholesale, bytes
and duration zeroed, `stats.flushed` accumulated, published generation is
`max(current, new)` (never regresses, 341-345), `pending_gap` merges counts
with saturating adds (357-370), `notify_all` on both condvars (371-372) so
parked producers wake (Stale) and parked consumers wake (Discontinuity).
— Discontinuity-before-items ordering — verified: `try_pop` (498-500) and
`pop_wait` (520-522) take `pending_gap` before any `take_front`, exactly as
documented ("a consumer that was not polling at flush time still observes
the boundary before any new-generation item").
— Bounded park (push_wait 428-488) — verified: re-checks closed, staleness
(451-459), and capacity after every wake; `deadline: None` parks
unboundedly only with the caller's consent (documented); deadline expiry
counts `full_rejections` (476) matching the `QueueStats` doc (154-156);
`wait_while`/`wait_timeout_while` predicate re-check makes spurious wakeups
safe.
— Pre-allocation bound — verified: `VecDeque::with_capacity(min(max_packets,
1024))` (243) — a 500k-packet cap cannot pre-allocate half a gigabyte.
— Executed: every baseline open/tick/seek drives `new`/`try_push`/`try_pop`/
`flush`/`close` through the engine (pipeline.rs sites cited in the Field
row); rc-smoke 6/6 checks with 60/60 frames decoded is impossible without
this queue passing packets.

## 3. Findings

### F-001 [S4]
Where: crates/packet-queue/src/queue.rs:394 (try_push) and 441/453 (push_wait)
Code:
```rust
        if item.generation < state.generation {
            state.stats.stale_rejected = state.stats.stale_rejected.saturating_add(1);
            return PushOutcome::Stale {
```
(and the park predicate's mirror at 441:)
```rust
            !s.closed
                && stamped_generation >= s.generation
```
What is wrong: only the **older** direction is refused. An item stamped
*newer* than the queue's published generation (`item.generation >
state.generation`) is accepted and later popped as ordinary data. Invariant
Q2 (doc 03 §5 checking point 3) is worded one-directionally too, so this
matches the letter of the contract — but nothing in the queue's own docs
says future stamps are legal, and the type system stays silent about it.
Why it matters: under the engine's single-writer tick model this is
unreachable (the mint, the queue flush, and the next demux push are ordered
within one tick; the demux stamp and the flush publish move together), so
today it is a latent edge. But the module doc advertises "MPMC-capable"
infrastructure (line 6): a second producer (or a future host thread)
stamping from a generation source ahead of the queue's published one would
insert items across a generation boundary with no `Stale`/`Discontinuity`
signal — precisely the class of silent boundary crossing Q2 exists to
prevent.
What correct looks like: refuse or flag stamps newer than the published
generation (a `PushOutcome::Ahead` or an assert-adjacent typed outcome), or
document the one-directional check as intentional with the single-writer
argument written down.
Confidence: HIGH — Verification: read (engine ordering traced: tracks.rs doc
step 2 "the demux driver stamps every packet from the next tick on with
G+1" happens after the flush in the same effect application; pipeline.rs
push path 2201 runs in the tick's demux slice).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `unpoison` | 29-34 | REAL | — | Sound premise verified (see §2). |
| `PushOutcome<T>` (enum) | 41-72 | REAL | — | `#[must_use]`; all variants carry the item back. |
| `PushAborted<T>` (enum) | 77-102 | REAL | — | Same. |
| `PopOutcome<T>` (enum) | 106-122 | REAL | — | `#[must_use]`; Discontinuity/Item/Empty/Closed total. |
| `Discontinuity` (struct) | 132-144 | REAL | — | `#[must_use]`; u64/usize/Nanoseconds accounting fields. |
| `QueueStats` (struct) | 148-163 | REAL | — | u64 counters, all increments saturating. |
| `Slot<T>` / `State<T>` | 167-187 | REAL | — | Weight cached at push (pops never re-run user code under lock). |
| `State::accept` | 190-199 | REAL | — | Saturating arithmetic. |
| `State::take_front` | 201-209 | REAL | — | Saturating subs; popped count. |
| `State::usage` | 211-217 | REAL | — | packets = len (usize), consistent. |
| `BoundedQueue::new` | 240-256 | REAL | — | Capacity pre-allocation capped at 1024. |
| `BoundedQueue::with_default_watermarks` | 261-263 | REAL | — | Delegation to from_limits. |
| `limits` / `generation` / `is_closed` / `usage` / `watermark` / `stats` | 267-301 | REAL | — | Lock + read; watermark evaluation delegated. |
| `close` | 310-315 | REAL | — | notify_all both condvars — no stranded parkers. |
| `flush` | 336-374 | REAL | — | Never-regressing publish; merged gap; verified in §2. |
| `try_push` | 383-412 | REAL, EXECUTED | S4 | F-001 future-stamp leniency; stale-then-full check order correct (stale wins — a stale item never occupies capacity). |
| `push_wait` | 428-488 | REAL (UNREACHABLE from shipped hosts) | S4 | F-001 mirror in the predicate; deadline logic verified; used only by transcode. |
| `try_pop` | 496-509 | REAL, EXECUTED | — | Gap-before-items; notify_one(space) on item. |
| `pop_wait` | 514-548 | REAL (UNREACHABLE from shipped hosts) | — | Drains items before Closed; parkable predicate exact. |

## 5. Test honesty (tests live in crates/packet-queue/tests/*.rs — not in my assignment; execution cited)
| Test binary | Result (no-fail-fast baseline) | Would it fail if the code were broken? |
|---|---|---|
| lib unittests (17) | ok | The unit layer of the same behaviors; green. |
| backpressure.rs (13) | ok | Parking/deadline semantics — the suite that pins PushAborted. |
| backpressure_property.rs (2) | ok | Property layer. |
| concurrency.rs (7) | ok | MPMC stress — the poison/wake claims. |
| generation.rs (9) | ok | Q2 semantics incl. flush/discontinuity. |
| limits.rs (3) | ok | Cap arithmetic (limits.rs is a sibling file). |
| property.rs (4) | ok | FIFO/ordering properties. |
| zero_alloc_queue.rs (1) | ok | The steady-state no-alloc claim. |
All 56 tests across the crate passed in `orchrun-test-workspace-nofailfast.log.txt` (lines 3534-3740 region). I did not audit the test bodies (not assigned) — the pass counts are cited, the mutation-sensitivity of each body is UNVERIFIED by me.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the lock is held only for deque pushes/pops, never across I/O, decode or user code" | module doc 6-7 | REAL | Verified: Weigh at 386-387/433-434 runs pre-lock; no in-lock calls into non-module code except `Watermark::evaluate` (pure). |
| "Every critical section is infallible by construction" | module doc 8-10 | REAL | Enumerated in §2; `unpoison`'s premise holds. |
| "fresh data is never dropped" (Q3) | PushOutcome doc 38-40 | REAL | All non-Pushed outcomes return the item; queue unmodified except on accept. |
| "parked producers wake with PushAborted::Stale within one condvar notification" | flush doc 324-326 | REAL | flush notify_all(space) at 371; predicate sees the moved generation. |
| "Consecutive flushes the consumer has not yet observed merge — counts accumulate, never vanish" | Discontinuity doc 130-131 | REAL | pending_gap merge with saturating adds at 357-370. |
| "The published generation never regresses" | flush doc 329-332 | REAL | `published = max(current, new)` at 341-345. |
| "unknown-duration items count as 0 toward the duration limit" | stamped.rs Weigh doc 92-94 (cross-ref) | REAL (cross-file) | Weigh contract; queue passes duration into binding_axis untouched. |

## 7. Dead / duplicated / unreachable code
- `push_wait`/`pop_wait` are unreachable from the shipped CLI (only
  transcode's runner uses them; no CLI transcode surface). Not dead — real
  callers exist in-tree and the crate's own suite exercises them.
- No duplication found.

## 8. Dependencies & flags
- std-only (VecDeque, Mutex, Condvar, PoisonError, Instant) +
  media-types (GenerationId, Nanoseconds) + crate siblings limits/stamped.
  No feature gates. Consistent with the crate manifest (W01-A01 audited).

## 9. Open questions for the next agent
- limits.rs (`binding_axis`, `WatermarkThresholds::from_limits`,
  `Watermark::evaluate`) is the arithmetic heart this queue trusts — it is
  NOT in my assignment; whoever takes it should verify the three-axis
  binding order and the high/low mark math against QueuePlan's
  "packets are the starvation axis" degeneration (pipeline.rs part1).
- If a future host serves `transcode`, `push_wait`/`pop_wait` become
  shipped code paths — F-001's future-stamp edge should be settled first.
