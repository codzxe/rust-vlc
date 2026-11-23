# AUDIT: crates/renderer-wgpu/tests/presenter_seam.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-23 |
| Lines / functions | 358 / 7 helpers + 5 tests |
| File verdict | REAL (as a scheduler/queue/software-renderer seam proof; the crate's wgpu namesake is NOT exercised — no wgpu code exists in the tree) |
| Reachable from shipped entry point? | Test: NO. Components under test: `video_engine::{PresentationScheduler, PresentationQueue}` and `renderer_wgpu::Renderer` — both production code; the desktop app's present path routes through them (software tier by default per negotiate.rs). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `./target/debug/deps/presenter_seam-c985618d2dc2893e --nocapture` → **5 passed / 0 failed** (0.01s). `rg wgpu crates/renderer-wgpu/src` → zero wgpu API usage (only doc comments and the Lead-gated import note); `renderer.rs:87-95` shows `new_headless` = `HeadlessSurface` + `SoftwareCompositor`. |

## 1. One-paragraph verdict
A well-constructed seam test: the REAL video-engine scheduler and presentation queue drive the Renderer through the Presenter trait on a virtual clock, and — the strong part — every ordering/staleness/retry/generation claim is checked against CONTENT via a per-frame marker pixel read back from the surface, not against counters. Conservation stats (pushed/presented/dropped), exact staging-copy accounting, in-order retry after surface loss, and wholesale generation purge are all asserted and all pass. The caveat that matters: the "Renderer" being driven is the crate's software compositor (`new_headless` → `SoftwareCompositor`); the crate named renderer-wgpu contains no wgpu code at all (the import is Lead-gated per docs/dependencies/records/wgpu-30.0.1.md), so this test is evidence for the presentation-seam logic only — nothing here or anywhere in the crate proves a single GPU-rendered frame.

## 2. What actually works (verified)
- 5/5 tests pass (output above): ac_scheduler_queue_renderer_end_to_end, repeat_actions_re_present_and_the_surface_persists, background_actions_leave_the_renderer_idle, detached_surface_stops_the_drain_and_reattachment_resumes_in_order, generation_publish_purges_stale_requests_before_the_renderer.
- Content-level assertions executed: surface marker == the frame index just presented (60/60 slots), repeats re-present, background stages nothing, detached drain refuses with typed `render.surface.missing` and retries in order (both frames land), stale generation purged + refused at the boundary (only the live frame composites).
- Exact accounting verified: `pushed=60 presented=60 dropped=0`, `staging_copies=60`, `bytes_staged=60*32*24*4`; repeat budget (4 transfers then Background).

## 3. Findings
F-001 [S3]
Where: crates/renderer-wgpu/tests/presenter_seam.rs:52-61 (with renderer.rs:87-95, negotiate.rs:11-13)
Code:
```rust
fn renderer() -> Renderer {
    Renderer::new_headless(
        EXTENT,
        SurfaceConfig {
            format: SurfaceFormat::Bgra8Srgb,
            present_mode: PresentMode::Fifo,
        },
    )
    .must()
}
```
What is wrong: not the test's logic — its evidence scope. `new_headless` constructs `HeadlessSurface` + `SoftwareCompositor` (renderer.rs:87-95: "driven by the software compositor — runs on any CPU"), and the crate contains no wgpu code (rg: only comments; negotiate.rs: "BackendTier::Hardware — the wgpu compositor. **Absent in**..."). The test header presents this as "this crate's [`Renderer`]" driving "the wiring shape doc 03 §4.10 c describes" — true at the trait level, but any reader (or feature-matrix row) taking presenter_seam as evidence that wgpu/GPU presentation works is mistaken; nothing in the tree tests GPU rendering. The honest scope is stated in negotiate.rs and the Cargo.toml header, not in this test's header. What correct looks like: the test header should say "software compositor tier" explicitly. Confidence: HIGH — Verification: read (new_headless body, negotiate.rs, rg for wgpu) + ran.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| default_cfg | 47-49 | REAL | — | SchedulerCfg::for_frame_duration |
| renderer | 52-61 | REAL | S3 | F-001 software tier |
| marked_bytes / marked_frame / marked_request | 66-99 | REAL | — | marker pixel (5,5) = frame index; PTS-stamped |
| surface_marker | 102-106 | REAL | — | readback identifies the on-screen frame |
| step | 113-143 | REAL | — | production-shaped tick→enqueue→drain; asserts delivered == enqueued |
| ac_scheduler_queue_renderer_end_to_end (test) | 149-197 | REAL | — | 60 frames, per-slot content assert, conservation, staging accounting, final full-frame compare |
| repeat_actions_re_present_and_the_surface_persists (test) | 203-245 | REAL | — | repeat budget, Background after, staging counts |
| background_actions_leave_the_renderer_idle (test) | 251-269 | REAL | — | zero staging, untouched surface |
| detached_surface_stops_the_drain_and_reattachment_resumes_in_order (test) | 275-310 | REAL | — | typed refusal, request retained, in-order resume |
| generation_publish_purges_stale_requests_before_the_renderer (test) | 316-357 | REAL | — | purge count, boundary refusal, live-only compositing |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ac_scheduler_queue_renderer_end_to_end | YES | per-slot marker-pixel identity + conservation + staging counts; a dropped/reordered/duplicated frame breaks the marker or the stats |
| repeat_actions... | YES | exact repeat count, budget, staging copies |
| background_actions... | YES | zero-stats assertions |
| detached_surface... | YES | typed error shape + delivered==0 + in-order retry |
| generation_publish... | YES | purge/refusal/stale_rejected counts + content |
No mocks of the thing tested: the scheduler, queue, and renderer are the production types; only the clock is virtual (its purpose) and the frames are synthetic (deterministic by design, documented).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "video-engine's **real** PresentationScheduler and PresentationQueue driving this crate's Renderer through the Presenter trait" | header L2-5 | REAL (trait-level) | 5/5 pass with real video_engine types |
| "every surface readback identifies exactly which frame is on screen" | header L8-12 | REAL | marker-pixel asserts executed |
| (implied) the renderer side is wgpu | crate name + header | FALSE-by-implication | no wgpu code exists; software compositor (F-001) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Workspace-only (renderer-wgpu, video-engine, media-*, testkit, media-clock via tests/common). No GPU/environment dependency — which is exactly why the wgpu tier is untestable here. No feature gates on the tests.

## 9. Open questions for the next agent
- Does any feature-matrix row cite presenter_seam.rs (or this crate) as evidence of "wgpu"/GPU rendering? If yes, that row is FALSE — trace it in the matrix audit lane.
- negotiate.rs's probe/skip machinery ("wgpu import Lead-gated") is the honest record; whether the desktop product ever presents through anything other than the software tier is a desktop-lane question.
