# AUDIT: crates/subtitles/src/srt/render.rs
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-21 |
| Lines / functions | 470 / 17 (1 const, TextMetrics+2 impls, SrtRenderError+2 impls, 4 free fns + bad_anchor, 5 test fns + 6 tests) |
| File verdict | UNREACHABLE (real geometry code, library-tested; no shipped entry point reaches the crate, and no in-tree consumer outside tests/fuzz calls it) |
| Reachable from shipped entry point? | NO — crate-level proof (only `fuzz/Cargo.toml` depends on `subtitles`); additionally `rg "srt::render\|place_plain_text\|text_box"` over crates/**: the ONLY callers are this file's `#[cfg(test)]` module and `srt/fuzz.rs:35,43-45`. No renderer adapter, no integration test, no UI. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "place_plain_text\|text_box\|cue_layout" crates/subtitles` (callers; ass/* hits are a DIFFERENT `ass::layout::cue_layout`); `rg -n "srt::render" crates/**` (none outside srt); read of model/position.rs (Percent 0..=100 validated, LineNumber(i64)); read of srt/fuzz.rs + srt/mod.rs re-exports; baseline `orchrun-test-workspace-nofailfast.log.txt:5269+` (267/0 incl. `srt::render::tests::*`). |

## 1. One-paragraph verdict
This is a clean, honest geometry module: exact-rational anchors, typed
errors at every failure point, a bounded probe allocation, and strong
pixel-exact tests (green in the no-fail-fast baseline). Its problem is
that it is a seam with nothing on the other side: the module computes
where a subtitle box should go and by its own design produces ONLY
geometry — no fonts, no rasterization — and the renderer adapter that
was supposed to consume `TextMetrics`/`Rect` (renderer-wgpu, "a later
card") never landed. Combined with the crate's total isolation from
every shipped binary, the "render" half of the SRT lane is a
placeholder metric table plus arithmetic that nothing executes in
production. The matrix row's "Parse and render" claim is therefore
half SHALLOW.

## 2. What actually works (verified)
— Exact mapping verified by trace: Percent → rational (183-191),
LineNumber(i64) → clamped rational via saturating i64 math with no
overflow path (195-208: `n.saturating_mul(lh)`, `h.saturating_add`,
`raw.clamp(0, h)`); `Percent` is 0..=100 by construction
(model/position.rs:41-50) so the `as u32` casts cannot wrap.
— Baseline execution: `orchrun-test-workspace-nofailfast.log.txt:5269`
→ 267/0 subtitles unit tests incl. all 6 render tests
(`default_cue_lands_bottom_center_inside_title_safe` asserts an exact
`Rect { x: 910, y: 986, width: 100, height: 40 }` after the 5 %
title-safe clamp); `srt::fuzz` exercised via
`tests/srt_fuzz_survival.rs` (green).
— Memory bound: probe = `w*h ≤ min(extent)²` and refused above
`MAX_PROBE_PIXELS` (291-295); `vec![0u8; pixels*4]` ≤ 16 MiB, freed on
return; `pixels` computed in u64 (290) — no overflow for any u32 pair.

## 3. Findings

### F-001 [S3] The render seam is one-sided: geometry produced, nothing consumes it; "render" is placeholder metrics
Where: crates/subtitles/src/srt/render.rs:5-9, 73-78; srt/mod.rs:65-67
Code:
```rust
//! Division of labor (doc 04 §5: "text rendering itself is a
//! renderer concern"): this module owns **geometry**, the renderer
//! (`renderer-wgpu`, a later card) owns fonts and rasterization.
//! The renderer supplies nominal metrics via [`TextMetrics`] (a
//! seam, not a font stack), and this module produces:
```
```rust
impl Default for TextMetrics {
    /// A nominal 1080p subtitle metrics (40 px line, 20 px cell) —
    /// a stand-in until the renderer's real font metrics arrive.
    fn default() -> Self {
        Self::new(40, 20)
```
What is wrong: the consuming half of the seam does not exist anywhere
in the tree: no renderer adapter, no caller of `place_plain_text`
outside this file's tests and `srt/fuzz.rs` (verified by rg). The
public re-exports (`srt/mod.rs:65-67`) are consumed by nobody. The
"render path" is therefore: tag-stripped text → character-count
arithmetic → a zero-filled probe bitmap → a `Rect` — with placeholder
monospace metrics (40×20) standing in for fonts indefinitely.
Why it matters: docs/feature-matrix.toml row `subtitle-srt` (892-903)
claims "Parse **and render** SubRip subtitles incl. bold/italic/
underline styling runs" with status implemented ×3 platforms. Parsing
is real and differential-tested; "render" delivers no pixels, no
fonts, no consumer — the styling runs collapse into `plain_text`
anyway (143-148), so even the box ignores styles (bold/italic have no
metric effect). The row overstates the lane.
What correct looks like: either the renderer adapter that supplies
real metrics and rasterizes into the returned `Rect`, or a matrix row
that says "parse implemented, render geometry-only, rasterization
not started" (the compositing row's own notes already admit "every
text cue currently renders as placeholder blocks").
Confidence: HIGH — Verification: read + rg (caller search) + baseline
log (tests green, and nothing else calls it).

### F-002 [S4] "Unicode honesty" is char-counting, not measuring — wide/combining glyphs break the box
Where: crates/subtitles/src/srt/render.rs:250-261
Code:
```rust
    // Characters, not bytes: Unicode honesty in the box width.
    let line_count = text.split('\n').count().max(1);
    let longest = text
        .split('\n')
        .map(|l| l.chars().count())
```
What is wrong: every char is charged the same `char_width_px`
(258-261): CJK/fullwidth glyphs (~2 cells) produce boxes too narrow
for their text; zero-width joiners/combining marks produce boxes too
wide. The comment markets this as "Unicode honesty" while it is only
byte-count avoidance.
Why it matters: as long as metrics are the placeholder seam this is
inert (F-001), but the comment will mislead whoever wires real metrics
into believing the width math is glyph-aware.
What correct looks like: a per-char advance lookup on the `TextMetrics`
seam (e.g. `char_width(c)`), or a comment that says "nominal cells,
not glyph advances".
Confidence: HIGH — Verification: read.

### F-003 [S4] Inert `baseline_px` + per-call text reallocation
Where: crates/subtitles/src/srt/render.rs:160-161, 270; 249
Code:
```rust
    /// The baseline row (bottom-aligned box: equals `height_px`).
    pub baseline_px: u32,
```
```rust
        baseline_px: height,
```
What is wrong: (a) `baseline_px` is always `height_px` and the mapped
alignment vocabulary (224-228) never produces
`LayoutVertical::Baseline`, so the field cannot influence any output
through this module; (b) `text_box` materializes the full `plain_text`
`String` (249) just to count lines/chars — an allocation per call that
`place_plain_text` repeats per probe.
Why it matters: dead-ish public field invites misuse; minor allocation
churn in a crate that advertises allocation discipline.
What correct looks like: derive baseline from metrics when a Baseline
alignment is actually mapped; iterate segments without collecting.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAX_PROBE_PIXELS` | 49-50 | REAL | — | 4 M px = 16 MiB RGBA8; enforced at 291 |
| `TextMetrics::new` / `Default` | 63-79 | REAL | F-001 | const ctor; Default is the placeholder stand-in |
| `SrtRenderError` + `Display` + `Error` | 82-138 | REAL | — | `#[non_exhaustive]`, 5 typed variants, correct `source()` chaining |
| `plain_text` | 142-148 | REAL | F-001 (style collapse) | one alloc; styles discarded for geometry |
| `TextBoxLayout` | 152-162 | REAL | F-003 | inert baseline field |
| `cue_layout` | 172-233 | REAL | — | exact rationals; Percent validated upstream (0..=100); LineNumber i64 saturating+clamp — no overflow; BadAnchor truly defensive |
| `bad_anchor` | 235-237 | REAL (trivial) | — | |
| `text_box` | 243-272 | REAL | F-002 | clamp-to-extent then ≥1; u64 math; char-count width |
| `place_plain_text` | 282-305 | REAL | — | probe ceiling, zero-filled probe, `place_cue` mapped to typed Compose; transient alloc documented as plan-time |
| tests: `cue` builder | 320-331 | REAL | — | |
| `plain_text_joins_segments` | 332-353 | REAL | — | |
| `default_cue_lands_bottom_center_inside_title_safe` | 355-373 | REAL (strong) | — | exact Rect incl. title-safe clamp arithmetic |
| `multiline_and_long_text_boxes` | 375-390 | REAL | — | clamp path |
| `percent_and_line_placements_map_exactly` | 392-436 | REAL (strong) | — | exact anchors for %, +n, −n lines incl. clamps |
| `layout_mapping_fallbacks` | 438-447 | REAL | — | |
| `typed_failures` | 449-469 | REAL | — | ZeroExtent + ProbeTooLarge both asserted |

## 5. Test honesty (tests in this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| plain_text_joins_segments | YES | exact strings both arms |
| default_cue_lands_bottom_center_inside_title_safe | YES | exact Rect; any anchor/align/safe-margin mutation moves a pixel |
| multiline_and_long_text_boxes | YES | exact dims + clamp |
| percent_and_line_placements_map_exactly | YES | exact Rects for 3 placement modes |
| layout_mapping_fallbacks | YES | exact rational parts |
| typed_failures | YES | variant-level matches on both error arms |
| (gap) | — | no test exercises a Percent >100 or a Baseline-mapped alignment (dead paths noted in F-003); fuzz harness pins extent 1920×1080 so its ProbeTooLarge arm never fires from fuzzing (covered only by the unit test) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Parse and render SubRip subtitles incl. bold/italic/underline styling runs and encoding detection" — status windows/linux/macos = implemented | docs/feature-matrix.toml subtitle-srt (892-903) | parse REAL / render SHALLOW (library level); both UNREACHABLE in shipped builds | render = geometry + placeholder metrics, zero non-test consumers (rg); crate not a dependency of any app |
| "5.Q2-V: 6/6 ffmpeg 7.1.5 differential … 230/0 current tree" | same row evidence | REAL-UNPROVEN here | differential artifacts not present in sandbox; the 267/0 no-fail-fast run covers the unit suite only — the differential claim rests on the record doc (docs/formats/sub-srt.md, not re-verified this session) |
| "text rendering itself is a renderer concern" (doc 04 §5, quoted at :5) | module doc | REAL | consistent with the seam — but the renderer half never landed (F-001) |

## 7. Dead / duplicated / unreachable code
- The whole module is unreachable from shipped binaries; within the
  crate its only consumers are its own tests and `srt/fuzz.rs`.
- `baseline_px` (F-003a) and the `BadAnchor` pub-field escape hatch
  (unreachable through validated model types, documented as such at
  169-171).

## 8. Dependencies & flags
- No feature flags; depends only on crate-internal compositing/model
  modules. `renderer-wgpu` does NOT depend back on this crate — the
  seam is one-directional and unimplemented (F-001).

## 9. Open questions for the next agent
- Whoever audits `crates/renderer-wgpu`: confirm no subtitle-text
  adapter exists there (expected absent — F-001).
- Matrix-rows owner: subtitle-srt row's "render" word needs a
  status correction or the renderer adapter needs to land; same
  pattern will apply to vtt (row 918) — does vtt render to anything,
  or is it parser-only?
