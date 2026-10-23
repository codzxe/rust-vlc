//! W6-03 MJPEG color-pipeline benchmark: the YCbCr→RGBA conversion +
//! chroma upsampling path (`video/mjpeg/color.rs`) measured **in
//! isolation**, before vs after, under one identical protocol
//! (honest-numbers mandate: medians, N≥15 recorded per-rep raw data,
//! load conditions printed by the run itself).
//!
//! # What is measured
//!
//! One full-frame compose (all output rows, exactly the loop shape of
//! `decoder::assemble_rgba_into`: one `RgbComposer` built per frame,
//! one `compose_row` per output row) over realistic frame sizes
//! (640×480, 1280×720, 3840×2160) × samplings (4:2:0, 4:2:2, 4:4:4,
//! grayscale) × two plane fills:
//!
//! - `random` — uniform random bytes (the cache-adversarial worst
//!   case; conservative),
//! - `smooth` — low-frequency gradient + small noise (image-like
//!   chroma locality).
//!
//! Plane geometry mirrors the decoder's padded planes (MCU-aligned
//! strides, ⌈w·h_i/h_max⌉ valid extents) so the measured memory
//! traffic carries the same padding overhead as a real decode.
//!
//! # Arms
//!
//! - `legacy` — the pre-W6-03 kernel, kept **verbatim** below (source:
//!   `crates/codecs/src/video/mjpeg/color.rs` @ merge commit 62c48b6,
//!   non-test code only). Frozen; never edited after this file lands.
//! - `current` — the live `color.rs`, included by `#[path]` so this
//!   example always benchmarks the exact shipped code (compiled in
//!   this example's crate — same source, same flags; the inclusion is
//!   how a `pub(crate)` kernel is benched without widening the
//!   library's public API).
//! - `scalar` — the live kernel's forced-scalar entry (`compose_row`),
//!   measuring the algorithmic gain alone.
//! - `simd` — the live kernel's SIMD entry when the CPU + build offer
//!   it (added with the W6-03 optimization; absent from the baseline
//!   phase binary — recorded here so the delta in binary contents
//!   between phases is explicit, not silent).
//! - `float_ref` — an f32 reference implementation (measurement-only;
//!   floats are NOT bit-exact with the shipped fixed-point path, so
//!   this arm is context, never a candidate default).
//!
//! # Protocol
//!
//! Per configuration: 2 untimed warm-up rounds, then 21 timed reps,
//! arms **interleaved** within each round (drift in machine load hits
//! all arms equally). Reported number = median of the 21. Every rep
//! is printed as TSV (raw data); medians as `MEDIAN` lines.
//!
//! Run: `cargo run --release -p codecs --example color_bench -- <phase>`
//! with `<phase>` a free-form tag recorded in every output line
//! (e.g. `baseline-1`, `after-2`).
//!
//! # Decode-share context
//!
//! The run closes with a whole-decode measurement (synthetic dense-AC
//! 4K frames through the public `decode_image` entry, N=15 medians)
//! so the isolated compose numbers can be read as an honest share of
//! full 4:2:2 / 4:4:4 decode time, not asserted from folklore.

use std::time::Instant;

/// The pre-W6-03 kernel, verbatim (see arm docs above for provenance).
/// ONLY its names live under this module; not one operator changed.
mod legacy {
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

        /// The hot color path: all rows proven, direct indexing, zero
        /// divisions, zero per-sample branches beyond the step carries.
        fn compose_color_row(
            &self,
            mut luma: RowSampler<'_>,
            mut cb: RowSampler<'_>,
            mut cr: RowSampler<'_>,
            width: usize,
            out: &mut [u8],
        ) {
            for x in 0..width {
                let y = luma.sample();
                let cbv = cb.sample();
                let crv = cr.sample();
                let [r, g, b] = ycc_to_rgb(y, cbv, crv);
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
}

/// The live kernel: `color.rs` itself, compiled into this example so
/// the benchmark always exercises the exact shipped source.
#[path = "../src/video/mjpeg/color.rs"]
mod now;

/// One arm's composer contract (name-split from the inherent methods
/// so trait dispatch can never shadow the kernel under measurement).
trait RowComposer {
    fn row(&self, row_y: usize, width: usize, out: &mut [u8]);
}

impl RowComposer for legacy::RgbComposer<'_> {
    #[inline]
    fn row(&self, row_y: usize, width: usize, out: &mut [u8]) {
        self.compose_row(row_y, width, out);
    }
}

impl RowComposer for now::RgbComposer<'_> {
    #[inline]
    fn row(&self, row_y: usize, width: usize, out: &mut [u8]) {
        self.compose_row(row_y, width, out);
    }
}

/// Plane content style.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fill {
    Random,
    Smooth,
}

impl Fill {
    fn tag(self) -> &'static str {
        match self {
            Fill::Random => "random",
            Fill::Smooth => "smooth",
        }
    }
}

/// Sampling shape under test.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sampling {
    Gray,
    X420,
    X422,
    X444,
}

impl Sampling {
    fn tag(self) -> &'static str {
        match self {
            Sampling::Gray => "gray",
            Sampling::X420 => "420",
            Sampling::X422 => "422",
            Sampling::X444 => "444",
        }
    }
    fn hv(self) -> (usize, usize) {
        match self {
            Sampling::Gray => (1, 1),
            Sampling::X420 => (2, 2),
            Sampling::X422 => (2, 1),
            Sampling::X444 => (1, 1),
        }
    }
}

/// Padded planar YCbCr data with decoder-mirroring geometry.
struct Planes {
    w: usize,
    h: usize,
    h_max: usize,
    v_max: usize,
    gray: bool,
    y: Vec<u8>,
    y_stride: usize,
    cb: Vec<u8>,
    cr: Vec<u8>,
    c_stride: usize,
}

/// Deterministic xorshift64* (seed printed by the run).
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
}

/// Build the padded planes for one configuration, mirroring
/// `decoder.rs`'s plane math (`block_cols·8` strides, ⌈w·h_i/h_max⌉
/// valid extents, MCU-padded row counts).
fn build_planes(w: usize, h: usize, sampling: Sampling, fill: Fill) -> Planes {
    let (h_max, v_max) = sampling.hv();
    let gray = sampling == Sampling::Gray;
    let mcu_w = w.div_ceil(8 * h_max);
    let mcu_h = h.div_ceil(8 * v_max);
    let y_stride = mcu_w * h_max * 8;
    let c_stride = mcu_w * 8;
    let y_rows = mcu_h * v_max * 8;
    let c_rows = mcu_h * 8;
    let mut y = vec![0u8; y_stride * y_rows];
    let c_len = c_stride * c_rows;
    let mut cb = vec![0u8; c_len];
    let mut cr = vec![0u8; c_len];
    let mut rng = Rng(0x5eed_0000_0000_0603);
    match fill {
        Fill::Random => {
            for slot in y.iter_mut() {
                *slot = (rng.next_u64() >> 24) as u8;
            }
            for slot in cb.iter_mut().chain(cr.iter_mut()) {
                *slot = (rng.next_u64() >> 24) as u8;
            }
        }
        Fill::Smooth => {
            // Image-like: luma carries structure, chroma is low-
            // frequency with small noise (the locality real decoded
            // chroma has).
            for row in 0..y_rows {
                for col in 0..y_stride {
                    let grad = (col * 3 + row * 5) % 199;
                    let noise = (rng.next_u64() >> 60) as u8;
                    y[row * y_stride + col] = (grad + noise as usize) as u8;
                }
            }
            for row in 0..c_rows {
                for col in 0..c_stride {
                    let band = ((col / 4 + row / 4) % 17) as i32 - 8;
                    let noise = (rng.next_u64() >> 62) as u8;
                    let v = 128 + band * 2 + noise as i32;
                    cb[row * c_stride + col] = v.clamp(0, 255) as u8;
                    let band2 = ((col / 6 + row / 3) % 13) as i32 - 6;
                    let v2 = 128 + band2 * 2 + noise as i32;
                    cr[row * c_stride + col] = v2.clamp(0, 255) as u8;
                }
            }
        }
    }
    if gray {
        // Grayscale through the composer: no chroma planes (None).
        cb.clear();
        cr.clear();
    }
    Planes {
        w,
        h,
        h_max,
        v_max,
        gray,
        y,
        y_stride,
        cb,
        cr,
        c_stride,
    }
}

/// Compose one full frame through a module's `RgbComposer` (the exact
/// `assemble_rgba_into` loop shape: one composer, one row call per
/// output row). Returns a checksum over the output (DCE guard +
/// cross-arm equality evidence).
fn compose_frame<C: RowComposer>(composer: &C, w: usize, h: usize, out: &mut [u8]) -> u64 {
    for row_y in 0..h {
        let start = row_y * w * 4;
        let row = &mut out[start..start + w * 4];
        composer.row(row_y, w, row);
    }
    let mut sum = 0u64;
    for (i, byte) in out.iter().enumerate().step_by(4097) {
        sum = sum.wrapping_add(u64::from(*byte).wrapping_mul(i as u64 & 0xff));
    }
    sum
}

/// The f32 reference arm (measurement-only; NOT the shipped path —
/// float rounding cannot be bit-exact with the Q16 fixed point).
struct FloatComposer<'a> {
    y: &'a [u8],
    y_stride: usize,
    cb: Option<(&'a [u8], usize)>,
    cr: Option<(&'a [u8], usize)>,
    h_max: usize,
    v_max: usize,
}

impl RowComposer for FloatComposer<'_> {
    /// Structurally matched to the optimized fixed-point kernels
    /// (fused chroma pairs, no per-pixel modulo, hoisted row bases) so
    /// the int-vs-float comparison isolates the *formula*, not the
    /// loop shape.
    #[inline]
    fn row(&self, row_y: usize, width: usize, out: &mut [u8]) {
        let y_row = &self.y[row_y * self.y_stride..];
        let stride_c = self.c_stride();
        let (cb_row, cr_row): (&[u8], &[u8]) = match (self.cb, self.cr) {
            (Some((cb, _)), Some((cr, _))) => {
                let base = row_y / self.v_max * stride_c;
                (&cb[base..], &cr[base..])
            }
            _ => (&[][..], &[][..]),
        };
        let has_chroma = self.cb.is_some();
        if !has_chroma {
            let pxs = out.as_chunks_mut::<4>().0;
            for (px, y) in pxs.iter_mut().zip(y_row.iter()).take(width) {
                let y = *y;
                px[0] = y;
                px[1] = y;
                px[2] = y;
                px[3] = 255;
            }
            return;
        }
        if self.h_max == 2 {
            let pairs = width / 2;
            let pair_chunks = out.as_chunks_mut::<8>().0;
            for (i, pair) in pair_chunks.iter_mut().take(pairs).enumerate() {
                let cb = f32::from(cb_row[i]) - 128.0;
                let cr = f32::from(cr_row[i]) - 128.0;
                let (r, g, b) = (1.402 * cr, -0.344_136 * cb - 0.714_136 * cr, 1.772 * cb);
                let y0 = f32::from(y_row[2 * i]);
                let y1 = f32::from(y_row[2 * i + 1]);
                let (pa, pb) = pair.split_at_mut(4);
                pa[0] = (y0 + r).clamp(0.0, 255.0).round() as u8;
                pa[1] = (y0 + g).clamp(0.0, 255.0).round() as u8;
                pa[2] = (y0 + b).clamp(0.0, 255.0).round() as u8;
                pa[3] = 255;
                pb[0] = (y1 + r).clamp(0.0, 255.0).round() as u8;
                pb[1] = (y1 + g).clamp(0.0, 255.0).round() as u8;
                pb[2] = (y1 + b).clamp(0.0, 255.0).round() as u8;
                pb[3] = 255;
            }
            if width % 2 == 1 {
                let i = pairs;
                let cb = f32::from(cb_row[i]) - 128.0;
                let cr = f32::from(cr_row[i]) - 128.0;
                let y = f32::from(y_row[2 * i]);
                let base = width * 4 - 4;
                if let Some(px) = out.get_mut(base..base + 4) {
                    px[0] = (y + 1.402 * cr).clamp(0.0, 255.0).round() as u8;
                    px[1] = (y - 0.344_136 * cb - 0.714_136 * cr)
                        .clamp(0.0, 255.0)
                        .round() as u8;
                    px[2] = (y + 1.772 * cb).clamp(0.0, 255.0).round() as u8;
                    px[3] = 255;
                }
            }
        } else {
            let pxs = out.as_chunks_mut::<4>().0;
            for (i, px) in pxs.iter_mut().take(width).enumerate() {
                let cb = f32::from(cb_row[i]) - 128.0;
                let cr = f32::from(cr_row[i]) - 128.0;
                let y = f32::from(y_row[i]);
                px[0] = (y + 1.402 * cr).clamp(0.0, 255.0).round() as u8;
                px[1] = (y - 0.344_136 * cb - 0.714_136 * cr)
                    .clamp(0.0, 255.0)
                    .round() as u8;
                px[2] = (y + 1.772 * cb).clamp(0.0, 255.0).round() as u8;
                px[3] = 255;
            }
        }
    }
}

impl FloatComposer<'_> {
    fn c_stride(&self) -> usize {
        self.cb.map_or(0, |(_, stride)| stride)
    }
}

fn median(samples: &mut [u128]) -> u128 {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn print_machine_header(phase: &str) {
    let model = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|line| line.starts_with("model name"))
                .and_then(|line| line.split(':').nth(1).map(|v| v.trim().to_string()))
        })
        .unwrap_or_else(|| "unknown".to_string());
    let load = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let load = load
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");
    println!(
        "# cpu=\"{model}\" cores={}",
        std::thread::available_parallelism().map_or(0, |n| n.get())
    );
    println!("# loadavg_start=\"{load}\"");
    println!("# seed=0x5eed000000000603 reps=21 warmup=2 interleaved");
    println!("# REP columns: tag phase arm fill WxH sampling rep_idx ns");
    println!(
        "# MEDIAN columns: tag phase arm fill WxH sampling median_ns best_ns bitexact_vs_legacy"
    );
    println!("# phase={phase}");
}

fn print_load_footer() {
    let load = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let load = load
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");
    println!("# loadavg_end=\"{load}\"");
}

/// One benchmark arm: builds a fresh composer per rep (the per-frame
/// construction shape) and composes the full frame.
type ArmRun<'a> = Box<dyn FnMut(&mut [u8]) + 'a>;

struct Arm<'a> {
    name: &'static str,
    run: ArmRun<'a>,
}

fn main() {
    let phase = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "unlabeled".to_string());
    print_machine_header(&phase);
    let sizes: [(usize, usize); 3] = [(640, 480), (1280, 720), (3840, 2160)];
    let samplings = [
        Sampling::Gray,
        Sampling::X420,
        Sampling::X422,
        Sampling::X444,
    ];
    let fills = [Fill::Random, Fill::Smooth];
    const REPS: usize = 21;
    const WARMUP: usize = 2;

    for &sampling in &samplings {
        for &fill in &fills {
            for &(w, h) in &sizes {
                let planes = build_planes(w, h, sampling, fill);
                let out_len = w * h * 4;
                let mut reference = vec![0u8; out_len];
                let mut scratch = vec![0u8; out_len];

                // Build the arms. `legacy` and `current` first; `scalar`
                // and `simd` are added by the W6-03 optimization (their
                // absence in the baseline binary is a recorded protocol
                // fact, not a silent difference).
                let mut arms: Vec<Arm<'_>> = Vec::new();
                arms.push(Arm {
                    name: "legacy",
                    run: Box::new(|out: &mut [u8]| {
                        let composer = build_legacy(&planes, w, h);
                        compose_frame(&composer, w, h, out);
                    }),
                });
                arms.push(Arm {
                    name: "current",
                    run: Box::new(|out: &mut [u8]| {
                        let composer = build_now(&planes, w, h);
                        compose_frame(&composer, w, h, out);
                    }),
                });
                arms.push(Arm {
                    name: "scalar",
                    run: Box::new(|out: &mut [u8]| {
                        let composer = build_now(&planes, w, h);
                        compose_frame_scalar(&composer, w, h, out);
                    }),
                });
                if now::simd_available() {
                    arms.push(Arm {
                        name: "simd",
                        run: Box::new(|out: &mut [u8]| {
                            let composer = build_now(&planes, w, h);
                            compose_frame_simd(&composer, w, h, out);
                        }),
                    });
                }
                arms.push(Arm {
                    name: "float_ref",
                    run: Box::new(|out: &mut [u8]| {
                        let composer = build_float(&planes);
                        compose_frame(&composer, w, h, out);
                    }),
                });

                // Bit-exactness evidence on the bench data itself: every
                // fixed-point arm must reproduce `legacy` byte-for-byte
                // (float_ref is exempt — different rounding by design).
                (arms[0].run)(&mut reference);
                let mut checks = Vec::new();
                for arm in &mut arms {
                    if arm.name == "float_ref" {
                        continue;
                    }
                    scratch.fill(0xAA);
                    (arm.run)(&mut scratch);
                    checks.push(format!("{}={}", arm.name, scratch == reference));
                }
                let checks = checks.join(",");

                let mut times: Vec<Vec<u128>> = vec![Vec::new(); arms.len()];
                for rep in 0..(REPS + WARMUP) {
                    for (i, arm) in arms.iter_mut().enumerate() {
                        scratch.fill(0xAA);
                        let start = Instant::now();
                        (arm.run)(&mut scratch);
                        let elapsed = start.elapsed().as_nanos();
                        if rep >= WARMUP {
                            times[i].push(elapsed);
                        }
                    }
                }
                for (i, arm) in arms.iter().enumerate() {
                    for (rep, &ns) in times[i].iter().enumerate() {
                        println!(
                            "REP\t{phase}\t{}\t{}\t{}x{}\t{}\t{}\t{ns}",
                            arm.name,
                            fill.tag(),
                            w,
                            h,
                            sampling.tag(),
                            rep
                        );
                    }
                    let mut samples = times[i].clone();
                    let med = median(&mut samples);
                    let best = times[i].iter().min().unwrap_or(&0);
                    println!(
                        "MEDIAN\t{phase}\t{}\t{}\t{}x{}\t{}\t{med}\t{best}\t{checks}",
                        arm.name,
                        fill.tag(),
                        w,
                        h,
                        sampling.tag()
                    );
                }
            }
        }
    }
    decode_share(&phase);
    print_load_footer();
}

/// Composer construction, per module (the modules have distinct types;
/// the duplication is the `#[path]`-include price).
fn build_legacy<'a>(p: &'a Planes, _w: usize, _h: usize) -> legacy::RgbComposer<'a> {
    let geom_y = legacy::ComponentGeom {
        h: p.h_max,
        v: p.v_max,
        valid_w: p.w,
        valid_h: p.h,
    };
    let c_w = p.w.div_ceil(p.h_max);
    let c_h = p.h.div_ceil(p.v_max);
    let geom_c = legacy::ComponentGeom {
        h: 1,
        v: 1,
        valid_w: c_w,
        valid_h: c_h,
    };
    let chroma = if p.gray {
        None
    } else {
        Some((
            legacy::PlaneView {
                data: &p.cb,
                stride: p.c_stride,
            },
            legacy::PlaneView {
                data: &p.cr,
                stride: p.c_stride,
            },
        ))
    };
    legacy::RgbComposer::new(
        legacy::PlaneView {
            data: &p.y,
            stride: p.y_stride,
        },
        chroma,
        [geom_y, geom_c, geom_c],
        p.h_max,
        p.v_max,
    )
}

fn build_now<'a>(p: &'a Planes, _w: usize, _h: usize) -> now::RgbComposer<'a> {
    let geom_y = now::ComponentGeom {
        h: p.h_max,
        v: p.v_max,
        valid_w: p.w,
        valid_h: p.h,
    };
    let c_w = p.w.div_ceil(p.h_max);
    let c_h = p.h.div_ceil(p.v_max);
    let geom_c = now::ComponentGeom {
        h: 1,
        v: 1,
        valid_w: c_w,
        valid_h: c_h,
    };
    let chroma = if p.gray {
        None
    } else {
        Some((
            now::PlaneView {
                data: &p.cb,
                stride: p.c_stride,
            },
            now::PlaneView {
                data: &p.cr,
                stride: p.c_stride,
            },
        ))
    };
    now::RgbComposer::new(
        now::PlaneView {
            data: &p.y,
            stride: p.y_stride,
        },
        chroma,
        [geom_y, geom_c, geom_c],
        p.h_max,
        p.v_max,
    )
}

fn build_float<'a>(p: &'a Planes) -> FloatComposer<'a> {
    FloatComposer {
        y: &p.y,
        y_stride: p.y_stride,
        cb: if p.gray {
            None
        } else {
            Some((&p.cb, p.c_stride))
        },
        cr: if p.gray {
            None
        } else {
            Some((&p.cr, p.c_stride))
        },
        h_max: p.h_max,
        v_max: p.v_max,
    }
}

/// Forced-scalar full frame (live kernel; `compose_row` dispatch may
/// pick SIMD — this arm pins the scalar path).
fn compose_frame_scalar(
    composer: &now::RgbComposer<'_>,
    w: usize,
    h: usize,
    out: &mut [u8],
) -> u64 {
    for row_y in 0..h {
        let start = row_y * w * 4;
        let row = &mut out[start..start + w * 4];
        composer.compose_row_scalar(row_y, w, row);
    }
    checksum(out)
}

/// Forced-SIMD full frame (live kernel; no-op mapping until the SIMD
/// entry exists — added with the W6-03 optimization).
fn compose_frame_simd(composer: &now::RgbComposer<'_>, w: usize, h: usize, out: &mut [u8]) -> u64 {
    for row_y in 0..h {
        let start = row_y * w * 4;
        let row = &mut out[start..start + w * 4];
        composer.compose_row_simd(row_y, w, row);
    }
    checksum(out)
}

fn checksum(out: &[u8]) -> u64 {
    let mut sum = 0u64;
    for (i, byte) in out.iter().enumerate().step_by(4097) {
        sum = sum.wrapping_add(u64::from(*byte).wrapping_mul(i as u64 & 0xff));
    }
    sum
}

/// Whole-decode context: synthetic dense-AC 4K frames through the
/// public `decode_image` entry, N=15 medians, so the isolated compose
/// numbers can be stated as a share of real full-decode time.
fn decode_share(phase: &str) {
    for (tag, h_sampling, v_sampling) in [("422", 2usize, 1usize), ("444", 1usize, 1usize)] {
        let jpeg = dense_4k_jpeg(h_sampling, v_sampling);
        let image = match codecs::video::mjpeg::decode_image(&jpeg) {
            Ok(image) => image,
            Err(e) => unreachable!("dense 4K {tag} frame decodes: {e}"),
        };
        if image.concealed {
            unreachable!("dense 4K {tag} frame must not conceal");
        }
        // Warm-up + timed passes.
        let mut times = Vec::new();
        for rep in 0..17 {
            let start = Instant::now();
            let decoded = match codecs::video::mjpeg::decode_image(&jpeg) {
                Ok(image) => image,
                Err(e) => unreachable!("timed decode: {e}"),
            };
            let elapsed = start.elapsed().as_nanos();
            if rep >= 2 && decoded.data.len() == image.data.len() {
                times.push(elapsed);
            }
        }
        let full = median(&mut times);

        // Isolated compose on the same geometry, current kernel.
        let sampling = match (h_sampling, v_sampling) {
            (2, 1) => Sampling::X422,
            _ => Sampling::X444,
        };
        let planes = build_planes(3840, 2160, sampling, Fill::Smooth);
        let mut out = vec![0u8; 3840 * 2160 * 4];
        let mut compose_times = Vec::new();
        for rep in 0..17 {
            let composer = build_now(&planes, 3840, 2160);
            let start = Instant::now();
            compose_frame(&composer, 3840, 2160, &mut out);
            let elapsed = start.elapsed().as_nanos();
            if rep >= 2 {
                compose_times.push(elapsed);
            }
        }
        let compose = median(&mut compose_times);
        let share = compose as f64 / full as f64 * 100.0;
        println!(
            "SHARE\t{phase}\t{tag}\tfull_decode_ns={full}\tcompose_ns={compose}\tshare_pct={share:.1}"
        );
    }
}

/// Synthetic dense-AC 4K JPEG (the `benches.rs` construction: every
/// block carries 63 non-zero ACs — the content-density worst case),
/// parameterized by luma sampling (h, v).
fn dense_4k_jpeg(h_sampling: usize, v_sampling: usize) -> Vec<u8> {
    const WIDTH: u16 = 3840;
    const HEIGHT: u16 = 2160;
    let mut jpeg = vec![0xff, 0xd8];
    let seg = |jpeg: &mut Vec<u8>, marker: u8, payload: &[u8]| {
        jpeg.push(0xff);
        jpeg.push(marker);
        jpeg.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        jpeg.extend_from_slice(payload);
    };
    // DQT: Tq=0, all 16s.
    let mut dqt = vec![0u8];
    dqt.extend(std::iter::repeat_n(16u8, 64));
    seg(&mut jpeg, 0xdb, &dqt);
    // DHT DC0: two 1-bit codes, both size-2.
    let mut dht = vec![0x00u8, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    dht.extend(&[0x02, 0x02]);
    seg(&mut jpeg, 0xc4, &dht);
    // DHT AC0: two 1-bit codes, both (run 0, size 1).
    let mut dht = vec![0x10u8, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    dht.extend(&[0x01, 0x01]);
    seg(&mut jpeg, 0xc4, &dht);
    // SOF0: 4K, sampling (h, v) for luma, 1x1 chroma.
    let mut sof = vec![8u8];
    sof.extend(HEIGHT.to_be_bytes());
    sof.extend(WIDTH.to_be_bytes());
    sof.push(3);
    sof.extend([1u8, (h_sampling as u8) << 4 | v_sampling as u8, 0]);
    sof.extend([2u8, 0x11, 0]);
    sof.extend([3u8, 0x11, 0]);
    seg(&mut jpeg, 0xc0, &sof);
    // SOS: all three components, Td=0 Ta=0.
    seg(&mut jpeg, 0xda, &[3, 1, 0x00, 2, 0x00, 3, 0x00, 0, 63, 0]);
    // Entropy: 129 bits per block over the MCU grid.
    let mcu_w = 3840usize.div_ceil(8 * h_sampling);
    let mcu_h = 2160usize.div_ceil(8 * v_sampling);
    let blocks = mcu_w * mcu_h * (h_sampling * v_sampling + 2);
    let entropy_bytes = blocks * 129 / 8 + 1;
    jpeg.resize(jpeg.len() + entropy_bytes, 0x55);
    jpeg.extend(&[0xff, 0xd9]);
    jpeg
}
