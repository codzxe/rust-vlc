# AUDIT: crates/renderer-wgpu/src/surface.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-21 |
| Lines / functions | 594 / 22 rows |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/pipeline_host.rs:1314` `Renderer::new_headless(extent, …)` (extent = decoded frame size, pipeline_host.rs:1310-1313) → `renderer.rs:88` `HeadlessSurface::new` → surface.rs:205; per-present: `Renderer::render` → `SoftwareCompositor::composite` → `RenderSurface::fill_rect`/`write_rect`/`present`/`readback_rgba`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "HeadlessSurface|RenderSurface|write_rect|fill_rect"` (usage map: renderer.rs + tests only); read renderer.rs entirely (caller context); `rg -n "new_headless"` (CLI wiring at pipeline_host.rs:1314); `rg -n "MAX_PIXELS|ResourceLimit"` in codecs mjpeg (upstream bound: decoder.rs:105 `MAX_PIXELS: u64 = 8_388_608`, enforced :564); `rg -n "compute_placement"` + read params.rs:210–240 (rect provenance); `rg -n "surface::tests::|renderer_wgpu"` in nofailfast baseline; `sed -n` on all quoted ranges. |

## 1. One-paragraph verdict
This is one of the few crates where the code matches its documentation and the execution evidence is strong: the headless surface is a real in-memory framebuffer with typed construction/resize/write errors, byte-exact round-trip tests including the native BGRA swizzle (verified white-box at L476), and the whole path runs in the shipped binary (CLI sizes the surface from the decoded frame; baseline `renderer_wgpu` 60/60 lib tests green incl. all 7 surface tests, nofailfast :4766–4824; `png_hash` AC green :4846; the first-vertical `final_frame_is_current` failure is committed-artifact drift, not a render failure — the fresh run produced the frame). Two robustness gaps remain: the bounds check itself can overflow (`rect.x + rect.width` unchecked u32 addition — panic where the trait promises a typed error), and the framebuffer allocation trusts the caller's extent with no cap (abort, not error, on a hostile extent — currently fenced only by codec-side MAX_PIXELS).

## 2. What actually works (verified)
— Construction/resize/write validation incl. typed `ZeroExtent`/`PresentModeUnsupported`/`UploadSizeMismatch`: verified by `surface::tests` green in baseline (nofailfast :4817–4824) — exact-error equality asserts (`headless_construction_validates_extent_and_mode`, `write_rect_rejects_short_buffers_and_bad_rects`).
— Byte-exact fill/write/readback round trip through native BGRA order: `fill_write_readback_round_trip_through_native_order` (:4818) asserts both the RGBA readback and the raw BGRA buffer (`&s.buffer[4..8] == &[0,0,255,255]`) — white-box pin of the swizzle.
— End-to-end present path through the shipped CLI lane: baseline `png_hash` `ac_synthetic_frame_headless_render_produces_the_expected_png_hash … ok` (:4846) and first-vertical run-to-end fresh renders (the `final_frame_is_current` nofailfast failure at evidence.rs:162 is drift of the COMMITTED png vs a fresh run — the fresh render itself succeeded; cite: nofailfast summary item 3).

## 3. Findings
### F-001 [S3]
Where: crates/renderer-wgpu/src/surface.rs:242-243
Code:
```
        if rect.x + rect.width > self.extent.width || rect.y + rect.height > self.extent.height {
            return Err(RendererError::ZeroExtent {
```
What is wrong: Unchecked `u32` addition in the bounds check. `rect.x = 4_294_967_000`, `rect.width = 296` sums to exactly 2³² → wraps to 0 in release, the check passes, and `fill_rect`/`write_rect` then index `self.buffer` at `rect.x as usize * 4 ≈ 17.2·10⁹` → slice index panic. In debug builds the addition itself panics (`attempt to add with overflow`). Either way the trait's documented contract ("rectangle outside the surface extent" → typed `RendererError`, trait docs L150–152) is broken for hostile rects.
Why it matters: `RenderSurface` is a public trait (`Box<dyn RenderSurface>` injection seam for the future wgpu backend); any implementor-side or host-side rect producer that is off by a wrap gets a process panic instead of the typed refusal the error model is built around. In-repo callers cannot reach it today (rects come from `compute_placement`, params.rs:210–240, which starts from `Rect::covering(frame)`/target extent and only shrinks) — but the check exists precisely to defend against bad rects, and it fails on the baddest ones.
What correct looks like: `rect.width > self.extent.width - rect.x.min(self.extent.width)` style subtractive check, or `u64` widening before compare.
Confidence: HIGH — Verification: read (L242–249, L282–288, L302–308); overflow arithmetic hand-derived; in-repo reachability excluded by reading compute_placement.
### F-002 [S3]
Where: crates/renderer-wgpu/src/surface.rs:213-217
Code:
```
        let bytes = extent.rgba8_bytes()?;
        Ok(HeadlessSurface {
            extent,
            config,
            buffer: vec![0; bytes as usize],
```
What is wrong: No allocation cap at this layer. `rgba8_bytes` (params.rs:45–47) only saturates at u64 — it does not bound. `vec![0; w*h*4]` with a hostile extent aborts the process (allocation failure → `handle_alloc_error` → abort), not a typed error. The shipped chain feeds the DECODED FRAME's extent straight in: `apps/cli/src/pipeline_host.rs:1310-1314` (`extent = Extent2D { width: frame.size.0, height: frame.size.1 }` → `Renderer::new_headless(extent, …)`), so the surface's allocation size is file-controlled via the first video frame's dimensions.
Why it matters: The only thing standing between a crafted media file and a 17 GB allocation abort is the codec-side pixel cap. For the first-vertical MJPEG lane that cap is real (mjpeg/decoder.rs:105 `MAX_PIXELS = 8_388_608` enforced at :564 — worst case ~32 MiB framebuffer), but surface.rs neither knows nor enforces any bound; any decoder shipped without a cap (or a future wgpu surface constructor given a window extent from a hostile source) turns into a denial-of-service abort. `resize` (L273–275) has the same shape.
What correct looks like: A `MAX_SURFACE_BYTES` cap in `HeadlessSurface::new`/`resize` returning a typed `ResourceLimit`-class `RendererError`, independent of codec-side goodwill.
Confidence: HIGH — Verification: read (L213–219, L266–276) + CLI wiring traced (pipeline_host.rs:1310–1316) + mjpeg cap verified (decoder.rs:105, :564). H.264 sw decoder cap NOT verified here (codecs wave owns it) — UNVERIFIED whether every decoder path bounds pixels.
### F-003 [S4]
Where: crates/renderer-wgpu/src/surface.rs:295-300
Code:
```
        if pixels.len() < needed {
            return Err(RendererError::UploadSizeMismatch {
                expected: needed as u64,
                found: pixels.len() as u64,
            });
```
What is wrong: Only short buffers are refused; a buffer LONGER than `rect.width*rect.height*4` is silently accepted and the excess bytes ignored (the row loop reads exactly `row_bytes` per row). The trait doc (L154–161) specifies the exact byte count; over-supply is a caller bug swallowed without a peep.
Why it matters: The typed-mismatch error exists precisely to catch size disagreements; catching only one direction leaves the other half undiagnosable.
What correct looks like: `pixels.len() != needed` with the same typed error (expected/found already carry both directions).
Confidence: HIGH — Verification: read (L292–312).
### F-004 [S4]
Where: crates/renderer-wgpu/src/surface.rs:242-246
Code:
```
        if rect.x + rect.width > self.extent.width || rect.y + rect.height > self.extent.height {
            return Err(RendererError::ZeroExtent {
                extent: rect.extent(),
```
What is wrong: An out-of-bounds rectangle is reported as `RendererError::ZeroExtent` carrying a NON-zero extent (`rect.extent()` of the offending rect). The trait doc (L150–152) documents this conflation, so it is a designed lie, not an accident — but a caller distinguishing "empty rect" from "rect off the surface" cannot, and error logs show `ZeroExtent { extent: 640x480 }`.
Why it matters: Typed-refusal granularity is this crate's selling point (`SurfaceConfig::validate` "never a silent downgrade", L33–35); the rect path undermines it.
What correct looks like: A distinct `RendererError::RectOutOfBounds { rect, surface }` variant.
Confidence: HIGH — Verification: read (L235–250, trait docs L148–153).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PresentMode` (enum) | 43–54 | REAL | — | wgpu vocabulary mirror; three variants. |
| `PresentMode::wgpu_present_mode` | 60–66 | REAL | — | String names pinned by test; used by future hw backend. |
| `PresentMode::is_vsync` | 71–73 | REAL | — | Classification pinned by test. |
| `SurfaceConfig` + `DEFAULT` | 78–92 | REAL | — | Bgra8Srgb + Fifo default. |
| `SurfaceConfig::validate` | 101–110 | REAL | — | Typed refusal naming supported set; pinned by exact-equality test. |
| `impl Default for SurfaceConfig` | 113–117 | REAL | — | Delegates to DEFAULT. |
| `trait RenderSurface` | 136–174 | REAL | — | Object-safe CPU protocol; 7 methods; contract docs (non-fatal resize, explicit loss, sync Fifo). |
| `HeadlessSurface` (struct) | 186–194 | REAL | — | buffer + presented counter. |
| `HeadlessSurface::new` | 205–220 | REAL | S3 | F-002 unbounded alloc; validates mode THEN extent (order irrelevant). |
| `HeadlessSurface::config` / `presented_count` | 224–232 | REAL | — | Trivial accessors. |
| `HeadlessSurface::checked_span` | 235–250 | REAL | S3/S4 | F-001 overflow; F-004 error conflation. |
| `RenderSurface::extent`/`format`/`supported_present_modes` (impl) | 254–264 | REAL | — | Trivial; headless reports Fifo only. |
| `RenderSurface::resize` | 266–277 | REAL | S3 | Zero-extent typed error; content discarded (documented, test-pinned); same unbounded alloc as new (F-002). |
| `RenderSurface::fill_rect` | 279–290 | REAL | — | Per-pixel 4-byte copies; correct; `packed` computed once; O(w·h). |
| `RenderSurface::write_rect` | 292–312 | REAL | S4 | F-003 over-supply silent; swizzle per pixel via `format.pack`; `as_chunks::<4>` guards ragged rows. |
| `RenderSurface::present` | 314–319 | REAL | — | Counter bump only — documented trivial Fifo (single-buffered); no-op semantics pinned by count test. |
| `RenderSurface::readback_rgba` | 321–332 | REAL | — | Un-swizzle per pixel; O(w·h); allocation per call (readback path, not hot). |
| `tests::surface` (helper) | 340–352 | REAL | — | Bgra8Srgb+Fifo 4x2 default surface. |
| `tests::present_mode_wgpu_names_and_vsync_classification` | 354–362 | REAL | — | String pins; would fail on rename. |
| `tests::config_validation_refuses_unsupported_modes_loudly` | 364–386 | REAL | — | Exact error equality both directions. |
| `tests::headless_construction_validates_extent_and_mode` | 388–431 | REAL | — | Exact errors for zero extent + Immediate mode. |
| `tests::fill_write_readback_round_trip_through_native_order` | 433–477 | REAL | — | Strongest test: RGBA round trip + raw BGRA buffer pin + present count. |
| `tests::write_rect_rejects_short_buffers_and_bad_rects` | 479–529 | REAL | — | expected:16/found:7 exact; OOB + zero rects → ZeroExtent (see F-004). |
| `tests::resize_is_non_fatal_and_reallocates` | 531–566 | REAL | — | Cleared buffer + typed zero error. |
| `tests::rgba_surface_format_skips_the_swizzle` | 568–593 | REAL | — | RGBA-format passthrough pinned byte-exact. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| present_mode_wgpu_names_and_vsync_classification | YES | String/bool pins on all variants. |
| config_validation_refuses_unsupported_modes_loudly | YES | Exact error struct equality incl. supported slice. |
| headless_construction_validates_extent_and_mode | YES | Exact error equality for both refusal classes. |
| fill_write_readback_round_trip_through_native_order | YES | Any swizzle/stride/placement bug shifts bytes; raw-buffer assert catches pack errors the RGBA readback would hide. |
| write_rect_rejects_short_buffers_and_bad_rects | YES | Exact expected/found counts; OOB and zero rects asserted as errors (though conflated type — F-004). |
| resize_is_non_fatal_and_reallocates | YES | Buffer length + zeroed content + typed error asserted. |
| rgba_surface_format_skips_the_swizzle | YES | Byte-exact passthrough both in buffer and readback. |

All 7 executed green in the no-fail-fast baseline (orchrun log :4817–4824). Not covered anywhere: the u32-overflow rect (F-001) and the hostile-extent allocation (F-002).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "HeadlessSurface … runs everywhere a CPU does … testkit target and the fallback" | surface.rs L6–11 | REAL | 60/60 renderer_wgpu lib tests green (nofailfast :4766–4824); png_hash AC green (:4846); CLI wires it (pipeline_host.rs:1314). |
| "a rectangle outside the surface extent" → typed error | trait doc L150–152 | FALSE at the overflow corner | F-001: wraps past 2³² → panic, not error. |
| "never a partial write" | L160–161 | REAL (short side) / silent on over-supply | L295–300 refuses short; F-003 for the long side. |
| "Fifo presents are synchronous" | L134–135 | REAL (trivially) | Headless present is a counter bump (L314–319) — the front buffer IS the buffer. |

## 7. Dead / duplicated / unreachable code
None. Every item has callers (renderer.rs production; tests). `PresentMode::Mailbox`/`Immediate` have no runtime producer in-repo (headless refuses them) but are the documented wgpu contract surface — not dead, unwired by design.

## 8. Dependencies & flags
No external deps in this file (color/error/params are crate-local). No cfg gates. `testkit::{Must, assert_err}` test-only. Crate built clean in baseline build #1; note the crate name says "wgpu" but NO wgpu dependency is imported in this file — the hardware path is future work (crate docs; negotiate.rs is my next audit).

## 9. Open questions for the next agent
- To the codecs wave: verify EVERY decoder enforces a pixel/dimension cap before frames reach `pipeline_host.rs:1310` (MJPEG verified: decoder.rs:105/:564; h264 sw, vp9, av1, dnx, hevc, mpeg12 UNVERIFIED here). F-002 becomes S1 if any path is uncapped.
- To W03-A05 (negotiate.rs auditor — me, next): the "auto-negotiated wgpu backend → software present" fallback chain (L10–11) — confirm what actually exists vs what is promised.
- Baseline cross-ref: `first-vertical evidence.rs::final_frame_is_current` is one of the 7 nofailfast failures — committed final-frame.png drift. The fresh render produced a frame (the assert compares two existing PNGs), so surface.rs is not the suspect; the committed artifact is stale or the render is run-dependent. Evidence wave owns it.
