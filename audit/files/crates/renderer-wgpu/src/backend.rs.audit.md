# AUDIT: crates/renderer-wgpu/src/backend.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Lines / functions | 384 / 12 (trait + impl + 2 free fns + 5 test helpers, 7 tests) |
| File verdict | REAL — a correct CPU point-sampling compositor; the "wgpu compositor" half of the module doc is owed, not present (matches renderer.rs finding) |
| Reachable from shipped entry point? | YES — `SoftwareCompositor` linked into the shipped CLI via `Renderer::new_headless` (pipeline_host.rs:1314 → renderer.rs:91); it is the only `Compositor` implementation in the crate |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `./target/debug/deps/renderer_wgpu-cea255c0e8d5c422 backend` -> 7 passed / 0 failed; `./target/debug/deps/png_hash-cee1d8862e16caad` -> 5 passed; `./target/debug/deps/zero_alloc_present-a1a58fac9a1f3d5b` -> 1 passed; `./target/debug/deps/backend_fallback-42547c38abb89860` -> 5 passed |

## 1. One-paragraph verdict
This is the answer to the hostile question, in code: `Compositor` is a one-method trait whose ONLY implementation is `SoftwareCompositor` — a CPU rasterizer that fills the whole target with background, then point-samples the staged RGBA frame into the placement rectangle (fast contiguous row copy at 1:1 width, checked saturating per-pixel path otherwise). The module doc is honest about the split ("v0 links exactly this one"), the sampling math is exact and exhaustively boundary-tested (identity at equal sizes, always `< src_n`, singleton-center), and the persistent scratch block removes the per-present full-frame allocation (the C5b fix), which the `zero_alloc_present` integration test proves by allocation counter. Zero GPU anything. What defects exist are hygiene-level: one unchecked `usize` product in a spot where every neighboring computation is checked, a Debug impl whose comment misstates what it renders, and the same surface-seam `u32` addition nit.

## 2. What actually works (verified)
— `./target/debug/deps/renderer_wgpu-cea255c0e8d5c422 backend` -> **7/7 pass**: byte-exact 1:1 passthrough through a BGRA surface (swizzle handled by the surface protocol), full-target background fill with 1x1 letterbox, upscale `[10,10,20,20]`, downscale center-pick `[10,30]`, source-crop band `[20,30]`, nearest-math bounded/identity/center grid (17×17×17 sweep), identity string "software".
— `png_hash` integration (5/5), `zero_alloc_present` (1/1 — present path allocates zero after warm-up, proving the C5b scratch reuse), `backend_fallback` (5/5 — hardware-first chain with typed absence).
— Answering renderer.rs's open question: `composite()` DOES write every target pixel — `target.fill_rect(Rect::covering(target_extent), background)` (L104) covers the whole surface before the frame rect is written.

## 3. Findings
F-001 [S4]
Where: backend.rs:122-124
Code:
        let src = placement.source;
        let needed = dst.width as usize * dst.height as usize * 4;
        self.block.clear();
        self.block.reserve(needed);
What is wrong: `needed` is an unchecked `usize` product while the row-pointer math 6 lines down is carefully `checked_mul`/`checked_add` (L130-132, L138-145). With absurd placement geometry (≥2^31-scale extents) this overflows: panics in debug, wraps in release. Practical reachability is ~nil — destination is bounded by the surface extent, and a surface that big cannot allocate its own buffer — but the file's own doc promises "never panics on pixel coordinates", and the inconsistency with the neighboring checked math is a latent trap if the placement source ever widens. What correct looks like: `checked_mul` chain with the typed ZeroExtent/overflow error. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: backend.rs:74-79
Code:
impl std::fmt::Debug for SoftwareCompositor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Same rendering the old derive produced.
        f.debug_struct("SoftwareCompositor").finish()
What is wrong: the comment is false — a derived Debug on the struct (which owns `block: Vec<u8>`) would print `SoftwareCompositor { block: [...] }`; this prints `SoftwareCompositor` with no fields. Harmless, but a comment asserting equivalence where there is none. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: surface.rs:244 (the write protocol this file depends on)
Code:
        if rect.x + rect.width > self.extent.width || rect.y + rect.height > self.extent.height {
What is wrong: unchecked `u32` additions in the bounds check — a rect with `x`/`width` summing past `u32::MAX` wraps and can pass. Not reachable through the current placement math (which clamps to extent), defense-in-depth gap only. Confidence: HIGH — Verification: read (surface.rs:234-247).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| trait Compositor (name/composite) | 38-58 | REAL | — | the seam; one impl exists |
| SoftwareCompositor::new | 81-88 | REAL | — | empty scratch |
| Debug impl | 74-79 | REAL | S4/F-002 | comment misstates derive parity |
| Compositor::name | 91-93 | REAL | — | "software", asserted |
| Compositor::composite | 95-165 | REAL | S4/F-001 | full background fill + 1:1 row fast path + saturating per-pixel fallback; verified byte-exact |
| needed_width_bytes | 169-171 | REAL | — | checked (contrast with F-001) |
| nearest | 180-188 | REAL | — | exact (i+0.5)·src/dst floor, clamped; 17³ grid-tested |
| tests: headless/staged/full_placement helpers | 198-233 | REAL | — | — |
| tests: 7 #[test] fns | 235-383 | REAL (ran) | — | byte-exact, letterbox, up/downscale, crop, math grid, identity |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| one_to_one_placement_is_a_byte_exact_passthrough | YES (ran) | full-buffer equality incl. BGRA swizzle |
| background_fills_what_the_frame_does_not_cover | YES (ran) | corner/center pixel colors exact |
| upscale_repeats_source_pixels_nearest_neighbor | YES (ran) | exact red-channel sequence |
| downscale_picks_source_pixel_centers | YES (ran) | exact center picks [10,30] |
| source_crop_rect_selects_the_visible_band | YES (ran) | exact band [20,30] |
| nearest_math_is_bounded_identity_and_center | YES (ran) | exhaustive 1..17 sweep + identity + centers |
| compositor_reports_its_identity | YES (ran) | name equality |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The wgpu compositor (owed to the Lead-gated import …): same trait, real pipelines" | backend.rs:15-20 | FALSE-as-current-code / honest deferral | no wgpu dep, single impl (renderer.rs audit F-002); wording says "owed", so not a false claim — recorded as aspiration |
| "Point sampling needs no gamma-domain conversion: source bytes are copied, never blended" | backend.rs:24-27 | REAL | 1:1 passthrough test byte-exact; upscale duplicates bytes unblended |
| "never panics on pixel coordinates (the placement math bounds them; sampling saturates)" | backend.rs:49-51 | REAL-UNPROVEN | per-pixel path saturates (verified read of staging.pixel + fallback comment L151-153); the unchecked `needed` product (F-001) is the one gap in that promise |

## 7. Dead / duplicated / unreachable code
None dead. The `Compositor` trait exists to host a second implementation that does not exist yet (forward seam — same posture as renderer.rs's `with_backend`).

## 8. Dependencies & flags
In-crate only (color/error/params/staging/surface). No unsafe, no wgpu, no features. Integration tests (png_hash, zero_alloc_present, backend_fallback, presenter_seam) all built and green in target/debug/deps.

## 9. Open questions for the next agent
- staging.rs `StagedFrame::pixel` saturation semantics: confirm the documented "byte-identical to the documented saturation rule" claim for degenerate short-RGBA frames (the fall-through at L151-153 depends on it; not exercised by any test I ran — UNVERIFIED).
