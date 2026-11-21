# AUDIT: crates/codecs/src/video/mjpeg/color.rs#part2
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-21 |
| Chunk boundary audited | **lines 481-861** — the entire `#[cfg(test)] mod tests`. Part1 (W06-A01) = lines 1-480 (production: constants, ycc_to_rgb, fast path, RowSampler, compose fns, RgbComposer, sample_up) — already audited there as `color.rs#part1 (lines 1–480)` |
| Lines / functions | 380 / 14 (10 tests + plane helper + compose_row_reference oracle + Rng) |
| File verdict (this part) | REAL (test module; executed) |
| Reachable from shipped entry point? | The module under test is YES-reachable (`MjpegDecoder` = the first-vertical shipped lane: pipeline_host.rs:242-246; baseline AVI 60/60 frames decoded — mp4-smoke-bench-avi-mjpeg-pcm.txt). The tests themselves run under `cargo test` — executed in baseline |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg mjpeg::color in baseline logs (both runs: 10/10 ok, :666-675); hand-recomputation of the BT.601 goldens (see §5); read of mjpeg_field_differential.rs header (external-oracle coverage of the sampling geometries); sed -n quote checks |

## 1. One-paragraph verdict
This is the strongest test module I have audited in wave 6, and it is executed:
all 10 tests pass in baseline (test-workspace.log.txt:666-675). The two
equivalence sweeps are genuinely exhaustive where it matters — every (Cb,Cr)
pair crossed with all three channels' clamp-boundary Y values for the fast
path, and a 4-geometry × 6-width × 5-row randomized hot-vs-cold sweep for the
row-hoisted composer — and the value-level goldens are hand-computed and
correct (I re-derived them independently: 90/110/200 → 191,45,58 and
200/90/240 → 255,133,133 both check out under BT.601 full-range). The hostile
tests pin the defensive contract (geometry that outruns data reads neutral
128; short output rows write a prefix without panic). Two S4 gaps: vertical
subsampling has no hand-computed golden in this module (v-direction truth
rests on the hot-vs-cold oracle, which shares `sample_up` with the cold path —
the real safety net is the end-to-end ffmpeg differential in
mjpeg_field_differential.rs), and the width=0 edge is untested.

## 2. What actually works (verified)
— Baseline execution: `test-workspace.log.txt:666-675` — all 10
`video::mjpeg::color::tests::*` ok (and again in the no-fail-fast repair run
:629-638).
— Goldens re-derived by hand (mine, not the file's): BT.601 full-range
R = Y + 1.402·(Cr−128), G = Y − 0.344136·(Cb−128) − 0.714136·(Cr−128),
B = Y + 1.772·(Cb−128):
  (90,110,200) → (190.94, 44.78, 58.10) → (191, 45, 58) ✓ matches :562;
  (30,220,120) → (18.78, 4.05, 193.02) → (19, 4, 193) ✓ matches :566;
  (255,255,255) → (clamp, 120.6→121, clamp) ✓ matches :580;
  (200,90,240) → (357→255, 133.1→133, 132.7→133) ✓ matches :654.
— Mutation reasoning (no edits made): breaking `ycc_to_rgb`'s rounding or
clamping fails known_triples/out_of_gamut/neutral_chroma; breaking the
RowSampler stepping fails compose_row_matches_the_per_pixel_reference (byte
equality vs an independent per-pixel loop); breaking bounds safety fails the
two hostile tests by panic. Every test in the module has a failure mode.

## 3. Findings

### F-001 [S4] Vertical subsampling has no value-level golden in this module; the unit oracle shares `sample_up` with the cold path
Where: color.rs:664-694 (oracle) and 697-784 (the sweep)
Code:
```rust
    /// The old per-sample reference compose (the pre-rewrite loop),
    /// kept as the equivalence oracle for the row-hoisted stepper.
    fn compose_row_reference(
        composer: &RgbComposer<'_>,
        row_y: usize,
        width: usize,
        out: &mut [u8],
    ) {
        for x in 0..width {
            let luma = composer.sample_up(&composer.luma, &composer.geoms[0], x, row_y);
```
What is wrong: `compose_row_reference` builds its expected output with
`composer.sample_up` — the cold path of the SAME module. The equivalence sweep
therefore proves hot==cold, not that either chooses the right ROW for v_max=2.
The hand-computed goldens (tests at 557-658) all use v_max=1, so the vertical
sampling convention (which row of the subsampled plane serves output row y) is
pinned here only by the shared-primitive oracle. A same-way-wrong row index in
both paths would pass this module.
Why it matters: bounded by the repo's end-to-end net —
mjpeg_field_differential.rs decodes real 4:2:0 corpus files against ffmpeg
dumps (±1 LSB, its own header lines 9-32) and passed in baseline (:1115-1121
region, 3/3 ok), so a vertical misconception WOULD be caught one level up.
That is why this is S4 and not S3.
What correct looks like: one hand-computed two-row 4:2:0 golden (two luma
rows over one chroma row, distinct chroma values) in this module.
Confidence: HIGH — Verification: read + oracle-coupling analysis + coverage
map of the goldens.

### F-002 [S4] The width=0 edge is untested
Where: color.rs:697-784 (the sweep's width list) — `for width in [1usize, 2, 3, 5, 8, 13]`
Code:
```rust
            for width in [1usize, 2, 3, 5, 8, 13] {
```
What is wrong: `compose_row(row, 0, out)` (an empty row — what a
zero-width-plane hostile input produces) is never driven; the loops would
no-op, but nothing pins "no panic, no writes" for it.
Why it matters: the module's own charter is hostile-input safety; width=0 is
the cheapest case it did not take.
What correct looks like: one assert that a width-0 compose writes nothing and
returns.
Confidence: HIGH — Verification: read (no test constructs width 0).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| tests::plane (helper) | 485-490 | REAL | — | PlaneView literal constructor |
| branchless_round_matches_the_reference_everywhere | 493-511 | REAL | — | negative-tie sweep at 4 magnitudes + dense ±300k; executed; fails on any rounding-form divergence |
| fast_conversion_matches_the_reference_everywhere | 514-544 | REAL | — | 65536 chroma pairs × clamp-boundary Ys (~1.5M conversions); exhaustive over the fast path's risk surface |
| neutral_chroma_is_identity | 547-555 | REAL | — | BT.601 identity pin |
| known_triples_convert_correctly | 558-574 | REAL | — | hand goldens — INDEPENDENTLY re-derived by me (§2); correct |
| out_of_gamut_triples_clamp | 577-584 | REAL | — | clamp pins, hand values check out |
| grayscale_row_passes_through_luma | 587-621 | REAL | — | gray passthrough + alpha 255 |
| chroma_upsample_box_replication | 624-658 | REAL | — | h-subsample box replication golden — hand values check out |
| compose_row_reference (oracle) | 664-694 | REAL | S4 | shares sample_up with the cold path (F-001); prefix-safe writes |
| compose_row_matches_the_per_pixel_reference | 697-784 | REAL | — | xorshift64 (deterministic, fixed seed), 4 geometries × 6 widths × 5 rows × color+gray; byte-equality hot vs cold; the strongest test here |
| inconsistent_plane_geometry_keeps_the_neutral_reads | 787-822 | REAL | — | hostile geometry → neutral-128 reads, hot==cold, no panic |
| short_output_rows_stay_prefix_safe | 825-859 | REAL | — | short out → prefix write, no panic |
| Rng (impl in test 8) | 703-713 | REAL | — | xorshift64, correct shift triple, no modulo bias issue for byte fill use |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| fast/branchless sweeps | Yes | exhaustive over the stated risk domain; equality vs the reference form |
| known_triples / out_of_gamut / neutral | Yes | hand-computed constants (I re-derived them independently — they are correct BT.601) |
| compose_row_matches_the_per_pixel_reference | Yes for hot/cold divergence | independent per-pixel loop; NOT a ground-truth oracle for sampling conventions (F-001) — the ground truth one level up is mjpeg_field_differential vs ffmpeg (executed, 3/3 ok in baseline) |
| hostile-geometry / short-out | Yes for regressions of the defensive contract | panics fail tests; neutral-read semantics pinned to exact bytes |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the row-hoisted, division-stepped hot path must be byte-identical to the per-sample reference" (700-702) | test doc | REAL | executed, passing, byte equality asserted per row |
| "missing samples read as neutral 128, never a panic" (790-791) | test doc | REAL | executed; exact bytes pinned at :821 |
| Hand-computed goldens "with the Q16 constants above (round-half-away)" (559, 627) | test comments | REAL | independently re-derived (§2) |

## 7. Dead / duplicated / unreachable code
- Nothing dead in the chunk; the oracle's duplication of the compose logic is
  its purpose (pre-rewrite reference kept alive as the oracle — a legitimate
  pattern, noted in F-001 for its coupling cost).

## 8. Dependencies & flags
- cfg(test) only; no unsafe; no external deps; the only construct outside
  std is the module's own part1 types. MJPEG is unconditional in `codecs`
  (video/mod.rs:33-35) and is the shipped first-vertical decoder.

## 9. Open questions for the next agent
- part1 (W06-A01): does `sample_up`'s row selection use
  `y.div_ceil(v_max)` (matching RowSampler's half_row) — the two must agree;
  a F-001 golden would enforce it forever.
- If the mjpeg corpus differential ever drops its 4:2:0 lanes, F-001 becomes
  a live hole — the unit-level net does not replace it.
