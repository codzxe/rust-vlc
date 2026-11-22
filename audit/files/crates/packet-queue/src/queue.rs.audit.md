# AUDIT: crates/packet-queue/src/queue.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 549 / 18 production fns + 8 types |
| File verdict | REAL (try_push/try_pop/flush/close path); the parking half (push_wait/pop_wait) is REAL code with NO production caller |
| Reachable from shipped entry point? | YES — CLI open → host.rs:890 `open_session` → playback-engine `SessionConfig` → pipeline.rs:1114 `BoundedQueue::new(limits, thresholds)`, :2201 `try_push`, :2352 `try_pop`, :1117/:1158/:1255/:1784 `flush`, :1159/:1785 `close`. `push_wait`/`pop_wait`: NO shipped caller (rg over apps/ + crates/playback-engine: zero hits outside packet-queue's own tests). |
| Findings | S0:0 S1:0 S2:1 S3:1 S4:1 |
| Commands run | rg for `BoundedQueue|try_push|push_wait|pop_wait|flush\(|close\(` across apps/, crates/playback-engine, crates/renderer-wgpu; sed verification of quotes. No cargo (adaptation). |

## 1. One-paragraph verdict

The bounded, generation-aware queue core is competently built: weights computed outside the lock, in-lock critical sections genuinely infallible (deque ops + saturating arithmetic only — verified line by line), typed outcomes that hand the item back on every non-success path, wholesale flush accounting that merges rather than vanishes, and preallocation capped at 1024 slots so config cannot request absurd allocation up front. The reachability split is the story: the synchronous single-threaded v0 engine uses `try_push`/`try_pop`/`flush`/`close` only; the entire condvar parking machinery (`push_wait`, `pop_wait`, `PushAborted`, ~120 lines) exists for the multi-threaded pipeline the comments themselves admit is "Phase 3+" — the module doc's "the demux thread parks until the pipeline moves it" describes a thread that does not exist. Within that dormant parking API there is one real defect: `notify_one` on pop can starve a small-item producer behind a large-item one under the MPMC usage the type advertises.

## 2. What actually works (verified)

- Lock discipline: every critical section is deque push/pop, field copy, saturating add/sub, or condvar notify — no user code, no I/O, no unwraps (verified across 383-548). `Weigh::size_bytes`/`media_duration` run before `lock()` in both `try_push` (386-388) and `push_wait` (433-434). The `unpoison` soundness argument (23-34) therefore holds.
- Typed no-loss contract: every non-`Pushed`/non-`Item` outcome carries the item or the drop accounting (`PushOutcome::Full/Closed/Stale` 49-71, `PushAborted` 78-102, `PopOutcome::Discontinuity` 109-121); `#[must_use]` on all three.
- Flush accounting merge (357-370): consecutive unobserved flushes accumulate `dropped`/`dropped_bytes`/`dropped_duration` with saturating adds; the consumer sees the merged marker before any new item (498-500, 520-522).
- Generation monotonicity on flush (341-345): a regressing `new_generation` still flushes but never lowers the published high-water mark.
- Single-pending backpressure in the consumer of this queue (playback-engine pipeline.rs:2122-2133) re-offers before pulling new events and ends the slice on refusal — the earlier single-slot overwrite bug (their comment, "6.FIX-pcm") is fixed; no data loss path remains in that wiring.
- Test execution: UNVERIFIED (no cargo); crate suite is part of W00's 1751-passing baseline. backpressure.rs (my file #11) exercises this file.

## 3. Findings

### F-001 [S2, latent] `notify_one` on pop can starve a fitting producer behind a non-fitting one
Where: crates/packet-queue/src/queue.rs:501-504 (try_pop) and 523-525 (pop_wait)
Code (501-505):
```rust
        match state.take_front() {
            Some(item) => {
                self.space.notify_one();
                PopOutcome::Item(item)
            }
```
What is wrong: One pop frees space and wakes exactly one parked producer. The woken producer's `parkable` predicate (439-445) re-evaluates **its own** item's fit; if its item is larger than what was freed, `wait_while` re-parks it **without notifying any other waiter**. A second producer whose smaller item would fit is never woken until another pop occurs. Under the MPMC usage this type explicitly advertises (module doc line 5-6: "MPMC-capable") with heterogeneous packet sizes (a 4 KiB audio packet vs a 2 MiB video packet sharing caps), this is priority-inversion starvation: the small producer can wait indefinitely while pops keep landing on the big producer's wake slot.
Why it matters: Wrong behavior in exactly the scenario the API documents for itself ("the demux thread parks until the pipeline moves it", 419-421). Today no shipped code parks (F-002), so the bug is latent — but the day a producer thread is wired, this is a livelock-class defect in the core queue.
What correct looks like: `notify_all` on the space condvar (producers are few; thundering herd is cheaper than starvation), or a size-classed wakeup, or document `notify_one` as valid only for homogeneous items.
Confidence: HIGH on mechanism (read), reachability today: none — Verification: read.
### F-002 [S3] The parking API (`push_wait`/`pop_wait`) has no production caller; the documented threading model does not exist
Where: crates/packet-queue/src/queue.rs:414-421 (doc), 428-488 (push_wait), 511-548 (pop_wait); consumer side: crates/playback-engine/src/pipeline.rs:2225-2229
Code (queue.rs:417-421):
```rust
    /// Wakes on: space, generation movement ([`BoundedQueue::flush`]),
    /// queue close, or `deadline` — never spins (Q3), never blocks
    /// unboundedly without the caller's consent (`deadline: None` *is*
    /// that consent: the demux thread parks until the pipeline moves
    /// it).
```
Code (pipeline.rs:2225-2229):
```rust
            // Stale and Closed can only strike once mints and
            // teardown race producer threads (multi-threaded Phase
            // 3+); the synchronous v0 engine never observes them
            // here, and the queue's own stats count both.
            PushOutcome::Stale { .. } | PushOutcome::Closed { .. } => {}
```
What is wrong: The module presents condvar parking as the pipeline's operating mode; the actual v0 engine is synchronous (single poll loop: demux push and decode pop on one thread), and rg proves zero non-test callers of `push_wait`/`pop_wait` anywhere in apps/ or crates/. The two condvars, `PushAborted`, the deadline arms, and the poison-recovery paths on those arms are maintained, tested (backpressure.rs), and dead in every shipped binary.
Why it matters: Doc-vs-reality: doc 03 §4.4(b)'s parking sketch is presented as implemented behavior; the shipped behavior is poll-and-hold. ~25% of this file is unreachable production surface, and F-001 hides inside it.
What correct looks like: Either wire the threaded pipeline or mark the parking API `#[doc(hidden)]`/feature-gated until it ships, and fix F-001 before it does.
Confidence: HIGH — Verification: read (rg call-graph).
### F-003 [S4] Future-generation items are silently accepted
Where: crates/packet-queue/src/queue.rs:394
Code:
```rust
        if item.generation < state.generation {
```
What is wrong: Only a **behind** stamp is refused. An item stamped with a generation **ahead** of the published one is accepted and becomes resident; it survives until some flush publishes past it. Q2's boundary language ("items stamped older … can neither be popped nor pushed") says nothing about future stamps, so this is unspecified rather than wrong, but a buggy producer stamping far-future generations could pre-fill the queue with items the current generation's consumer will pop and (per the engine's stamp discipline) treat as current.
Why it matters: Hygiene/robustness: the invariant enforced is one-sided; nothing counts or signals forward-dated items.
What correct looks like: Refuse `item.generation != state.generation` for pushes (or document the forward-accept as a seam), and count it.
Confidence: HIGH — Verification: read.

## 4. Per-function table (production code)

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `unpoison` | 29-34 | REAL | — | Sound given infallible critical sections (verified). |
| `State::accept` | 190-199 | REAL | — | Saturating resident accounting; weight precomputed. |
| `State::take_front` | 201-209 | REAL | — | Symmetric saturating subtraction; stats bumped. |
| `State::usage` | 211-217 | REAL | — | Copy out; cheap. |
| `BoundedQueue::new` | 240-256 | REAL | — | Prealloc capped `.min(1024)` — no config-driven huge alloc. Reachable (pipeline.rs:1114). |
| `BoundedQueue::with_default_watermarks` | 261-263 | REAL | — | Delegates; used by packet-queue tests only in production graph (lib.rs doc example). |
| `BoundedQueue::limits` | 267-269 | REAL | — | Accessor. |
| `BoundedQueue::generation` | 273-275 | REAL | — | Accessor. |
| `BoundedQueue::is_closed` | 279-281 | REAL | — | Accessor. |
| `BoundedQueue::usage` | 285-287 | REAL | — | Accessor. |
| `BoundedQueue::watermark` | 292-295 | REAL | — | Q4 seam; consumed by pipeline (Buffering transitions). |
| `BoundedQueue::stats` | 299-301 | REAL | — | Copy out. |
| `BoundedQueue::close` | 310-315 | REAL | — | Notifies both condvars; reachable (pipeline.rs:1159, 1785). |
| `BoundedQueue::flush` | 336-374 | REAL | — | High-water generation; merged pending_gap; notify both. Reachable (pipeline.rs:1117 etc.). |
| `BoundedQueue::try_push` | 383-412 | REAL | — | Closed→Stale→caps order; the shipped push path (pipeline.rs:2201). |
| `BoundedQueue::push_wait` | 428-488 | UNREACHABLE | S2/S3 | Parking producer; correct deadline math (pre-check 475-478, saturating duration 481); no production caller (F-002); contains the F-001 starvation exposure. |
| `BoundedQueue::try_pop` | 496-509 | REAL | S2 | Gap-before-item; space notify_one (F-001). Shipped pop path (pipeline.rs:2352). |
| `BoundedQueue::pop_wait` | 514-548 | UNREACHABLE | S3 | Parking consumer; deadline pre-check 535-538; no production caller. |

## 5. Test honesty

No tests in this file. The crate's behavioral tests live in `tests/backpressure.rs` (assigned to me, audited separately) and `tests/stamped.rs` coverage of `Weigh`. This file's logic is additionally exercised indirectly by playback-engine's pipeline tests (W03 colleagues).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the lock is held only for deque pushes/pops, never across I/O, decode or user code" (lines 6-7) | module doc | REAL | Verified every critical section (383-548); Weigh outside lock. |
| "MPMC-capable" (line 6) | module doc | SHALLOW | The type supports multiple waiters structurally, but the wakeup discipline (F-001) makes multi-producer parking incorrect under mixed sizes. |
| "the demux thread parks until the pipeline moves it" (419-421) | push_wait doc | FALSE (as a description of shipped behavior) | No demux thread exists; pipeline.rs:2225-2229 admits the engine is synchronous v0; push_wait has zero production callers. |
| "fresh data is never dropped" (doc 03 Q3, quoted line 39-40) | module doc | REAL | In the shipped wiring: Full hands the item back and pipeline.rs:2223 holds it; the re-offer slice ends before overwrite (2116-2133). |

## 7. Dead / duplicated / unreachable code

- `push_wait` (428-488), `pop_wait` (514-548), `PushAborted` (78-102): production-dead, test-only. See F-002.
- `with_default_watermarks`: used by the lib.rs doc example and tests; no production constructor path (pipeline.rs:1114 uses `new` with explicit thresholds).

## 8. Dependencies & flags

- No feature flags, no `unsafe`, std + media-types only. `packet-queue` is a dependency of playback-engine (Cargo.toml:39) and plugin-host-wasm (Cargo.toml:70) — the plugin-host edge is untested here (W12's domain).

## 9. Open questions for the next agent

- W03 colleague auditing `limits.rs`: does `binding_axis` treat `max_packets` as inclusive (off-by-one at exactly-full)? `try_push` asks "would one more exceed" — confirm the comparison is `>` not `>=` against the cap.
- W12 (plugin-host-wasm): does the plugin host actually park on `push_wait`? If yes, F-001 becomes live there — check its threading.
