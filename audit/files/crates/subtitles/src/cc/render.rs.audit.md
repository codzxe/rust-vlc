# AUDIT: crates/subtitles/src/cc/render.rs
| Field | Value |
|---|---|
| Auditor | W09-A04 |
| Date | 2026-11-21 |
| Lines / functions | 315 / 4 fns+methods, 5 consts, 2 type defs, 4 tests |
| File verdict | UNREACHABLE (from shipped builds; REAL and wired inside the crate) |
| Reachable from shipped entry point? | NO — crate linked only by `fuzz/` + own tests. In-crate production chain: `cc::mod` `CcCueBuilder::cues` → `render::render_grid` + `render::grid_cue` (cc/mod.rs:435-439), fed by the EIA-608 machine; exercised end-to-end by tests/cc_acceptance.rs (11/11 green, incl. `pac_colors_reach_the_bitmap`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `rg "render_grid|grid_cue|RenderedCcGrid|cell_color_of" crates/ apps/ tools/ tests/ fuzz/` (callers: cc/mod.rs only; `cell_color_of` none) · `rg -A14 "pub struct CcCell" cc/eia608.rs` (italic field exists, render ignores it) · `sed -n` on compositing/layout.rs `Rational::new`/`Anchor::new` (domains: den≠0, num≤den — layout()'s "infallible" claim verified) · orchrun log: cc::render::tests 4/4 ok (5343-5346), cc_acceptance 11/11 ok (5552-5570) |

## 1. One-paragraph verdict
Small, tight, honest: the 608 grid rasterizes to the documented solid-block placeholder with exact integer geometry (22×32 cells tiling 704×480 exactly, 25-row ascent, 3-row underline bar), a provably-infallible rational layout anchor, per-cell colors, bounded 1.29 MiB allocation, and no panics. It is genuinely wired into the crate's CC pipeline and covered by 4 unit + 11 acceptance tests that ran and passed. Defects are hygiene-grade only: a dead public helper nobody calls, a MB/MiB arithmetic slip in the bound claim, and an "italic recorded" claim that means a code comment, not a runtime record. As with the rest of the crate: no shipped binary can reach it.

## 2. What actually works (verified)
— Geometry verified by arithmetic: 32·22=704, 15·32=480 (exact tiling), ascent 25 ≤ 32, underline [25,28) ⊂ [0,32); all rasterizer offsets traced in-bounds (cell_top+28 ≤ rows·32 since (rows−1)·32+28 ≤ rows·32 ⟺ 28 ≤ 32).
— `layout()` "infallible by construction" claim proven against compositing/layout.rs:49-60,98-108: `Rational::new` only errs on den=0 (dens are the constants 32/15), `Anchor::new` only errs on num>den (u ≤ 31/32; v = (15−min_row)/15 ∈ [1/15, 1] since min_row ∈ [0,14] for a non-empty grid — num=den=15 is legal because the check is `r.num > r.den`).
— 4/4 unit tests ok (orchrun log:5343-5346) + 11/11 cc_acceptance ok (log:5552-5570), including `pac_colors_reach_the_bitmap` and `classic_rollup_scrolls_through_the_full_stack` which exercise this render end-to-end through the real 608 state machine.

## 3. Findings

### F-001 [S4]
Where: crates/subtitles/src/cc/render.rs:205-208
Code:
```
#[must_use]
pub fn cell_color_of(color: CcColor) -> (u8, u8, u8) {
    color.rgb()
}
```
What is wrong: zero callers anywhere in the workspace — not in this file's tests, not in tests/cc_acceptance.rs, not in apps/tools/fuzz (rg over all of crates/, apps/, tools/, tests/, fuzz/). Its own doc calls it "test vocabulary", but no test uses it.
Why it matters: dead public API shipped in the crate's surface; a future reader assumes a consumer exists.
What correct looks like: delete it, or use it in the acceptance test's color assertions.
Confidence: HIGH — Verification: read + rg sweep (no callers).

### F-002 [S4]
Where: crates/subtitles/src/cc/render.rs:31-33
Code:
```
//! Panics or unbounded allocation: the bitmap is bounded by the
//! 15×32 grid (704×480 RGBA8 = 1.35 MiB, allocation-checked through
//! [`CueBitmap::from_rgba`]).
```
What is wrong: 704×480×4 = 1,351,680 bytes = 1.29 MiB (1.35 MB decimal). The claim says MiB; the number is MB. Off by 5% in the wrong unit.
Why it matters: this file's selling point is exact integer honesty; a wrong unit in the one size bound it advertises is the kind of sloppiness the crate elsewhere avoids.
What correct looks like: "≈1.29 MiB (1.35 MB)".
Confidence: HIGH — Verification: arithmetic.

### F-003 [S4]
Where: crates/subtitles/src/cc/render.rs:24-27
Code:
```
//! Per-cell colors are honored (white/green/blue/cyan/red/yellow/
//! magenta/black — opaque black, see [`super::eia608::CcColor`]);
//! italics cannot slant a solid block (recorded, not faked); a
//! blank cell is transparent.
```
What is wrong: "recorded" implies a runtime record (the crate's own pattern is a damage/observation ledger — DVB's `DvbDamage`, 708's "native windows recorded" honesty table). No runtime record of italic cells exists: `render_grid` reads `cell.color` and `cell.underline` and silently ignores `cell.italic` (CcCell.italic exists, eia608.rs:142-144; the only "record" is source comments).
Why it matters: a downstream consumer cannot distinguish "no italics in this stream" from "italics discarded by the placeholder" — the honest-degradation story the crate tells elsewhere (typed counters) is not implemented for this tag.
What correct looks like: either a counter on the rendered result (e.g. `italic_cells: u32` on `RenderedCcGrid`) or the doc word changed to "documented in comments, not recorded".
Confidence: HIGH — Verification: read of render_grid (no italic read) + rg over cc/ for italic (model field only).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts `CC_REF_EXTENT`…`UNDERLINE_PX` | 44-56 | REAL | — | 22·32=704, 15·32=480 exact; 25/32≈78% (doc says ≈80%, honest). |
| `RenderedCcGrid` struct | 60-72 | REAL | — | Pure data + grid coords. |
| `RenderedCcGrid::layout` | 79-93 | REAL | — | Infallibility claim PROVEN (see §2); `unwrap_or` fallbacks unreachable — if the invariant ever broke they'd silently misplace the cue (bottom-center), the price of the no-panic lint; Start/Top alignment matches the top-left anchor semantics; v measured from frame bottom, verified for rows 0 and 14. |
| `render_grid` | 102-166 | REAL | — | Bounding-box scan O(480); bounded 1,351,680-byte alloc; underline within cell box; baseline (rows−1)·32+25; empty grid → typed None; italic ignored (F-003). |
| `grid_cue` | 173-202 | REAL | S4-note | Clones the bitmap (`rendered.bitmap.clone()`, :198) — up to 1.29 MiB per cue per update; correct but a take-by-value/Cow would avoid the copy on the roll-up path (one event per scroll step). ns TimeBase literal bypasses validation with a provably-true justification comment. |
| `cell_color_of` | 205-208 | DEAD | S4 | F-001. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `empty_grid_renders_none` (225-227) | YES | Empty → None pinned. |
| `bounding_box_and_metrics_are_exact` (230-259) | YES | (min/max row/col), width/height/baseline, white block pixel, descent-gap transparency, right-edge pixel — all concrete. |
| `per_cell_colors_and_underline` (262-296) | YES | Red/green pixel values, underline rows [25,28) lit, row 28 dark. |
| `layout_anchor_is_rational_exact` (299-314) | YES | u=10/32, v=1/15, reference extent, TitleSafe pinned. |
All 4 executed ok (orchrun log:5343-5346); end-to-end corroboration via cc_acceptance 11/11 (log:5552-5570). Gap: no test pins the top-row anchor (min_row=0 → v=15/15=1) or min_col=0 (u=0) — the boundary numerators of the "infallible" claim.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "render to CompositedCue over the 704x480 NTSC reference; 26 unit + 11 cc_acceptance tests, all green" | docs/feature-matrix.toml:1022-1033 (subtitle-cc-608-708) | REAL-UNPROVEN → unit+acceptance proven in-tree (orchrun log), but "implemented" platform statuses are UNREACHABLE in any shipped build (crate not linked; pipeline_host.rs:1238 drops subtitle frames). |
| "italics … (recorded, not faked)" | render.rs:26 | SHALLOW | No runtime record exists (F-003). |
| "the bitmap is bounded … 1.35 MiB" | render.rs:32 | FALSE (unit) | 1.29 MiB (F-002). |
| Solid-block placeholder policy | render.rs:7-9 | HONEST/REAL | Code matches exactly (verified pixel tests). |

## 7. Dead / duplicated / unreachable code
`cell_color_of` (dead, F-001). Third copy of the monospace placeholder geometry family (ASS render, font/pipeline, this file) — here constants are CC-specific (22/32/25) and honestly NOT shared with `GlyphMetrics`; acceptable but see pipeline.rs F-002 for the family drift risk.

## 8. Dependencies & flags
Imports: cc::eia608 (COLS/ROWS/CcColor/CcGrid), compositing (bitmap/layout), media-types time. No features, no unsafe, no panics (verified by read; the two `unwrap_or` fallbacks are unreachable, §2).

## 9. Open questions for the next agent
1. cc/mod.rs + eia608.rs auditor: does any 608 cell ever carry a non-default italic=true in the acceptance fixtures (F-003 would then be observable as a lost attribute)?
2. compositing/layout.rs auditor: `Anchor::new` allows num==den (v=1, top edge) — confirm the compositor's scaling/clamp treats v=1 + TitleSafe correctly for a full-grid caption (row 0 occupied).
