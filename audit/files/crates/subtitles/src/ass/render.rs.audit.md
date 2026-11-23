# AUDIT: crates/subtitles/src/ass/render.rs
| Field | Value |
|---|---|
| Auditor | W09-A03 |
| Date | 2026-11-15 |
| Lines / functions | 1148 / 9 fns + 5 From impls + 14 tests |
| File verdict | UNREACHABLE (code itself is REAL: executes, produces real RGBA pixels — but NO shipped host compiles this crate) |
| Reachable from shipped entry point? | NO — `subtitles` is a dependency of `fuzz/` only; apps/cli, apps/desktop, apps/desktop-tauri do not list it (verified) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | prebuilt binary `target/debug/deps/subtitles-70f89de07410626f ass::render` -> 14 passed / 0 failed (exit 0); python3 arithmetic repro (see F-001); sed quote verification |

## 1. One-paragraph verdict
Hostile-focus answer: this is NOT a glyph rasterizer and has NO font dependency — every
non-space codepoint is filled as a SOLID CELL BLOCK (monospace-cell metrics from
`crate::font::GlyphMetrics`; the "font stack" is a seam, `font::pipeline::MonospaceCellPipeline`
rasterizes the same blocks with faux-bold/underline strips — and render.rs does not even use that
pipeline, it fills inline). The module doc admits this ("placeholder glyph coverage"), so it is not
fraud at the file level — geometry, colour, placement, timing are real and test-verified end to end
onto a real surface. The bigger problem: the whole `subtitles` crate is an island — nothing in the
shipped release lane (lumen-cli, desktop) depends on it; only `fuzz/Cargo.toml:50` does — so this
"render path" renders for no user, ever. On top of that sit one real arithmetic defect (unchecked
u64 multiply in the byte guard) and the silent end-to-end drop of `\b`/`\i`/`\u` tags.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-70f89de07410626f ass::render` ->
`test result: ok. 14 passed; 0 failed` — includes `end_to_end_compose_draws_the_cue_pixels`,
which renders -> places -> blends yellow pixels onto a 384x288 surface and asserts exact
pixel positions, and `junk_inputs_never_panic` (2048 splitmix64 junk cues, both arms exercised).
Pixels ARE written (blocky, but real RGBA layout: ascent fill, descent gap, space cells blank).

## 3. Findings
F-001 [S2]
Where: crates/subtitles/src/ass/render.rs:393-396 and :520-523
Code: (verbatim, both sites identical)
```
    let total_bytes = u64::from(width) * u64::from(height) * 4;
    if total_bytes > MAX_BITMAP_BYTES {
        return Err(AssRenderError::TooLarge { width, height });
    }
```
What is wrong: the multiply is unchecked. width/height are attacker-influenced
(`max_chars * advance_px`, `lines * line_height_px`) and both fit u32 while their product*4
exceeds u64. Verified by python3: font 8192 (advance 4915, line_height 9830 per
font/pipeline.rs:106-108), one line of 873,000 chars + 436,000 `\N` breaks (~1.7 MB text)
gives width=4,290,795,000 and height=4,285,880,000 — both pass the u32 `try_from` guard at
:381-392, and true bytes = 7.36e19 ≥ 2^64. In a debug/overflow-checks build (cargo test, the
in-tree fuzz harness) line 393 panics ("attempt to multiply with overflow") — directly
contradicting the module claim at :55-59: "Every failure is [`AssRenderError`] (MP §8) ... and
bitmaps past the [`MAX_BITMAP_BYTES`] guard (which also keeps the allocation bounded — no
capacity-overflow abort)". In release the value wraps; if a wrapped value < 64 MiB occurs the
guard passes, `vec![0u8; total_bytes as usize]` under-allocates and the fill loop at :415
(`rgba[off]`) writes out of bounds -> panic. Why it matters: subtitle text is untrusted file
input; the API's core promise is typed refusals, never panics. What correct looks like:
`width.checked_mul(height).and_then(|v| v.checked_mul(4))` -> `TooLarge`.
Confidence: HIGH that the multiply overflows with ~1.7 MB valid input (arithmetic reproduced);
release-mode guard-bypass (small wrapped residue) UNVERIFIED — Diophantine search over all
fonts with 2-adic order ≥ 8 found no sub-64MiB wrap; shipped-release trigger additionally
blocked by F-reachability (crate compiled by no host).
Verification: ran (python3 repro; prebuilt tests do not cover >40-char texts at font >70, so
the suite cannot catch this).

F-002 [S3]
Where: crates/subtitles/src/ass/render.rs:259-267 (run type) vs ass/mod.rs:4-8
Code:
```
pub struct AssRenderRun {
    /// The run's text (may span `\N` line breaks).
    pub text: String,
    /// Resolved primary colour (style primary ⊕ `\c`/`\1c`/`\alpha`).
    pub color: ModelColor,
    /// Resolved font size in script (PlayRes) px (style size ⊕ `\fs`).
    pub font_size_px: u32,
}
```
What is wrong: `ass/mod.rs` advertises "interprets the G-2 override-tag minimum natively
(issue #17: `\b \i \u \s ...` → styled runs", but the run type carries only colour + size —
bold/italic/underline/strike have no field, so they are no-ops end to end. render.rs:240-242
pushes the blame upstream ("Bold/italic/underline ... the parser's projection, not this
path's"), yet no projection target exists; `font::pipeline` implements decoration geometry
(`MonospaceCellPipeline::rasterize`, pipeline.rs:282-298) that this render path never calls.
Why it matters: tag support is claimed (mod docs) and built (pipeline) but disconnected.
What correct looks like: `AssRenderRun` carries a decoration bitset consumed by the fill loop
(or render.rs calls the pipeline). Confidence: HIGH — Verification: read + rg (no consumer of
GlyphDecorations outside font/pipeline.rs itself).

F-003 [S3]
Where: crates/subtitles/src/ass/render.rs:377-446 vs :507-575
Code: (both blocks restate the same tail)
```
    let cue_layout = match cue.pos {
        Some(pos) => layout::cue_layout_at(cue.style.alignment, play_res, pos)?,
        None => layout::cue_layout(cue.style.alignment, cue.style.margins, play_res)?,
    };
```
What is wrong: ~110 lines of the two public render fns are copy-paste (u32 guard, byte guard,
baseline rule, layout match, Timestamp::from_parts + map_err(Timebase) x2, CompositedCue::new)
— the exact drift class F-001 lives in twice. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/subtitles/src/ass/render.rs:385-390 (and :512-517)
Code:
```
                return Err(AssRenderError::TooLarge {
                    width: u32::MAX,
                    height: u32::MAX,
                });
```
What is wrong: on u32-overflow the error reports fabricated dimensions (u32::MAX) instead of
the real u64 ones; Display then claims a `4294967295x4294967295 px` bitmap. Misleading
diagnostics. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/subtitles/src/ass/render.rs:495-496, :506, :432-435
Code:
```
            widths[li] += u64::try_from(line.chars().count()).unwrap_or(u64::MAX)
                * u64::from(metrics[ri].advance_px);
```
(+:506 `u64::try_from(n_lines).unwrap_or(u64::MAX)`) — the fallback multiplies u64::MAX by
advance/line_height, itself an overflow-panic if ever taken; the arm is dead on all targets
(usize→u64 is infallible) so this is self-defeating defensive code. Same class: `Timebase`
(:432-435, :561-564) maps a `Timestamp::from_parts` failure that cannot happen
(media-types/src/time.rs:155-157 only fails via TimeBase::new on an invalid base). Confidence:
HIGH — Verification: read + media-types time.rs:155-157.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Display for AssRenderError | 144-180 | REAL | — | all 12 variants rendered; matches variants |
| impl Error + 4 From impls | 182-211 | REAL | — | MetricsError mapping tested (metrics_pin test) |
| AssRenderStyle::style_token | 243-252 | REAL | — | colour-only projection; see F-002 |
| ass_colour_to_model | 304-311 | REAL | — | BBGGRR + inverted alpha; 8-entry table test passes |
| split_lines | 322-323 | REAL | — | byte-index slicing safe (breaks are ASCII); treats soft `\n` as hard (documented deviation) |
| render_plain_text_cue | 356-446 | REAL (with F-001) | S2 | solid-block fill, guards, layout, ns span; end-to-end test verified |
| render_styled_cue | 461-575 | REAL (with F-001/F-003) | S2/S3 | per-run colour/size; delegates when runs empty; decorations dropped (F-002) |
| tests (14) | 577-1148 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| colour_conversion_table | YES | 8-row byte-order table incl. alpha inversion |
| metrics_pin_the_documented_ratios | YES | pins 12/24/19 etc. + 0/8193 guard errors |
| line_splitting_honors_all_three_breaks | YES | all 3 break forms + trailing-backslash |
| nine_position_grid_lands_pixel_exact | YES | 9 hand-derived rects through place_cue |
| zero_margins_still_land_inside_the_title_safe_box | YES | pins the 5% clamp behaviour |
| margins_and_bitmap_scale_with_the_video_extent | YES | 2x extent scaling pinned |
| multi_line_stacking_via_hard_break | YES | pixel-level ascent/descent/baseline asserts |
| composed_cue_carries_identity_timing_and_z | YES | half-open boundary asserts at ns edges |
| end_to_end_compose_draws_the_cue_pixels | YES | real surface blend, exact pixels + inactive-clock arm |
| degenerate_inputs_are_typed_never_panics | PARTIAL | covers empty/zero/8192-px/300-char guard, zero PlayRes, margin overflow, negative time — but NOT the u64 multiply overflow (needs ~1.7MB text) |
| junk_inputs_never_panic | NO (for F-001) | text ≤ 40 chars, font ≤ 70: never approaches the overflow region; otherwise good sweep (aligns 0-11, zero play_res, reversals, both arms >200) |
| styled_cue_rasters_per_run_colour_and_size | YES | per-run colour/size/geometry pinned |
| styled_cue_edge_inputs | YES | delegation + spaces-only + zero size + empty |
| pos_overrides_the_plain_layout_anchor | YES | anchor fractions asserted |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "placeholder glyph coverage ... deliberately crude pixels" | render.rs:31-35 module doc | REAL | fill loop :406-423 writes solid blocks; no font dep in Cargo.toml (media-types only) |
| "Every failure is AssRenderError ... no input panics" | render.rs:55-59 | FALSE (for large valid input in debug; release bypass UNVERIFIED) | F-001 python3 repro |
| "status = partial (plain-text parse + placeholder-glyph render)" | docs/formats/sub-ass-ssa.md:12 | REAL | matches code |
| "renders per-run colour and font size (render_styled_cue)" | docs/formats/sub-ass-ssa.md:47-48 | REAL | test executed, passed |
| "keeps the allocation bounded — no capacity-overflow abort" | render.rs:57-59 | PARTLY FALSE | bound holds only while the u64 multiply doesn't wrap (F-001) |

## 7. Dead / duplicated / unreachable code
- THE FILE IS UNREACHABLE from shipped hosts: `rg 'subtitles'` over all Cargo.tomls -> only
  `fuzz/Cargo.toml:50` depends on it; apps/cli, apps/desktop, apps/desktop-tauri manifests
  have zero references (verified by Grep over apps/). The shipped release lane
  (`cargo build --release -p lumen-cli -p desktop`) never compiles this code.
- F-003 duplication (two render fns share ~110 lines).
- Dead defensive arms: F-005 (unwrap_or(u64::MAX), Timebase).

## 8. Dependencies & flags
Crate deps: `media-types` only (std-only claim holds); dev: testkit, media-clock. No feature
flags gate this file. No unsafe.

## 9. Open questions for the next agent
- Does ass/parser.rs cap event text length? If not, F-001's ~1.7MB input flows parser ->
  render unmodified once any host wires the crate (escalation path to S1).
- Does anything validate `cue.pos` against play_res before `cue_layout_at` (layout.rs is
  another agent's file)?
- Who is ever supposed to call `render_styled_cue`? The Lead "seam" (mod.rs:39-48) has not
  landed: parser.rs exists but nothing in-tree constructs AssRenderCue outside this file's
  tests (rg verified: zero external callers).
