# AUDIT: crates/video-engine/src/sink.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 103 / 1 method + 1 trait fn + 3 types |
| File verdict | REAL (contract) with one pseudo-observable field |
| Reachable from shipped entry point? | YES — `Presenter` implemented by renderer-wgpu `Renderer` (renderer.rs:218, behind the CLI `VideoLimb` render path pipeline_host.rs:746-758 `LatchingPresenter`), by the desktop `CanvasBridge` (apps/desktop/src/bridge.rs:240), and by test recorders. `PresentRequest` minted by the scheduler on the shipped path. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | rg `impl Presenter|PresentInfo \{` across crates/ and apps/; rg `presented_at` consumers; sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

The presentation-boundary contract: one request type (frame + scheduled slot + generation), one info type, one trait. Clean, minimal, no unsafe, no logic to get wrong. The one real defect is `PresentInfo::presented_at`: the field doc promises "the clock position at which the presenter actually put the frame on screen" and says the 2.D1 renderer "fills it with its vsync-quantized position" — in reality every production implementation echoes the request's scheduled slot (`request.when`), no consumer reads the field anywhere, and the desktop bridge even documents the echo as a policy ("Slot echo: the draw instant is downstream's fact"). The A/V-sync observability primitive this field claims to be is a schedule-tautology: it can never measure drift by construction.

## 2. What actually works (verified)

- `PresentRequest` shape (33-50): frame + `when` (nano-base slot) + `generation`; consumed by scheduler/queue/presenters on the shipped path.
- `PresentRequest::pts` (57-60): correct re-expression; the "missing never happens here" precondition is the scheduler's missing-PTS inference (scheduler.rs — another agent's file; claim left REAL-UNPROVEN in §6).
- `Presenter` contract doc (74-103): order guarantee, repeat semantics, non-fatal degradation policy — accurately mirrored by the LatchingPresenter wiring (pipeline_host.rs:746-758 keeps the request on error paths and surfaces typed errors).
- All re-exports resolve; no dead items at this level.

## 3. Findings

### F-001 [S3] `presented_at` is a pseudo-observable: production renderers echo the schedule, nothing consumes it
Where: crates/video-engine/src/sink.rs:66-72; crates/renderer-wgpu/src/renderer.rs:222-228; apps/desktop/src/bridge.rs:189-192
Code (sink.rs:66-72):
```rust
pub struct PresentInfo {
    /// The clock position at which the presenter actually put the frame
    /// on screen `[PROPOSAL: the A/V-sync observability primitive (MP
    /// §11) — 2.D1's renderer fills it with its vsync-quantized
    /// position; test presenters echo the request's slot]`.
    pub presented_at: Timestamp,
```
Code (renderer.rs:222-228):
```rust
        self.render(&request.frame)
            .map_err(RendererError::to_engine_error)?;
        // The synchronous Fifo present lands the frame on the surface
        // at the scheduled slot — see the module docs' timestamp
        // honesty note.
        Ok(PresentInfo {
            presented_at: request.when,
        })
```
What is wrong: The doc's claim splits implementations into "2.D1's renderer fills it with its vsync-quantized position" vs "test presenters echo the request's slot". The actual 2.D1 renderer (renderer-wgpu, the production presenter for both CLI and desktop) **echoes** `request.when`, exactly like the test presenters; the comment asserts a "synchronous Fifo present lands the frame on the surface at the scheduled slot", which is an approximation (the render happens when the host pump runs — under `advance`-driven or wall-clock ticks, not at the media slot). rg over all of crates/ and apps/ finds zero non-test consumers of `presented_at`.
Why it matters: The field is labeled the "A/V-sync observability primitive (MP §11)". As implemented it can only ever equal the schedule, so any future drift measurement built on it reports zero by construction — a fabricated observable wearing a measurement's name. Today it is dead weight plus a false doc sentence about the production renderer.
What correct looks like: Either delete the field until a renderer can stamp a real draw instant (vsync callback / presentation time), or have renderer-wgpu record the clock position at draw time and say so. At minimum, correct the doc sentence that attributes vsync quantization to 2.D1.
Confidence: HIGH — Verification: read (all impls + consumer scan).

## 4. Per-function table (production code)

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PresentRequest::pts` | 57-60 | REAL | — | Slot re-expression; precondition owned by scheduler. |
| `Presenter::present` (trait fn) | 95-103 | REAL | — | Contract only; three production impls + test recorders. |
| `PresentRequest` (type) | 32-50 | REAL | — | Data; shipped path. |
| `PresentInfo` (type) | 65-72 | SHALLOW | S3 | F-001 pseudo-observable. |
| `Presenter` (trait) | 95-103 | REAL | — | `Send` only (present takes `&mut self`) — sufficient for the host-owned wiring. |

## 5. Test honesty

No tests in this file. The trait's implementations are tested where they live (renderer-wgpu, bridge, pipeline_host); renderer.rs:302 and bridge.rs:331 assert `presented_at == req.when` — tests that pin the echo behavior (i.e., they would fail if a renderer started telling the truth). Noted as circularity pressure on F-001, not a separate finding: the tests verify what the code does, which is the problem.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the clock position at which the presenter actually put the frame on screen" (67-68) | field doc | FALSE | All impls echo `request.when` (F-001). |
| "2.D1's renderer fills it with its vsync-quantized position" (69-70) | field doc | FALSE | renderer.rs:225-227 echoes. |
| "missing never happens here: the scheduler infers or holds missing-PTS frames before emitting" (54-56) | method doc | REAL-UNPROVEN | scheduler.rs is another agent's file; lib.rs module map lists "missing-PTS inference" as a scheduler feature. |
| "requests arrive in non-decreasing PTS order, one screen-refresh's worth at a time" (85-87) | trait doc | UNVERIFIED | PresentationQueue ordering is queue.rs (video-engine) — another agent's file. |
| "the Phase-2 webview prototype's canvas bridge" implements Presenter (76) | trait doc | REAL | apps/desktop/src/bridge.rs:240 `impl<T: FrameTransport> Presenter for CanvasBridge<T>`. |

## 7. Dead / duplicated / unreachable code

`PresentInfo::presented_at` (F-001) — written by every impl, read by no production code.

## 8. Dependencies & flags

No flags, no unsafe, media-types only. The `Presenter` seam is the crate's only outward behavior surface.

## 9. Open questions for the next agent

- W03 colleague with scheduler.rs: does `SyncSample`/`DriftEstimator` derive presentation error from the clock vs `when`, or does anything plan to consume `presented_at`? If the former, F-001 stays dormant; if the latter, it is a correctness bug in waiting.
- W06/W15 (renderer-wgpu, ui): is there any real vsync timestamp available in the wgpu present path that could populate the field honestly?
