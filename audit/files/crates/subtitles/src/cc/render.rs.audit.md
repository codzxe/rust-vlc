# AUDIT: crates/subtitles/src/cc/render.rs
| Field | Value |
|---|---|
| Auditor | W09-A09 |
| Date | 2026-11-30 |
| Lines / functions | 315 / 4 prod (RenderedCcGrid::layout, render_grid, grid_cue, cell_color_of) + 4 tests |
| File verdict | SHALLOW (real block rasterizer; placement vertically mirrored; glyph coverage is a documented placeholder; tests pin the buggy formula) |
| Reachable from shipped entry point? | NO — `render_grid`/`grid_cue` exported via `cc` module inside `crates/subtitles`; only tree-wide dependent is `fuzz/Cargo.toml:50` (non-member). No engine/CLI caller exists. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/subtitles-85f02b91c21cc80b cc::render` → 4 passed 0 failed (exit 0); `rg -n '\.italic' crates/subtitles/src/cc/render.rs` → no match (exit 1); `rg -n 'cell_color_of' crates/` → 1 hit (the definition only) |

## 1. One-paragraph verdict
Answering the hostile-focus question first: "render" produces RGBA **bitmaps of solid color blocks** — one 22×32 px rectangle per occupied grid cell (25-row ascent block + optional 3-row underline bar) in the cell's pen color, cropped to the occupied bounding box, plus an exact-rational CueLayout. No glyphs, no text shapes — readable captions are NOT produced; the file says so up front (fonts belong to the renderer). The rasterization itself is correct and tested (4/4 pass), but the placement anchor is vertically MIRRORED: `v = (ROWS - min_row)/ROWS` is a bottom-up fraction, while `place_cue` interprets v top-down (`ay = v·height` from the top — proven by BOTTOM_CENTER 9/10 landing at y≈972). A bottom-row caption (row 14) gets v=1/15 → its top edge at y=32, i.e. the TOP of the frame; a top-row caption lands at the bottom. The module's own doc (line 18) specifies the correct formula ("top-of-first-occupied-row / 15") and the code contradicts it; the unit test enshrines the wrong value, so green tests prove nothing.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-85f02b91c21cc80b cc::render` → `test result: ok. 4 passed; 0 failed`. Bounding box, 22×32 tiling (32·22=704, 15·32=480 — exact NTSC tiling), per-cell colors, underline bar rows [25,28), transparent gaps, baseline `(rows-1)·32+25`, 1.35 MiB allocation ceiling — all re-derived and consistent.

## 3. Findings
F-001 [S2]
Where: render.rs:75-83 (doc + formula), render.rs:17-18 (module doc), layout.rs:290-291/305-306 (consumer semantics)
Code: (verbatim)
```
    /// The exact layout: top-left anchored at
    /// `(min_col / 32, (15 - min_row) / 15)` — v measured from the
    /// frame bottom — with the CC reference extent for scaling.
```
```
        let u = Rational::new(self.min_col as u32, COLS as u32).unwrap_or(Rational::ZERO);
        let v = Rational::new((ROWS - self.min_row) as u32, ROWS as u32).unwrap_or(Rational::ZERO);
```
What is wrong: `place_cue` measures v TOP-DOWN: `let ay = layout.anchor.v.scale_u64(extent.1)` then `y0 = ay.saturating_sub(ry)` with `Rect.y` the top edge (layout.rs:290-306); the crate's own BOTTOM_CENTER (v=9/10, "90 % down") confirms v=1 is the frame bottom in top-down pixels. The CC formula `(ROWS - min_row)/ROWS` is a bottom-up fraction. Concrete: a caption on grid row 14 (bottom — and `Cc608Channel` starts its cursor at `ROWS - 1`, eia608.rs:365, so this is the NORMAL case) gets v=1/15 → cue top at y=32 of 480 — the top of the screen. Grid row 0 (top) gets v=1 → clamped to the bottom. Every CC cue renders at the mirrored vertical position. The module doc itself specifies the correct anchor ("top-of-first-occupied-row / 15", line 18) — code contradicts doc. What correct looks like: `v = Rational::new(min_row, ROWS)` (with Top alignment), per the module doc.
Confidence: HIGH — Verification: read (full derivation through place_cue + clamp_axis; no executable placement test exists — none of the 4 tests calls place_cue with the CC layout).

F-002 [S3]
Where: render.rs:7-9, render.rs:134-143
Code: (verbatim)
```
//! stack. The 608 grid is therefore rasterized with the same honest
//! placeholder coverage as the ASS plain-text path: every cell is a
//! SOLID BLOCK in the cell's pen color. The geometry is exact:
```
What is wrong: glyph coverage is a placeholder — a wired engine would display colored rectangles instead of letters for every CC caption. It is honestly documented (fonts deferred to the renderer per doc 04 §5), so this is a declared gap, not a hidden stub — but the deliverable "CC render" does not produce readable captions today. Blast radius currently zero (crate unreachable from shipped apps).
Confidence: HIGH — Verification: read (render loop writes solid blocks; no glyph/glyph-metric code exists).

F-003 [S4]
Where: render.rs:26-27
Code: (verbatim)
```
//! magenta/black — opaque black, see [`super::eia608::CcColor`]);
//! italics cannot slant a solid block (recorded, not faked); a
```
What is wrong: "recorded" is false. `cell.italic` is never read in render.rs (`rg '\.italic' render.rs` → no match); the flag is parsed and stored (eia608.rs:144,554-559) and then silently dropped — no damage event, no ledger, nothing. Should say "ignored".
Confidence: HIGH — Verification: read + rg (exit 1).

F-004 [S4]
Where: render.rs:204-208
Code: (verbatim)
```
/// The color of one rendered pixel (test vocabulary).
#[must_use]
pub fn cell_color_of(color: CcColor) -> (u8, u8, u8) {
    color.rgb()
}
```
What is wrong: dead public function — zero callers in the entire tree, including the tests it claims to serve ("test vocabulary"). Pure forwarding of `CcColor::rgb()`. Shipped-API surface pollution.
Confidence: HIGH — Verification: rg across crates/ → only the definition.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| RenderedCcGrid::layout | 79-93 | REAL (buggy) | S2 | u correct; v mirrored (F-001); infallible-by-construction claim OK (denominators are consts) |
| render_grid | 102-166 | REAL | — | bbox, tiling, colors, underline, baseline all correct; bounded 1.35 MiB; `unwrap_or` fallbacks unreachable |
| grid_cue | 173-202 | REAL | — | TimeBase{1,1e9} literal valid (coprime); spans open/closed correct |
| cell_color_of | 206-208 | STUB-ish dead code | S4 | zero callers (F-004) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_grid_renders_none | YES | empty grid → None |
| bounding_box_and_metrics_are_exact | YES | bbox (3,4,5,7), sizes 66x64, baseline 57, pixel colors asserted |
| per_cell_colors_and_underline | YES | red/green blocks + underline rows discriminated |
| layout_anchor_is_rational_exact | NO — it PINS the bug | asserts v=1/15 for row 14 (F-001); would FAIL if the mirror were fixed — the test enshrines wrong behavior |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "anchors at (min_col / 32, top-of-first-occupied-row / 15) of the frame" | render.rs:17-18 | FALSE vs code | F-001: code uses (15 - min_row)/15 |
| "v measured from the frame bottom" | render.rs:76-77 | FALSE vs consumer | place_cue v is top-down (layout.rs:290-291, BOTTOM_CENTER 9/10 → y≈972) |
| "every cell is a SOLID BLOCK … placeholder coverage" | render.rs:7-9 | REAL (honest placeholder) | render loop |
| "italics … (recorded, not faked)" | render.rs:26 | FALSE | F-003: never read, never recorded |
| "704×480 … tiles it EXACTLY: 22 px … 32 px" | render.rs:11-13 | REAL | consts 22/32; 32·22=704, 15·32=480 |
| "Panics or unbounded allocation" never (≤1.35 MiB) | render.rs:29-33 | REAL | grid-bounded allocation |

## 7. Dead / duplicated / unreachable code
- `cell_color_of` dead (F-004). Whole file unreachable from shipped entry points (crate-level).

## 8. Dependencies & flags
- `cc::eia608` (grid/colors), `compositing::{bitmap,layout}`, `media_types::time`. No unsafe, no features. Clone of the bitmap per cue (`rendered.bitmap.clone()` in grid_cue) — ≤1.35 MiB per cue, acceptable.

## 9. Open questions for the next agent
- eia608.rs / cea708.rs owners: does any doc/matrix row claim "CC captions display at the authored row"? If yes, F-001 makes that row FALSE, not just SHALLOW.
- Whoever wires an engine: fix F-001 before any screenshot/AC evidence is gathered — the mirror inverts every caption position.
