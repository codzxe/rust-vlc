# AUDIT: crates/video-engine/src/sink.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-23 |
| Lines / functions | 103 / 1 method + 2 types + 1 trait |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — renderer-wgpu/src/renderer.rs:218 `impl Presenter for Renderer` (the real wgpu renderer) and apps/cli/src/pipeline_host.rs:746 `impl Presenter for LatchingPresenter` + :806 `queue.drain_to(&mut LatchingPresenter {..})` — the trait is the live video boundary of the shipped CLI (`lumen open` video path: scheduler.tick → PresentAction::Present(PresentRequest) (scheduler.rs:815) → PresentationQueue → Presenter) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | grep reachability (renderer.rs:218, pipeline_host.rs:746/806, scheduler.rs:815); `./target/debug/deps/video_engine-189b232358e17f55` -> 48 passed (the contract's consumers); sed quote checks |

## 1. One-paragraph verdict
The presentation boundary contract, and it is exactly what it claims: a data type (`PresentRequest` — frame + scheduled ns-base slot + generation), a reply (`PresentInfo`), and the `Presenter` trait the renderer side implements. It contains no logic beyond `pts()` (a re-wrap of `when` as a `Stamp`), it touches no GPU/window/surface API (the doc's "Never: GPU/rendering APIs" holds — the module imports only media-types), and it sits on the shipped video path with a real production implementor (renderer-wgpu's `Renderer`, driven through the CLI's `LatchingPresenter` wrapper). The two doc adaptations (frame-as-VideoFrame instead of a handle type; `present(&PresentRequest)` instead of `present(&VideoFrame)`) are recorded with rationale. No findings — checked items listed below.

## 2. What actually works (verified) — zero-findings checklist
- Checked fakeness (6.1): no placeholder bodies; `pts()` is a total one-liner whose output depends on input; no ignored params.
- Checked correctness (6.2): `pts()` (58-60) always answers `Stamp::Present(self.when)` — the "missing never happens" precondition is the scheduler's inference/hold logic (scheduler.rs:733-740 `held_missing` machinery exists; full inference path is scheduler.rs's audit, marked REAL-UNPROVEN there); no arithmetic, no panics, no unsafe; `Send` supertrait on Presenter is the cross-thread render-seam requirement.
- Checked doc-vs-code (6.7): "Emitted by [`crate::PresentationScheduler::tick`]" (29-31) — verified: scheduler.rs:815 constructs `PresentRequest` inside `tick()`, returned as `PresentAction::Present/RepeatLast` (152-160); "delivered in order by [`crate::PresentationQueue`]" — verified in queue.rs audit (FIFO, ran).
- Checked the adaptation notes (11-20): both are present in code exactly as described (field `generation` with the Rust-2024 `gen` reservation note at 47-48).
- Consumers executed: video-engine unit suites 48/48 green (queue tests drive `Presenter`); renderer-wgpu's presenter_seam/backend_fallback tests exist (grep) — renderer-wgpu is another agent's domain.

## 3. Findings
None. (Zero findings per spec §12: checklist above.)

## 4. Per-function table (file is 103 LOC — table included)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PresentRequest (type) | 32-50 | REAL | — | frame + ns-base slot + generation; Clone cheap (SharedBytes) |
| PresentRequest::pts | 52-61 | REAL | — | total re-wrap; precondition owned by scheduler |
| PresentInfo (type) | 63-72 | REAL | — | presented_at [PROPOSAL] — see open questions |
| Presenter (trait) | 95-103 | REAL | — | Send; never-panic contract; order is caller's guarantee |

## 5. Test honesty
No tests in this file (contract module). Its consumers' tests are honest: queue.rs 6/6 (executed, uses Presenter error path), renderer-wgpu tests (grep-verified to exist; not run — other agent's domain).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the concrete wgpu renderer is card 2.D1 ... which implements Presenter" (6-7) | module doc | REAL | renderer-wgpu/src/renderer.rs:218 (grep) |
| "this crate defines the contract only, and never touches a GPU, window or surface API" (7-9) | module doc | REAL | imports are media-types only (22-24) |
| "Requests whose generation differs from the live one are stale ... dropped wholesale, never presented" (44-46) | field doc | REAL | queue.rs publish/stale arms (tests ran) |
| "missing never happens here: the scheduler infers or holds missing-PTS frames before emitting" (54-56) | method doc | REAL-UNPROVEN | scheduler.rs:733-740 held_missing machinery seen; full inference path not audited by me |
| "test presenters echo the request's slot" (69-70) | PresentInfo doc | REAL | test_support.rs:189-191 returns presented_at == request.when |

## 7. Dead / duplicated / unreachable code
None. All three items consumed in production (CLI path) and tests.

## 8. Dependencies & flags
media_types only (VideoFrame, Timestamp, EngineError, GenerationId). No features, no unsafe, no env knobs.

## 9. Open questions for the next agent
- `PresentInfo::presented_at` is `[PROPOSAL]`: what does the REAL renderer (renderer-wgpu renderer.rs:218 impl) report — vsync-quantized position or the request's slot echo? If the echo, the A/V-sync observability primitive (MP §11) does not exist yet in production. (renderer-wgpu auditor.)
- The CLI's `LatchingPresenter` (pipeline_host.rs:746) adds a repeat flag and tap — whether repeats reach the wgpu renderer as no-ops or redraws per the contract's repeat note (89-92) is the CLI-host auditor's question.
