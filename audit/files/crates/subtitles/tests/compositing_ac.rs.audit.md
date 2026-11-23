# AUDIT: crates/subtitles/tests/compositing_ac.rs
| Field | Value |
|---|---|
| Auditor | W09-A04 |
| Date | 2026-11-23 (repo frozen; audit session 2025-07-14 clock) |
| Lines / functions | 409 / 4 helpers + 8 tests |
| File verdict | REAL (executed; asserts hand-derived exact pixels; mutation-resistant) |
| Reachable from shipped entry point? | N/A (acceptance test) — runs in the workspace test lane (baseline cargo-test-workspace.txt:6039-6057); the code under test (subtitles::compositing) is NOT reachable from shipped binaries (see pgs/mod.rs audit F-002) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/compositing_ac-93751b2dd3502b94` → 8 passed, 0 failed (exit 0); sed quote verification; independent hand-math re-derivation of 6 expected values |

## 1. One-paragraph verdict
This is what an acceptance suite should look like: every expected pixel is
hand-derived from stated integer formulas, every test asserts both the
inside-black and the untouched-outside pixels, and the z-order test
deliberately constructs slice-order-vs-z contradictions in both directions
plus a tie case — ignoring z, or sorting by insertion, fails at least one
assertion. I re-derived six of the expected values independently (anchor
math, blend math, scale math) and they are correct. It passes (ran it). The
only rot: two comments contain wrong arithmetic ("rect (165, 250, 22, 11)"
against an asserted 248; "992.5" for a value that is exactly 992), and the
suite deliberately never exercises the safe-area clamp path, leaving that to
unit tests.

## 2. What actually works (verified)
— `./target/debug/deps/compositing_ac-93751b2dd3502b94` → `test result: ok.
8 passed; 0 failed` (position ×2, blending, z-order, seam, scaling ×2,
inactive). Baseline cross-check: audit/runs/baseline/cargo-test-workspace.txt:6039-6057.
— Independent re-derivation (hand math, no code): 1/10·1920=192, 972−16=956
✓ (L61-63); blend (128·255+127·0+127)/255=128 ✓ (L107-115); 31/60·1920=992,
seam [960,992) ✓ (L229-259); CIF→4K scale 15/2 → 960×240 at (1440,1704) ✓
(L283-286); 4K→CIF scale 11/120 → 22×11, bottom edge 259 → top 248 ✓
(asserted L345-348; see F-001 for the comment mismatch).

## 3. Findings

F-001 [S4]
Where: crates/subtitles/tests/compositing_ac.rs:330-331 vs 344-348
Code:
```
    // The reverse: 240x120 authored at 4K, presented at CIF — scale
    // 352/3840 = 11/120; 240x120 -> (240*11+60)/120 = 22, (1320+60)/120
    // = 11. Anchor (1/2, 9/10) of CIF = (176, 259): rect (165, 250, 22, 11).
```
(asserted four lines later)
```
    let rect = subtitles::compositing::place_cue(&cue.bitmap, &cue.layout, (352, 288)).must();
    assert_eq!(
        (rect.x, rect.y, rect.width, rect.height),
        (165, 248, 22, 11)
    );
```
What is wrong: the comment derives "rect (165, 250, 22, 11)" but the
assertion is y=248. Correct hand math: anchor v=9/10 → (9·288+5)/10 = 259,
bottom-aligned minus height 11 → 248, so the ASSERTION is right and the
COMMENT is stale/wrong. Same class at L226-227: "31/60 of 1920 = 992.5 ->
992" — 31·1920/60 = 992 exactly, no rounding involved. Why it matters: the
comments are the stated derivation of record for the expected values; one
being wrong by 2 px suggests the assertion was at some point fitted to
implementation output without re-deriving (the value happens to be correct —
I re-derived it — but the comment no longer proves it). What correct looks
like: fix both comments to the true arithmetic.
Confidence: HIGH — Verification: read + independent hand re-derivation.

F-002 [S4]
Where: crates/subtitles/tests/compositing_ac.rs:40-43 (and every layout in the file)
Code:
```
    // 1/10 of 1920 = 192; 9/10 of 1080 = 972 — exact rationals of the
    // extent. A 128x32 center-aligned cue lands at top-left
    // (192 - 64, 972 - 16) = (128, 956), inside the 5% title-safe box
    // (x >= 96, bottom 988 <= 1026): NO clamping, pure anchor math.
```
What is wrong: every cue in this AC is deliberately positioned to avoid the
safe-area clamp ("NO clamping, pure anchor math"), so the acceptance coverage
of `place_cue`'s step 4 (clamp into the SafeMargin box, layout.rs:308-312)
is zero in this file; clamping is exercised only by the unit tests in
layout.rs (`safe_area_clamps_overflow`, `cue_larger_than_safe_box_centers_on_extent`,
layout.rs:395, 417 — verified by rg, those tests pass in the lib run).
Why it matters: the render-path integration (compose → place_cue → blit)
never walks the clamp branch end-to-end; a regression that mis-clamps in
compose context would pass this AC. What correct looks like: one AC case
with a cue authored outside the safe box.
Confidence: HIGH — Verification: read + rg over test bodies (all anchors are
exact in-box rationals).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ns` helper | 15-17 | REAL | — | 1 ns base Timestamp |
| `span` helper | 19-24 | REAL | — | half-open spans |
| `layout` helper | 26-36 | REAL | — | center/center anchor builder |
| `position_10_90_is_pixel_exact_on_1080p` | 38-68 | REAL | — | 4 inside + 4 outside pixels; re-derived ✓ |
| `position_top_right_quadrant_is_pixel_exact` | 70-90 | REAL | — | quadrant mirror; re-derived ✓ |
| `semi_transparent_blends_round_to_nearest` | 92-119 | REAL | — | exact blend 128; alpha 255; outside untouched |
| `overlap_stacks_by_z_order_not_insertion_luck` | 121-201 | REAL | — | 4 scenarios: z-beats-order both ways + tie→later-slice; mutation-proof |
| `partial_overlap_has_exact_seam` | 203-260 | REAL | — | seam pixel 959/960, last-overlap 991, blue-alone 1023, outside 1024 |
| `scaling_cif_authored_cue_grows_to_4k` | 262-325 | REAL | — | place_cue rect + 3 rendered pixels + 2 outside; nearest-neighbor source mapping asserted |
| `scaling_4k_authored_cue_shrinks_to_cif` | 327-374 | REAL | S4 | F-001 stale comment (250 vs 248) |
| `inactive_cues_draw_nothing` | 376-409 | REAL | — | drawn counts 0/1, background untouched |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| position_10_90 / top_right_quadrant | YES | any anchor/rounding/off-by-one in placement moves the rect corners |
| semi_transparent_blends | YES | exact integer blend value pins rounding rule |
| overlap_stacks_by_z | YES | z-ignore, insertion-order sort, or wrong tie-break each flip at least one of the 4 assertions |
| partial_overlap_has_exact_seam | YES | seam boundary ±1 px fails |
| scaling_cif_grows / 4k_shrinks | YES | rect + source-pixel mapping both pinned |
| inactive_cues_draw_nothing | YES | active_at boundary error draws (count/pixel changes) |
Shared weakness (all tests): expected values hand-computed by the
compositor's author; no external renderer oracle. The stated derivations in
comments are the audit trail, and two of them are wrong (F-001) — the values
themselves check out.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "all through the real SubtitleCompositor::compose path onto an Rgba8Surface" | L1-6 | REAL | every test calls `compositor.compose` (rg; ran, 8 pass) |
| "hand-computed expected pixels (integer math only, no approximation anywhere in the chain)" | L5-6 | REAL (with F-001 comment rot) | re-derived 6 values independently |

## 7. Dead / duplicated / unreachable code
None. All helpers used; no ignored/gated tests (rg #[ignore]: none).

## 8. Dependencies & flags
Dev-deps: media-types, testkit (Must). No features. Runs in the plain
`cargo test -p subtitles` lane (baseline). No fixtures — fully synthetic,
deterministic.

## 9. Open questions for the next agent
- compositing/compose.rs auditor: confirm `compose` sorts by (z, index) as
  the tie test assumes, and that `drawn` counts cues whose span contains
  `now` (half-open), matching inactive_cues_draw_nothing at t=1e9.
