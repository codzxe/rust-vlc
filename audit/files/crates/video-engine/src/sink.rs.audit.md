# AUDIT: crates/video-engine/src/sink.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-21 |
| Lines / functions | 103 / 5 rows (two structs + one method + one trait; no in-file tests — the seam is exercised by cross-crate suites) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/pipeline_host.rs:806` `queue.drain_to(&mut LatchingPresenter { renderer, … })` → `Presenter::present` → `crates/renderer-wgpu/src/renderer.rs:219` `impl Presenter for Renderer`; desktop lane: `apps/desktop/src/bridge.rs:241`. `PresentRequest` constructed by the scheduler every presented frame.) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "presented_at|PresentInfo"` repo-wide (every producer/consumer located: renderer.rs:226, bridge.rs:190, both echo `request.when`); read renderer.rs `impl Presenter` (my surface.rs-audit context); scheduler missing-PTS behavior cross-checked (`missing_pts_is_inferred_from_the_previous_sequence_position` green, nofailfast :5938); presenter-seam suites cited (renderer-wgpu presenter_seam, desktop pipeline, video-engine lib 48/48); `sed -n` on quoted ranges. |

## 1. One-paragraph verdict
A small, clean boundary contract — request (frame + slot + generation), reply (presented-at), one-method trait — whose adaptation ledger is accurate (the `gen`→`generation` rename is forced by Rust 2024's reserved identifier; the `&PresentRequest` signature change is justified and followed by every implementor). The contract is heavily exercised end-to-end: the shipped CLI drains through it every presented frame, the renderer and desktop bridge implement it, and the presenter-seam/pipeline integration suites are green in the no-fail-fast baseline. The one real gap is honestly labeled: `PresentInfo::presented_at` — advertised as the A/V-sync observability primitive — is an echo of the scheduled slot in every shipped implementation, so presentation jitter and drift are unobservable in v0; the `[PROPOSAL]` says the wgpu renderer will fill it, and no wgpu renderer exists.

## 2. What actually works (verified)
— `PresentRequest` shape + cheap-clone claim: `VideoFrame`'s `FrameStorage::Cpu(SharedBytes)` clone is a refcount bump (verified in pipeline.rs's D-1 doctrine notes); `Eq/PartialEq` used by queue tests (queue.rs Full-hand-back assert).
— `pts()` slot re-expression: `Stamp::Present(self.when)` — the "missing never happens" precondition is enforced upstream (scheduler infers or holds missing-PTS frames; `missing_pts_is_inferred_from_the_previous_sequence_position` green :5938).
— The order guarantee is real: `PresentationQueue` FIFO + refused-stays-front (my queue.rs audit) delivers non-decreasing PTS one refresh's worth at a time, exactly as the contract states.
— Error contract: presenters return typed `EngineError` (stage Present); the CLI's drain path propagates it and keeps the request queued (pipeline_host.rs:810–816, read) — "retry or degrade is its policy" is implemented, not aspirational.
— Execution evidence: video_engine 48/48 + doctest green (:5915–5955, :6564–6569); renderer-wgpu presenter_seam/backend_fallback green (:4854, :4832); desktop pipeline suite green in all-features run.

## 3. Findings
### F-001 [S4]
Where: crates/video-engine/src/sink.rs:66-71
Code:
```
pub struct PresentInfo {
    /// The clock position at which the presenter actually put the frame
    /// on screen `[PROPOSAL: the A/V-sync observability primitive (MP
    /// §11) — 2.D1's renderer fills it with its vsync-quantized
    /// position; test presenters echo the request's slot]`.
    pub presented_at: Timestamp,
```
What is wrong: In every shipped implementation `presented_at` is `request.when` echoed verbatim — renderer-wgpu (renderer.rs:225–227) and the desktop bridge (bridge.rs:189–191) both assign the scheduled slot. The field therefore carries zero information beyond the request: actual-vs-scheduled drift, vsync quantization, and late presentation — the entire A/V-sync observability story (MP §11) — cannot be measured in v0. The `[PROPOSAL]` defers filling it to the wgpu renderer, which does not exist (my negotiate.rs audit).
Why it matters: Any consumer building sync stats from `PresentInfo` today computes scheduled-vs-scheduled — a tautology presented as a measurement. The type system invites the misuse (the field exists, is public, and looks like a measurement).
What correct looks like: Either mark the semantic clearly in the type (e.g., `presented_at: PresentedAt::Scheduled(Timestamp)` until a real clock source exists) or document at the impl sites — which, to their credit, renderer.rs:18–23 and bridge.rs:29 already do.
Confidence: HIGH — Verification: rg of both impl sites (echo) + docs read; both impl-side doc notes acknowledge the echo.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs + adaptations ledger | 1–20 | REAL | — | Both adaptation claims verified (`gen` keyword reservation; `&PresentRequest` signature used by all impls). |
| `PresentRequest` (struct) | 32–50 | REAL | — | frame/when/generation; doc of `generation`'s rename accurate. |
| `PresentRequest::pts` | 52–61 | REAL | — | Slot re-expression; missing-PTS precondition enforced upstream (scheduler test green). |
| `PresentInfo` (struct) | 65–72 | REAL | S4 | F-001: inert echo in every shipped impl; honest `[PROPOSAL]`. |
| `Presenter` (trait) | 95–103 | REAL | — | `Send` only (single render thread — right bound); contract docs match queue/scheduler behavior verified in my queue.rs audit. |

## 5. Test honesty
No in-file tests; the seam is the subject of cross-crate suites (all green in baseline):
| Suite | Would it fail if the contract were broken? | Reason |
|---|---|---|
| renderer-wgpu tests/presenter_seam.rs | YES | Real scheduler + queue + renderer through this trait. |
| video-engine doctest (lib.rs:95) | YES | Full wiring incl. PresentInfo echo asserted. |
| desktop tests/pipeline.rs + renderer-wgpu backend_fallback.rs | YES | Error-path and repeat contracts exercised (bridge repeat test, fallback resume test). |
| apps/cli wiring (pipeline_host.rs:806) | YES (integration) | The shipped drain path; first-vertical run-to-end scenarios exercise it end-to-end. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the scheduler infers or holds missing-PTS frames before emitting" | sink.rs L55–56 | REAL | Scheduler test green (:5938). |
| "Requests whose generation differs … are dropped wholesale, never presented" | sink.rs L44–47 | REAL | queue.rs publish/try_push stale discipline (my queue.rs audit). |
| "requests arrive in non-decreasing PTS order, one screen-refresh's worth at a time" | sink.rs L85–88 | REAL | Queue FIFO + scheduler window (verified in queue.rs audit). |
| "`presented_at` … 2.D1's renderer fills it with its vsync-quantized position" | sink.rs L67–70 | PAPER-ONLY (no such renderer exists) | F-001; negotiate.rs audit (wgpu Lead-gated). |
| "Rust 2024 reserves that identifier" (gen) | sink.rs L47–48 | REAL | `gen` is a reserved keyword in edition 2024 (crate uses edition 2024). |

## 7. Dead / duplicated / unreachable code
None. The trait has two production implementors (renderer-wgpu Renderer, desktop bridge) plus test recorders; both structs are carried in every presented frame.

## 8. Dependencies & flags
media-types only (VideoFrame, Timestamp, GenerationId, EngineError) — matches the crate manifest (media-types + media-clock). No unsafe, no cfg gates, std-only. All dependent suites green in baseline (cited above).

## 9. Open questions for the next agent
- To the observability/metrics wave: if MP §11 sync metrics are claimed anywhere (docs/matrix), audit them against F-001 — any sync-accuracy number derived from `PresentInfo` in v0 is scheduled-slot-vs-scheduled-slot.
- To the desktop wave: bridge.rs:29 already documents the echo — keep that note synchronized with sink.rs's `[PROPOSAL]` if either changes.
