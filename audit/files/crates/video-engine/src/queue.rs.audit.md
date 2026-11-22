# AUDIT: crates/video-engine/src/queue.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 389 / 15 production items + 6 `#[test]` fns |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | **YES** — `lumen open` (video-bearing media) → decoded frame builds `VideoLimb { queue: PresentationQueue::with_generation(QUEUE_CAPACITY, …) }` (apps/cli/src/pipeline_host.rs:1322) → `VideoLimb::enqueue_and_drain` pushes (783) and drains (806) per scheduler tick on every host pump. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | rg for `PresentationQueue|try_push|drain_to` consumers (pipeline_host.rs:783/806/1322); full-file read; `grep -c "#\[test\]"` → 6 |

## 1. One-paragraph verdict

A small, correct, bounded FIFO between the scheduler and the presenter: `NonZeroUsize` capacity (unbounded is unconstructible), typed `Full`/`Stale` refusals that hand the request back untouched, generation-gated admission (strictly-older refused, newer accepted to wait — packet-queue Q2 semantics), wholesale flush on `publish` (including same-generation republish, deliberately), and an in-order `drain_to` whose error path keeps the refused request at the front so a retry neither skips nor duplicates. The six tests are behavioral (FIFO order, hard bound with retry-after-drain, capacity 1, stale refusals, publish flush, presenter-failure resume order). The only defect is inherited from its host: the module doc's "per vsync in production wiring" describes a render-thread cadence that does not exist — the queue is drained whenever the CLI pump runs (the same `advance`-only cadence as the scheduler; see my scheduler.rs audit F-001).

## 2. What actually works (verified)

- Reachability chain above (all hops read).
- FIFO + generation + error-path semantics verified by reading `try_push` (160-178), `publish` (150-156), `drain_to` (188-200): the refused request is NOT popped (197 returns before `pop_front`), so retry order holds.
- W00 baseline: the AVI vertical plays to Ended (advance-test.txt), which traverses this queue on every presented frame (UNVERIFIED by my own run — cited from W00).

## 3. Findings

### F-001 [S4] — Module doc's production-cadence claim describes wiring that does not exist

Where: crates/video-engine/src/queue.rs:7-9.

Code (7-9, verbatim):
```rust
//! The scheduler emits at most one request per tick; this queue is the
//! hand-off the render side drains (per vsync in production wiring —
//! the render thread is 2.D1's, the pacing is the playback engine's).
```

What is wrong: no render thread exists; the playback engine has no pacer; the only drains happen inside host pumps (`CliHost::advance` in the CLI, the tauri shell's ticker). "Per vsync in production wiring" is aspiration stated as shape. Same root as scheduler.rs F-001; logged here because this doc repeats it.

Confidence: HIGH — Verification: read + repo-wide grep for render threads/pacers (none in crates/).

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-25 | REAL-UNPROVEN | S4 | F-001 cadence claim |
| `EnqueueOutcome` (enum) | 41-62 | REAL-UNPROVEN | — | request handed back on refusals; `#[must_use]` |
| `DrainError` + Display/Error | 74-96 | REAL-UNPROVEN | — | delivered count + verbatim source; not Clone (chain opaque) |
| `QueueStats` (struct) | 100-112 | REAL-UNPROVEN | — | 5 counters |
| `PresentationQueue::new`/`with_generation` | 127-140 | REAL-UNPROVEN | — | NonZero capacity |
| `publish` | 150-156 | REAL-UNPROVEN | — | wholesale flush; same-gen republish flushes too (documented, tested) |
| `try_push` | 160-178 | REAL-UNPROVEN | — | stale check before capacity check; newer generations enqueue |
| `drain_to` | 188-201 | REAL-UNPROVEN | — | in-order; error keeps front intact |
| `len`/`is_empty`/`capacity`/`published`/`stats` | 205-231 | REAL-UNPROVEN | — | accessors |
| tests (6) | 247-387 | REAL-UNPROVEN | — | §5 |

## 5. Test honesty (6 `#[test]`)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fifo_delivery_in_order_to_the_sink` (248-262) | YES | order asserted via RecordingPresenter |
| `capacity_is_a_hard_bound_with_the_request_handed_back` (265-294) | YES | refusal + retry-after-drain sequence |
| `capacity_one_works` (297-309) | YES | degenerate bound |
| `stale_generation_pushes_are_refused_counted_and_returned` (312-335) | YES | boundary semantics both directions |
| `publish_flushes_wholesale_and_rejects_the_old_generation` (338-361) | YES | flush count + same-gen re-flush |
| `presenter_failure_keeps_the_refused_request_at_the_front` (364-387) | YES | delivered=1, error code verbatim, resume order [2,3] |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "per vsync in production wiring" | queue.rs:8 | PAPER-ONLY | no render thread/pacer in crates/; drains happen on host pumps only |
| "an unbounded presentation queue is unconstructible" | queue.rs:18-19 | REAL | `NonZeroUsize` capacity enforced at construction |
| Card 2.C4 bounded delivery | module docs 1-14 | REAL-UNPROVEN | implemented; exercised on the AVI vertical per W00 |

## 7. Dead / duplicated / unreachable code

None. All pub items consumed by apps/cli pipeline_host (783, 806, 1322) and tests.

## 8. Dependencies & flags

Depends on `media_types::GenerationId`, `crate::sink::{PresentRequest, Presenter}`. No feature gates.

## 9. Open questions for the next agent

- `Presenter`/`PresentRequest` (crates/video-engine/src/sink.rs) — the drain target's contract; another agent's file.
- QUEUE_CAPACITY's value and the headless renderer's refusal workaround (`render_request` fallback at pipeline_host.rs:785-790) — apps/cli territory (W13); the comment there admits a pacing defect is bug-masked.
