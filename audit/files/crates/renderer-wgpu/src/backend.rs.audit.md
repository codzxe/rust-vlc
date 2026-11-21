# AUDIT: crates/renderer-wgpu/src/backend.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-21 |
| Lines / functions | 384 / 12 (1 trait + 2 methods, 1 struct + 3 impls, 2 free fns, module docs; 7 tests + 3 helpers) |
| File verdict | REAL (the shipped compositor — a CPU rasterizer; the crate name's "wgpu" does not exist in code — F-001) |
| Reachable from shipped entry point? | YES (chain: `lumen` video limb → apps/cli/src/pipeline_host.rs:1314 `Renderer::new_headless` → renderer.rs:91 `Box::new(SoftwareCompositor::new())` → renderer.rs:212-213 `compositor.composite(...)` per frame; executed by the AVI first-vertical lane, baseline INDEX §contrast) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | `cat crates/renderer-wgpu/Cargo.toml` (deps: media-types + video-engine only — no wgpu) · `rg -c "wgpu::\|use wgpu" src/` (hits are doc comments only) · `rg -n "impl Compositor"` (1 prod impl + 1 test impl) · `rg -n "pub fn pixel" -A 12 staging.rs` (saturating sampling verified :170-182) · baseline log: backend::tests 7/7 ok (:4769-4780); renderer_wgpu unittests at :4766 |

## 1. One-paragraph verdict
This is the only compositor the shipped binary links, and it is a correct one: a point-sampled nearest-neighbor CPU rasterizer with a persistent scratch block (no per-frame full-frame allocation), a contiguous-row fast path when source and destination widths match, saturating out-of-bounds sampling, typed zero-extent refusal, and seven exact-pixel tests that all passed in the no-fail-fast baseline. The defects are about identity, not behavior: the crate is named `renderer-wgpu` and this file's module docs describe a wgpu compositor with "GPU color conversion, bilinear filtering in linear light, subtitle-layer blending in the shader" — none of which exists anywhere in the tree (Cargo.toml has no wgpu dependency at all; the deferral is honestly recorded in the Cargo.toml header and `records/wgpu-30.0.1.md`, and `engine-info`'s `video.software-compositor` is accurate). Second, the allocation-size arithmetic (`needed`) is unchecked in the same file that ships a checked `needed_width_bytes` helper for the identical u32→usize concern — overflow is unreachable for real geometries, but the inconsistency contradicts the file's own discipline.

## 2. What actually works (verified)
— backend::tests 7/7 passed in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4769-4780`: `one_to_one_placement_is_a_byte_exact_passthrough` (the sRGB no-double-conversion invariant at compositor level), `background_fills_what_the_frame_does_not_cover`, `upscale_repeats_source_pixels_nearest_neighbor`, `downscale_picks_source_pixel_centers` ([10,30] — pins the center-mapping formula), `source_crop_rect_selects_the_visible_band`, `nearest_math_is_bounded_identity_and_center` (16×16×16 sweep: always `< src_n`, identity at equal sizes, center for singletons), `compositor_reports_its_identity`.
— Shipped execution: every rendered frame in the AVI first-vertical lane flows through `SoftwareCompositor::composite` (chain in header); the lane's final-frame PNG evidence exists in the baseline (INDEX §contrast; first-vertical evidence suite).
— Sampling saturation verified: `StagedFrame::pixel` (staging.rs:170-182) uses `saturating_mul`/`saturating_add`/`.min(rgba.len())`/`checked_sub` — the trait's "never panics on pixel coordinates; sampling saturates" contract (:49-50) is upheld at the callee.
— `nearest` bounds: `(v.min(src_n - 1)) as u32` clamps the result in-bounds regardless of input; the row fast path guards with `end <= frame.rgba.len()` and falls through to the saturating path on short buffers (:138-154) — byte-identical to the documented rule.

## 3. Findings
### F-001 [S4]
Where: crates/renderer-wgpu/src/backend.rs:15-20 (the promised backend), crates/renderer-wgpu/Cargo.toml:20-22 (the dependency list)
Code (backend.rs:15-18):
```
//! - The wgpu compositor (owed to the Lead-gated import — the crate
//!   docs and `records/wgpu-30.0.1.md`): same trait, real pipelines —
//!   GPU color conversion (doc 03 §4.11 e), bilinear filtering in
```
Code (Cargo.toml [dependencies]):
```
[dependencies]
media-types = { path = "../media-types" }
video-engine = { path = "../video-engine" }
```
What is wrong: the crate is named `renderer-wgpu`, this module's docs describe a wgpu compositor ("GPU color conversion, bilinear filtering in linear light, subtitle-layer blending in the shader"), and no wgpu code exists — the crate depends only on media-types and video-engine; the sole production `impl Compositor` is `SoftwareCompositor` (backend.rs:90). Every `wgpu::` mention in src/ is a doc comment.
Why it matters: naming that lies — a reader/auditor selecting "the GPU renderer" finds a CPU rasterizer. Mitigations are real and substantial: the Cargo.toml header records the Lead-gated deferral with the sandbox probe evidence, `records/wgpu-30.0.1.md` is cited, and the shipped `engine-info` honestly reports `video.software-compositor`. No user-visible claim is false; this is structural debt.
What correct looks like: rename the crate `renderer` (or `renderer-soft`) until the wgpu import lands, or land the import.
Confidence: HIGH — Verification: Cargo.toml read; rg for `use wgpu`/`wgpu::` (doc-comment-only hits); impl enumeration.
### F-002 [S4]
Where: crates/renderer-wgpu/src/backend.rs:122 vs :169-171
Code (backend.rs:122-124):
```
        let src = placement.source;
        let needed = dst.width as usize * dst.height as usize * 4;
        self.block.clear();
        self.block.reserve(needed);
```
Code (backend.rs:169-171):
```
fn needed_width_bytes(width: u32) -> Option<usize> {
    (width as usize).checked_mul(4)
}
```
What is wrong: `needed` multiplies unchecked (`dst.width as usize * dst.height as usize * 4` overflows usize for geometries beyond ~2^31×2^31 — ~2^66 bytes), while the same file ships `needed_width_bytes` with `checked_mul` for the identical u32→usize width concern ("defense in depth", :167-168). Likewise `src.y + nearest(...)` (:129) and `src.x + nearest(...)` (:159) are unchecked u32 adds. None of this is reachable: destination extents come from real surfaces whose backing allocation (w·h·4) fails long before the arithmetic wraps, and `compute_placement` keeps source rects inside the frame. In release a wrapped `needed` would merely mis-reserve (sampling still saturates); in debug it would panic, contradicting the trait's "never panics on pixel coordinates" (:49-50) in theory.
Why it matters: internal inconsistency — the checked discipline is applied to the row path but not the block path; a future caller with synthetic placements (tests do build placements by hand) inherits the assumption.
What correct looks like: `let needed = needed_width_bytes(dst.width)?.checked_mul(dst.height as usize)?.checked_mul(4)?` with the same typed fallback, and checked adds on the sample coordinates.
Confidence: HIGH — Verification: read; overflow bounds computed; reachability bounded by surface allocation.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-28 | REAL | S4 | honest about the sandbox/GPU reality; the described wgpu backend is vapor (F-001); "point sampling needs no gamma conversion" — correct (byte copy, tested) |
| `trait Compositor::name` | 42 | REAL | — | used by Renderer::backend_name capability reporting |
| `trait Compositor::composite` | 51-57 | REAL | — | contract: clears uncovered area, propagates typed errors, never panics (upheld — §2) |
| `SoftwareCompositor` + `Debug` + `new` | 67-88 | REAL | — | persistent scratch (C5b) — no per-frame full-frame alloc; Debug hides the buffer |
| `Compositor::composite` (impl) | 95-165 | REAL | S4 | background pass → zero-extent refusal → row fast path (contiguous slice, bounds-checked) → per-pixel saturating path; `needed` unchecked (F-002) |
| `needed_width_bytes` | 169-171 | REAL | — | checked — the discipline F-002 wants everywhere |
| `nearest` | 180-188 | REAL | S4 | center-mapping formula, u64 arithmetic overflows only beyond ~2^31-pixel dimensions (F-002 class); result clamped in-bounds always; identity at equal sizes test-pinned |

## 5. Test honesty (7 tests, all passed — baseline :4769-4780)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `one_to_one_placement_is_a_byte_exact_passthrough` | YES | full-buffer equality — any blend/swizzle breaks it |
| `background_fills_what_the_frame_does_not_cover` | YES | exact corner + center pixels against white bars |
| `upscale_repeats_source_pixels_nearest_neighbor` | YES | exact [10,10,20,20] row |
| `downscale_picks_source_pixel_centers` | YES | exact [10,30] — pins `(i+0.5)·src/dst` math |
| `source_crop_rect_selects_the_visible_band` | YES | exact band passthrough [20,30] |
| `nearest_math_is_bounded_identity_and_center` | YES | 16³ sweep + identity + center asserts — any off-by-one breaks bounds |
| `compositor_reports_its_identity` | YES | trivial but pins the capability string |
| Gap | — | no test exercises the degenerate short-rgba fall-through (:151-154) — the "byte-identical to the documented saturation rule" claim is UNVERIFIED by tests |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a minimal CPU rasterizer — point sampling ... background fill outside. It runs anywhere a CPU runs" | backend.rs:6-9 | REAL | 7 tests passed; shipped lane renders through it |
| "This sandbox has no GPU stack at all ... so v0 links exactly this one" | backend.rs:11-14 | REAL | Cargo.toml has no wgpu dep; one prod impl |
| wgpu compositor: "same trait, real pipelines — GPU color conversion, bilinear ..., subtitle-layer blending" | backend.rs:15-20 | PAPER-ONLY | F-001: no code anywhere; recorded Lead-gated deferral |
| "Point sampling needs no gamma-domain conversion: source bytes are copied, never blended" | backend.rs:24-26 | REAL | passthrough test byte-exact |
| "never panics on pixel coordinates (the placement math bounds them; sampling saturates)" | backend.rs:49-50 | REAL for reachable inputs | pixel() saturates (staging.rs:170-182); theoretical `needed`/`nearest` overflow beyond allocation-bounded dims (F-002) |

## 7. Dead / duplicated / unreachable code
- No dead items: `name` consumed by renderer.rs capability reporting; `needed_width_bytes` used in the fast path.
- The FlakyBackend test impl (tests/backend_fallback.rs:105) is a test-only second Compositor — legitimate seam exercise.

## 8. Dependencies & flags
- media-types + video-engine only (std). No wgpu, no feature gates — the "wgpu" in the crate name is not a dependency (F-001). Built + tested in baseline #1 and the no-fail-fast run.

## 9. Open questions for the next agent
- staging.rs auditor: does `StagedFrame::pixel`'s saturating fallback return `[0,0,0,0]` or clamp-to-last — and does the short-rgba fall-through at backend.rs:151-154 produce identical bytes (the untested claim)?
- surface.rs auditor: `fill_rect`/`write_rect` bounds behavior for rects partially outside the target (the compositor trusts `compute_placement` in-bounds guarantees; a hand-built Placement in tests bypasses them).
- Whoever owns docs/dependencies/records/wgpu-30.0.1.md: confirm the recorded probe evidence matches the sandbox facts (the crate defers on it).
