# AUDIT: crates/subtitles/src/font/pipeline.rs
| Field | Value |
|---|---|
| Auditor | W09-A04 |
| Date | 2026-11-21 |
| Lines / functions | 543 / 11 fns+methods+impls, 4 type defs, 9 tests |
| File verdict | UNREACHABLE (and 60% dead-in-crate) |
| Reachable from shipped entry point? | NO. Crate-level: `subtitles` is linked only by `fuzz/` and its own tests (rg over all Cargo.tomls). Crate-internal: `GlyphMetrics`/`MAX_FONT_SIZE_PX`/`MetricsError` ARE used (ass/render.rs:72 re-export; render.rs:366,478 `from_font_size`); but `GlyphPipeline`, `MonospaceCellPipeline`, `rasterize`, `overhang`, `GlyphDecorations`, `GlyphMask`, `GlyphMaskError` have ZERO production callers — ass/render.rs:403-421 and 528-556 rasterize their own inline solid blocks; cc/render.rs has its own inline cell fill. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg "MonospaceCellPipeline|GlyphPipeline|rasterize\(|GlyphDecorations|GlyphMask" crates/subtitles/src` (only self + font/mod.rs re-export) · `rg "bold|italic|underline|deco" crates/subtitles/src/ass/{tags,render}.rs` (parser tracks flags; render drops them) · `sed -n` quotes · orchrun log grep (`font::pipeline::tests` 9/9 ok at lines 5402-5410) |

## 1. One-paragraph verdict
The module is honest about being a placeholder and its math is correct (exact integer ratios, bounds-checked masks, total rasterizer, 9 pixel-exact tests that all ran and passed) — but the central promise, a glyph-pipeline SEAM that "the render paths" use, is false in this tree: no render path calls the trait; the two real rasterizers (ASS render, CC render) each hand-roll their own solid-block fill, and the decoration-aware masks this file exists to provide (bold/italic/underline/strikeout — the G-2 override tags) are never invoked, so those tags have ZERO visual effect anywhere. Only `GlyphMetrics` escaped this file into production use. Combined with the crate not being linked into any shipped binary, the entire decoration/raster half is doubly dead.

## 2. What actually works (verified)
— `GlyphMetrics::from_font_size` math verified by hand: size 20 → advance (20·3+2)/5=12, line (20·12+5)/10=24, ascent (24·8+5)/10=19 — matches test `metrics_ratios_are_the_pinned_ones`; u64 intermediates, size ≤ 8192 → max value 9830 < 2^14, comment at :109 correct; zero/oversize typed errors.
— 9/9 unit tests passed: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5402-5410 (`font::pipeline::tests::* ... ok`, incl. `rasterize_is_total_over_junk_chars` over U+0000/U+007F/é/漢/U+10FFFF/\t/U+00A0).
— `GlyphMask::from_alpha` size check in u64 (no overflow); `alpha_at` bounds-checked (OOB → 0, test at :531 pins it).
— Italic shear/strip/bold bounds traced: `put` closure indices provably in-bounds (shift ≤ shear, x < width, y < height; strike top+strip_h ≤ ascent for all ascent ≥ 1 — checked exhaustively for small ascents and by inequality in general).

## 3. Findings

### F-001 [S2]
Where: crates/subtitles/src/font/pipeline.rs:14-19 and 230-248
Code:
```
//! * [`MonospaceCellPipeline`] — the no-new-deps implementation:
//!   today's documented placeholder block glyphs, extended with
//!   DECORATION-AWARE masks (bold/italic/underline/strikeout get
//!   honest placeholder geometry — the G-2 override-tag subset needs
//!   visible, testable per-tag differences, not silent uniformity).
```
and (the seam):
```
pub trait GlyphPipeline {
    fn name(&self) -> &'static str;
    fn metrics(&self, size_px: u32) -> Result<GlyphMetrics, MetricsError>;
    fn rasterize(&self, ch: char, metrics: &GlyphMetrics, deco: &GlyphDecorations) -> GlyphMask;
    fn overhang(&self, metrics: &GlyphMetrics, deco: &GlyphDecorations) -> u32;
```
What is wrong: the seam is connected to nothing. `rg` over the whole crate finds zero non-test callers of `GlyphPipeline`, `MonospaceCellPipeline`, `rasterize`, `overhang`, `GlyphDecorations`, `GlyphMask` — the actual render paths hand-roll their own fills: ass/render.rs:403-421 ("Rasterize: one solid cell block per non-space codepoint" — inline double loop, no pipeline) and ass/render.rs:528-556 (per-run inline fill), cc/render.rs with its own cell geometry. Worse, the decoration payload the module exists for is dropped before render: `AssRenderRun` (ass/render.rs:260-267) carries only `text`/`color`/`font_size_px` — no bold/italic/underline/strikeout fields — and render.rs:240-242 states it outright: "Bold/italic/underline … **they do not change the bitmap geometry**". The parser dutifully tracks the flags (tags.rs:227-230) and splits runs on them, then the render discards them.
Why it matters: the file's own header declares the G-2 requirement "visible, testable per-tag differences, **not silent uniformity**" — and silent uniformity is exactly what every render path delivers. A user enabling `\b1`/`\i1`/`\u1`/`\s1` in an ASS cue gets bit-identical pixels to the undecorated cue. The 9 decoration tests test a component no user can reach, giving a green suite that proves nothing about the product.
What correct looks like: either wire `render_styled_cue` (ass/render.rs) through `GlyphPipeline::rasterize` with `GlyphDecorations` carried on `AssRenderRun`, or delete the decoration machinery and the trait until a real font stack lands — a seam with zero implementations-in-use is not a seam, it is dead code with documentation.
Confidence: HIGH — Verification: read + rg caller sweep (no callers).

### F-002 [S3]
Where: crates/subtitles/src/font/pipeline.rs:21-36 vs crates/subtitles/src/ass/render.rs:20-33 vs crates/subtitles/src/cc/render.rs:52
Code (pipeline.rs):
```
//! * advance `3/5` em, line box `6/5` em, ascent `80 %` of the box —
//!   the ratios `ass::render` pinned (monospace-widest-char policy).
```
What is wrong: the monospace placeholder geometry is now defined THREE times independently: pipeline.rs (`GlyphMetrics::from_font_size`), ass/render.rs (its own pinned table + inline fill), cc/render.rs ("The glyph-block (ascent) portion of the cell line box, ≈80 %", own cell math). pipeline.rs claims to be the canonical home the ratios were "moved" to, yet the other two paths do not derive from it (cc/render.rs never calls `from_font_size`; ass/render.rs only borrows `GlyphMetrics` for layout, not for its ink loops' decoration behavior).
Why it matters: three copies of the same design constants with no single source of truth — the next editor changes one, the other two drift, and the "interchangeable at the render paths" story (F-001) makes the drift invisible until pixels disagree between paths.
What correct looks like: one shared `GlyphMetrics` construction for all three sites (cc/render.rs should call `from_font_size`), or the removal of the copies that the pipeline was supposed to replace.
Confidence: HIGH — Verification: read of all three sites.

### F-003 [S4]
Where: crates/subtitles/src/font/pipeline.rs:282-298
Code:
```
        let width = u64::from(advance) + u64::from(bold_extra) + u64::from(shear);
        let height = u64::from(ascent) + u64::from(underline_rows);
        // Bounded by MAX_FONT_SIZE_PX-derived metrics (< 2^14 each):
        // no overflow.
        let mut alpha = vec![0u8; (width * height) as usize];
```
What is wrong: at the accepted maximum size (8192 px) one glyph mask allocates ≈ (4915+819+1228)×(7865+983) ≈ 61.6 MB, with no glyph cache — a hypothetical caller rendering a 40-char line at the guard would allocate ~2.5 GB transiently. Today this is unreachable dead code (F-001), so it is hygiene, not an outage.
Why it matters: the moment someone wires the seam (its stated purpose), the per-glyph `Vec` allocation becomes a hostile-`\fs8192` memory amplifier; the size guard bounds a single mask, not a line of them.
What correct looks like: a per-cue aggregate byte guard (like ass/render.rs's `MAX_BITMAP_BYTES`) or a mask cache keyed by (char, metrics, deco), documented at the trait.
Confidence: HIGH (arithmetic) — impact currently moot because unreachable.

### F-004 [S4]
Where: crates/subtitles/src/font/pipeline.rs:15-16
Code:
```
//! * [`MonospaceCellPipeline`] — the no-new-deps implementation:
//!   today's documented placeholder block glyphs, extended with
```
What is wrong: "today's documented placeholder block glyphs" implies continuity with what the render paths draw today, but today's render paths (ass/render.rs:403, cc/render.rs) draw decoration-FREE blocks — the decoration extension exists only here. The comment describes an aspiration as a present fact ("extended with DECORATION-AWARE masks" — extended relative to what the product draws: nothing).
Why it matters: doc-reality drift inside the crate's own honesty ledger; a reader concluding the ASS renderer honors `\b`/`\i` from this header would be wrong (F-001).
What correct looks like: "(unused by the render paths until G-5 wiring lands — see ass::render)".
Confidence: HIGH — Verification: read + caller sweep.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MetricsError` + `Display`/`Error` | 49-73 | REAL | — | Typed, fine. Used by ass/render.rs mapping. |
| `GlyphMetrics::from_font_size` | 96-115 | REAL | — | Correct rounded ratios; u64 intermediates; bound comment verified (9830 < 2^14). The one part of this file in production use (ass/render.rs:366,478). |
| `GlyphDecorations` + `NONE` | 120-140 | DEAD | S2 | No production reader; see F-001. |
| `GlyphMask::from_alpha` | 161-175 | DEAD (in-crate) | — | Correct u64-checked constructor; only tests call it. |
| `GlyphMask::alpha_at` | 180-186 | DEAD (in-crate) | — | Bounds-checked, OOB→0. |
| `GlyphMask::is_blank` | 190-193 | DEAD (in-crate) | — | Trivial, correct. |
| `GlyphMaskError` + impls | 196-220 | DEAD (in-crate) | — | Fine. |
| `trait GlyphPipeline` | 230-248 | DEAD | S2 | Zero implementors-in-use; F-001. |
| `MonospaceCellPipeline::{strip_height,bold_extra,shear}` | 255-271 | DEAD | — | Integer math correct (max(1,·) guards). |
| `<MonospaceCellPipeline as GlyphPipeline>::{name,metrics}` | 274-280 | DEAD | — | metrics delegates correctly. |
| `<MonospaceCellPipeline>::rasterize` | 282-336 | DEAD | S4 | Correct total rasterizer (bounds traced); 62 MB/glyph at guard (F-003); decorative only. |
| `<MonospaceCellPipeline>::overhang` | 338-347 | DEAD | — | Matches rasterize's width extension (bold+shear); no caller. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `metrics_ratios_are_the_pinned_ones` (361-398) | YES | Four concrete value triples + both error variants pinned. |
| `plain_block_and_space_geometry` (401-413) | YES | (12,19) dims, corner pixels 255, space blank, overhang 0. |
| `bold_widens_the_block` (416-431) | YES | width 14, pixel at x=13, tiny-advance ≥1 column. |
| `italic_shears_top_row_only_at_the_top` (434-455) | YES | Top shift floor(3·18/19)=2 pinned pixel-by-pixel; bottom shift 0; width 15, height unchanged. |
| `underline_sits_in_the_descent` (458-473) | YES | height 21, rows 19-20 lit. |
| `strikeout_cuts_through_the_ascent` (476-495) | YES | Rows 8-9 lit, 7/10 dark, on a SPACE (documented behavior pinned). |
| `combined_decorations_stack_geometrically` (498-515) | YES | (17,21), overhang 5, edge pixels. |
| `mask_validates_its_own_shape` (518-532) | YES | SizeMismatch typed; OOB read 0. |
| `rasterize_is_total_over_junk_chars` (535-542) | YES (weakly) | Asserts dims only, not ink — a rasterize that always returned blank would PASS this one; the other tests carry the ink assertions. |
All 9 executed: orchrun log:5402-5410, all ok. NOTE: they test a component with no production caller (F-001) — green ≠ product behavior.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "G-2 override-tag subset needs visible, testable per-tag differences, not silent uniformity" | pipeline.rs:17-19 | FALSE (as a property of the product) | Render path drops the flags: AssRenderRun (ass/render.rs:260-267) has no deco fields; render.rs:240-242 "they do not change the bitmap geometry"; zero callers of the decoration masks. |
| "the placeholder implementation and the audited real-font stack are interchangeable at the render paths" | pipeline.rs:1-5 | FALSE (no render path uses the trait) | rg: no non-test callers; ass/render.rs:403,528 inline fills. |
| "real stack (swash + skrifa per the audit record)" [PROPOSAL] | pipeline.rs:228-229 | PAPER-ONLY (proposal, labeled as such) | docs/dependencies/records/font-stack-0.2.10.md exists; no swash/skrifa in Cargo.toml (deps: media-types only) — correctly labeled [PROPOSAL]. |
| Matrix row subtitle-ass-ssa: "\\b \\i \\u \\s … -> styled runs + position override + per-run render" | docs/feature-matrix.toml:905-916 | SHALLOW | Parse-side true (tags.rs:227-230 splits runs); "per-run render" true for color/size only; bold/italic/underline/strikeout have zero render effect (F-001). |
| Matrix rows subtitle-srt/vtt/pgs/dvb/cc/vobsub status "implemented" (3 desktop platforms) | docs/feature-matrix.toml:892-1047 | UNREACHABLE | The `subtitles` crate is not a dependency of lumen-cli/apps; pipeline_host.rs:1238 discards Frame::Subtitle. "implemented" claims are unreachable in any shipped build. |

## 7. Dead / duplicated / unreachable code
DEAD (in-crate, production-orphaned): `GlyphPipeline` trait, `MonospaceCellPipeline` (all 4 trait methods + 3 helpers), `GlyphDecorations`, `GlyphMask` (3 methods), `GlyphMaskError` ≈ lines 118-348 minus `GlyphMetrics`. LIVE: `MAX_FONT_SIZE_PX`, `MetricsError`, `GlyphMetrics::from_font_size` (via ass/render.rs). Duplication: F-002.

## 8. Dependencies & flags
No new deps (module doc claim holds: Cargo.toml deps = media-types only). No feature flags. Workspace lints respected (no unwrap/expect/panic in non-test code — `put` closure indexes but bounds hold; verified by read).

## 9. Open questions for the next agent
1. ass/render.rs auditor (G-2 render path): confirm at pixel level that `\b1`/`\i1` cues rasterize identically to plain cues (F-001 makes it certain from the run struct, but a rendering test proving the uniformity would pin it).
2. Whoever audits font/mod.rs (FontStack/FontProvider, issue #17): is the FontStack seam also caller-free? If so the whole `font` module except `GlyphMetrics` is scaffolding.
3. Wave gate: decide whether the G-2 "visible per-tag differences" claim stays in the matrix notes — today it is parse-only.
