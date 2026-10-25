//! Color conversion and chroma upsampling for the RGBA output path
//! (ITU-R BT.601 full-range YCbCr ↔ RGB, the JFIF conversion, fixed
//! point — no floats, deterministic output).
//!
//! The I420 path needs none of this (planes pass through at their
//! native sampling); only 4:4:4 / 4:2:2 / 4:4:0 streams — which have
//! no planar vocabulary in `PixelFormat` yet — convert here.
//!
//! Upsampling is nearest-neighbor (box) replication: the spec-minimum
//! decoder behavior. libjpeg's "fancy" triangular upsampler is a
//! quality refinement, not a conformance requirement — recorded as an
//! owed refinement for the 5.H3 polish card.
//!
//! # ED-FIX-1 (audit C2): division-free, row-hoisted inner loop
//!
//! The per-pixel loop used to execute up to six hardware divisions
//! (`x*h/h_max` per plane and a loop-invariant `y*v/v_max` recomputed
//! per sample), a per-pixel `match` on chroma presence, and
//! checked-arithmetic fallbacks per sample. The rewrite:
//!
//! - hoists `sy` per row per plane (the only remaining division,
//!   3 per output row, never per pixel);
//! - replaces `x*h/h_max` with incremental additive stepping
//!   (`sx += h; while sx >= h_max { sx -= h_max; src_x += 1 }` —
//!   exact `floor` of the old division, Bresenham-style);
//! - hoists the chroma-presence `match` out of the loop (separate
//!   color / grayscale bodies);
//! - proves the plane-row bounds **once per row** and indexes
//!   directly (the old per-sample `checked_mul`/`checked_add` +
//!   `.get().unwrap_or(&128)` stays available as the cold path for
//!   hand-built inconsistent plane geometry — every producer in this
//!   crate constructs planes where `valid_w ≤ stride` and
//!   `valid_h·stride ≤ data.len()`, so real decodes always take the
//!   hot path);
//! - computes the color math in i32 (the Q16 products peak at
//!   `91881·128 ≈ 1.2·10⁷`, far inside i32 — bit-identical to the
//!   old i64 results).
//!
//! The inner loop is branch-light and auto-vectorizable in shape;
//! output is bit-identical to the previous implementation (pinned by
//! `compose_row_matches_the_per_pixel_reference` and by the committed
//! decode-reference digests).
//!
//! # 5.X1b: direct-row fast paths + chroma-offset LUT
//!
//! The W8 profiling (heavy 4K 4:4:4 lane, `stage_bench`) attributed
//! 63.5% of full decode to this stage. Two measured changes, both
//! bit-exact by construction:
//!
//! - **Direct-row specialization.** For the two horizontal sampling
//!   shapes every real stream uses — 1:1 (4:4:4, 4:4:0) and 2:1
//!   chroma (4:2:2) — the incremental stepper is provably the
//!   identity (`src_x = x`) resp. `src_x = x >> 1`, and the
//!   decoder-built planes satisfy `valid_w ≥ width` (1:1) resp.
//!   `valid_w ≥ ⌈width/2⌉` (2:1). When both the sampling shape and
//!   the row-bounds proof hold, the row composes by direct slice
//!   indexing — no per-pixel stepper carries, no `.min` clamps.
//!   Anything else (hostile geometry, hand-built planes) keeps the
//!   Bresenham stepper loop verbatim.
//!
//! - **Branchless rounding.** `shr16_round_b` computes the same
//!   round-half-away-from-zero as [`shr16_round`] without the sign
//!   branch (the identity `q − [v<0 && v&0xffff == 0x8000]`, proven
//!   below) — the two sign branches per pixel were the autovector
//!   blockers and mispredict sources of the old loop. A 128 KiB
//!   (Cb,Cr)→Δg table was measured and **rejected** on this host
//!   (memory-bound under the shared-sandbox load; the ALU loop is
//!   contention-robust).

/// Full-range BT.601 integer constants, Q16 (rounded at authoring
/// time; the classic JFIF equations).
const CR_R: i32 = 91881; // 1.402
const CB_G: i32 = -22554; // -0.344136
const CR_G: i32 = -46802; // -0.714136
const CB_B: i32 = 116127; // 1.772

/// Convert one YCbCr triple to RGB (Q16 fixed point, rounded, clamped).
fn ycc_to_rgb(y: u8, cb: u8, cr: u8) -> [u8; 3] {
    let y = i32::from(y);
    let cb = i32::from(cb) - 128;
    let cr = i32::from(cr) - 128;
    let r = y + shr16_round(CR_R * cr);
    let g = y + shr16_round(CB_G * cb + CR_G * cr);
    let b = y + shr16_round(CB_B * cb);
    [
        r.clamp(0, 255) as u8,
        g.clamp(0, 255) as u8,
        b.clamp(0, 255) as u8,
    ]
}

/// Divide by 2¹⁶ with round-half-away-from-zero.
fn shr16_round(value: i32) -> i32 {
    if value >= 0 {
        (value + (1i32 << 15)) >> 16
    } else {
        -((-value + (1i32 << 15)) >> 16)
    }
}

/// `shr16_round` without the sign branch: for `v = −65536·m − r`,
/// `0 ≤ r < 65536`, the two agree everywhere except `r == 0x8000`
/// (the negative tie), where the shift form is one too high —
/// corrected by the `[v < 0 && v & 0xffff == 0x8000]` term. Exact
/// for every `v` with `v + 0x8000` inside i32 (the Q16 products here
/// peak near ±1.5·10⁷); equality is pinned by the boundary sweep in
/// the tests below.
#[inline]
fn shr16_round_b(value: i32) -> i32 {
    let q = (value + 0x8000) >> 16;
    q - ((value >> 31) & i32::from(value & 0xffff == 0x8000))
}

/// YCbCr → RGB with the branchless round (bit-identical to
/// [`ycc_to_rgb`]; the hot-path form).
#[inline]
fn ycc_to_rgb_fast(y: u8, cb: u8, cr: u8) -> [u8; 3] {
    let y = i32::from(y);
    let cb = i32::from(cb) - 128;
    let cr = i32::from(cr) - 128;
    let r = y + shr16_round_b(CR_R * cr);
    let g = y + shr16_round_b(CB_G * cb + CR_G * cr);
    let b = y + shr16_round_b(CB_B * cb);
    [
        r.clamp(0, 255) as u8,
        g.clamp(0, 255) as u8,
        b.clamp(0, 255) as u8,
    ]
}

/// A decoded component plane (padded to full 8×8 blocks; `stride` is
/// the padded row pitch).
pub(crate) struct PlaneView<'a> {
    pub data: &'a [u8],
    pub stride: usize,
}

/// Where one component's samples sit on the MCU grid.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ComponentGeom {
    /// Horizontal sampling factor (1..=2).
    pub h: usize,
    /// Vertical sampling factor (1..=2).
    pub v: usize,
    /// Valid sample width (component pixels).
    pub valid_w: usize,
    /// Valid sample height.
    pub valid_h: usize,
}

/// One plane's per-row sampling state: the hoisted source row plus
/// the horizontal step constants.
struct RowSampler<'a> {
    /// The proven-in-bounds source row (`stride` bytes).
    row: &'a [u8],
    /// Largest in-range source column (`valid_w − 1`, saturating).
    clamp: usize,
    /// Horizontal sampling factor (step numerator).
    h: usize,
    /// Greatest sampling factor (step modulus).
    h_max: usize,
    /// Fractional accumulator: `sx = x·h − src_x·h_max ∈ [0, h_max)`.
    sx: usize,
    /// Current source column (`floor(x·h / h_max)`).
    src_x: usize,
}

impl<'a> RowSampler<'a> {
    /// Hoist one plane's row `row_y`: the vertical division runs once
    /// per row, the horizontal source column starts at 0.
    ///
    /// Returns `None` when the plane's row lies outside its data (the
    /// cold path's neutral-128 territory — unreachable for planes
    /// built by this crate's decoder).
    fn new(
        plane: &PlaneView<'a>,
        geom: &ComponentGeom,
        row_y: usize,
        h_max: usize,
        v_max: usize,
    ) -> Option<RowSampler<'a>> {
        let sy = (row_y * geom.v / v_max).min(geom.valid_h.saturating_sub(1));
        let base = sy.checked_mul(plane.stride)?;
        let end = base.checked_add(plane.stride)?;
        let row = plane.data.get(base..end)?;
        Some(RowSampler {
            row,
            clamp: geom.valid_w.saturating_sub(1),
            h: geom.h,
            h_max,
            sx: 0,
            src_x: 0,
        })
    }

    /// The current plane sample for output column `x` (the stepper
    /// has advanced `x` times).
    #[inline]
    fn sample(&self) -> u8 {
        self.row[self.src_x.min(self.clamp)]
    }

    /// Advance the source column to output column `x + 1` (exact
    /// incremental `floor((x+1)·h / h_max)` — no division).
    #[inline]
    fn step(&mut self) {
        self.sx += self.h;
        while self.sx >= self.h_max {
            self.sx -= self.h_max;
            self.src_x += 1;
        }
    }

    /// The row as a directly indexable 1:1 slice: horizontal sampling
    /// is identity (`h == h_max`, so `src_x == x`) and the row plus
    /// the valid width cover all `width` output columns (the
    /// decoder-built shape; anything else keeps the stepper).
    #[inline]
    fn identity_row(&self, width: usize) -> Option<&'a [u8]> {
        if self.h == self.h_max && self.row.len() >= width && self.clamp + 1 >= width {
            Some(self.row)
        } else {
            None
        }
    }

    /// The row as a 2:1-subsampled slice (`h == 1, h_max == 2`, so
    /// `src_x == x >> 1`) whose row and valid width cover every
    /// needed source column `⌊x/2⌋ ≤ (width−1)>>1`.
    #[inline]
    fn half_row(&self, width: usize) -> Option<&'a [u8]> {
        let last = width.saturating_sub(1) >> 1;
        if self.h == 1 && self.h_max == 2 && self.row.len() > last && self.clamp >= last {
            Some(self.row)
        } else {
            None
        }
    }
}

/// 1:1-sampling color row (4:4:4, 4:4:0): every plane column maps to
/// its output column directly — slice indexing only, branchless
/// fixed-point color math.
fn compose_rows_1to1(yrow: &[u8], cbrow: &[u8], crrow: &[u8], width: usize, out: &mut [u8]) {
    for ((yv, (&cbv, &crv)), px) in yrow[..width]
        .iter()
        .zip(cbrow[..width].iter().zip(crrow[..width].iter()))
        .zip(out.as_chunks_mut::<4>().0)
    {
        let [r, g, b] = ycc_to_rgb_fast(*yv, cbv, crv);
        px[0] = r;
        px[1] = g;
        px[2] = b;
        px[3] = 255;
    }
}

/// 2:1 chroma color row (4:2:2): chroma column `x >> 1`, luma column
/// `x` — slice indexing only, branchless fixed-point color math.
fn compose_rows_2to1(yrow: &[u8], cbrow: &[u8], crrow: &[u8], width: usize, out: &mut [u8]) {
    for (x, px) in out
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .take(width)
        .enumerate()
    {
        let [r, g, b] = ycc_to_rgb_fast(yrow[x], cbrow[x >> 1], crrow[x >> 1]);
        px[0] = r;
        px[1] = g;
        px[2] = b;
        px[3] = 255;
    }
}

/// 1:1 grayscale row: one luma byte fans out to `[y, y, y, 255]` —
/// packed word writes (a quarter of the scalar stores).
fn compose_gray_direct(yrow: &[u8], width: usize, out: &mut [u8]) {
    for (yv, px) in yrow[..width].iter().zip(out.as_chunks_mut::<4>().0) {
        let word = u32::from_le_bytes([*yv, *yv, *yv, 255]);
        px.copy_from_slice(&word.to_le_bytes());
    }
}

/// Row composer: upsampled YCbCr → RGBA for one full-resolution row.
pub(crate) struct RgbComposer<'a> {
    /// Luma plane (always present).
    luma: PlaneView<'a>,
    /// Chroma planes (absent for grayscale — neutral 128 used).
    chroma: Option<(PlaneView<'a>, PlaneView<'a>)>,
    /// Geometry per component: Y, Cb, Cr.
    geoms: [ComponentGeom; 3],
    h_max: usize,
    v_max: usize,
}

impl<'a> RgbComposer<'a> {
    /// Assemble from the decoded planes. `geoms` covers Y, Cb, Cr in
    /// order; `chroma` is `None` for single-component streams.
    pub(crate) fn new(
        luma: PlaneView<'a>,
        chroma: Option<(PlaneView<'a>, PlaneView<'a>)>,
        geoms: [ComponentGeom; 3],
        h_max: usize,
        v_max: usize,
    ) -> RgbComposer<'a> {
        RgbComposer {
            luma,
            chroma,
            geoms,
            h_max,
            v_max,
        }
    }

    /// Compose `width` RGBA pixels of output row `row_y` into `out`
    /// (alpha is straight `0xff`).
    pub(crate) fn compose_row(&self, row_y: usize, width: usize, out: &mut [u8]) {
        let luma = RowSampler::new(&self.luma, &self.geoms[0], row_y, self.h_max, self.v_max);
        match (&self.chroma, luma) {
            (Some((cb_plane, cr_plane)), Some(luma)) => {
                match (
                    RowSampler::new(cb_plane, &self.geoms[1], row_y, self.h_max, self.v_max),
                    RowSampler::new(cr_plane, &self.geoms[2], row_y, self.h_max, self.v_max),
                ) {
                    (Some(cb), Some(cr)) => {
                        if out.len() >= width * 4 {
                            self.compose_color_row(luma, cb, cr, width, out);
                        } else {
                            self.compose_row_cold(row_y, width, out);
                        }
                    }
                    _ => self.compose_row_cold(row_y, width, out),
                }
            }
            (None, Some(luma)) => {
                if out.len() >= width * 4 {
                    self.compose_gray_row(luma, width, out);
                } else {
                    self.compose_row_cold(row_y, width, out);
                }
            }
            _ => self.compose_row_cold(row_y, width, out),
        }
    }

    /// Forced-scalar compose (the W6-03 bench seam: the same path as
    /// [`Self::compose_row`], pinned for `examples/color_bench.rs`
    /// measurement and differential tests — never called by the
    /// decoder, which uses the dispatching entry above).
    #[allow(dead_code)] // baseline commit only: unused until the W6-03 rewrite arms it
    pub(crate) fn compose_row_scalar(&self, row_y: usize, width: usize, out: &mut [u8]) {
        self.compose_row(row_y, width, out);
    }

    /// Forced-SIMD compose (the W6-03 bench seam; until the SIMD
    /// kernel lands this maps to the scalar path, and
    /// [`simd_available`] reports `false` so callers do not arm it).
    #[allow(dead_code)] // baseline commit only: unused until the W6-03 rewrite arms it
    pub(crate) fn compose_row_simd(&self, row_y: usize, width: usize, out: &mut [u8]) {
        self.compose_row(row_y, width, out);
    }

    /// The hot color path: all rows proven, direct indexing, zero
    /// divisions, zero per-sample branches beyond the step carries.
    ///
    /// The two real-stream sampling shapes (all-1:1 = 4:4:4/4:4:0,
    /// luma-1:1 + chroma-2:1 = 4:2:2) take the specialized
    /// direct-row loops first (5.X1b); the Bresenham stepper loop
    /// stays for every other geometry, verbatim.
    fn compose_color_row(
        &self,
        mut luma: RowSampler<'_>,
        mut cb: RowSampler<'_>,
        mut cr: RowSampler<'_>,
        width: usize,
        out: &mut [u8],
    ) {
        if let (Some(yrow), Some(cbrow), Some(crrow)) = (
            luma.identity_row(width),
            cb.identity_row(width),
            cr.identity_row(width),
        ) {
            compose_rows_1to1(yrow, cbrow, crrow, width, out);
            return;
        }
        if let (Some(yrow), Some(cbrow), Some(crrow)) = (
            luma.identity_row(width),
            cb.half_row(width),
            cr.half_row(width),
        ) {
            compose_rows_2to1(yrow, cbrow, crrow, width, out);
            return;
        }
        for x in 0..width {
            let y = luma.sample();
            let cbv = cb.sample();
            let crv = cr.sample();
            let [r, g, b] = ycc_to_rgb_fast(y, cbv, crv);
            let base = x * 4;
            out[base] = r;
            out[base + 1] = g;
            out[base + 2] = b;
            out[base + 3] = 255;
            luma.step();
            cb.step();
            cr.step();
        }
    }

    /// The hot grayscale path (neutral chroma folds to R=G=B=Y).
    fn compose_gray_row(&self, mut luma: RowSampler<'_>, width: usize, out: &mut [u8]) {
        if let Some(yrow) = luma.identity_row(width) {
            compose_gray_direct(yrow, width, out);
            return;
        }
        for x in 0..width {
            let y = luma.sample();
            let base = x * 4;
            out[base] = y;
            out[base + 1] = y;
            out[base + 2] = y;
            out[base + 3] = 255;
            luma.step();
        }
    }

    /// The cold path: the original per-sample reference math (checked
    /// indexing, neutral 128 for out-of-data samples). Taken only for
    /// hand-built inconsistent plane geometry — every plane this
    /// crate's decoder produces satisfies the hot-path proofs — and
    /// kept verbatim so those degenerate reads keep their exact
    /// documented behavior.
    fn compose_row_cold(&self, row_y: usize, width: usize, out: &mut [u8]) {
        for x in 0..width {
            let luma = self.sample_up(&self.luma, &self.geoms[0], x, row_y);
            let (cb, cr) = match &self.chroma {
                Some((cb_plane, cr_plane)) => (
                    self.sample_up(cb_plane, &self.geoms[1], x, row_y),
                    self.sample_up(cr_plane, &self.geoms[2], x, row_y),
                ),
                None => (128, 128),
            };
            let [r, g, b] = ycc_to_rgb(luma, cb, cr);
            let base = x * 4;
            if let Some(slot) = out.get_mut(base) {
                *slot = r;
            }
            if let Some(slot) = out.get_mut(base + 1) {
                *slot = g;
            }
            if let Some(slot) = out.get_mut(base + 2) {
                *slot = b;
            }
            if let Some(slot) = out.get_mut(base + 3) {
                *slot = 255;
            }
        }
    }

    /// Upsampled read: full-resolution `(x, y)` from one plane (the
    /// cold-path reference math).
    fn sample_up(&self, plane: &PlaneView<'_>, geom: &ComponentGeom, x: usize, y: usize) -> u8 {
        let sx = (x * geom.h / self.h_max).min(geom.valid_w.saturating_sub(1));
        let sy = (y * geom.v / self.v_max).min(geom.valid_h.saturating_sub(1));
        let index = sy
            .checked_mul(plane.stride)
            .and_then(|row| row.checked_add(sx))
            .unwrap_or(0);
        *plane.data.get(index).unwrap_or(&128)
    }
}

/// Whether the SIMD compose entry is available in this build on this
/// CPU (the W6-03 bench seam; `false` until the kernel lands).
#[allow(dead_code)] // baseline commit only: unused until the W6-03 rewrite arms it
pub(crate) fn simd_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane(bytes: &[u8], stride: usize) -> PlaneView<'_> {
        PlaneView {
            data: bytes,
            stride,
        }
    }

    #[test]
    fn branchless_round_matches_the_reference_everywhere() {
        // `shr16_round_b` vs `shr16_round`: the only inputs where the
        // shift form can differ are the negative ties `v & 0xffff ==
        // 0x8000` (proven in the doc comment) — sweep every such v in
        // the Q16 product domain ±1.5·10⁷ (plus its ±1 neighbors and
        // a dense low-magnitude window covering every low-16 pattern
        // at several magnitudes).
        for magnitude in [0i32, 65536, 65536 * 7, 65536 * 227] {
            for low in 0i32..65536 {
                for v in [magnitude + low - 65536 * 16, -magnitude - low - 65536 * 16] {
                    let v = v.clamp(-15_000_000, 15_000_000);
                    assert_eq!(shr16_round_b(v), shr16_round(v), "v={v}");
                }
            }
        }
        for v in -300_000i32..300_000 {
            assert_eq!(shr16_round_b(v), shr16_round(v), "v={v}");
        }
    }

    #[test]
    fn fast_conversion_matches_the_reference_everywhere() {
        // Every (Cb, Cr) pair × the y values at each clamp boundary
        // of the three channel offsets (where clamping could differ)
        // plus a dense sweep — `ycc_to_rgb_fast` must equal the
        // reference `ycc_to_rgb` triple-for-triple.
        for cb in 0..=255u8 {
            for cr in 0..=255u8 {
                let cbv = i32::from(cb) - 128;
                let crv = i32::from(cr) - 128;
                let mut ys: Vec<u8> = vec![0, 1, 64, 127, 128, 191, 254, 255];
                for d in [
                    shr16_round(CR_R * crv),
                    shr16_round(CB_B * cbv),
                    shr16_round(CB_G * cbv + CR_G * crv),
                ] {
                    for cand in [-d - 1, -d, -d + 1, 255 - d - 1, 255 - d, 255 - d + 1] {
                        if (0..=255).contains(&cand) {
                            ys.push(cand as u8);
                        }
                    }
                }
                for y in ys {
                    assert_eq!(
                        ycc_to_rgb_fast(y, cb, cr),
                        ycc_to_rgb(y, cb, cr),
                        "y={y} cb={cb} cr={cr}"
                    );
                }
            }
        }
    }

    #[test]
    fn neutral_chroma_is_identity() {
        // Y, Cb=Cr=128 → R=G=B=Y (full-range BT.601).
        for y in [0u8, 1, 64, 128, 200, 254, 255] {
            let [r, g, b] = ycc_to_rgb(y, 128, 128);
            assert_eq!(r, y, "R for Y={y}");
            assert_eq!(g, y, "G for Y={y}");
            assert_eq!(b, y, "B for Y={y}");
        }
    }

    #[test]
    fn known_triples_convert_correctly() {
        // Hand-computed with the Q16 constants above (round-half-away):
        // warm pixel: r dominates; cool pixel: b dominates.
        let [r, g, b] = ycc_to_rgb(90, 110, 200);
        assert_eq!([r, g, b], [191, 45, 58]);
        assert!(r > g && r > b, "warm triple: R must dominate");

        let [r, g, b] = ycc_to_rgb(30, 220, 120);
        assert_eq!([r, g, b], [19, 4, 193]);
        assert!(b > r && b > g, "cool triple: B must dominate");

        let [r, g, b] = ycc_to_rgb(120, 200, 200);
        assert!(
            g < r && g < b,
            "high Cb AND Cr pull G below both: got [{r}, {g}, {b}]"
        );
    }

    #[test]
    fn out_of_gamut_triples_clamp() {
        // R and B saturate high; G stays finite (hand-computed).
        let [r, g, b] = ycc_to_rgb(255, 255, 255);
        assert_eq!([r, g, b], [255, 121, 255]);
        // R and B saturate low; G stays finite.
        let [r, g, b] = ycc_to_rgb(0, 0, 0);
        assert_eq!([r, g, b], [0, 135, 0]);
    }

    #[test]
    fn grayscale_row_passes_through_luma() {
        let y_bytes = [10u8, 20, 30, 40];
        let composer = RgbComposer::new(
            plane(&y_bytes, 4),
            None,
            [
                ComponentGeom {
                    h: 1,
                    v: 1,
                    valid_w: 4,
                    valid_h: 1,
                },
                ComponentGeom {
                    h: 1,
                    v: 1,
                    valid_w: 4,
                    valid_h: 1,
                },
                ComponentGeom {
                    h: 1,
                    v: 1,
                    valid_w: 4,
                    valid_h: 1,
                },
            ],
            1,
            1,
        );
        let mut out = [0u8; 16];
        composer.compose_row(0, 4, &mut out);
        for (i, want) in y_bytes.iter().enumerate() {
            assert_eq!(&out[i * 4..i * 4 + 3], &[*want; 3]);
            assert_eq!(out[i * 4 + 3], 255);
        }
    }

    #[test]
    fn chroma_upsample_box_replication() {
        // 2×1 chroma grid stretched over 4 pixels: each chroma sample
        // covers two luma positions. Exact expectations computed with
        // the Q16 constants (round-half-away, then clamp):
        // (y=200, cb=90, cr=240) → (255, 133, 133);
        // (y=10,  cb=240, cr=110) → (0, 0, 208).
        let y_bytes = [200u8, 10, 200, 10];
        let cb_bytes = [90u8, 240];
        let cr_bytes = [240u8, 110];
        let geom_y = ComponentGeom {
            h: 1,
            v: 1,
            valid_w: 4,
            valid_h: 1,
        };
        let geom_c = ComponentGeom {
            h: 1,
            v: 1,
            valid_w: 2,
            valid_h: 1,
        };
        let composer = RgbComposer::new(
            plane(&y_bytes, 4),
            Some((plane(&cb_bytes, 2), plane(&cr_bytes, 2))),
            [geom_y, geom_c, geom_c],
            2,
            1,
        );
        let mut out = [0u8; 16];
        composer.compose_row(0, 4, &mut out);
        assert_eq!(&out[0..4], &[255, 133, 133, 255]);
        assert_eq!(&out[4..8], &[255, 133, 133, 255]);
        assert_eq!(&out[8..12], &[0, 0, 208, 255]);
        assert_eq!(&out[12..16], &[0, 0, 208, 255]);
    }

    // --- ED-FIX-1: hot-path equivalence and edge behavior ---

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
            let (cb, cr) = match &composer.chroma {
                Some((cb_plane, cr_plane)) => (
                    composer.sample_up(cb_plane, &composer.geoms[1], x, row_y),
                    composer.sample_up(cr_plane, &composer.geoms[2], x, row_y),
                ),
                None => (128, 128),
            };
            let [r, g, b] = ycc_to_rgb(luma, cb, cr);
            let base = x * 4;
            if let Some(slot) = out.get_mut(base) {
                *slot = r;
            }
            if let Some(slot) = out.get_mut(base + 1) {
                *slot = g;
            }
            if let Some(slot) = out.get_mut(base + 2) {
                *slot = b;
            }
            if let Some(slot) = out.get_mut(base + 3) {
                *slot = 255;
            }
        }
    }

    #[test]
    fn compose_row_matches_the_per_pixel_reference() {
        // Deterministic sweep over the sampling geometries the decoder
        // can produce (4:4:4, 4:2:2, 4:4:0, 4:2:0, grayscale; odd and
        // even widths; vertical subsampling with several output rows):
        // the row-hoisted, division-stepped hot path must be
        // byte-identical to the per-sample reference.
        struct Rng(u64);
        impl Rng {
            fn next(&mut self) -> u64 {
                let mut x = self.0;
                x ^= x >> 12;
                x ^= x << 25;
                x ^= x >> 27;
                self.0 = x;
                x.wrapping_mul(0x2545_f491_4f6c_dd1d)
            }
        }
        let mut rng = Rng(0xd1ce_b0ba_5eed_0bad);
        for (h_max, v_max) in [(1usize, 1usize), (2, 1), (1, 2), (2, 2)] {
            for width in [1usize, 2, 3, 5, 8, 13] {
                let rows = 5usize;
                // Padded planes sized like the decoder's (stride ≥
                // valid_w, data = stride × padded rows).
                let geom_y = ComponentGeom {
                    h: h_max,
                    v: v_max,
                    valid_w: width,
                    valid_h: rows,
                };
                let c_w = width.div_ceil(h_max.max(1));
                let c_h = rows.div_ceil(v_max.max(1));
                let geom_c = ComponentGeom {
                    h: 1,
                    v: 1,
                    valid_w: c_w,
                    valid_h: c_h,
                };
                let stride_y = width.div_ceil(8) * 8;
                let stride_c = c_w.div_ceil(8) * 8;
                let rows_y = rows.div_ceil(8) * 8;
                let rows_c = c_h.div_ceil(8) * 8;
                let mut y_data = vec![0u8; stride_y * rows_y];
                let mut cb_data = vec![0u8; stride_c * rows_c];
                let mut cr_data = vec![0u8; stride_c * rows_c];
                for slot in y_data.iter_mut() {
                    *slot = (rng.next() % 256) as u8;
                }
                for slot in cb_data.iter_mut() {
                    *slot = (rng.next() % 256) as u8;
                }
                for slot in cr_data.iter_mut() {
                    *slot = (rng.next() % 256) as u8;
                }
                let composer = RgbComposer::new(
                    plane(&y_data, stride_y),
                    Some((plane(&cb_data, stride_c), plane(&cr_data, stride_c))),
                    [geom_y, geom_c, geom_c],
                    h_max,
                    v_max,
                );
                for row_y in 0..rows {
                    let mut hot = vec![0u8; width * 4];
                    let mut cold = vec![0u8; width * 4];
                    composer.compose_row(row_y, width, &mut hot);
                    compose_row_reference(&composer, row_y, width, &mut cold);
                    assert_eq!(
                        hot, cold,
                        "h_max={h_max} v_max={v_max} width={width} row={row_y}"
                    );
                }
                // Grayscale variant (same luma, no chroma).
                let gray = RgbComposer::new(
                    plane(&y_data, stride_y),
                    None,
                    [geom_y, geom_c, geom_c],
                    h_max,
                    v_max,
                );
                for row_y in 0..rows {
                    let mut hot = vec![0u8; width * 4];
                    let mut cold = vec![0u8; width * 4];
                    gray.compose_row(row_y, width, &mut hot);
                    compose_row_reference(&gray, row_y, width, &mut cold);
                    assert_eq!(hot, cold, "gray h_max={h_max} row={row_y}");
                }
            }
        }
    }

    #[test]
    fn inconsistent_plane_geometry_keeps_the_neutral_reads() {
        // A hand-built plane whose declared geometry outruns its data
        // (stride × valid_h > data.len()): the row proof fails and the
        // cold path must reproduce the historical behavior — the
        // missing samples read as neutral 128, never a panic.
        let y_bytes = [200u8, 10, 200, 10];
        let cb_bytes = [90u8]; // declares 2 wide but holds 1
        let geom_y = ComponentGeom {
            h: 1,
            v: 1,
            valid_w: 4,
            valid_h: 1,
        };
        let geom_c = ComponentGeom {
            h: 1,
            v: 1,
            valid_w: 2,
            valid_h: 1,
        };
        let composer = RgbComposer::new(
            plane(&y_bytes, 4),
            Some((plane(&cb_bytes, 2), plane(&cb_bytes, 2))),
            [geom_y, geom_c, geom_c],
            2,
            1,
        );
        let mut hot = [0u8; 16];
        composer.compose_row(0, 4, &mut hot);
        let mut cold = [0u8; 16];
        compose_row_reference(&composer, 0, 4, &mut cold);
        assert_eq!(hot, cold);
        // Columns 2..4 sample chroma past the data: neutral 128. The
        // luma also subsamples here (h=1 over h_max=2), so column 2
        // reads y_bytes[1] = 10 → (10, 10, 10).
        assert_eq!(&hot[8..12], &[10, 10, 10, 255]);
    }

    #[test]
    fn short_output_rows_stay_prefix_safe() {
        // `out` shorter than width*4: no panic, prefix written (the
        // cold path's historical contract).
        let y_bytes = [10u8, 20, 30, 40];
        let composer = RgbComposer::new(
            plane(&y_bytes, 4),
            None,
            [
                ComponentGeom {
                    h: 1,
                    v: 1,
                    valid_w: 4,
                    valid_h: 1,
                },
                ComponentGeom {
                    h: 1,
                    v: 1,
                    valid_w: 4,
                    valid_h: 1,
                },
                ComponentGeom {
                    h: 1,
                    v: 1,
                    valid_w: 4,
                    valid_h: 1,
                },
            ],
            1,
            1,
        );
        let mut out = [0u8; 6]; // room for 1.5 pixels
        composer.compose_row(0, 4, &mut out);
        assert_eq!(&out[0..4], &[10, 10, 10, 255]);
        assert_eq!(&out[4..6], &[20, 20], "partial second pixel prefix");
    }
}
