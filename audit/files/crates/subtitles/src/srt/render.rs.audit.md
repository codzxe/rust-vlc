# AUDIT: crates/subtitles/src/srt/render.rs
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-23 |
| Lines / functions | 470 / 13 (TextMetrics::{new,Default}, SrtRenderError Display+Error, plain_text, cue_layout, bad_anchor, text_box, place_plain_text, 6 tests) |
| File verdict | UNREACHABLE (geometry is real and exact; no production caller — the promised renderer consumer does not exist) |
| Reachable from shipped entry point? | NO — `place_plain_text` callers are srt/fuzz.rs:45, tests/srt_canonical_differential.rs:151, and unit tests only; renderer-wgpu's only subtitle surface is a never-populated `Option<u64>` (params.rs:159,170 — grep `subtitle_layer` across crates/: 2 hits, declaration + `None` default, never read) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | Grep `place_plain_text|srt::render` in crates/ (exit 0, callers listed above); Grep `subtitle_layer` in crates/ (2 hits only); Grep `subtitle|TextMetrics` in renderer-wgpu (doc comments + params only); sed/Read quote verifications (exit 0); baseline citation below |

## 1. One-paragraph verdict
The model→compositing geometry is genuinely exact (integer rationals, correct percent/line-number math, honest probe-bitmap memory bound at 16 MiB, typed errors) and the unit tests pin pixel-exact rects including the title-safe clamp — but the module is the middle of a pipeline whose other two thirds do not exist: nothing production-side calls `place_plain_text`, and the "renderer (renderer-wgpu, a later card) owns fonts and rasterization" that this module's doc names as its consumer has no font stack, no subtitles dependency, and a `subtitle_layer: Option<u64>` field that is never set to `Some` nor read anywhere in the tree. Additionally, SRT text is never wrapped (box width = longest line clamped to the frame), so long SRT lines — the normal case, since SRT has no author-side wrap — produce a clamped box that the nonexistent renderer would have to squeeze into; and the WebVTT `line:-N` default-alignment coupling produces a one-line-height vertical error by this module's own anchor math.

## 2. What actually works (verified)
— Baseline `cargo test -p subtitles` (audit/runs/baseline/cargo-test-workspace.txt:5959-5964): all 6 `srt::render::tests` pass, plus `render_path_over_canonical_cues ... ok` at :6103 (tests/srt_canonical_differential.rs places every canonical-cue on 1080p and asserts in-bounds rects). The bottom-center default math (`y = 1026-40 = 986` for a 40 px cue on 1080p), percent mapping (25 %/75 % → (440,770)), and line-number mapping (+2 rows → y=80 exactly) are all pinned by tests and re-derived by hand here — correct.

## 3. Findings
F-001 [S2]
Where: crates/subtitles/src/srt/render.rs:5-9 (cross-evidence crates/renderer-wgpu/src/params.rs:156-170)
Code:
```
//! Division of labor (doc 04 §5: "text rendering itself is a
//! renderer concern"): this module owns **geometry**, the renderer
//! (`renderer-wgpu`, a later card) owns fonts and rasterization.
//! The renderer supplies nominal metrics via [`TextMetrics`] (a
//! seam, not a font stack), and this module produces:
```
What is wrong: the named consumer never materialized. renderer-wgpu contains no font stack, no `TextMetrics` implementation, no dependency on the subtitles crate; its entire subtitle surface is `pub subtitle_layer: Option<u64>` (params.rs:159) with default `None` (params.rs:170) and zero read sites (grep `subtitle_layer` across crates/: exactly 2 hits). `TextMetrics::default()` is a made-up 40×20 constant ("a stand-in until the renderer's real font metrics arrive", render.rs:75-76) and is the only metrics source that will ever run in this tree. The geometry this module computes is therefore consumed by nothing but tests/fuzz. Same crate-level unreachability as the rest of `subtitles` (no shipped app depends on it). Confidence: HIGH — Verification: read (greps above).

F-002 [S3]
Where: crates/subtitles/src/srt/render.rs:250-261
Code:
```
    // Characters, not bytes: Unicode honesty in the box width.
    let line_count = text.split('\n').count().max(1);
    let longest = text
        .split('\n')
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0)
        .max(1);
    let width = (longest as u64)
        .saturating_mul(u64::from(metrics.char_width_px))
        .min(u64::from(extent.0))
        .max(1) as u32;
```
What is wrong: no line wrapping anywhere in the SRT path — the box is the longest raw line clamped to the frame width (test `multiline_and_long_text_boxes` asserts 500 chars → `width_px == 1920`). SRT carries no author-side wrap; VLC/every real player wraps subtitle text to the frame. Here the over-wide box is clamped, `place_cue` centers it on overflow, and the renderer (which would have to wrap or squeeze) does not exist (F-001) — so the end-to-end behavior for the most common long-line SRT case is undefined, and the only defined part (the box) is wrong-shaped. Confidence: HIGH (code) — Verification: read.

F-003 [S3]
Where: crates/subtitles/src/srt/render.rs:195-208
Code:
```
            let h = i64::from(extent.1);
            let lh = i64::from(metrics.line_height_px);
            let raw = if n >= 0 {
                n.saturating_mul(lh)
            } else {
                h.saturating_add(n.saturating_mul(lh))
            };
            let num = raw.clamp(0, h) as u32;
```
What is wrong: the anchor `v` is the line offset (negative = from the bottom), but WHICH cue edge binds to it is whatever `positioning.alignment.vertical` happens to be. The VTT parser only sets vertical when an explicit lineAlign suffix is present (vtt/settings.rs:120-133 sets it only for `,start|,center|,end`); a bare `line:-1` — WebVTT's default form, lineAlign defaults to `start` — leaves the model default Bottom, so the cue's BOTTOM edge binds at `H-40` → box `[H-80, H-40]`, one full line above the spec-default `start` placement (top edge at the offset → box `[H-40, H]`). The parser's own test pins only the explicit `line:-1,end` form (vtt/settings.rs:247-253). One-line-height vertical error for every bare negative WebVTT line setting, if the spec reading is right. Confidence: MEDIUM — Verification: read (code math verified; W3C spec default-lineAlign comparison UNVERIFIED — no spec/browser in sandbox).

F-004 [S4]
Where: crates/subtitles/src/srt/render.rs:160-161 and 270
Code:
```
    /// The baseline row (bottom-aligned box: equals `height_px`).
    pub baseline_px: u32,
```
What is wrong: `baseline_px` is hard-wired to `height` for every cue regardless of Top/Middle vertical alignment, and the mapping never produces `LayoutVertical::Baseline` — so the compositing `Baseline` anchor mode is dead vocabulary from this path. Harmless today (nothing consumes the rect), misleading vocabulary. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `TextMetrics::new` | 63-70 | REAL | — | Plain constructor. |
| `TextMetrics::default` | 73-79 | STUB-ish | S2 (via F-001) | Made-up 40×20 constant; the only metrics source in the tree. |
| `SrtRenderError` Display/Error | 114-138 | REAL | — | Complete error impl with sources. |
| `plain_text` | 142-148 | REAL | — | One-alloc segment join; correct. |
| `cue_layout` | 172-233 | REAL | S3 | Exact percent/line math; defensive error arms documented dead (Percent validated 0..=100 upstream, model/position.rs:41-45); F-003 edge-binding coupling. |
| `bad_anchor` | 235-237 | REAL | — | Helper. |
| `text_box` | 243-272 | REAL | S3 | Char-count Unicode honesty; no wrap (F-002); clamps to extent. |
| `place_plain_text` | 282-305 | REAL | — | Bounded probe (≤16 MiB) reusing place_cue; per-call alloc claimed once-per-cue (no caller to verify). |
| 6 tests | 332-469 | REAL | — | Pixel-exact, honest; none cover wrap or bare negative line. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| plain_text_joins_segments | YES | Exact string equality. |
| default_cue_lands_bottom_center_inside_title_safe | YES | Full Rect equality incl. title-safe y=986. |
| multiline_and_long_text_boxes | YES | Exact (100,120,120) + clamp-to-1920. |
| percent_and_line_placements_map_exactly | YES | Exact rects for percent, +2 line, and -1 line (Top-aligned) cases. |
| layout_mapping_fallbacks | YES | Exact anchor num/den. |
| typed_failures | YES | ZeroExtent + ProbeTooLarge variants exercised (7.2 M px > 2^22). |
All pass in baseline (:5959-5964). Gap: no test for a bare negative line with default alignment (F-003's failure mode).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the renderer (renderer-wgpu, a later card) owns fonts and rasterization" | render.rs:6-7 | PAPER-ONLY | renderer-wgpu has no fonts, no subtitles dep; `subtitle_layer` never set/read (F-001). |
| "it is freed on return and is never per-frame (plan once per cue)" | render.rs:37-38 | UNVERIFIED | No caller exists to establish call frequency. |
| "line count → (n · line_height) / height as an exact rational (negative from the bottom, clamped)" | render.rs:22-25 | REAL | Math verified (F-003 is the edge-binding, not this formula). |

## 7. Dead / duplicated / unreachable code
Whole render path has no production consumer. `LayoutVertical::Baseline` never produced. Dead defensive arms (documented as such — acceptable).

## 8. Dependencies & flags
compositing (bitmap/layout/target), model. No unsafe, no I/O, no features. MAX_PROBE_PIXELS = 2^22 honored before allocation.

## 9. Open questions for the next agent
- vtt/ auditor: does any path emit `line:-N` WITHOUT an explicit lineAlign in real fixtures, and is the default-vertical Bottom pinned anywhere (F-003)?
- renderer-wgpu auditor: `subtitle_layer` — confirm no shader/texture path consumes it (grep found zero reads; the doc itself says "[PROPOSAL] ... Phase-5 card").
- Who was to own SRT text wrapping — renderer (nonexistent) or this module? Neither does it today.
