# AUDIT: crates/renderer-wgpu/src/params.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 664 (344 source + 320 tests) / 11 non-test fns + 11 tests |
| File verdict | REAL (pure integer aspect/crop math, correct, baseline-tested) with a dead-in-production feature surface (F-001) |
| Reachable from shipped entry point? | YES for `compute_placement` with default params: apps/cli/src/pipeline_host.rs:1314 `Renderer::new_headless` → renderer.rs:208 `compute_placement(staged.extent, surface.extent(), &self.params)` on the present path. NO for Fill/Stretch/Crop16x9 and user crop (F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Read of full file; `rg`/Grep caller map (`compute_placement`, `set_params`, `AspectMode` — no external references to set_params/AspectMode); manual recomputation of all 9 reference rectangles in the tests; baseline citation audit/runs/baseline/cargo-test-workspace.txt:5215-5255 |

## 1. One-paragraph verdict
Correct, overflow-conscious pure placement math. I independently recomputed every reference rectangle the tests assert (640x360→320x240 Fit = (0,30,320,180); 320x240→640x360 Fill source crop (0,30,320,180); 4:3 Crop16x9 = (0,30,320,180); etc.) and the integer math matches, including the div_round half-up rounding and the min(tgt) clamps that catch rounding overshoot. The 202,500-case grid sweep test asserting non-empty in-bounds placements passed in the baseline (11/11 params tests ok, baseline:5215-5255). Two real complaints: (1) three of the four aspect modes and the entire user-crop path are dead code in shipped builds — nothing outside renderer-wgpu ever calls `Renderer::set_params`, so the shipped CLI renders only Fit/no-crop; (2) `Rect::intersect` does unchecked u32 addition that panics in debug on crop rects near u32::MAX — latent today only because the crop path is unreachable. This is a library of honest pure functions whose feature surface outran its wiring.

## 2. What actually works (verified)
— Manual recomputation of tests' expected rectangles (Fit letterbox/pillarbox/identity, Fill both axes, Crop16x9, user-crop-before-aspect) — all match the code's formulas.
— Baseline: `params::tests::*` 11/11 ok including `placement_is_never_empty_or_inverted_on_a_size_grid` (audit/runs/baseline/cargo-test-workspace.txt:5215-5255).
— Fit scaling guard `dst_w.min(tgt_w)`/`dst_h.min(tgt_h)` (params.rs:261-262) verified against rounding-overshoot inputs by inspection; u64 products of two u32s cannot overflow.

## 3. Findings
F-001 [S3]
Where: params.rs:130-160 (AspectMode/RenderParams surface); evidence apps/cli/src/pipeline_host.rs:1314-1333, crates/renderer-wgpu/src/renderer.rs:116-122
Code: (verbatim, params.rs:142-143)
```
    /// Center-crop the frame to 16:9 first, then [`Fit`](AspectMode::Fit).
    Crop16x9,
```
What is wrong: `AspectMode::Fill`, `Stretch`, `Crop16x9`, `RenderParams.crop`, and `RenderParams.background` are selectable only through `Renderer::set_params` (renderer.rs:122) — and a whole-repo search finds ZERO callers of `set_params` or constructors of `AspectMode` outside renderer-wgpu's own tests. The shipped CLI builds the renderer via `Renderer::new_headless(...)` (pipeline_host.rs:1314) and never updates params, so every shipped present runs Fit with no crop and black background. The module header markets `Crop16x9` as something that "ships" (params.rs:15-16: "`AspectMode::Crop16x9` ships as the doc names it"). It ships as dead vocabulary. Why it matters: any user-facing aspect/crop setting would silently do nothing. What correct looks like: wire product-config's playback settings to `set_params`, or mark the modes `#[cfg(any(test, feature = "..."))]` until wired.
Confidence: HIGH — Verification: `Grep set_params|AspectMode` whole repo → only renderer-wgpu internal + tests.

F-002 [S4]
Where: params.rs:89-93
Code: (verbatim)
```
    pub fn intersect(self, other: Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
```
What is wrong: `self.x + self.width` is unchecked u32 addition — a Rect with `x = u32::MAX - 2, width = 5` panics in debug builds (overflow) and wraps in release (silently wrong `right`). The only current caller feeds it a user crop, which is unreachable today (F-001), so this is latent. What correct looks like: `u64::from(self.x) + u64::from(self.width)` like the rest of the file already does.
Confidence: HIGH — Verification: read (arithmetic; not executed).

F-003 [S4]
Where: params.rs:156-160
Code: (verbatim)
```
    /// An optional subtitle layer to alpha-blend over the video
    /// (doc 03 §4.11 "Subtitle compositing [PROPOSAL]" — the texture
    /// id vocabulary; blending itself is the Phase-5 card).
    pub subtitle_layer: Option<u64>,
```
What is wrong: `subtitle_layer` is written by exactly one expression (`subtitle_layer: None` in `RenderParams::new`, line 170) and read by nothing in the entire repo. A published struct field that no code consumes. The doc comment honestly labels it a proposal, which keeps this at S4 rather than higher.
Confidence: HIGH — Verification: Grep `subtitle_layer` → params.rs only.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Extent2D::rgba8_bytes | 44-47 | REAL | — | saturating_mul(4), zero-extent typed |
| Extent2D::checked_pixels | 50-59 | REAL | — | u64 product of u32s, zero typed |
| Rect::covering | 77-85 | REAL | — | trivial |
| Rect::intersect | 88-104 | REAL | S4 | unchecked u32 add (F-002) |
| Rect::extent | 107-113 | REAL | — | trivial |
| Rect::centered_in | 117-127 | REAL | — | saturating_sub, min-clamped ✓ |
| RenderParams::new / Default | 164-179 | REAL | — | Fit/black/none |
| compute_placement | 209-286 | REAL | — | mode math verified; Fit clamps correct |
| div_round | 193-196 | REAL | — | half-up = ties-away for positives ✓ |
| crop_to_ratio | 289-314 | REAL | — | u64 math, .max(1), verified vs tests |
| crop_source_to_target_ratio | 317-343 | REAL | — | cover crop both axes verified |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| reference_rectangles_fit / _fill_and_stretch / _crop16x9 | YES | exact expected rects, independently recomputed by me |
| user_crop_applies_before_aspect | YES | source rect must equal the crop |
| disjoint_crop_is_a_typed_error | YES | exact CropEmpty payload |
| zero_extents_are_typed_errors | YES | exact ZeroExtent payloads both sides |
| placement_is_never_empty_or_inverted_on_a_size_grid | YES | 202,500 combos, in-bounds + non-empty asserts, count asserted |
| fit_never_upscales_past_the_target | YES | ≤ target on both axes across 20,736 combos |
| rect_intersection_geometry / centered_in_clamps / extent_bytes | YES | exact values |
| GAP: no test exercises a crop rect with x+width > u32::MAX | — | F-002 latent; sizes in the grid stop at 255 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Scaling and aspect placement is pure integer math, unit-tested against reference rectangles ... it never touches a GPU" | params.rs:3-6 | REAL | read + recomputation + baseline 11/11 |
| "`AspectMode::Crop16x9` ships as the doc names it" | params.rs:15-16 | SHALLOW | the enum ships; nothing can select it (F-001) |
| "the config-driven extension set is `[PROPOSAL]`" | params.rs:16 | REAL | honest — no config wiring exists at all, not even for the four shipped modes |

## 7. Dead / duplicated / unreachable code
Fill/Stretch/Crop16x9 arms of compute_placement (lines 247-283), `RenderParams.crop`/`background`/`subtitle_layer` fields: production-unreachable (F-001/F-003); exercised only by this file's unit tests.

## 8. Dependencies & flags
crate::color::Rgba, crate::error (in-crate). No feature gates. renderer-wgpu is a dependency of apps/cli (Cargo.toml:60) — the headless present path is shipped.

## 9. Open questions for the next agent
1. renderer.rs auditor: confirm `Renderer::present` actually consumes `Placement.source` (texture coords) and `destination` — i.e., that Fit is applied to real pixels in the shipped path, not just computed.
2. product-config auditor: is there any settings key (aspect/crop/background) that was supposed to feed set_params? If yes, it is "declared but never read".
