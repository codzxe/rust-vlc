# AUDIT: crates/video-engine/src/queue.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-21 |
| Lines / functions | 388 / 20 rows |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/pipeline_host.rs:1322` `PresentationQueue::with_generation(QUEUE_CAPACITY=4, frame.generation)` (VideoLimb field, :705) → per-tick `pipeline_host.rs:806` `queue.drain_to(&mut LatchingPresenter { renderer, … })` → `Presenter::present` → renderer-wgpu `Renderer::render`). Also driven by desktop `apps/desktop/tests/pipeline.rs` and renderer-wgpu `tests/presenter_seam.rs`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "drain_to|PresentationQueue|QUEUE_CAPACITY"` (wiring map: CLI :140/:705/:806/:1322, desktop tests, presenter_seam tests); read `pipeline_host.rs:780–850` (the Full/Stale outcome handling — the queue's contract consumed correctly); `sed -n '5915,5955p'` nofailfast baseline (video_engine 48/48 lib tests green, all 6 queue tests :5941–5946); `sed -n` on quoted ranges. |

## 1. One-paragraph verdict
This is the cleanest file audited so far: a small, bounded, generation-aware FIFO whose every refusal hands the request back untouched, whose error path preserves in-order delivery exactly (the refused request stays at the front — no skip, no duplicate), and whose semantics are pinned by six honest tests, all green in the no-fail-fast baseline (:5941–5946). It is genuinely wired into the shipped CLI (capacity 4, drained through the LatchingPresenter each tick) and the CLI's outcome handling (`Full` → present directly, `Stale` → drop at boundary) consumes the contract correctly. The single defect is a comment inside a test that contradicts its own next statement about future-generation pushes. The future-generation tolerance itself (a request stamped ahead of the published generation is silently enqueued) is a real semantic choice documented nowhere but that test.

## 2. What actually works (verified)
— FIFO in-order delivery: `fifo_delivery_in_order_to_the_sink` (baseline green) — RecordingPresenter asserts slot order [10,20,30].
— Hard capacity bound with the request handed back: `capacity_is_a_hard_bound_with_the_request_handed_back` — `Full { request, capacity }` returned with `request.when` intact; retry-after-drain succeeds ([1,2,3] delivered). Green in baseline.
— Generation discipline: strictly-older refused+counted (`stale_generation_pushes_are_refused_counted_and_returned`), wholesale flush on publish including same-generation republish (`publish_flushes_wholesale_and_rejects_the_old_generation`) — both green.
— Error-path ordering: `presenter_failure_keeps_the_refused_request_at_the_front` — one delivered, DrainError carries the verbatim presenter error (`video.presenter.refused`), queue keeps 2, healthy resume delivers [2,3] with no skip/duplicate. Green.
— Shipped consumption: `pipeline_host.rs:786–817` — `Full` → direct present (never dropped), `Stale` → boundary drop, `DrainError.delivered` folded into the presented counter. Read and verified.

## 3. Findings
### F-001 [S4]
Where: crates/video-engine/src/queue.rs:324-333
Code:
```
        // Current-generation pushes flow; newer generations wait
        // (packet-queue boundary semantics: strictly older is stale).
        assert_eq!(
            q.try_push(request(2, GenerationId::new(7))),
            EnqueueOutcome::Enqueued
        );
        assert_eq!(
            q.try_push(request(3, GenerationId::new(9))),
            EnqueueOutcome::Enqueued
        );
```
What is wrong: The comment says "newer generations wait" — the assertion immediately below it proves the opposite: a gen-9 request pushed into a queue publishing gen 7 is `Enqueued` on the spot. Nothing waits. The future-generation tolerance (accept `request.generation > published`) is a real API semantic — a caller bug or a scheduling race can seat a next-generation request in the current queue, where a later `publish` of that generation will flush it as dead — and its only documentation is this self-contradicting test comment; `try_push`'s own doc (L158–159) does not mention it.
Why it matters: The comment misleads exactly the reader trying to learn the boundary rule; the unmentioned semantic has a flushing consequence (a future-gen request enqueued pre-publish dies at publish) that belongs in the method contract.
What correct looks like: Fix the comment ("newer generations enqueue — only strictly older is stale; a publish of that newer generation later flushes residents wholesale") and add one sentence to `try_push`'s docs.
Confidence: HIGH — Verification: read (L160–178, L311–335).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1–25 | REAL | — | Boundedness, wholesale replacement, depth-1 proposal — all match code. |
| `EnqueueOutcome` (enum) | 39–62 | REAL | — | Every non-Enqueued variant returns the request; `#[must_use]`. |
| `DrainError` (struct) | 74–80 | REAL | — | delivered + verbatim source; not Clone/PartialEq by design (documented). |
| `Display for DrainError` / `Error for DrainError` | 82–96 | REAL | — | Correct delegation to the inner error's source. |
| `QueueStats` (struct) | 100–112 | REAL | — | 5 lifetime counters; plain `+=` (u64 — no practical overflow). |
| `PresentationQueue` (struct) | 116–122 | REAL | — | capacity NonZeroUsize (unbounded unconstructible), VecDeque, published gen. |
| `PresentationQueue::new` / `with_generation` | 127–140 | REAL | — | Construction; stats default. |
| `PresentationQueue::publish` | 150–156 | REAL | — | Wholesale flush; same-gen still flushes (documented L146–149, pinned by test); returns dropped count. |
| `PresentationQueue::try_push` | 160–178 | REAL | S4 | F-001: future-gen tolerance undocumented here; stale-then-capacity check order correct (a stale push to a full queue counts as stale — right priority). |
| `PresentationQueue::drain_to` | 188–201 | REAL | — | Bounded by residents (≤capacity); refused stays front; stats.delivered only on success; no duplication. |
| `PresentationQueue::len`/`is_empty`/`capacity`/`published`/`stats` | 204–231 | REAL | — | Trivial accessors. |
| `tests::ns` (helper) | 243–245 | REAL | — | Timestamp helper. |
| `tests::fifo_delivery_in_order_to_the_sink` | 247–262 | REAL | — | Order asserted via RecordingPresenter. |
| `tests::capacity_is_a_hard_bound_with_the_request_handed_back` | 264–294 | REAL | — | Full→hand-back→retry-after-drain; exact stats. |
| `tests::capacity_one_works` | 297–309 | REAL | — | Depth-1 (the doc's steady-state proposal). |
| `tests::stale_generation_pushes_are_refused_counted_and_returned` | 311–335 | REAL | S4 | F-001 comment contradiction. |
| `tests::publish_flushes_wholesale_and_rejects_the_old_generation` | 337–361 | REAL | — | Flush count, old-gen refusal, same-gen reflush — all exact. |
| `tests::presenter_failure_keeps_the_refused_request_at_the_front` | 363–387 | REAL | — | The strongest test: error verbatim, delivered count, resume order [2,3]. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| fifo_delivery_in_order_to_the_sink | YES | Sink-observed order asserted. |
| capacity_is_a_hard_bound_with_the_request_handed_back | YES | Full variant fields + untouched request + retry path. |
| capacity_one_works | YES | Depth-1 boundary. |
| stale_generation_pushes_are_refused_counted_and_returned | YES | Stale variant fields + untouched request + stats. |
| publish_flushes_wholesale_and_rejects_the_old_generation | YES | Dropped count, published gen, post-publish refusal, same-gen reflush. |
| presenter_failure_keeps_the_refused_request_at_the_front | YES | Scripted failure at index 1; delivered/error/len/resume all exact. |

All six green in nofailfast baseline (:5941–5946). The `RecordingPresenter` is a test fake of the SINK (not of the subject) — legitimate.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "hard capacity bound … an unbounded presentation queue is unconstructible" | queue.rs L18–19 | REAL | NonZeroUsize capacity; capacity-one test. |
| "every resident request is dropped wholesale" on generation publish | queue.rs L10–14 | REAL | `publish` L150–156 + test. |
| "the queue never silently swallows a present" | queue.rs L36–38 | REAL | All three refusal shapes return the request; CLI consumes them (pipeline_host.rs:786–799). |
| "the refused request stays at the front … no request is skipped or duplicated" | queue.rs L68–70 | REAL | drain_to L190–198 + resume test. |

## 7. Dead / duplicated / unreachable code
None. All methods have shipped or test callers; no duplication.

## 8. Dependencies & flags
media-types (GenerationId) + crate::sink types only; std VecDeque/NonZeroUsize. testkit + media-clock (NANO_BASE) dev-only. No cfg gates, no unsafe. video_engine lib 48/48 green in baseline (:5915–5955).

## 9. Open questions for the next agent
- To the pipeline_host auditor (apps/cli wave): the future-generation tolerance (F-001) means a request stamped with a to-be-published generation, pushed before the session's `publish`, will be flushed as dead when that publish lands. In the single-threaded CLI tick the seek path publishes before scheduling resumes (deterministic), so this should be unreachable — worth one confirming look at the seek→publish→schedule ordering in pipeline_host.rs.
- To the video-engine wave (scheduler.rs owners): the scheduler's own generation purge (`generation_change_purges_the_window_and_kills_repeats`, baseline green) is the upstream twin of `publish` — confirm the two can't disagree about which generation is live at the seam.
