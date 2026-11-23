# AUDIT: crates/subtitles/src/font/pipeline.rs
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-23 |
| Lines / functions | 543 / 22 (13 prod items, 9 tests) |
| File verdict | STUB — honest placeholder; ZERO font loading; seam has ZERO production consumers |
| Reachable from shipped entry point? | NO — twice over: crate not a dep of any app (only fuzz/Cargo.toml:50), and no in-crate production code calls the trait |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 1 |
| Commands run | rg for swash/skrifa/ab_glyph/font-kit/rusttype in all Cargo.toml+rs -> only pipeline.rs:228 comment (exit 0); rg `MonospaceCellPipeline|GlyphPipeline` across crates -> only font/mod.rs re-export + pipeline.rs self-tests (exit 0); prebuilt libtest `./target/debug/deps/subtitles-70f89de07410626f font::pipeline` -> 9 passed / 0 failed (exit 0); sed quote verification |

## 1. One-paragraph verdict
There are no fonts in the font pipeline. `MonospaceCellPipeline::rasterize` uses the character `ch` for exactly one thing — `ch == ' '` (line 285); every other codepoint becomes an identical solid rectangle `advance × ascent`, tinted by the caller. No TTF/OTF/WOFF parsing, no system-font enumeration, no embedded face, no I/O of any kind — the real stack (`swash` + `skrifa`) exists only as `[PROPOSAL — Lead approval pending]` in docs/dependencies/records/font-stack-0.2.10.md ("Nothing in this record is admitted: no Cargo.toml row"), confirmed by rg: zero font crates in any manifest. The placeholder's integer geometry (0.6 em advance, 1.2 em line box, faux-bold widening, oblique shear, strike/underline strips) is real, exact, and tested (9/9 pass) — but the "seam" this file exists for is decorative: NO production code path calls `GlyphPipeline::rasterize/overhang`; `ass::render` rasterizes its own inline solid blocks (render.rs:403, 528) while claiming to be "a CONSUMER of the seam".

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-70f89de07410626f font::pipeline` -> `test result: ok. 9 passed; 0 failed` (exit 0). The metrics ratios (3/5, 6/5, 80%), zero/oversize guards (MAX_FONT_SIZE_PX 8192), bold +2px, italic shear 3 with per-row shift floor(3·(18−y)/19), strikeout rows 8-9, underline rows 19-20, combined 17×21/overhang 5, `GlyphMask` size validation and OOB-as-0 reads all execute and match hand-computed values.

## 3. Findings
F-001 [S2]
Where: crates/subtitles/src/font/pipeline.rs:282-315 (whole impl); docs/dependencies/records/font-stack-0.2.10.md:1-16
Code:
```
    fn rasterize(&self, ch: char, metrics: &GlyphMetrics, deco: &GlyphDecorations) -> GlyphMask {
        let advance = metrics.advance_px;
        let ascent = metrics.ascent_px;
        let is_space = ch == ' ';
```
What is wrong: the entire "font pipeline" loads no fonts. Formats accepted: none. System fonts: not consulted. Embedded: none in this crate (the only repo font is ui/src/assets/fonts/nunito-latin-var.woff2 per font/mod.rs:49-51, a WOFF2 the seam explicitly refuses). `ch` is discarded except the space check, so 'A', 'é' and '漢' rasterize to the same block. The record is honest ("Nothing in this record is admitted"), so this is a disclosed placeholder — but the feature-matrix subtitle rows rest on it: rendered ASS text is unreadable colored rectangles. Why it matters: any claim of subtitle "rendering" (subtitle-ass-ssa status `partial`, evidence "9/9 alignment-grid pixel-exact") proves block geometry, not typography. What correct looks like: admit the font stack or mark the render rows placeholder in the matrix. Confidence: HIGH — Verification: ran (rg + test run) + read record.
F-002 [S2]
Where: crates/subtitles/src/font/pipeline.rs:1-5 vs crates/subtitles/src/ass/render.rs:63-72,403,528
Code:
```
//! The glyph pipeline seam (issue #17, G-5): span metrics + per-glyph
//! rasterization behind one trait, so the placeholder implementation
//! and the audited real-font stack ([PROPOSAL] — see
//! `docs/dependencies/records/font-stack-0.2.10.md`) are
//! interchangeable at the render paths.
```
vs render.rs:69-72:
```
// (issue #17 G-5, `crate::font::pipeline`) — re-exported here so the
// historical `ass::render::GlyphMetrics` path (and its tests) keep
// resolving; the render path is now a CONSUMER of the seam.
pub use crate::font::{GlyphMetrics, MAX_FONT_SIZE_PX};
```
and render.rs:403: `// Rasterize: one solid cell block per non-space codepoint.`
What is wrong: the "interchangeable at the render paths" claim is false. rg proves the trait, `MonospaceCellPipeline`, `GlyphMask`, `GlyphDecorations`, and `overhang` are referenced ONLY by pipeline.rs's own `#[cfg(test)]` module and re-export lists — no production caller exists. `ass::render` imports exactly one item (`crate::font::MetricsError`, render.rs:63) and re-exports two data types; it rasterizes its own solid blocks inline (render.rs:403 and 528). Swapping in a real font stack behind this trait would change nothing anywhere. Why it matters: the G-5 "seam" deliverable is dead code; the future integration will have to rewrite render.rs anyway, contradicting the record's "admission is a behind-the-seam swap" premise. What correct looks like: render.rs routes per-glyph work through `dyn GlyphPipeline`/generic. Confidence: HIGH — Verification: ran (rg over all crates).
F-003 [S3]
Where: crates/subtitles/src/font/pipeline.rs:240-241, 316-323
Code:
```
    /// Rasterize one character under `deco` — total, never panics.
    fn rasterize(&self, ch: char, metrics: &GlyphMetrics, deco: &GlyphDecorations) -> GlyphMask;
```
```
        // Strikeout: strip centred on ascent/2, full mask width.
        if deco.strikeout {
            let top = u64::from(ascent / 2) - u64::from(strip_h / 2);
            for y in top..top + u64::from(strip_h) {
                for x in 0..width {
                    put(&mut alpha, x, y);
```
What is wrong: totality is guaranteed only for metrics produced by `GlyphMetrics::from_font_size` (min ascent 1). `GlyphMetrics` has public fields and no validation; a caller-built `GlyphMetrics { advance_px: 5, line_height_px: 0, ascent_px: 0 }` with `strikeout: true` yields `width=5, height=0, alpha=[]` and the strikeout loop calls `put(x, 0)` -> `alpha[x]` -> index-out-of-bounds PANIC, violating the documented "never panics" contract. Not reachable in-tree (no production caller at all, per F-002). Confidence: HIGH — Verification: read (arithmetic traced; not executed — no harness writable outside audit/, cargo-build-budget). What correct looks like: debug_assert/validate metrics at trait entry or make fields private.
F-004 [S4]
Where: crates/subtitles/src/font/pipeline.rs:469-472
Code:
```
        // The strip is BELOW the glyph block top area (row 0 is the
        // block itself for a solid placeholder — the distinguishing
        // geometry is the descent rows existing at all).
```
What is wrong: the test's own comment concedes the underline assertion is nearly tautological against the plain block (the only distinguishing check is height 21 and rows 19-20 being set — which any descent-area bug preserving those rows would pass). Weak but not zero. Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MAX_FONT_SIZE_PX | 45 | REAL | — | 8192 guard |
| MetricsError + Display + Error | 49-73 | REAL | — | typed, no panics |
| GlyphMetrics::from_font_size | 90-116 | REAL | — | u64 math, ties-up rounding; bounded < 2^14 (comment verified) |
| GlyphDecorations + NONE | 120-140 | REAL | — | data |
| GlyphMask::from_alpha | 155-175 | REAL | — | u64 len check, no overflow |
| GlyphMask::alpha_at | 177-186 | REAL | — | OOB=0, no panic |
| GlyphMask::is_blank | 188-193 | REAL | — | |
| GlyphMaskError + Display | 195-220 | REAL | — | single variant |
| trait GlyphPipeline | 230-248 | UNREACHABLE | S2 | zero production implementors-called/consumers (F-002) |
| MonospaceCellPipeline helpers | 255-271 | REAL | — | strip_height/bold_extra/shear exact ratios |
| impl::name/metrics | 273-280 | REAL | — | delegates to from_font_size |
| impl::rasterize | 282-336 | STUB | S2/S3 | block placeholder, `ch` ignored except space (F-001); degenerate-metrics panic hole (F-003); in-range math verified |
| impl::overhang | 338-347 | REAL | — | bold+shear only; never called in production |
| tests (9) | 350-543 | REAL | S4 | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| metrics_ratios_are_the_pinned_ones | YES | 4 exact tables + both error arms |
| plain_block_and_space_geometry | YES | exact 12×19, corner coverage, blank space |
| bold_widens_the_block | YES | width 14, col 13 set, overhang 2, tiny-advance min 1 |
| italic_shears_top_row_only_at_the_top | YES | exact shift 2 at top, 0 at bottom, col 12 empty |
| underline_sits_in_the_descent | WEAKLY | F-004: near-tautological vs plain block |
| strikeout_cuts_through_the_ascent | YES | rows 8/9 set, 7/10 clear, on a space |
| combined_decorations_stack_geometrically | YES | 17×21, overhang 5, boundary cols |
| mask_validates_its_own_shape | YES | SizeMismatch exact, OOB=0 |
| rasterize_is_total_over_junk_chars | PARTIAL | junk chars only with NONE decorations — the F-003 panic path (degenerate metrics + strikeout) is untested |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "interchangeable at the render paths" | pipeline.rs:1-5 | FALSE | F-002: no render path consumes the trait (rg) |
| "the render path is now a CONSUMER of the seam" | ass/render.rs:71 | FALSE | render.rs:63 imports only MetricsError; rasterizes inline at 403/528 |
| "the rasterizer is total over `char`" / "total, never panics" | pipeline.rs:39, 241 | SHALLOW | total over char with valid metrics; degenerate hand-built metrics panic (F-003) |
| swash+skrifa are the audited real stack | pipeline.rs:228-229 | PAPER-ONLY | record: "Nothing in this record is admitted"; rg: no manifest row |
| "admission is a behind-the-seam swap" | font-stack record §R0 context | FALSE | seam uncalled (F-002): a swap changes nothing |
| subtitle-ass-ssa `partial` + "9/9 alignment-grid pixel-exact" | feature-matrix.toml:911,914 | SHALLOW | pixel-exact against block placeholders (F-001), test run above |

## 7. Dead / duplicated / unreachable code
Everything: whole file unreachable from shipped builds (crate dep of nothing, see tags.rs F-001); within the crate, trait+impl+GlyphMask+overhang dead except self-tests (F-002). Placeholder rasterization DUPLICATED: pipeline.rs blocks vs ass/render.rs:403,528 inline blocks (two solid-block rasterizers, slightly different feature sets — render's lacks decoration geometry).

## 8. Dependencies & flags
No new deps by design (Cargo.toml [dependencies]: media-types only; dev: testkit, media-clock). Note: Cargo.toml:29-31 comment "No dev-dependencies either" contradicts the actual [dev-dependencies] section — hygiene debt in a file owned by another agent. No features, no unsafe.

## 9. Open questions for the next agent
- Who owns the G-5 swap plan — if render.rs never calls the trait, the Lead's "behind-the-seam swap" premise needs a rewrite card.
- font/mod.rs `FontStack::empty` claim ("the stack the render paths use") — verify whether ANY render path consults FontStack (audit owner of font/mod.rs).
- If a real face is ever admitted, F-003's degenerate-metrics hole becomes reachable from parsed `\fs` values — fix before admission.
