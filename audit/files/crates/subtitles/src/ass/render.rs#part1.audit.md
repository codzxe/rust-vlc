# AUDIT: crates/subtitles/src/ass/render.rs#part1 (lines 1-576)
| Field | Value |
|---|---|
| Auditor | W09-A09 |
| Date | 2026-11-21 |
| Lines / functions | 1-576 (chunk; tests at 577-1148 are A08's part2) / 19 items |
| File verdict | UNREACHABLE — real code, tests baseline-executed green, but no non-test caller reaches either render entry point |
| Reachable from shipped entry point? | NO — and not even reachable from the crate's own parser (see F-001) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | `rg "render_plain_text_cue|render_styled_cue"` (crate) → only render.rs itself+tests; `rg "AssRenderCue|AssRenderStyle|AssRenderRun"` → constructed only in render.rs `mod tests`; `rg "subtitles" **/Cargo.toml` → fuzz only; read of media-types/src/time.rs (from_parts/as_nanos), compositing/bitmap.rs (CompositedCue::new, CueError), font/pipeline.rs (GlyphMetrics, MAX_FONT_SIZE_PX=8192); `sed -n` quote verifies; baseline: orchrun-test-workspace-nofailfast.log.txt:5297-5310, 5538 (render tests ok) + :5540 (lib 267/0) |

## 1. One-paragraph verdict
The plain and styled placeholder-glyph renderers are carefully written — typed errors everywhere, u64 dimension math with u32-checked narrowing, a 64 MiB allocation guard, exact `&HAABBGGRR` colour inversion — and the arithmetic all checks out by hand. But the module is a locked room: `render_plain_text_cue`/`render_styled_cue` are called ONLY by the test module at :577+, the entry types `AssRenderCue`/`AssRenderStyle`/`AssRenderRun` are constructed ONLY in those tests, and the parser half emits a different run type (`model::cue::StyledSegment` via `ass::tags`) that nothing converts. The module doc's own promise — "the Lead wires parser output into these types at merge" — was never kept: the merge unioned `mod` declarations (ass/mod.rs:49-50) but no adapter exists. No parsed ASS document can ever be rendered by this code. (Its tests DID run green in the baseline no-fail-fast artifact — orchrun-test-workspace-nofailfast.log.txt:5297-5310 + :5538, lib 267/0 at :5540 — which makes the code REAL and the SEAM the defect, not the tests.)

## 2. What actually works (verified)
— `ass_colour_to_model(0x00FF_FFFF)` → (r,g,b,a)=(255,255,255,255) and `0x0000_FFFF` → opaque yellow — hand-computed against :305-311; layout matches ASS `&HAABBGGRR` (alpha inverted via `255 - ass_alpha`).
— `split_lines` byte-scan verified: `\N`, `\n`, and raw `\n` split at ASCII boundaries (no multi-byte cut possible — break decision happens on ASCII bytes); trailing lone `\` handled (`i+1 < bytes.len()` guard :335).
— Dimension guards: `max_chars * advance` and `lines * line_height` computed in u64 then `u32::try_from`-narrowed with typed `TooLarge` (:378-396); `w*h*4` u64 overflow needs ≥9.6e10 in-memory chars at MAX_FONT_SIZE_PX=8192 (advance≤4915, line_height≤9831) — verified unreachable in practice, noted in per-function table.
— `GlyphMetrics::from_font_size` guards (font/pipeline.rs:96-116) map onto `ZeroFontSize`/`FontSizeTooLarge` via the `From<MetricsError>` impl (:202-211) — trace verified.
— Executed in baseline: render tests green in orchrun-test-workspace-nofailfast.log.txt:5297-5310, :5538 (lib 267/0 at :5540) — with hand-built fixtures only (F-001).

## 3. Findings

### F-001 [S2]
Where: crates/subtitles/src/ass/render.rs:5-12
Code:
```
//! This module defines its own entry types ([`AssRenderCue`],
//! [`AssRenderStyle`]) STANDALONE — the parallel 5.M2a half owns the
//! parser (`ass/parser.rs`), and the Lead wires parser output into
//! these types at merge (either by having the parser produce them or
//! through an adapter; both halves' types are intentionally small and
//! structural so the union is mechanical).
```
What is wrong: The wiring never happened. The merge shipped (ass/mod.rs declares `pub mod parser;` + `pub mod layout; pub mod render;` — the [LEAD SEAM] note at mod.rs:39-48 describes exactly this union), but NO code in the repo converts parser output into these types: `AssRenderCue`/`AssRenderStyle`/`AssRenderRun` are constructed only in render.rs's `#[cfg(test)] mod tests` (:598-606, :1002-1009, :1041-1140); the parser/tags engine emits `model::cue::{CueText, StyledSegment}` + model `Positioning` (tags.rs:418-501), a disjoint vocabulary with no renderer. Both render entry points have zero non-test callers.
Why it matters: The record (docs/formats/sub-ass-ssa.md:37-46) sells G-2 as "override-tag MINIMUM now native — … per-run colour and font size rendering (`render_styled_cue`)" — a composed parse→typeset→render story. In reality the tag engine's runs dead-end in the model, and `render_styled_cue` only ever rasterizes hand-built test fixtures. An ASS file parsed by this crate can never be rendered by this crate. This is the wave's core answer: the ASS "subset" is two disconnected halves.
What correct looks like: One function `fn render_document(doc: &AssDocument, play_res) -> Result<Vec<CompositedCue>, _>` that maps model cues (incl. `CueText::Styled` segments and `PositionOverride`) onto `AssRenderCue`/`AssRenderRun`, plus a test parsing a real ASS string and rendering it.
Confidence: HIGH — Verification: read + rg census (commands above).

### F-002 [S3] — CORRECTED v2 (was S2 "zero executed evidence")
Where: crates/subtitles/src/ass/render.rs:356-575 (both entry points) + audit/runs/baseline/INDEX.md
Code:
```
pub fn render_plain_text_cue(
    cue: &AssRenderCue,
    play_res: (u32, u32),
) -> Result<CompositedCue, AssRenderError> {
```
What is wrong: The v1 draft of this finding claimed no baseline run had executed these functions. FALSE: `orchrun-test-workspace-nofailfast.log.txt` ran the full subtitles suite — this file's tests green at :5297-5310 (`nine_position_grid_lands_pixel_exact`, `end_to_end_compose_draws_the_cue_pixels`, `junk_inputs_never_panic` :5538, etc.), lib result **267/0** at :5540. The real defect is upstream of the evidence: INDEX.md's table cites only the two fail-fast runs (#3 stops at `-p desktop --test zero_alloc_transport`, #4 at `-p first-vertical --test evidence`), neither reaching `subtitles` (alphabetically later), and lists NONE of the 10 orchrun-* artifacts — despite AGENT-INSTRUCTIONS.md's "INDEX.md lists everything" contract. An auditor following the instructions lands on a false no-evidence conclusion (this one did, in five files).
Why it matters: The misleading index, not the code, is the finding — but the residual true point stands: the ONLY execution these entry points have is their own test module (F-001), so "green" proves the fixtures render, never a parsed document.
What correct looks like: INDEX.md rows for orchrun-* artifacts; --no-fail-fast in the gate; a parse→render bridge test.
Confidence: HIGH — Verification: read of all three baseline logs.

### F-003 [S3]
Where: crates/subtitles/src/ass/render.rs:541-549
Code:
```
                if ch != ' ' {
                    for y in top..top + u64::from(m.ascent_px) {
                        for px in x..x + u64::from(m.advance_px) {
                            let off = ((y * u64::from(width) + px) * 4) as usize;
```
What is wrong: In `render_styled_cue`, `top = li * line_height` uses the cue-wide MAX line height (:532), but each run's block is drawn from the TOP of the line box down its OWN ascent (`m.ascent_px`, the run's 80% of its own smaller box). Smaller-font runs therefore float to the top of the line instead of sitting on the shared baseline — every reference typesetter (libass, browsers, CSS) baseline-aligns mixed-size runs on one line, which is the normal use of `\fs` overrides. The module doc's "correct geometry (box, baseline, multi-line stacking…)" (:31-32) is contradicted for the mixed-size case it exists for.
Why it matters: Mixed-size lines (e.g. `{\fs30}Big{\fs18}small`) misplace the small run vertically vs any reference renderer; the cue-wide baseline (:526) is computed as if runs were baseline-aligned, so the anchor math and the ink disagree.
What correct looks like: `y` start per run = `top + (max_ascent - m.ascent_px)` (top-of-ascent relative to shared baseline), or baseline = `top + line_height - descent` with per-run descent.
Confidence: HIGH — Verification: read of :524-554 + font/pipeline.rs metrics ratios (ascent = 80% of OWN line_height).

### F-004 [S4]
Where: crates/subtitles/src/ass/render.rs:136-141
Code:
```
    /// A timestamp conversion failure (negative or overflowing µs).
    Time(TimeError),
    /// The internal `1/1_000_000` time base failed to construct —
    /// defensive only: the base is compile-time valid, so this variant
    /// exists to keep the no-panic rule total, not to fire.
    Timebase,
```
What is wrong: Neither doc matches the reachable behavior. `Timestamp::from_parts(x, 1, 1_000_000)` cannot fail (media-types/src/time.rs:155-157 — only `TimeBase::new` zero-component errors; constants are valid), so `Timebase` is correctly dead. But negative/overflowing µs does NOT produce `Time` either: `Timestamp` accepts negative ticks (time.rs:134, :147), and the failure fires later inside `CompositedCue::new` → `as_nanos()` → `TimeError` → `CueError::Time` → `AssRenderError::Cue` (compositing/bitmap.rs:49-50, 218-233; render.rs:445). The `Time` variant is dead code and its doc points callers at the wrong variant for the negative case.
Why it matters: A caller matching on `Time` to detect bad timestamps will never match; error-taxonomy drift.
What correct looks like: Delete `Time` or map the span-conversion failure explicitly; document that negative µs arrives as `Cue(CueError::Time(TimeError::Negative))`.
Confidence: HIGH — Verification: read of media-types time.rs + bitmap.rs call chain.

### F-005 [S4]
Where: crates/subtitles/src/ass/render.rs:384-391
Code:
```
            _ => {
                return Err(AssRenderError::TooLarge {
                    width: u32::MAX,
                    height: u32::MAX,
                });
```
What is wrong: On u32-narrowing failure the error fabricates `width: u32::MAX, height: u32::MAX` instead of reporting the actual computed u64 dimensions (which the function has at :378-379). The second `TooLarge` site (:394-395) reports real values; the first lies. Same pattern duplicated in `render_styled_cue` at :512-517.
Why it matters: Diagnostics claim a 4294967295×4294967295 bitmap; the caller cannot distinguish which dimension overflowed or how close the cue was to the guard.
What correct looks like: Add the computed u64 dims (or a saturated representation) to the variant, or a distinct `TooLargeU64 { width, height }` variant.
Confidence: HIGH — Verification: read.

### F-006 [S4]
Where: crates/subtitles/src/ass/render.rs:368-375
Code:
```
    let max_chars = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    if max_chars == 0 {
        return Err(AssRenderError::EmptyText);
```
What is wrong: A cue whose text is only spaces (e.g. `" "`) has `max_chars == 1`, passes the `EmptyText` guard, and rasterizes a fully transparent bitmap (spaces skip the fill loop :409-411) — an invisible cue that still composites. `EmptyText`'s doc says "no renderable line (empty, or only `\N`)" — space-only text is equally unrenderable but is accepted. `render_styled_cue`'s `any_char` check (:497-499) has the same hole (`!line.is_empty()` counts space-only fragments as content).
Why it matters: Inconsistent degenerate-input handling; downstream gets garbage cues instead of the typed refusal the variant exists for.
What correct looks like: Track `any_non_space` while counting and refuse on false.
Confidence: HIGH — Verification: read of both paths.

## 4. Per-function table (part1 items)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| re-export `GlyphMetrics`/`MAX_FONT_SIZE_PX` | 68-72 | REAL | — | keeps historical `ass::render::` path resolving; guard moved to font seam |
| `MAX_BITMAP_BYTES` | 74-77 | REAL | — | 64 MiB; matches record claim "64 MiB bitmap cap" |
| `AssRenderError` (enum, 12 variants) | 83-142 | REAL | S4 | 12 variants matches record; `Time` dead / doc-drifted (F-004); `Timebase` honestly documented as unreachable |
| `Display for AssRenderError` | 144-180 | REAL | — | every variant covered; µs/px units in messages |
| `impl Error` | 182 | REAL | — | trivial |
| `From<CueError>` / `From<ModelError>` / `From<TimeError>` | 184-200 | REAL | S4 | `From<TimeError>` has no producer (F-004) |
| `From<MetricsError>` | 202-211 | REAL | — | variant-for-variant, verified vs font/pipeline.rs:50-60 |
| `AssRenderStyle` | 216-227 | REAL | — | production-dead entry type (F-001) |
| `AssRenderStyle::ASS_DEFAULT` | 232-237 | REAL | — | bottom-center, 10/10/10, `&H00FFFFFF`, 20 px — matches ASS Default template |
| `AssRenderStyle::style_token` | 243-251 | REAL | — | colour-only projection; b/i/u documented as parser's concern |
| `AssRenderRun` | 259-267 | REAL | — | constructed only in tests (F-001) |
| `AssRenderCue` | 271-295 | REAL | — | constructed only in tests (F-001); `pos`/`runs` fields exist solely for the G-2 story |
| `ass_colour_to_model` | 304-311 | REAL | — | byte order + alpha inversion verified by hand; const fn |
| `split_lines` | 322-345 | REAL | — | ASCII-boundary-safe; handles `\N`/`\n`/`\n`; lone trailing `\` safe |
| `render_plain_text_cue` | 356-446 | REAL-DEAD | S2/S4 | zero non-test callers (F-001); baseline-executed green via its own tests (orchrun :5297-5310) — fixtures only; fabricated TooLarge dims (F-005); space-only hole (F-006); u64 `w*h*4` overflow needs ≥9.6e10 chars — practically unreachable (MAX_FONT_SIZE_PX=8192 bounds advance·line_height ≈ 4.8e7) |
| `render_styled_cue` | 461-575 | REAL-DEAD | S2/S3 | delegates on empty runs (:465-466) — the only cross-entry call in the crate; baseline-executed green via own tests (orchrun :5308-5309, :5538); mixed-size top-alignment defect (F-003); same TooLarge/EmptyText holes (F-005/F-006); x-cursor + widths accounting verified consistent (:491-505 vs :533-553) |

## 5. Test honesty
No tests in part1 (tests are part2, A08). Note for A08: the test module is the ONLY caller of everything in this part — check each test's fixture provenance; from the grep census every `AssRenderCue` is hand-built, none derived from `parser::parse` output.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "per-run colour + font size rendering (`render_styled_cue`)" as part of native G-2 | docs/formats/sub-ass-ssa.md:42-46 | FALSE (as a composed path) | render_styled_cue exists but no parsed document reaches it — tag engine emits `CueText::Styled`/model types, never `AssRenderRun` (F-001) |
| "e2e render→place→blend draws expected pixels on a real surface" | docs/formats/sub-ass-ssa.md:73-74 | SHALLOW | "e2e" starts from a hand-built cue, not a parsed ASS file; the parse half is never in the loop (F-001) |
| "placeholder block glyphs … advance 3/5 em, box 6/5 em, ascent 80% — all integer ratios" | docs/formats/sub-ass-ssa.md:54-56 | REAL | font/pipeline.rs:106-108 exactly `(3s+2)/5`, `(12s+5)/10`, `(lh*8+5)/10` |
| "64 MiB bitmap cap; typed `AssRenderError` (12 variants)" | docs/formats/sub-ass-ssa.md:56-57 | REAL | :77, :83-142 (12 counted) |
| "G-2: 267/0 lib tests incl. per-tag suites (1bd5519)" | docs/feature-matrix.toml:914 | REAL | orchrun-test-workspace-nofailfast.log.txt:5540: `267 passed; 0 failed; 0 ignored` |
| matrix `subtitle-ass-ssa` = partial ×3 desktop platforms | docs/feature-matrix.toml:913 | UNREACHABLE | crate not linked into any shipped binary (fuzz/Cargo.toml is the only reverse-dep) |

## 7. Dead / duplicated / unreachable code
- `AssRenderError::Time` variant: no producer (F-004).
- `render_plain_text_cue` / `render_styled_cue` / all three entry structs: unreachable outside the test module (F-001) — the entire part1 API surface.
- Duplication: the dimension-guard + rasterize + layout + timestamp tail is near-identical in both render functions (:377-445 vs :505-575) — ~70 lines duplicated with drift risk (F-003/F-005 exist in both).

## 8. Dependencies & flags
- `media-types::time` (Timestamp/TimeSpan/Stamp) — the crate's one real dependency use in this file; `crate::font` seam for metrics. No feature flags. No unsafe. No unwrap/expect/index-out-of-bounds on reachable paths (fill-loop bounds verified: ascent ≤ line_height ≤ height, x-cursor ≤ width).

## 9. Open questions for the next agent
- A08 (part2): does ANY part2 test parse a real ASS document and render it (proving the seam wrong)? My census says no — if you find one, downgrade F-001.
- Doc auditor: escalate F-001 against sub-ass-ssa.md's "Working subset" composition claim — candidate S0 for the record/matrix, not for this file (the module's own doc is honest about the seam).
