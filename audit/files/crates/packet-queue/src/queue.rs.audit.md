# AUDIT: crates/packet-queue/src/queue.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-23 |
| Lines / functions | 549 / 18 (no in-file `#[test]`; coverage lives in the crate's integration suites) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: crates/playback-engine/src/pipeline.rs:552 `pub queue: Arc<BoundedQueue<Packet>>` and :1114 `BoundedQueue::new(limits, thresholds)` — one queue per limb of every session; sessions open through apps/cli/src/host.rs:981-984 (lumen-cli, desktop) and apps/web/src/host.rs:1527. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -p packet-queue --test property` -> 4 passed (exercises this file's entire API against the model oracle); prebuilt `target/debug/deps/generation-641a90c85a705c6e` -> 9 passed; `cargo test -p packet-queue` (full suite) -> FAILED TO RUN: disk full (ENOSPC, `No space left on device`, ld bus error) — environment limit, NOT a repo defect; baseline cargo-test-workspace.txt packet-queue section: lib 17, backpressure 13, backpressure_property 2, concurrency 7, generation 9, limits 3, property 4, zero_alloc_queue 1 — all ok. |

## 1. One-paragraph verdict
The queue core is a clean, correct MPMC `Mutex<VecDeque>` + two-condvar design with typed, item-preserving outcomes on every non-success path. The lock is held only for deque ops and saturating arithmetic; `Weigh` runs outside the lock; poison recovery is sound because critical sections are infallible by construction. Stale-generation rejection, wholesale flush with merged consumer-side discontinuity markers, non-regressing published generation, FIFO-within-generation, and the conservation identity are all implemented exactly as the property suite's oracle demands — and that suite (which I executed) drives every public method here through 1.536M adversarial ops. I found no correctness defect; the findings are theoretical-overflow masking, a doc tension on Q1's wording, and the in-file-test absence.

## 2. What actually works (verified)
— verified by `cargo test -p packet-queue --test property` -> `4 passed` (the suite's `run()` calls `try_push`, `push_wait`, `try_pop`, `pop_wait`, `flush`, `close`, `usage`, `stats`, `generation` on `BoundedQueue` — the complete public surface of this file — checking exact accounting after every op). Plus prebuilt generation binary: `9 passed`. Baseline adds backpressure (13, incl. `oversized_item_admits_alone_and_bounds_accumulation`), concurrency (7), zero_alloc (1).

## 3. Findings
F-001 [S4]
Where: crates/packet-queue/src/queue.rs:196-198, 203-206, 355, 395, 408, 454, 476
Code:
```
        self.resident_bytes = self.resident_bytes.saturating_add(size);
        self.resident_duration = Nanoseconds(self.resident_duration.0.saturating_add(duration.0));
        self.stats.pushed = self.stats.pushed.saturating_add(1);
```
What is wrong: every counter and resident sum uses saturating arithmetic — at u64/usize extremes (≈1.8e19) an overflow would be silently clamped, masking an accounting bug instead of failing. Purely theoretical for media workloads (at 100k ops/s, u64 overflows after ~5.8 million years); the property suite's exact-equality checks would catch drift long before. Not worth stronger severity. Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/packet-queue/src/lib.rs:9-14 vs crates/packet-queue/src/limits.rs:234-239
Code (limits.rs):
```
        // The livelock invariant: a queue with nothing resident admits
        // one item unconditionally (see the method docs — the caps
        // bound accumulation, never a lone producer's admission).
        if resident == QueueUsage::EMPTY {
            return None;
        }
```
What is wrong: doc tension, not a code bug. lib.rs Q1 states "Three simultaneous caps — packets, bytes, media duration — and *whichever hits first* refuses the next push" with no exception; a lone item heavier than the byte/duration cap IS admitted (queue.rs `try_push` → `binding_axis` returns None for the empty queue), so `usage().bytes` can exceed `max_bytes` by that one item's weight. The exception is properly documented at limits.rs:206-239 and pinned by tests (limits.rs:436-437 unit assert; backpressure.rs:194 end-to-end, baseline ok), but the top-level Q1 wording overstates the invariant. The property suite's generator never produces an oversized item (sizes ≤ 24 < 64-byte cap), so the Q1 per-op check there never explores the exception region. Confidence: HIGH — Verification: read both files + ran property suite + baseline citation for backpressure.

F-003 [S4]
Where: crates/packet-queue/src/queue.rs (whole file)
Code:
```
//! The bounded queue core: [`BoundedQueue`] and its typed signals
//! (doc 03 §4.4).
```
What is wrong: no `#[cfg(test)]` module in this file — all coverage is integration-level (tests/: property, generation, backpressure, concurrency, limits, zero_alloc) plus the lib unit tests living in sibling modules. Coverage is genuinely strong (see §2), so this is a hygiene note, not a gap: an internal-detail regression (e.g. `Slot` weight caching) is protected only indirectly through public-API property checks. Confidence: HIGH — Verification: read (rg for `#[test]` in file: none).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| unpoison | 29-34 | REAL | — | sound: in-lock code is infallible (verified by reading every critical section) |
| PushOutcome / PushAborted / PopOutcome / Discontinuity / QueueStats (types) | 41-163 | REAL | — | every non-success path hands the item back; `#[must_use]` on all |
| Slot | 167-171 | REAL | — | weight cached at push — pops never re-run user code under the lock |
| State::accept / take_front / usage | 190-217 | REAL | S4 | saturating sums (F-001); add/sub symmetric → no drift (property-verified) |
| BoundedQueue::new | 240-256 | REAL | — | bounded prealloc `min(cap, 1024)`; publishes FIRST |
| with_default_watermarks | 261-263 | REAL | — | |
| limits / generation / is_closed / usage / watermark / stats | 267-301 | REAL | — | read-only under lock |
| close | 310-315 | REAL | — | closed=true + notify_all both condvars; residents preserved |
| flush | 336-374 | REAL | — | drops wholesale; publishes max(new, current) — never regresses; merges pending_gap with saturating adds; returns synchronous gap |
| try_push | 383-412 | REAL | — | Weigh outside lock; Closed → Stale → capacity order; `items.notify_one` on accept |
| push_wait | 428-488 | REAL | — | parkable = open ∧ not-stale ∧ full; stale recheck every wake; deadline checked AFTER fits (no spurious abort); Deadline counts full_rejections |
| try_pop | 496-509 | REAL | — | pending_gap surfaced before any item; Closed only when drained; `space.notify_one` on item |
| pop_wait | 514-548 | REAL | — | parkable = empty ∧ no-gap ∧ open; gap before item before closed; expired deadline → Empty without parking |

## 5. Test honesty
No in-file tests (F-003). Cross-file: every method here is driven by the property suite I executed (exact model oracle, per-op invariants — see property.rs.audit.md §5); `push_wait`/`pop_wait` real parking is exercised by tests/concurrency.rs and tests/backpressure.rs (baseline ok, 13+7 passed). Mutation reasoning: removing the stale check in `try_push` fails the property suite ("Stale for stamp ≥ published" / stale item popped); breaking a resident sum fails exact accounting; regressing the generation on flush fails "published ≠ model".

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the lock is held only for deque pushes/pops, never across I/O, decode or user code" | module docs 4-11 | REAL | verified by reading all 5 lock sites (268, 274, 280, 286, 293-295, 300, 311, 337, 388, 446, 497, 518); `Weigh` called at 386-387/433-434 outside the lock |
| "Every critical section is infallible by construction" | module docs 8-11 | REAL | in-lock code: VecDeque ops, field assigns, saturating arithmetic, condvar notifies — no panicking path found (VecDeque push/pop on non-huge deques cannot fail) |
| Q1 "whichever hits first refuses the next push" | lib.rs:9-14 | PARTLY FALSE as worded | lone-oversized-item exception (F-002), documented at limits.rs:206-239 |
| Q2 "a flush drops every resident item wholesale... a later push of an older-generation item is dropped at the boundary" | lib.rs:23-30 | REAL | flush 336-374; try_push 392-400; property suite executed |
| "Consecutive flushes the consumer has not yet observed merge — counts accumulate, never vanish" | queue.rs:130-131 | REAL | flush 357-370 merge logic; property model's pending_gap merge matches (executed) |
| "The published generation never regresses" | queue.rs:329-332 | REAL | `published = max(new, current)` at 341-345; property check at property.rs:585-591 (executed) |

## 7. Dead / duplicated / unreachable code
None. All 18 items are used (condvar `space`/`items` both notified; `watermark()` consumed by playback-engine's Buffering policy per Q4 — pipeline.rs uses `Watermark` from the same crate).

## 8. Dependencies & flags
media_types (GenerationId, Nanoseconds), crate::limits / crate::stamped. No unsafe. No feature gates. std-only synchronization (Mutex + 2 Condvars + Instant).

## 9. Open questions for the next agent
- tests/zero_alloc_queue.rs (1 test, baseline ok) — confirm it pins the no-allocation claim on the hot paths (`try_push`/`try_pop`) — not my file.
- Whether any real demuxer can produce a single packet heavier than the configured byte cap (16 MiB product default) — if yes, Q1's per-item exception is reachable in production and the engine's Buffering watermark math should tolerate usage > cap (watermark evaluate: `low && !high` semantics — limits.rs:473-474).
