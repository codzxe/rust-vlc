# AUDIT: crates/codecs/src/video/mjpeg/color.rs#part1 (lines 1–480)
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-21 |
| Lines / functions | 480 (of 861) / 22 |
| File verdict | REAL-UNPROVEN (shipped-reachable via the MJPEG RGBA assembly; unit+corpus-test-executed in baseline; no shipped-path RGBA execution evidence — baseline CLI runs exercised only the I420 arm) |
| Reachable from shipped entry point? | YES (chain: `lumen open` AVI/MJPEG → codecs::video::mjpeg decoder → `assemble_rgba_into` (decoder.rs:1140-1191) → `RgbComposer::compose_row` (decoder.rs:1189) — the RGBA arm taken for 4:4:4/4:2:2/4:4:0/grayscale MJPEG). The baseline's shipped execution (avi-firstvertical, 60/60 frames) is 4:2:0 → I420 arm → color.rs NOT on that path; executed in baseline only through the codecs test suite (all 10 in-file tests `ok`, test-workspace.log.txt:666-675). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `sed -n` quote verification (70-75, 28-34, 196-201, 463-471, 474-479; decoder.rs 738-741, 1150-1190); python3 brute force of the CB_B constant deviation (3/255 Cb values differ by 1 LSB vs the rounded constant); `rg -n "compose_row\|simd_available"` (decoder.rs:1189 production caller; color_bench.rs:711,900,911 seam consumers); `rg -n "mjpeg::color::tests" audit/runs/baseline/test-workspace.log.txt` |

## Chunk boundary note
Assigned part1 ≈ lines 1–430 of 861; extended to 480 so ALL production code stays in part1 and W06-A09's part2 = 481–861 is the complete test module (a clean functional split). Part2 owns `mod tests` incl. `compose_row_reference` (664) — I read tests 558-586 and 782-860 only as evidence for part1 findings (circularity, coverage gaps), flagged below for A09.

## 1. One-paragraph verdict
This is the MJPEG color-conversion/chroma-upsampling core for the RGBA output path: BT.601 full-range fixed-point YCbCr→RGB, a division-free Bresenham row sampler, direct-row fast paths for the two real sampling shapes, and a cold per-sample reference path for degenerate geometry. The math is genuinely good — I hand-verified the branchless rounder equals the reference rounder on ties and negatives, the JFIF integer constants (three of four), and the slice-safety proofs of the fast paths; ten unit tests plus the per-pixel-reference equivalence test all ran green in the baseline workspace run. Two real defects: the hot path indexes by an UNCHECKED invariant (`valid_w ≤ stride` — a hand-built pub(crate) plane panics, and the module doc overstates what is "proven"), and the blue-channel Q16 constant is 116127 where its own comment's "rounded" rule gives 116130 — a brute-forced 1-LSB blue deviation on 3 of 255 Cb values that the hand-computed test triples cannot see. Shipped reachability is real but unproven end-to-end: the baseline's only shipped MJPEG execution is 4:2:0→I420, which bypasses this file entirely.

## 2. What actually works (verified)
— All 10 in-file tests `ok` in baseline run #3 (test-workspace.log.txt:666-675), incl. `branchless_round_matches_the_reference_everywhere`, `compose_row_matches_the_per_pixel_reference`, `inconsistent_plane_geometry_keeps_the_neutral_reads`, `short_output_rows_stay_prefix_safe`.
— `shr16_round_b` ≡ `shr16_round` hand-verified: positive (correction term zeroed by `value >> 31 == 0`), negative tie v=−0x8000 (shift form 0, corrected to −1 by `(value>>31)&(v&0xffff==0x8000)`), negative non-tie v=−32769 (both −1). Products bounded ±1.2·10⁷, so `value + 0x8000` cannot overflow i32 (read + arithmetic).
— Fast-path slice safety hand-verified: `compose_rows_1to1` indexes `yrow[..width]` only after `identity_row` proved `row.len() ≥ width`; `compose_rows_2to1`'s `x>>1` bounded by `half_row`'s `last = (width−1)>>1 < row.len()`; `out` gated by `out.len() ≥ width*4` at compose_row:327/337; `as_chunks_mut::<4>` zip stops at the shortest (read).
— Decoder producer discipline verified at the source: `stride = block_cols*8 ≥ 8·ceil(w·h/h_max) ≥ valid_w` (decoder.rs:738-766) — the shipped path cannot build a plane violating the hot-path assumption.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/mjpeg/color.rs:196-201 (with module-doc claim 28-34)
Code:
```rust
    /// The current plane sample for output column `x` (the stepper
    /// has advanced `x` times).
    #[inline]
    fn sample(&self) -> u8 {
        self.row[self.src_x.min(self.clamp)]
    }
```
What is wrong: `RowSampler::new` (175-194) proves only that the ROW exists (`data[base..base+stride]`); it never proves `clamp = valid_w − 1 < row.len()` (i.e. `valid_w ≤ stride`). `sample()` then indexes `row[src_x.min(clamp)]` — for a hand-built plane with `valid_w > stride` but `data.len() ≥ stride` (row proof passes, fast paths fail their width checks), the stepper loop indexes past the row and PANICS. The module doc claims the rewrite "proves the plane-row bounds once per row and indexes directly" — the row is proven, the column bound is producer discipline, not proof:
```rust
//! - proves the plane-row bounds **once per row** and indexes
//!   directly (… every producer in this crate constructs planes where
//!   `valid_w ≤ stride` and
//!   `valid_h·stride ≤ data.len()`, so real decodes always take the
//!   hot path);
```
Why it matters: `RgbComposer` is `pub(crate)` — today only the decoder (discipline verified, decoder.rs:738-766: `stride = 8·block_cols ≥ valid_w`) and tests build it, so no shipped input can trigger the panic; but the crate's stated convention is "no panics on any input" and the safety is an undocumented-in-code assumption. The in-file test for inconsistent geometry (787-822) builds the `data.len() < stride` flavor (row proof fails → cold path) and NOT the `valid_w > stride`-with-valid-row flavor — the gap is untested.
What correct looks like: `RowSampler::new` returns `None` (→ cold path) unless `geom.valid_w <= plane.stride`, or `sample()` clamps by `self.row.len() − 1`; add the missing hostile-geometry test.
Confidence: HIGH — Verification: read + hand-traced panic construction (data=[90,91], stride=2, valid_w=10, h=1/h_max=2, width=8 → row[3] on a 2-byte row) + test read.

### F-002 [S4]
Where: crates/codecs/src/video/mjpeg/color.rs:70-75
Code:
```rust
/// Full-range BT.601 integer constants, Q16 (rounded at authoring
/// time; the classic JFIF equations).
const CR_R: i32 = 91881; // 1.402
const CB_G: i32 = -22554; // -0.344136
const CR_G: i32 = -46802; // -0.714136
const CB_B: i32 = 116127; // 1.772
```
What is wrong: 1.772 × 2¹⁶ = 116129.79 — floor 116129, round 116130; the constant is 116127, neither, despite the comment "rounded at authoring time". The other three constants match their documented rule (91881.47→91881, −22554.34→−22554, −46802.64→−46802). Brute-forced impact (python3, all 255 signed Cb): blue differs by 1 LSB from the correctly-rounded constant for Cb−128 ∈ {−3, 54, 111} — i.e. 3 of 255 chroma values produce a 1-LSB blue deviation from the classic JFIF equations.
Why it matters: the module's pitch is "deterministic output" pinned by reference digests, and this file's constants define the shipped RGBA colors; a sub-visual 1-LSB blue error on ~1% of chroma values is a conformance nit, and the comment is false about how the value was chosen. The hand-computed test triples (`known_triples_convert_correctly`, :558-576 — "Hand-computed with the Q16 constants above") are computed from the same 116127, so the suite is circular for this purpose and cannot catch it.
What correct looks like: `const CB_B: i32 = 116130;` (round) or `116129` (floor), with the reference-digest impact re-generated and the comment made true.
Confidence: HIGH — Verification: arithmetic + python3 brute force + test read.

### F-003 [S4]
Where: crates/codecs/src/video/mjpeg/color.rs:347-362, 474-479
Code:
```rust
    #[allow(dead_code)] // baseline commit only: unused until the W6-03 rewrite arms it
    pub(crate) fn compose_row_scalar(&self, row_y: usize, width: usize, out: &mut [u8]) {
        self.compose_row(row_y, width, out);
    }
```
and
```rust
#[allow(dead_code)] // baseline commit only: unused until the W6-03 rewrite arms it
pub(crate) fn simd_available() -> bool {
    false
}
```
What is wrong: a three-piece bench seam (`compose_row_scalar`, `compose_row_simd`, `simd_available`) is compiled into the shipped lib as dead code behind `#[allow(dead_code)]`; `simd_available()` is a constant `false` (the SIMD kernel does not exist). The real consumers live only in `examples/color_bench.rs`'s `#[path]`-include tree (color_bench.rs:711, 900, 911).
Why it matters: honest (comments say exactly what they are) but it is dead weight plus a `#[allow]` hiding a dead_code lint in production code, and the "until the W6-03 rewrite arms it" promise has no owner/date — seam debt.
What correct looks like: gate the seam behind a bench-only cfg (as stage_bench includes do) or land the kernel and delete the allow.
Confidence: HIGH — Verification: read + rg consumers.

### F-004 [S4]
Where: crates/codecs/src/video/mjpeg/color.rs:463-471
Code:
```rust
    fn sample_up(&self, plane: &PlaneView<'_>, geom: &ComponentGeom, x: usize, y: usize) -> u8 {
        let sx = (x * geom.h / self.h_max).min(geom.valid_w.saturating_sub(1));
        let sy = (y * geom.v / self.v_max).min(geom.valid_h.saturating_sub(1));
        let index = sy
            .checked_mul(plane.stride)
            .and_then(|row| row.checked_add(sx))
            .unwrap_or(0);
        *plane.data.get(index).unwrap_or(&128)
    }
```
What is wrong: on arithmetic overflow the index silently becomes 0 — the cold path then reads `data[0]` (a real sample) rather than the documented neutral-128 behavior for unreadable positions. The two failure classes (overflow vs out-of-data) are conflated.
Why it matters: cold-path only, hand-built geometry only, one-sample fidelity — hygiene.
What correct looks like: `.map_or(None, |i| plane.data.get(i).copied()).unwrap_or(128)` — overflow must read neutral, not sample 0.
Confidence: HIGH — Verification: read.

## 4. Per-function table (part1: lines 1–480)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-68 | REAL | S3 | ED-FIX-1/5.X1b claims mostly verified; "proves the plane-row bounds" overstates (F-001) |
| `CR_R/CB_G/CR_G/CB_B` consts | 72-75 | REAL | S4 | F-002: CB_B off from its own rounding rule |
| `ycc_to_rgb` | 78-90 | REAL | — | reference form; clamp 0..255 |
| `shr16_round` | 93-99 | REAL | — | round-half-away-from-zero, verified |
| `shr16_round_b` | 109-112 | REAL | — | ≡ reference, hand-verified incl. negative tie; no overflow at product magnitudes |
| `ycc_to_rgb_fast` | 117-129 | REAL | — | hot form; bit-identical given the rounder equivalence |
| `PlaneView` | 133-136 | REAL | — | pub(crate) data+stride |
| `ComponentGeom` | 140-149 | REAL | — | h/v/valid_w/valid_h |
| `RowSampler` struct | 153-166 | REAL | — | sx/src_x stepper state |
| `RowSampler::new` | 175-194 | REAL | S3 | proves ROW only — F-001; checked_mul/add, None → cold |
| `RowSampler::sample` | 199-201 | REAL | S3 | F-001 unchecked clamp < row.len() |
| `RowSampler::step` | 206-212 | REAL | — | exact incremental floor, verified |
| `RowSampler::identity_row` | 219-225 | REAL | — | safe preconditions for 1to1 |
| `RowSampler::half_row` | 231-238 | REAL | — | safe preconditions for 2to1 |
| `compose_rows_1to1` | 244-256 | REAL | — | slice-safe via identity_row proofs |
| `compose_rows_2to1` | 260-274 | REAL | — | slice-safe via half_row proof |
| `compose_gray_direct` | 278-283 | REAL | — | packed word writes |
| `RgbComposer::new` | 300-314 | REAL | — | |
| `RgbComposer::compose_row` | 318-345 | REAL | — | hot/cold dispatch; out.len() ≥ width*4 gate |
| `compose_row_scalar` | 352-354 | STUB | S4 | F-003 dead seam, delegates to compose_row |
| `compose_row_simd` | 360-362 | STUB | S4 | F-003 dead seam, no SIMD kernel exists |
| `compose_color_row` | 371-409 | REAL | — | fast paths then stepper; stepper inherits F-001 |
| `compose_gray_row` | 412-426 | REAL | — | stepper fallback inherits F-001 |
| `compose_row_cold` | 434-459 | REAL | S4 | per-byte get_mut prefix-safe (test pins); F-004 overflow→data[0] |
| `sample_up` | 463-471 | REAL | S4 | F-004 |
| `simd_available` | 477-479 | STUB | S4 | constant false |

## 5. Test honesty (part2 bodies, evidence for part1)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| branchless_round_matches_the_reference_everywhere (:493) | YES | exhaustive sweep over the rounder domain |
| fast_conversion_matches_the_reference_everywhere (:514) | YES | full u8³-classes equivalence of fast vs reference |
| compose_row_matches_the_per_pixel_reference (:697) | YES | the hot/stepper vs cold-path equivalence across geometries — the ED-FIX-1 license |
| inconsistent_plane_geometry_keeps_the_neutral_reads (:787) | YES (partial) | pins the data<stride flavor only; the valid_w>stride-with-valid-row panic case (F-001) is NOT covered — mutation of `clamp` to `row.len()-1` would not fail any test |
| known_triples_convert_correctly (:558) | NO for F-002 | expectations hand-computed WITH the code's own constants — circular w.r.t. constant correctness |
| short_output_rows_stay_prefix_safe (:825) | YES | pins the cold prefix contract |
All `ok` in baseline run #3 (test-workspace.log.txt:666-675).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no floats, deterministic output" | color.rs :2-3 | REAL | all-i32 fixed point (read) |
| "Upsampling is nearest-neighbor (box) replication: the spec-minimum decoder behavior" | color.rs :9-11 | REAL | stepper/box logic read; spec-minimum claim consistent with ITU-T T.81 (libjpeg fancy is optional) |
| "proves the plane-row bounds once per row and indexes directly" | color.rs :28-29 | SHALLOW | row proven, column clamp assumed (F-001) |
| "Q16 (rounded at authoring time; the classic JFIF equations)" | color.rs :70-71 | FALSE (for CB_B) | 116127 is neither floor nor round of 116129.79 (F-002) |
| "bit-identical to the previous implementation (pinned by compose_row_matches_the_per_pixel_reference…)" | color.rs :40-42 | REAL | test ran ok in baseline |
| 5.X1b "63.5% of full decode" attribution | color.rs :46-47 | REAL-UNPROVEN | stage_bench measurement record (worklog), not re-run |

## 7. Dead / duplicated / unreachable code
Dead seam trio (F-003): `compose_row_scalar`, `compose_row_simd`, `simd_available` — lib-dead, example-alive. No other dead code in part1; every other item is on the production compose path.

## 8. Dependencies & flags
std-only, no unsafe, no features. `as_chunks_mut` (stable slice API) used for the fast rows. pub(crate) visibility — the internal seam that carries F-001's unchecked invariant.

## 9. Open questions for the next agent
- W06-A09 (part2): add the hostile-geometry test for F-001 (valid_w > stride, row in-data) — if it panics as traced, F-001 upgrades to S2 for the internal API; if the cold path somehow catches it, downgrade to S4 and say why.
- A09: confirm `compose_row_reference` (:664) is a genuinely independent reference (not a copy of the hot path modulo the stepper).
- Whoever owns reference digests: F-002's constant fix will change committed digests for 4:4:4 lanes on 3/255 Cb values — plan the re-generation.
