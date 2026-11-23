# AUDIT: crates/video-engine/src/queue.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-23 |
| Lines / functions | 388 / 9 prod methods + 3 types + 6 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:1322 `let queue = PresentationQueue::with_generation(QUEUE_CAPACITY, frame.generation);` inside the CLI's VideoLimb (scheduler + queue + renderer), constructed on the shipped `lumen open` video path (lumen-cli main → host.rs → pipeline_host.rs); desktop tests/pipeline.rs drives the same type |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `./target/debug/deps/video_engine-189b232358e17f55 queue` -> 6 passed, 0 failed, exit 0 (binary 19:26 newer than source 17:35) |

## 1. One-paragraph verdict
A small, tight, correct bounded queue: hard `NonZeroUsize` capacity (unbounded is unconstructible), typed refusal outcomes that hand the request back untouched on every non-Enqueued path with `#[must_use]` enforcement, generation-aware wholesale `publish` (same-generation re-publish still flushes — the intent is the operation), strictly-older generation refusal at the boundary, and in-order `drain_to` that keeps the refused request at the front on presenter error (no skip, no duplicate — pinned by test). It is on the shipped CLI video path (pipeline_host.rs:1322). Six tests executed, all pass, each pinning a distinct failure mode. No findings — what was checked is enumerated in §2 and the per-function table.

## 2. What actually works (verified) — zero-findings checklist
- `./target/debug/deps/video_engine-189b232358e17f55 queue` -> `test result: ok. 6 passed; 0 failed` (exit 0).
- Checked fakeness (6.1): no todo!/unimplemented!/constant-return bodies; every method's return depends on state; no ignored parameters.
- Checked correctness (6.2): stale check ordering (stale before full — a stale push to a full queue is counted stale, not full; stats-only difference, verified by reading 160-178); `>=` capacity check cannot under- or over-admit; publish clears and counts exactly the resident set; drain error path leaves front intact and `stats.delivered` consistent with `DrainError::delivered`; `+=` on u64 stats unreachable-overflow (2^64 ops); no arithmetic on parsed input; no unsafe; no panics on any input (all refusals typed).
- Checked mixed generations (6.2): a push with generation NEWER than published is accepted (test 330-333 pins gen 9 into a queue publishing 7); FIFO within push order preserves per-generation order by construction.
- Checked tests (6.4): each of the 6 tests fails under a distinct mutation — reorder (FIFO test), off-by-one capacity (capacity test + capacity_one), dropped stale count/hand-back (stale test), publish-not-flushing (publish test), error-path skip/dup (presenter_failure test resumes [2,3] after failing at index 1).
- Checked `#[must_use]` on EnqueueOutcome (39) — the compiler forces callers to handle the handed-back request.

## 3. Findings
None. (Zero findings per spec §12: the checklist above lists what was checked and how.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| EnqueueOutcome (type) | 41-62 | REAL | — | must_use; requests handed back untouched |
| DrainError + Display/Error | 74-96 | REAL | — | delivered count + verbatim presenter error; source() delegates |
| QueueStats (type) | 100-112 | REAL | — | lifetime counters: enqueued/delivered/full_rejections/stale_rejected/flushed |
| PresentationQueue::new / with_generation | 127-140 | REAL | — | NonZeroUsize capacity; published generation from construction |
| PresentationQueue::publish | 150-156 | REAL | — | wholesale clear, same-gen re-publish flushes (pinned by test) |
| PresentationQueue::try_push | 160-178 | REAL | — | stale-then-full checks, typed outcomes, stats |
| PresentationQueue::drain_to | 188-201 | REAL | — | in-order; error keeps refused at front |
| PresentationQueue::len/is_empty/capacity/published/stats | 203-231 | REAL | — | accessors |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fifo_delivery_in_order_to_the_sink | YES | asserts delivered order [10,20,30] (ran) |
| capacity_is_a_hard_bound_with_the_request_handed_back | YES | third push Full{capacity:2}, request returned untouched (when == ns(3)), retry-after-drain succeeds (ran) |
| capacity_one_works | YES | depth-1 boundary (ran) |
| stale_generation_pushes_are_refused_counted_and_returned | YES | gen 3 vs published 7 → Stale + hand-back + count; current and newer gens flow (ran) |
| publish_flushes_wholesale_and_rejects_the_old_generation | YES | 3 flushed, old gen refused after, same-gen publish flushes too (ran) |
| presenter_failure_keeps_the_refused_request_at_the_front | YES | delivered=1, front kept, resume delivers [2,3] — no skip, no duplicate (ran) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "hard capacity bound and generation-aware wholesale replacement" (1-3) | module doc | REAL | NonZeroUsize + publish (tests ran) |
| "later pushes of the old generation are refused at the boundary, counted, never silent" (13-14) | module doc | REAL | stale arm 161-167 + must_use (test ran) |
| "the scheduler emits at most one request per tick; this queue is the hand-off the render side drains" (7-9) | module doc | REAL | scheduler.rs:683-685 confirms the wiring drives the queue (the CLI wiring does: pipeline_host.rs VideoLimb) |
| "depth 1 sufficient in the steady state [PROPOSAL]" (22-25) | module doc | REAL | honestly tagged proposal; CLI uses QUEUE_CAPACITY > 1 |

## 7. Dead / duplicated / unreachable code
None. All methods consumed (CLI host + desktop tests + own tests).

## 8. Dependencies & flags
media_types (GenerationId, EngineError), crate::sink (PresentRequest, Presenter). No features, no unsafe, no env knobs.

## 9. Open questions for the next agent
- The desktop host's mailbox (apps/desktop/src/mailbox.rs) mirrors this queue's publish discipline over the wire protocol — whether that mirror stays in sync is the desktop auditors' question.
- QUEUE_CAPACITY value chosen by the CLI host (pipeline_host.rs) — is depth vs the scheduler's one-request-per-tick cadence ever saturated in real playback? (scenario_13 long-media may answer; see that audit.)
