# AUDIT: crates/subtitles/src/ass/render.rs#part2 (lines 577–1148)
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-21 |
| Lines / functions | 572 (chunk 577–1148 of 1148) / 14 test fns + 1 fixture builder |
| Chunk boundary | part2 = the entire `#[cfg(test)] mod tests` (:577–1148). part1 (A09) = :1–576 production (errors, style/run/cue types, `ass_colour_to_model`, `split_lines`, `render_plain_text_cue`, `render_styled_cue`). Boundary is the only clean cut in the file. |
| File verdict | REAL (as tests — executed, pixel-exact, non-tautological) / the tested render path is UNREACHABLE and TEST-FIXTURE-DRIVEN ONLY (no non-test code constructs `AssRenderCue`) |
| Reachable from shipped entry point? | NO — crate-level: nothing depends on `subtitles` but `fuzz/Cargo.toml`. In-crate: `rg "render_plain_text_cue\|render_styled_cue\|AssRenderCue"` → hits ONLY render.rs itself (tests at :698–1145); `rg` over `crates/subtitles/tests/` and `fuzz/` → zero. The parser produces `AssDocument.cues: Vec<model::Cue>` (parser.rs:259) — no adapter exists. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -n "render_plain_text_cue\|render_styled_cue\|AssRenderCue\|AssRenderRun" crates/subtitles fuzz/` (only render.rs + tests dir negative); `rg -n "BadAlignment" crates/subtitles/src` (constructed in layout.rs:128,419 — not render.rs); Read ass/mod.rs (LEAD SEAM note :39-48), parser.rs structs (:80-109, :250-288), ass/fuzz.rs + fuzz/fuzz_targets/fuzz_sub_ass.rs (parser-only); baseline cites: `orchrun-test-workspace-nofailfast.log.txt:5269-5271` (`running 267 tests`), result line `267 passed; 0 failed`, `:5297-5310,5538` (all 14 render tests ok), `:5590-5596` (dvb_vobsub_fuzz_survival 2/2). Hand-re-derived the 9-grid expected rects (see §2). `sed -n` on every quote. |

## 1. One-paragraph verdict
These are the best tests in the crate: hand-derived pixel-exact geometry (I re-derived all nine numpad rects independently and they match), a real end-to-end render→place→blend with surface-pixel assertions, typed-error coverage for every reachable `AssRenderError` variant, and a 2048-round splitmix64 sweep with per-Ok in-bounds placement asserts — all executed green in the baseline (cited lines). But the wave's hostile question answers itself here: the entire suite drives the renderer through hand-built `AssRenderCue` fixtures because **no other code on Earth constructs that type** — the parser→render "[LEAD SEAM]" adapter promised in two module headers was never built, so the G-2 "override tags → styled runs → per-run render" story exists as two disconnected, individually-tested halves. The 14 tests prove the renderer; nothing proves (or even exercises) a subtitle reaching it from a file.

## 2. What actually works (verified)
— Baseline `orchrun-test-workspace-nofailfast.log.txt:5271` → `running 267 tests`, result `267 passed; 0 failed`; `:5297-5310, :5538` → all 14 `ass::render::tests::*` ok (names match this chunk 1:1).
— Hand re-derivation of `nine_position_grid_lands_pixel_exact` expected table (:686-696): "Hi"@20px → 24×24; PlayRes 384×288; bottom row y = 288−30(marginV)−24 = 234 ✓; middle y = (288−24)/2 = 132 ✓; top y = 30 ✓; x: 20 / 192−12=180 / 384−20−24=340 ✓. The comment's anchors (5/96 = 20/384, 91/96 = 340/384) check out. Pixel-exact claim is TRUE.
— `end_to_end_compose_draws_the_cue_pixels` (:855-895): yellow at (180,250)/(203,250), black at (179,250)/(180,269), 0 drawn at an inactive clock — the only in-crate test that exercises the actual blend (`SubtitleCompositor::compose` + `Rgba8Surface`).

## 3. Findings

### F-001 [S2]
Where: crates/subtitles/src/ass/render.rs:577-618 (chunk-wide; seam docs at :5-12 and ass/mod.rs:39-48)
Code:
```
mod tests {
    use super::{
        AssRenderCue, AssRenderError, AssRenderRun, AssRenderStyle, GlyphMetrics, MAX_BITMAP_BYTES,
```
(and the fixture builder :598-618 `fn cue(...) -> AssRenderCue { AssRenderCue { ... } }`)
What is wrong: every one of the 14 tests builds `AssRenderCue`/`AssRenderRun` by hand. Production constructor census: `rg` over the whole tree finds `AssRenderCue` constructed ONLY here. The documented plan — render.rs:7-12 "the Lead wires parser output into these types at merge (either by having the parser produce them or through an adapter)" and ass/mod.rs:43-48 "[LEAD SEAM] … the union is mechanical" — was never executed: the parser emits `AssDocument.cues: Vec<crate::model::Cue>` (parser.rs:258-259) plus `PositionOverride` (tags.rs:65), none of which convert to `AssRenderCue { runs, pos, style }`.
Why it matters: the matrix row's G-2 claim "\\b \\i \\u \\s \\fs \\fn \\c \\1c \\alpha \\1a \\pos \\move -> styled runs + position override + per-run render" (feature-matrix.toml:915) describes a pipeline whose second arrow is test-only. The tags engine's runs (A02's audit) and this renderer never meet; any real .ass file renders NOTHING through this path even if the crate were wired into an app.
What correct looks like: an `ass::document_to_render_cues(&AssDocument) -> Vec<AssRenderCue>` adapter (style resolution, run projection, pos) + one test that parses a real ASS text and renders it end-to-end.
Confidence: HIGH — Verification: read + rg census (commands in header).

### F-002 [S3]
Where: crates/subtitles/src/ass/render.rs:977-1016
Code:
```
        for _ in 0..2048 {
            let font = (next() % 70) as u32; // includes 0
            let align_raw = (next() % 12) as u8; // includes 0, 10, 11
```
(and :1007-1008 `runs: Vec::new(),` / `pos: None,`)
What is wrong: the fuzz-lite sweep exercises ONLY `render_plain_text_cue` with empty runs and no `\pos`. `render_styled_cue` — the G-2 flagship with the crate's most intricate arithmetic (per-run metrics, cross-run running x, per-line max line-height/ascent, :475-554) — and the `cue_layout_at` pos path get zero randomized inputs; the in-crate fuzz harness (`ass::fuzz::fuzz_sub_ass`, fuzz.rs:28-30) and the libFuzzer shim cover `parser::parse` only.
Why it matters: the matrix evidence string "2000+2048 fuzz-lite runs clean" (feature-matrix.toml:914) sits directly beside the G-2 per-run claims, reading as if the current feature set is fuzz-clean; the newest code has no fuzz coverage at all.
What correct looks like: extend the sweep to build 1-4 random runs (random sizes/colors/texts with `\N`) and random `pos`, driving `render_styled_cue`.
Confidence: HIGH — Verification: read (junk loop fields :1002-1016; ass/fuzz.rs:28-30 parser-only).

### F-003 [S4]
Where: crates/subtitles/src/ass/render.rs:782-787, 1054-1059, 1086-1091
Code:
```
        let transparent = Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
```
What is wrong: the same `transparent` literal is declared three times in one module (once inside a single test twice, :1054 and :1086 — shadowing within one function).
Why it matters: trivial duplication; a module-scope `const TRANSPARENT` would do.
What correct looks like: one `const` at mod scope next to `ASS_YELLOW` (:596).
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/subtitles/src/ass/render.rs:998-1001
Code:
```
            let Ok(alignment) = AssAlignment::new(align_raw) else {
                errs += 1;
                continue;
            };
```
What is wrong: `errs` conflates "fixture was invalid before the render call" (alignment 0/10/11 rejected here) with "render returned a typed Err" — the `errs > 200` arm-strength assert (:1032) is partially satisfied by inputs that never reached the function under test.
Why it matters: weakens the sweep's meaning as evidence about the render path (the assert cannot distinguish the two); a future regression that made render always return Ok would still pass if enough alignments were rejected.
What correct looks like: count pre-render fixture rejections separately (`invalid += 1`).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `cue` (fixture builder) | 598-618 | REAL | — | fixed style: 20/20/30 margins, yellow, 20px; reused by most tests |
| `colour_conversion_table` | 620-640 | REAL | — | 8-entry &HAABBGGRR table incl. inverted-alpha edges + `style_token` projection; exact asserts |
| `metrics_pin_the_documented_ratios` | 642-664 | REAL | — | pins 12/24/19, 6/12/10, 15/30/24 + zero-size and 8193>8192 guards |
| `line_splitting_honors_all_three_breaks` | 666-678 | REAL | — | `\N`, literal `\n`, U+000A, multi-byte, trailing lone backslash, empty |
| `nine_position_grid_lands_pixel_exact` | 680-726 | REAL | — | hand-derived rects independently re-derived by auditor (§2); also pins compositor alignment mapping |
| `zero_margins_still_land_inside_the_title_safe_box` | 728-745 | REAL | — | pins the documented 5%-clamp deviation (x 19, bottom edge 274) |
| `margins_and_bitmap_scale_with_the_video_extent` | 747-763 | REAL | — | 2× extent → 48×48, margins double; only integer scale pinned (see §9) |
| `multi_line_stacking_via_hard_break` | 765-817 | REAL | — | 60×72 baseline 67; pixel probes at line/descent/space-cell boundaries |
| `composed_cue_carries_identity_timing_and_z` | 819-853 | REAL | — | ns conversion, half-open boundaries at both ends, zero-duration legal, reversal typed |
| `end_to_end_compose_draws_the_cue_pixels` | 855-895 | REAL | — | the only blend-path test: real surface pixels + inactive-clock case |
| `degenerate_inputs_are_typed_never_panics` | 897-956 | REAL | — | EmptyText (3 forms) / blank-spaces render / ZeroFontSize / TooLarge with exact dims + compile-time guard pin / ZeroPlayRes / MarginExceedsPlayRes / negative time → `Cue(CueError::Time)` |
| `junk_inputs_never_panic` | 958-1033 | REAL | S3/S4 | 2048 splitmix64 rounds; in-bounds placement asserted per Ok at 3 extents; plain path only (F-002), errs conflation (F-004) |
| `styled_cue_rasters_per_run_colour_and_size` | 1037-1096 | REAL | — | mixed 20px/10px runs: width 66, per-run color/ascent pixels, `\N` inside a run, baseline 43 |
| `styled_cue_edge_inputs` | 1100-1126 | REAL | — | delegation (width equality only — see §5), spaces-only blank, per-run ZeroFontSize, empty → EmptyText |
| `pos_overrides_the_plain_layout_anchor` | 1128-1147 | REAL | — | anchor (1/2, 1/2) from `\pos(192,144)`; styled path honors pos identically |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| colour_conversion_table | YES | exact byte-order/inversion table; any channel swap breaks a row |
| metrics_pin_the_documented_ratios | YES | literal triples |
| line_splitting_honors_all_three_breaks | YES | exact vec equality incl. multi-byte and edge cases |
| nine_position_grid_lands_pixel_exact | YES | exact Rect per alignment (independently re-derived) |
| zero_margins_still_land_inside_the_title_safe_box | YES | exact clamped rect |
| margins_and_bitmap_scale_with_the_video_extent | YES | exact 2× rect |
| multi_line_stacking_via_hard_break | YES | exact dims + boundary pixels |
| composed_cue_carries_identity_timing_and_z | YES | exact ns/z values + boundary actives |
| end_to_end_compose_draws_the_cue_pixels | YES | surface pixel equality incl. negatives |
| degenerate_inputs_are_typed_never_panics | YES | variant-exact error matching |
| junk_inputs_never_panic | PARTIALLY | per-Ok in-bounds placement is strong; the oks/errs>200 asserts are weak (F-004) |
| styled_cue_rasters_per_run_colour_and_size | YES | exact pixels at run boundaries |
| styled_cue_edge_inputs | PARTIALLY | delegation proven by width equality only — z/span/layout/pixels of the delegated call are not compared (a `render_styled_cue` that delegates then corrupts non-width fields passes) |
| pos_overrides_the_plain_layout_anchor | YES | exact anchor fractions |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "9/9 alignment-grid pixel-exact" | feature-matrix.toml:914 (subtitle-ass-ssa evidence) | REAL | test :680-726; baseline `orchrun-test-workspace-nofailfast.log.txt:5306` ok; rects re-derived by auditor |
| "2000+2048 fuzz-lite runs clean" | feature-matrix.toml:914 | REAL (runs) / MISLEADING (coverage) | 2048 = :977 (plain path only — F-002); the 2000 parser-side runs never touch render |
| "G-2: … \\pos \\move -> … position override + per-run render" | feature-matrix.toml:915 (notes) | REAL-UNPROVEN (render half test-proven in isolation; parser→render dataflow ABSENT) | F-001 census; tests :1037-1147 are hand-built fixtures |
| "267/0 lib tests incl. per-tag suites" | docs/formats/sub-ass-ssa.md via matrix:914 | REAL | baseline nofailfast :5271 `running 267 tests` → `267 passed; 0 failed` |
| "the Lead wires parser output into these types at merge" | render.rs:9-10; ass/mod.rs:43-44 | FALSE (never happened) | parser emits `model::Cue` (parser.rs:259); no adapter anywhere (rg census) |

## 7. Dead / duplicated / unreachable code
The whole tested subject is production-dead twice over: crate unreachable from shipped binaries, and render entry types constructed by no production code (F-001). In-chunk duplication: F-003. `AssRenderError::BadAlignment` is never produced by render.rs itself (constructed only in layout.rs:128/:419 via `AssAlignment::new` — and the render path receives an already-valid `AssAlignment`, so via render the variant is unreachable; part1/A09 should confirm).

## 8. Dependencies & flags
Test-only deps: `testkit::Must` (dev-dependency, declared), `crate::compositing` internals, `media-types`. No features. `#[cfg(test)]` only — zero cost in release.

## 9. Open questions for the next agent
- A09 (render.rs#part1, layout.rs): is `render_plain_text_cue`'s `TooLarge { width: u32::MAX, height: u32::MAX }` sentinel (:386-389) reachable before the u64 byte-guard? (width u64 → u32 try_from fails first only when > 4 Gpx wide — needs ~5.6 M chars at 8192px; untested.) Also confirm `BadAlignment` reachability via `layout::cue_layout` given typed input.
- A05 (compose.rs): pixel-exactness of `xmap` nearest-neighbor at NON-integer scales is untested anywhere (both scaling tests use exact 2×; junk test only asserts in-bounds). A fractional-scale pixel test is owed.
- A02 (tags.rs): the runs your audit traces — record in your file that their consumer (`AssRenderRun`) is fed by no producer (F-001), so per-tag suite results never flow to a bitmap.
