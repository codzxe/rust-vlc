# AUDIT: crates/renderer-wgpu/src/renderer.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Lines / functions | 438 / 22 (13 methods/fns + Presenter impl + Debug impl, 8 tests + 2 helpers) |
| File verdict | REAL as a CPU/software presenter (verified by execution) — the "wgpu" in the crate name is aspirational: ZERO wgpu calls exist anywhere in the crate |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:81 imports, :1314 constructs `Renderer::new_headless` (software lane) inside the shipped CLI's VideoLimb; implements video-engine's `Presenter` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/renderer_wgpu-cea255c0e8d5c422 renderer::tests` -> 8 passed / 0 failed; `... negotiate` -> 5 passed / 0 failed; `cat crates/renderer-wgpu/Cargo.toml` -> deps = media-types + video-engine ONLY (no wgpu); rg "wgpu" src -> only doc comments and string constants |

## 1. One-paragraph verdict
Hostile-focus answer first: **this is a software compositor seam, not a wgpu renderer.** The crate `renderer-wgpu` does not depend on wgpu at all (Cargo.toml dependencies: media-types, video-engine; the wgpu import is an openly recorded Lead-gated deferral). Every "wgpu integration point" is a string constant (`PresentMode::wgpu_present_mode()` returns the *names* "Fifo"/"Mailbox"/"Immediate") or a typed refusal (`OpaqueHandleUnsupported`), and `negotiate.rs`'s hardware tier is a const `Unavailable`. That said, what IS here is real and works: a staged CPU upload with measured copy counts, pure aspect-placement math, a software compositor, byte-exact headless readback, and typed errors for every refusal class — I ran the 8 renderer tests and 5 negotiate tests, all green, including a pixel-level letterboxing assertion. The shipped CLI constructs exactly this headless software renderer (pipeline_host.rs:1314). The one registered user-visible defect: the CLI's window-full hold path stashes the UNCONVERTED I420 frame, which this renderer then refuses (`render.upload.format-unsupported`), freezing the surface — documented in-line as a known defect, unfixed.

## 2. What actually works (verified)
— `./target/debug/deps/renderer_wgpu-cea255c0e8d5c422 renderer::tests` -> **8/8 pass**: byte-exact round trip (`readback_rgba() == input bytes` at 1:1, staging stats `bytes_staged == 4*4*4`, `staging_copies == 1`), letterbox pixel assertions (rows 0/3 background white, rows 1-2 frame), typed refusal codes (`render.upload.format-unsupported`, `render.upload.opaque-handle`, `render.upload.size-mismatch`, `render.surface.missing`), detach/reattach, resize/zero-extent, headless constructor validation (Mailbox refused).
— `... negotiate` -> 5/5 pass: the fallback chain probes answer this build's truth (hardware tier const-`Unavailable` with the recorded reason string, software `Selected`).
— Presenter contract: `present()` returns `PresentInfo { presented_at: request.when }` — for the synchronous software Fifo present this is defensible and the module doc (L16-23) flags the vsync-quantized hardware reality as `[VERIFY]`.

## 3. Findings
F-001 [S2] (cross-file; mechanism in this file, bug registered in the CLI)
Where: renderer.rs:188-215 (refusal path) + apps/cli/src/pipeline_host.rs:1361-1385 (the registered defect)
Code: (pipeline_host.rs:1368-1372, verbatim from the in-tree comment)
            // [KNOWN DEFECT, registered as the W1-06 follow-up card
            // (Lead decision required): this arm stashes the
            // UNCONVERTED original (`frame.clone()`), so every
            // hold-retried I420 frame is refused at the renderer
            // (`render.upload.format-unsupported`) and the surface
            // freezes on an early window frame.
What is wrong: under a full scheduler window the CLI stashes the unconverted I420 frame; on retry this renderer's `render()` refuses it (`staging.stage` → `render.upload.format-unsupported`), so the video surface freezes. User-visible broken playback in a realistic case (early-frame burst). The repo honestly registers it but ships it. Why it matters: this is the shipped CLI's only video display path. What correct looks like: stash the converted RGBA frame (the one-line fix the comment itself names), with the pacing/backpressure follow-up. Confidence: HIGH — Verification: read both files; renderer tests confirm the refusal code fires for non-RGBA.
F-002 [S3]
Where: crates/renderer-wgpu/Cargo.toml:1,18 (name) and renderer.rs:1-3,40-42
Code:
//! The renderer itself: one frame → one surface present, behind the
//! [`Presenter`] contract video-engine delivers through (doc 01 §2.6
//! "`Renderer` … implements `Presenter`"; card 2.D1).
What is wrong: the crate is named `renderer-wgpu` ("GPU video presentation", Cargo.toml:1) but contains zero wgpu usage — no dependency, no adapter/device/queue/surface creation, no GPU upload; `with_backend`'s doc says the wgpu surface "join[s] behind the exact same constructor when the Lead-gated import lands". A name that asserts an implementation that does not exist. The deferral is honestly recorded (Cargo.toml comment, negotiate.rs typed reason, feature-matrix row `platform-vout-wgpu` = not_started on every platform with empty evidence — so no fraud at the claim level), but every consumer of the name is misled at the directory level. What correct looks like: rename to `renderer`/`renderer-soft` until the import lands. Confidence: HIGH — Verification: cat Cargo.toml (deps list), rg wgpu across src.
F-003 [S4]
Where: renderer.rs:218-228
Code:
impl Presenter for Renderer {
    fn present(&mut self, request: &PresentRequest) -> Result<PresentInfo, EngineError> {
        self.render(&request.frame)
            .map_err(RendererError::to_engine_error)?;
What is wrong: `presented_at` is an echo of the scheduled slot, never a measured present time; on the software path the doc argues this is exact, but nothing measures even software present duration. Marked `[VERIFY: where a compositor clock exists]` in the module docs — recorded deferral, low blast radius. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Renderer (struct) + Debug | 43-61 | REAL | — | truthful non-exhaustive debug |
| with_backend | 67-78 | REAL | — | injection point; no wgpu backend exists to inject |
| new_headless | 87-94 | REAL | — | shipped CLI constructor (pipeline_host.rs:1314) |
| detach_surface / attach_surface / has_surface | 99-112 | REAL | — | surface-loss degradation, tested |
| params / set_params | 116-124 | REAL | — | cheap param swap, pixel-verified in tests |
| resize / extent | 132-144 | REAL | — | typed NoSurface/ZeroExtent |
| backend_name | 149-151 | REAL | — | returns "software" today (test asserts) |
| swap_compositor | 159-161 | REAL | — | handover seam; only SoftwareCompositor exists to swap in |
| staging_stats | 165-167 | REAL | — | measured bytes/copies, asserted exact in tests |
| readback_rgba | 175-178 | REAL | — | byte-exact verified |
| render | 188-215 | REAL | — | validate→stage→place→composite→present; refuses OpaqueHandle/YUV/truncation typed |
| Presenter::present | 219-228 | REAL | S4/F-003 | presented_at echo (documented) |
| uploads_format | 239-241 | REAL | — | Rgba8/Bgra8 only; matches render refusals (test) |
| tests: request/headless_renderer helpers | 254-289 | REAL | — | — |
| tests: 8 #[test] fns | 291-437 | REAL (ran) | — | byte-exact, routing, refusals, detach, resize, letterbox pixels, constructor validation |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| presenter_round_trip_byte_exact_and_reports_the_slot | YES (ran) | byte-equality of full readback + exact staging stats |
| format_routing_matches_the_upload_path | YES (ran) | routing fn AND renderer refusal code both asserted |
| opaque_handles_are_refused_with_the_named_path | YES (ran) | exact error code |
| truncated_frames_are_decode_corruption_not_guesses | YES (ran) | exact code + message contains "truncated" |
| detached_renderer_degrades_not_crashes | YES (ran) | refusal then reattach works |
| resize_flows_to_the_surface | YES (ran) | extent round trip + zero-extent typed |
| params_swap_takes_effect_next_present | YES (ran) | per-pixel letterbox assertions — strongest test |
| headless_constructor_validates_like_the_surface | YES (ran) | zero extent + Mailbox exact-variant refusal |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "renderer-wgpu — GPU video presentation" | Cargo.toml:1 | FALSE at name level (no GPU/wgpu in crate) | cat Cargo.toml: no wgpu dep; rg: zero wgpu calls — F-002 |
| "the wgpu surface and compositor join behind the exact same constructor when the Lead-gated import lands" | renderer.rs:40-42 | REAL (honest deferral statement) | with_backend exists, no wgpu impl to join |
| "the headless/Fifo present is synchronous, so the frame is on the surface at the slot" | renderer.rs:18-20 | REAL-UNPROVEN | software present is synchronous; no timing measurement exists (F-003) |
| matrix `platform-vout-wgpu` not_started/empty evidence | docs/feature-matrix.toml:2411 | REAL (honest) | row matches the code's absence |
| negotiate hardware tier "Absent in this build … Honest, never faked" | negotiate.rs:9-17 | REAL | ran negotiate tests 5/5; probe returns typed Unavailable |

## 7. Dead / duplicated / unreachable code
- Nothing dead in this file; the wgpu-shaped seams (`with_backend`, `swap_compositor`, `PresentMode::wgpu_present_mode` string mapping in surface.rs) are forward-declared injection points with no implementation behind them — unreachable-by-design until ADR-003.

## 8. Dependencies & flags
media-types + video-engine only (no wgpu, no unsafe). dev-deps testkit/media-clock. Crate compiles into shipped CLI (apps/cli/Cargo.toml:60 with its own comment "the video path over the software compositor — no GPU").

## 9. Open questions for the next agent
- backend.rs (next in my queue): does SoftwareCompositor's composite actually write every pixel (background fill + placement) or only the frame rect? The letterbox test only checks 4 pixels of a 4x4 surface.
- apps/cli pipeline_host.rs audit should own F-001's registered defect (W1-06 follow-up) — verify whether the hold-retry freeze is reproducible in main's scenario tests (the comment claims the bug-masked shape is deliberately kept).
