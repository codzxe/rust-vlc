# AUDIT: crates/renderer-wgpu/src/params.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-21 |
| Lines / functions | 664 / 30 (19 prod items: 5 structs/enums, 11 methods/fns, 3 impls/consts; 11 tests + 2 helpers) |
| File verdict | REAL (math executed in shipped CLI; 3 of 4 aspect modes + crop parameter surface UNREACHABLE — F-001) |
| Reachable from shipped entry point? | YES for the default path (chain: `lumen` video limb → apps/cli/src/pipeline_host.rs:1314 `Renderer::new_headless` → renderer.rs:208 `compute_placement` per frame, always `RenderParams::new()` = Fit/no-crop); NO for Fill/Stretch/Crop16x9/crop/background≠black/subtitle (no shipped caller of `Renderer::set_params`) |
| Findings | S0:0 S1:0 S2:1 S3:2 S4:1 |
| Commands run | `rg -n "set_params\|RenderParams\|AspectMode" apps/ crates/` (no production consumer outside renderer-wgpu) · `rg -n "set_params"` → only tests/backend_fallback.rs:217 · `rg -n "video-aspect-ratio\|aspect-ratio" apps/cli/src/` → none; catalog-only matches in crates/commands · `rg -n "subtitle_layer" crates/ apps/ ui/` → only params.rs:159 · `rg -n "centered_in" crates/ apps/ tests/ ui/` → only params.rs (def + own test) · arithmetic overflow analysis of div_round/intersect (§3 F-003, §4) · baseline log: params::tests 11/11 ok (:4787-4820) |

## 1. One-paragraph verdict
The aspect/crop math is genuine, careful, and fully executed: pure u64 integer geometry, typed errors for zero extents and disjoint crops, reference-rectangle tests plus a 202,500-combination grid sweep, all passing in the no-fail-fast baseline. The defect is around the math, not in it: the entire parameter surface — Fill, Stretch, Crop16x9, user crop, background, subtitle layer — is unreachable in every shipped build because the only mutation seam, `Renderer::set_params`, has zero production callers and the CLI constructs the renderer with `RenderParams::new()` (Fit, no crop, black). Meanwhile the product's own surfaces promise this control: settings `video.aspect-ratio` (7 enum values) and `video.crop` (5 values) with UI placement and a `--video-aspect-ratio` CLI flag that does not exist in the CLI, and control-protocol commands `video.set-aspect-ratio`/`video.set-crop` that have catalog entries but no handler anywhere. Two smaller items: `Rect::centered_in` is dead code whose doc claims it is "used by Fill/Crop16x9 source crops" (false — those paths use private helpers), and `Rect::intersect` does unchecked u32 edge addition that overflows for large crop offsets.

## 2. What actually works (verified)
— All 11 params tests ran and passed in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4787-4820` (incl. `placement_is_never_empty_or_inverted_on_a_size_grid`, `reference_rectangles_*`, `disjoint_crop_is_a_typed_error`, `zero_extents_are_typed_errors`).
— The shipped render path executes this file's math: apps/cli/src/pipeline_host.rs:1314 builds `Renderer::new_headless(...)`; renderer.rs:208 calls `compute_placement(staged.extent, surface.extent(), &self.params)` on every frame; the first-vertical AVI lane renders frames end-to-end (baseline INDEX §"contrast: AVI first-vertical"; first-vertical evidence tests compare a rendered final frame — the frame exists because this path works).
— Overflow analysis of the Fit math: `div_round(tgt_w * src_h, src_w)` — worst case (2^32−1)² + (2^32−1)/2 < 2^64, no u64 overflow; `dst_w.min(tgt_w)` before subtraction kills underflow; `.max(1)` on every crop dimension kills zero-div and empty rects; `crop_to_ratio`/`crop_source_to_target_ratio` never widen (proven: div_round(a,b) ≤ ceil bound contradicts branch conditions), so `(w - new_w)` cannot underflow.
— `checked_pixels`: u32×u32 into u64 — no overflow; zero components typed-refused (test passed).

## 3. Findings
### F-001 [S2]
Where: crates/renderer-wgpu/src/params.rs:133-160 (the surface), evidence: crates/renderer-wgpu/src/renderer.rs:87-93 + :122, apps/cli/src/pipeline_host.rs:1314-1321, crates/renderer-wgpu/tests/backend_fallback.rs:217, schemas/settings.toml:959-973, crates/commands/src/catalog/spec.rs:362-372
Code (params.rs:147-160):
```
pub struct RenderParams {
    /// How the frame is fitted into the surface.
    pub aspect: AspectMode,
    /// User crop applied to the frame *before* aspect placement (frame
    /// pixel coordinates).
    pub crop: Option<Rect>,
    /// The clear color for areas the frame does not cover.
    pub background: Rgba,
```
Code (renderer.rs:87-93, the shipped constructor):
```
    pub fn new_headless(size: Extent2D, config: SurfaceConfig) -> RendererResult<Renderer> {
        let surface = HeadlessSurface::new(size, config)?;
        Ok(Renderer::with_backend(
            Box::new(surface),
            Box::new(SoftwareCompositor::new()),
            RenderParams::new(),
        ))
    }
```
What is wrong: nothing in any shipped app can ever change aspect, crop, background, or subtitle layer. The only mutation seam, `Renderer::set_params` (renderer.rs:122), is called nowhere in production — its sole caller repo-wide is a test (backend_fallback.rs:217). The shipped CLI locks the renderer to `RenderParams::new()` (Fit, no crop, black). Yet the product declares this control in three places: (1) `schemas/settings.toml:959-966` — setting `video.aspect-ratio`, enum `["automatic", "1:1", "4:3", "16:9", "16:10", "2.35:1", "2.39:1"]`, `placement = { screen = "video", ... ui = true }`, `cli = { flag = "--video-aspect-ratio" }`; (2) `schemas/settings.toml:971-973` — setting `video.crop` with 5 values; (3) `crates/commands/src/catalog/spec.rs:362-372` — wire commands `video.set-aspect-ratio` / `video.set-crop` (class S, target Engine). `rg "aspect-ratio" apps/cli/src/` finds nothing — the CLI flag does not exist; `rg "VideoSetAspectRatio"` outside the catalog finds no handler. Worse, the type vocabulary does not even line up: the setting enumerates 4:3, 16:10, 2.35:1, 2.39:1, but `AspectMode` offers only Fit/Fill/Stretch/Crop16x9 — there is no code path anywhere that produces a 4:3 or 2.39:1 crop (`crop_to_ratio` is private and invoked only with 16, 9 at params.rs:239).
Why it matters: a user changing aspect ratio or crop in the settings UI, via the documented flag, or over the control protocol gets zero effect (or an unhandled-command error), while the math this file perfects sits locked at defaults. This is "UI settings wired to nothing" plus a catalog command advertised (`system.list-commands` lists all catalog ids) with no implementation.
What correct looks like: either wire `video.aspect-ratio`/`video.crop` through to `Renderer::set_params` (mapping each enum value to a real placement mode — which requires generalizing `crop_to_ratio` beyond 16:9 and adding the missing ratios), or remove the settings rows, the flag column, and the catalog commands until the feature ships.
Confidence: HIGH — Verification: read all cited sites; rg exhaustions (`set_params` → 1 test caller; `aspect-ratio` in apps/cli/src → none; `VideoSetAspectRatio` outside catalog → none).

### F-002 [S3]
Where: crates/renderer-wgpu/src/params.rs:115-127
Code:
```
    /// Centered sub-rectangle of `frame`'s extent with the given size,
    /// clamped into the extent (used by Fill/Crop16x9 source crops).
    #[must_use = "pure geometry"]
    pub fn centered_in(size: Extent2D, frame: Extent2D) -> Rect {
```
What is wrong: `Rect::centered_in` has zero callers outside its own test (params.rs:624-647). Its doc claim "(used by Fill/Crop16x9 source crops)" is false: `compute_placement` implements Fill via `crop_source_to_target_ratio` (params.rs:278) and Crop16x9 via `crop_to_ratio` (params.rs:239) — neither calls `centered_in`.
Why it matters: dead public API with a doc comment that misattributes live behavior to it — a reader auditing the crop paths would be sent to the wrong function.
What correct looks like: delete it, or fix the doc to "kept for the config-driven crop set [PROPOSAL]" and mark it `#[allow(dead_code)]`-free by actually using it when arbitrary-ratio crop lands.
Confidence: HIGH — Verification: `rg -n "centered_in" crates/ apps/ tests/ ui/` → only params.rs def + test.

### F-003 [S3]
Where: crates/renderer-wgpu/src/params.rs:89-93
Code:
```
    pub fn intersect(self, other: Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
```
What is wrong: unchecked u32 addition on rectangle edges. A `Rect { x: u32::MAX, width: 1, .. }` (a legal value of the public type, e.g. a hostile or buggy user crop) makes `self.x + self.width` overflow: panic in debug builds, wrap to 0 in release (giving a wrong-but-typed `None`/garbage intersection instead of a correct answer). Today this is unreachable in shipped builds only because nothing sets `params.crop` (F-001); the moment crop wiring lands, this becomes a user-input panic path in debug builds.
Why it matters: the one function whose inputs are the least trusted (user crop coordinates) is the one place with unchecked arithmetic.
What correct looks like: compute edges in u64 (`u64::from(self.x) + u64::from(self.width)`), or `checked_add` with a typed overflow error — the file already establishes the typed-error pattern.
Confidence: HIGH — Verification: read; u32 overflow arithmetic checked; only caller is compute_placement's crop path (params.rs:231-233).
### F-004 [S4]
Where: crates/renderer-wgpu/src/params.rs:156-159
Code:
```
    /// An optional subtitle layer to alpha-blend over the video
    /// (doc 03 §4.11 "Subtitle compositing [PROPOSAL]" — the texture
    /// id vocabulary; blending itself is the Phase-5 card).
    pub subtitle_layer: Option<u64>,
```
What is wrong: `subtitle_layer` is written by no constructor and read by no consumer anywhere in the repo (rg over crates/, apps/, ui/ → only this declaration). A host that sets it gets nothing.
Why it matters: a public API field that silently does nothing; documented as a future phase, which keeps this at hygiene level.
What correct looks like: gate it out (`#[cfg(...)]`) or leave a `debug_assert`/doc note at the render call site stating it is ignored until Phase 5.
Confidence: HIGH — Verification: rg exhaustions.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-17 | REAL | — | honest adaptation notes (ColorDesc vs paintable Rgba; Crop16x9 only, general crop [PROPOSAL]) |
| `Extent2D` + `ONE` | 22-36 | REAL | — | const, used by aspect-math clamp docs; fine |
| `Extent2D::rgba8_bytes` | 44-47 | REAL | — | saturating ×4 documented; test passed |
| `Extent2D::checked_pixels` | 50-59 | REAL | — | u32×u32→u64 no overflow; typed ZeroExtent |
| `Rect` + `covering`/`extent` | 63-113 | REAL | — | trivial correct |
| `Rect::intersect` | 88-104 | REAL | S3 | F-003 unchecked u32 adds; touching-edges behavior tested |
| `Rect::centered_in` | 117-127 | REAL | S3 | F-002 dead + lying doc |
| `enum AspectMode` | 132-144 | REAL | — | Fit reachable; Fill/Stretch/Crop16x9 unreachable in shipped builds (F-001); doc honest about [PROPOSAL] extension set |
| `RenderParams` + `new` + `Default` | 147-179 | REAL | S2/S4 | F-001 (surface locked), F-004 (subtitle_layer dead) |
| `Placement` | 185-191 | REAL | — | consumed as uniforms/loop bounds by renderer.rs:208+, backend.rs:102-116 |
| `div_round` | 193-196 | REAL | — | ties-away correct; overflow-safe at max u32 inputs (proven §2) |
| `compute_placement` | 209-286 | REAL | — | executed per frame (Fit path) in shipped CLI; zero-extent and disjoint-crop typed errors tested; crop+Crop16x9 arms unreachable today (F-001) |
| `crop_to_ratio` | 289-314 | REAL | — | 16:9 only; never widens (proven); `.max(1)` guards; unreachable in shipped (Crop16x9) but tested |
| `crop_source_to_target_ratio` | 317-343 | REAL | — | same safety proof; Fill-only, unreachable shipped, tested |
| tests (11) | 345-663 | REAL | — | see §5 |

## 5. Test honesty (all 11 passed in baseline :4787-4820)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `reference_rectangles_fit` | YES | exact expected rects (letterbox/pillarbox/identity) — swapped axes or wrong min() fails |
| `reference_rectangles_fill_and_stretch` | YES | exact source-crop rects on both axes |
| `reference_rectangles_crop16x9` | YES | exact 16:9 band + Fit result |
| `user_crop_applies_before_aspect` | YES | crop rect propagated to source before placement |
| `disjoint_crop_is_a_typed_error` | YES | asserts the exact typed error payload |
| `zero_extents_are_typed_errors` | YES | both frame and target zero cases with exact payloads |
| `placement_is_never_empty_or_inverted_on_a_size_grid` | YES | 15⁴×4 = 202,500 combos, asserts non-empty + in-bounds both sides — inversion/overflow mutations fail |
| `fit_never_upscales_past_the_target` | YES | 12⁴ combos, dst ≤ target on both axes |
| `rect_intersection_geometry` | YES | includes the touching-edges zero-area case (off-by-one in `right > x` fails) |
| `centered_in_clamps_into_the_frame` | YES (but tests dead code) | F-002: the only caller of `centered_in` is this test |
| `extent_bytes_are_checked` | YES | 4×4→64, zero-width typed error |
| Gap | — | no test pins that Fill CROPS rather than stretches the source to target ratio — covered indirectly by `reference_rectangles_fill_and_stretch` exact rects; no fuzz beyond the grid (fine) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Scaling and aspect placement is pure integer math, unit-tested against reference rectangles" | params.rs:3-4 | REAL | tests passed (baseline :4787-4820) |
| "the same code on every backend (the shader path consumes its output as uniform data)" | params.rs:5-6 | REAL | renderer.rs:208 feeds both compositor seams; backend.rs:102-116 consumes Placement |
| "used by Fill/Crop16x9 source crops" (centered_in) | params.rs:116 | FALSE | F-002: zero callers; those paths use private helpers |
| "the config-driven extension set is [PROPOSAL]" | params.rs:130-131 | REAL (honest) | and consistent with F-001: nothing config-driven reaches AspectMode today |
| "blending itself is the Phase-5 card" (subtitle_layer) | params.rs:157-158 | REAL (honest) | field unread everywhere — F-004 |
| settings `video.aspect-ratio` (7 values, ui=true, `--video-aspect-ratio`) and `video.crop` (5 values) | schemas/settings.toml:959-973 | FALSE (as wired) | F-001: no CLI flag, no handler, no set_params caller |
| wire commands `video.set-aspect-ratio` / `video.set-crop` | crates/commands/src/catalog/spec.rs:362-372 | PAPER-ONLY | catalog entries exist; no dispatch/handler anywhere (rg) |

## 7. Dead / duplicated / unreachable code
- `Rect::centered_in` — dead (own test only) — F-002.
- `RenderParams.subtitle_layer` — dead field — F-004.
- `AspectMode::Fill/Stretch/Crop16x9`, `RenderParams.crop`, `background` (non-black) — unreachable from shipped entry points (F-001); reachable only from renderer-wgpu tests.
- Everything else has live consumers (renderer.rs:208-213, backend.rs:102-116, pipeline_host.rs:1310-1321).

## 8. Dependencies & flags
- Uses only crate-local `color::Rgba`, `error::{RendererError, RendererResult}` + std; testkit in tests. No feature gates. Crate builds in the default workspace build (baseline #1 clean); `lumen engine-info` reports `video.software-compositor`, the backend that consumes this file's output.

## 9. Open questions for the next agent
- backend.rs auditor (mine, later): does `SoftwareCompositor::composite` honor `Placement.source` cropping exactly (the Fill/Crop16x9 correctness of the whole chain is only as good as the compositor's source-rect sampling)?
- Whoever owns schemas/settings.toml + commands catalog: F-001's false surface — the settings generator (crates/settings-schema) and the catalog spec need a "wired" gate so unimplemented commands/settings cannot ship declared.
- ui/src aspect-ratio SettingField tests exist (SettingField.test.tsx:19-63) — UI wave should confirm the control renders and silently no-ops.
