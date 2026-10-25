//! The baseline scan decoder and output assembly (card 2.B4 core):
//! MCU walk, Huffman entropy decode, DC prediction, restart markers,
//! concealment, and the I420 / RGBA output paths.
//!
//! # Concelment policy (documented)
//!
//! Once the marker walk (parser) succeeds, the frame has enough
//! structure to be emitted honestly. Entropy-level damage — data
//! exhaustion, a Huffman code absent from the table, a coefficient
//! run past 63, a missing/misnumbered restart marker — does not fail
//! the frame: the undecodable blocks **from the failure point on**
//! are filled with mid-gray (the IDCT of zero coefficients, matching
//! libjpeg's missing-MCU behavior), the frame is emitted, and the
//! caller is told via `concealed = true` (the contract surfaces this
//! as `DecodeOutcome::Recovered { ConcealedFrame }` — bounded,
//! observable, never silent).
//!
//! Header-level damage (no SOI, no SOF, bad segments) emits nothing
//! and returns a typed error: fabricating a frame from nothing would
//! not be concealment. That error is still **per-frame** (doc 03
//! §4.5(d)'s `CorruptPacket` contract): the context holds no
//! inter-frame codec state, so the next packet decodes normally — the
//! engine's decode slice isolates the damaged frame (repeat-last-
//! frame concealment, damage events, bounded failure budget) instead
//! of tearing the session down (audit M9 / ED-FIX-4).
//!
//! # Decode context (ED-FIX-1, audit C4; W2-06 hardening)
//!
//! Each frame still decodes **independently** (MJPEG is one complete
//! JPEG per packet — there is no inter-frame codec state), but the
//! working memory is now held by [`DecodeContext`] and reused across
//! frames instead of being rebuilt per packet (audit C4: three fresh
//! multi-MB planes + four Huffman LUT rebuilds + crop intermediates
//! per frame ≈ 190 MB/s allocation churn at 1080p60, all of it pure
//! waste under D-1):
//!
//! - the **marker walk itself** reuses a persistent `JpegInfo` scratch
//!   (`parser::parse_into`): the component list, the scan list and the
//!   HUFFVAL storage are cleared and refilled in place, so the
//!   per-frame header parse allocates nothing either (W2-06 — the
//!   walk used to build two fresh metadata `Vec`s plus one `to_vec`
//!   per DHT table, every frame);
//! - the padded per-component planes are reallocated **only when the
//!   MCU geometry changes** (sampling factors × MCU grid × frame
//!   size). Reuse without clearing is safe and bit-exact because the
//!   parser proves the single interleaved scan covers *every* frame
//!   component (`Ns == Nf`, `parse_sos`), so every block of every
//!   plane is rewritten each frame — decoded blocks by
//!   [`Plane::write_block`], the concealed tail by
//!   [`Plane::gray_blocks`];
//! - the Huffman decoders (with their 8-bit look-ahead LUTs) are
//!   rebuilt **only when the DHT payload changes**, detected by an
//!   **allocation-free structural compare** of the raw BITS/HUFFVAL
//!   against the last-built key ([`DhtKey::matches`]; the key and its
//!   payload snapshot happen only on an actual change — W2-06: the
//!   pre-hardening path cloned the whole DHT payload every frame just
//!   to run the comparison, contradicting the count below);
//! - the per-component cursors are capacity-reused the same way;
//! - I420 output is assembled **in place**: each plane's valid region
//!   is cropped row-by-row directly into its offset of the output
//!   buffer (`Y ‖ Cb ‖ Cr`), with no intermediate crop `Vec` and no
//!   second copy (the pre-FIX-1 path built one temp `Vec` per plane
//!   and copied every byte twice).
//!
//! # Output handoff (W6-02, registered violation V-4 fixed)
//!
//! The decode is split in two: [`DecodeContext::decode_scan`] walks
//! headers + entropy into the reusable planes and returns the frame
//! metadata; [`DecodeContext::frame_len`] +
//! [`DecodeContext::assemble_frame`] then write the pixel bytes into
//! a caller-provided buffer. The contract path
//! ([`super::MjpegDecoder`]) points that buffer at a **pooled
//! `SharedBytes` slot** ([`FRAME_SLOTS`], the PCM decoder's
//! `CONVERT_SLOTS` pattern lifted to the video side): a warm slot of
//! exactly the frame's byte count is refilled in place through
//! `get_mut` and handed out as an `Arc` clone, so the steady-state
//! handoff allocates **nothing** — the pre-W6-02 path took the
//! context's `Vec` (`take_output`, leaving capacity 0), resized a
//! fresh buffer (zero-filling bytes the assembly overwrites anyway)
//! and copied it again into the `SharedBytes` `Arc` block: 2 allocs
//! + ~2 frames of wasted writes per frame (W5-15's registered V-4).
//!
//! Both assembly paths (I420 crop, RGBA compose) provably overwrite
//! **every** byte of the output buffer — pinned by the
//! stale-slot-poisoning differential tests in this module — so reuse
//! without zeroing cannot leak a previous frame's pixels. Bounded
//! memory: at most [`FRAME_SLOTS`] · current-frame-bytes (empty on
//! the one-shot stateless path, which keeps its own `Vec`).

use super::FieldPolarity;
use super::bitreader::{BitReader, ReaderStop};
use super::color::{ComponentGeom, PlaneView, RgbComposer};
use super::error::MjpegError;
use super::huffman::{HuffmanDecoder, Stop};
use super::idct_simd::idct_block_dispatch;
use super::parser::{self, ComponentInfo, JpegInfo, ZIGZAG};
use media_types::packet::SharedBytes;

/// Pixel-count cap — the fuzz-safety memory bound.
///
/// 8 MiP (8_388_608 pixels, e.g. 3840×2176 — full UHD) keeps the
/// worst-case decode footprint (padded component planes + a 4-byte
/// RGBA output) under the corpus' 64 MiB rss budget
/// `[PROPOSAL: the number; the invariant is the bound itself]`.
pub(crate) const MAX_PIXELS: u64 = 8_388_608;

/// Output format of a decoded frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    /// Planar 4:2:0: `Y (w×h) || Cb (⌈w/2⌉×⌈h/2⌉) || Cr` — codec-native
    /// (doc 01 §3.3: I420 is "the codec-native default"), zero
    /// conversion for 4:2:0 streams and the neutral-chroma
    /// representation of grayscale.
    I420,
    /// Interleaved RGBA (straight alpha, 0xff), for samplings with no
    /// planar vocabulary yet (4:4:4 / 4:2:2 / 4:4:0).
    Rgba8,
}

/// Why entropy decoding stopped early (concealment observability —
/// doc 03 §4.5(d): recovery is bounded *and observable*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConcealCause {
    /// The entropy-coded data ended before the scan completed
    /// (truncation).
    EndOfData,
    /// A marker terminated the scan before all MCUs were decoded.
    PrematureMarker(u8),
    /// A Huffman code absent from the table / a coefficient run past
    /// 63.
    InvalidCode,
    /// A restart marker was missing or misnumbered.
    RestartMismatch,
}

/// A decoded frame (internal form; `mod.rs` lifts it into `Frame`).
pub(crate) struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub format: OutputFormat,
    pub data: Vec<u8>,
    /// Entropy-level damage was concealed (see module docs).
    pub concealed: bool,
    /// The damage class when `concealed`.
    pub cause: Option<ConcealCause>,
    /// AVI1 APP0 field polarity, when the frame carried the marker
    /// (card 5.H3; see `mod.rs` module docs for the handling).
    pub field_polarity: Option<FieldPolarity>,
}

/// The frame metadata of a context decode: the pixel bytes stay in
/// the context's output buffer until [`DecodeContext::take_output`]
/// moves them out (callers that consume in place can read
/// [`DecodeContext::output`] without any handoff allocation).
#[derive(Debug, Clone, Copy)]
pub(crate) struct DecodedMeta {
    pub width: u32,
    pub height: u32,
    pub format: OutputFormat,
    pub concealed: bool,
    pub cause: Option<ConcealCause>,
    /// AVI1 APP0 field polarity, when the frame carried the marker
    /// (card 5.H3).
    pub field_polarity: Option<FieldPolarity>,
}

/// One padded per-component sample plane.
#[derive(Debug)]
pub(crate) struct Plane {
    data: Vec<u8>,
    /// Padded row pitch (bytes).
    stride: usize,
    /// Blocks per padded row (MCU grid width × h_i).
    block_cols: usize,
    /// Padded block rows (MCU grid height × v_i).
    block_rows: usize,
    /// Valid sample dimensions (component width/height).
    valid_w: usize,
    valid_h: usize,
}

impl Plane {
    /// Write one decoded block's samples at block position
    /// `(col, row)`.
    fn write_block(&mut self, samples: &[u8; 64], col: usize, row: usize) {
        let x0 = col * 8;
        let y0 = row * 8;
        for y in 0..8 {
            let Some(dst_row) = self.data.get_mut((y0 + y) * self.stride..) else {
                return;
            };
            let Some(dst) = dst_row.get_mut(x0..x0 + 8) else {
                return;
            };
            if let Some(src) = samples.get(y * 8..y * 8 + 8) {
                dst.copy_from_slice(src);
            }
        }
    }

    /// Concealment: fill `from_block..total` with mid-gray (128 = the
    /// IDCT of zero coefficients).
    fn gray_blocks(&mut self, from_block: usize) {
        let total = self.block_cols * self.block_rows;
        for index in from_block..total {
            let col = index % self.block_cols;
            let row = index / self.block_cols;
            let x0 = col * 8;
            let y0 = row * 8;
            for y in 0..8 {
                let Some(dst_row) = self.data.get_mut((y0 + y) * self.stride..) else {
                    return;
                };
                if let Some(dst) = dst_row.get_mut(x0..x0 + 8) {
                    dst.fill(128);
                }
            }
        }
    }
}

/// The identity of the padded-plane working set: everything that
/// determines plane sizes. Equal keys ⇒ byte-identical plane
/// layouts, so the buffers are reused as-is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlaneKey {
    /// Per-component sampling factors `(h_i, v_i)` in SOF order,
    /// zero-padded (component count ≤ 3 by the earlier gate).
    sampling: [(u8, u8); 4],
    /// MCU grid dimensions.
    mcu_w: usize,
    mcu_h: usize,
    /// Frame dimensions.
    width: usize,
    height: usize,
}

/// Per-block compute stages of the scan walk, as replaceable hooks
/// (the W7-b profiling seam — the ablation instrument behind
/// `examples/stage_bench.rs`).
///
/// The production walk always runs [`FullStages`], whose impls are
/// `#[inline]` one-line forwardings to the real stages, so after
/// monomorphization the production instantiation is codegen-identical
/// to the pre-seam body (the seam adds no branch, no call). The
/// stubbed impls exist so the **same walk, on the same bitstream**,
/// can be timed with one compute stage removed: the Huffman entropy
/// decode depends only on the tables and the reader, so
/// `decode_scan − decode_scan_with::<NoIdctStages>` is exactly the
/// dequant+IDCT share of a real decode (same for the plane store via
/// [`NoWriteStages`]). Bit-exactness of the full-stages instantiation
/// against the production path is pinned by a test below; the stubs
/// deliberately alter output and are therefore never callable from
/// the contract path.
pub(crate) trait ScanHooks {
    /// Dequantize + inverse-DCT one block into `out` samples
    /// (default: the real transform via [`idct_block_dispatch`] —
    /// SSE4.1 SIMD pass 1 where the CPU has it, scalar otherwise,
    /// bit-exact either way per `idct_simd`'s bounds proof).
    #[inline]
    fn transform(block: &[i16; 64], qt: &[u16; 64], out: &mut [u8]) {
        idct_block_dispatch(block, qt, out)
    }

    /// Store one block's samples into its plane slot (default: the
    /// real [`Plane::write_block`]).
    #[inline]
    fn store(plane: &mut Plane, samples: &[u8; 64], col: usize, row: usize) {
        plane.write_block(samples, col, row)
    }
}

/// The production stage set: all stages live (the trait defaults).
pub(crate) struct FullStages;

impl ScanHooks for FullStages {}

/// Profiling arm: the dequant+IDCT stubbed to a constant mid-gray
/// fill (the store still runs, so plane-write traffic is measured in
/// this arm). Constructed only by `examples/stage_bench.rs` (and the
/// seam test below) — the lib itself never stubs a stage.
#[allow(dead_code)] // profiling arm: constructed by examples/stage_bench.rs
pub(crate) struct NoIdctStages;

impl ScanHooks for NoIdctStages {
    #[inline]
    fn transform(_block: &[i16; 64], _qt: &[u16; 64], out: &mut [u8]) {
        out.fill(128)
    }
}

/// Profiling arm: the plane store stubbed to nothing but a
/// `black_box` anchor (the transform still runs, so its compute is
/// measured; the anchor keeps the optimizer from dead-code-eliminating
/// the transform whose output is no longer stored).
#[allow(dead_code)] // profiling arm: constructed by examples/stage_bench.rs
pub(crate) struct NoWriteStages;

impl ScanHooks for NoWriteStages {
    #[inline]
    fn store(_plane: &mut Plane, samples: &[u8; 64], _col: usize, _row: usize) {
        std::hint::black_box(samples);
    }
}

/// Profiling arm: both the transform and the store stubbed — the
/// walk is parse + Huffman entropy decode only.
#[allow(dead_code)] // profiling arm: constructed by examples/stage_bench.rs
pub(crate) struct NoIdctNoWriteStages;

impl ScanHooks for NoIdctNoWriteStages {
    #[inline]
    fn transform(_block: &[i16; 64], _qt: &[u16; 64], out: &mut [u8]) {
        out.fill(128)
    }

    #[inline]
    fn store(_plane: &mut Plane, samples: &[u8; 64], _col: usize, _row: usize) {
        std::hint::black_box(samples);
    }
}

/// The identity of the Huffman table set: the raw DHT payload. Equal
/// keys ⇒ identical decode tables, so the built decoders (including
/// their look-ahead LUTs) are reused as-is.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct DhtKey {
    present: [[bool; 4]; 2],
    bits: [[[u8; 17]; 4]; 2],
    vals: [[Vec<u8>; 4]; 2],
}

impl DhtKey {
    /// Structural compare against the borrowed DHT payload — the
    /// per-frame hot path (W2-06). Allocation-free by construction:
    /// the fixed arrays compare field-wise and the `HUFFVAL` `Vec`s
    /// compare *contents* through `==` (no clone, no hash, no fresh
    /// key — the pre-W2-06 `DhtKey::of`-per-frame clone is gone).
    fn matches(&self, info: &JpegInfo) -> bool {
        self.present == info.huff_present
            && self.bits == info.huff_bits
            && self.vals == info.huff_vals
    }

    /// Snapshot the DHT payload — taken **only on a table-set
    /// change** (never on the per-frame compare path), so the clone
    /// here is a once-per-stream cost, not churn.
    fn of(info: &JpegInfo) -> DhtKey {
        DhtKey {
            present: info.huff_present,
            bits: info.huff_bits,
            vals: info.huff_vals.clone(),
        }
    }

    /// Refill the snapshot in place (V-3,
    /// docs/perf/memory-audit-20261023.md): the `HUFFVAL` `Vec`s
    /// reuse their allocations (`clear` + `extend`; capacity bounded
    /// by the 256-symbol DHT limit), so a per-frame DHT change on a
    /// persistent context no longer allocates for the key either.
    /// Semantically `*self = DhtKey::of(info)`.
    fn update_from(&mut self, info: &JpegInfo) {
        self.present = info.huff_present;
        self.bits = info.huff_bits;
        for class in 0..2 {
            for td in 0..4 {
                let dst = &mut self.vals[class][td];
                dst.clear();
                dst.extend_from_slice(&info.huff_vals[class][td]);
            }
        }
    }
}

/// Reusable decode state across frames (see module docs). Holds no
/// inter-frame *codec* state — every field is scratch whose contents
/// are fully rewritten per frame — only the allocations persist.
#[derive(Debug)]
pub(crate) struct DecodeContext {
    /// Padded per-component planes (geometry-keyed reuse).
    planes: Vec<Plane>,
    plane_key: Option<PlaneKey>,
    /// Huffman decoders per (class, destination), built from the DHT
    /// payload (DHT-keyed reuse).
    dc_tables: [Option<HuffmanDecoder>; 4],
    ac_tables: [Option<HuffmanDecoder>; 4],
    dht_key: Option<DhtKey>,
    /// The persistent header scratch: the per-frame marker walk
    /// (`parser::parse_into`) refills it in place — components, scan
    /// list and HUFFVAL storage reuse their allocations (D-1, W2-06).
    headers: JpegInfo,
    /// Per-component decoded-block cursors (concealment fill points).
    cursors: Vec<usize>,
    /// The output assembly buffer (I420 crop / RGBA compose target);
    /// moved out per frame by [`DecodeContext::take_output`].
    out: Vec<u8>,
    /// Diagnostics: plane-set rebuilds / table-set rebuilds seen so
    /// far (steady state = one each per stream; observable by the
    /// reuse tests).
    pub(crate) plane_builds: u32,
    pub(crate) table_builds: u32,
}

impl Default for DecodeContext {
    fn default() -> DecodeContext {
        DecodeContext {
            planes: Vec::new(),
            plane_key: None,
            dc_tables: [const { None }; 4],
            ac_tables: [const { None }; 4],
            dht_key: None,
            headers: JpegInfo::empty(),
            cursors: Vec::new(),
            out: Vec::new(),
            plane_builds: 0,
            table_builds: 0,
        }
    }
}

impl DecodeContext {
    /// A fresh, empty context (no allocations until first decode).
    pub(crate) fn new() -> DecodeContext {
        DecodeContext::default()
    }

    /// Decode one complete JPEG into this context.
    ///
    /// On success the frame's pixel bytes are in
    /// [`DecodeContext::output`] (or moved out by
    /// [`DecodeContext::take_output`]); the metadata is returned
    /// here. Semantics are identical to the stateless [`decode`]
    /// (pinned bit-exact by the committed reference digests, which
    /// run 60 consecutive frames through one context).
    ///
    /// # Errors
    /// Typed [`MjpegError`]s, exactly as the stateless path.
    pub(crate) fn decode(&mut self, data: &[u8]) -> Result<DecodedMeta, MjpegError> {
        let meta = self.decode_scan(data)?;
        let total = self.frame_len()?;
        // 5.X1b: the same-length buffer is reused WITHOUT the zero
        // fill — both assembly paths provably overwrite every byte
        // (pinned by `pooled_slot_refill_is_bit_exact_and_stale_byte_safe`),
        // so `clear + resize(total, 0)` was a full-frame memset per
        // frame (measured ~20% of the heavy-lane full-decode time on
        // its own). `resize` still zeroes any grown tail; shrinking
        // truncates.
        self.out.resize(total, 0);
        let Self {
            headers,
            planes,
            out,
            ..
        } = self;
        assemble_frame_into(headers, planes, out)?;
        Ok(meta)
    }

    /// Walk the headers and decode the entropy scan into the
    /// reusable planes **without** assembling the output (the pooled
    /// handoff's first half, W6-02 / V-4): the caller reads
    /// [`DecodeContext::frame_len`], obtains a destination buffer of
    /// exactly that length (a pooled [`SharedBytes`] slot on the
    /// contract path), and fills it via
    /// [`DecodeContext::assemble_frame`]. The walked `JpegInfo`
    /// stays parked in `self.headers` for those two calls.
    ///
    /// # Errors
    /// Typed [`MjpegError`]s, exactly like [`DecodeContext::decode`]
    /// (the context remains usable for the next frame either way —
    /// the scratch is restored before any error escapes).
    pub(crate) fn decode_scan(&mut self, data: &[u8]) -> Result<DecodedMeta, MjpegError> {
        self.decode_scan_with::<FullStages>(data)
    }

    /// The staged-scan profiling entry (W7-b): the same walk as
    /// [`DecodeContext::decode_scan`] with the per-block compute
    /// stages selected by `H`'s hooks — [`NoIdctStages`] stubs the
    /// dequant+IDCT (constant mid-gray samples), [`NoWriteStages`]
    /// stubs the plane store. The Huffman entropy walk consumes the
    /// identical bitstream in every arm (it depends on the tables and
    /// the reader only), which is what makes the ablation deltas a
    /// valid stage attribution: `decode_scan − decode_scan_with::<NoIdct>`
    /// is the dequant+IDCT share of the real walk. Never used by the
    /// production contract path; exercised by `examples/stage_bench.rs`
    /// and the equivalence test below.
    ///
    /// # Errors
    /// Typed [`MjpegError`]s, exactly like [`DecodeContext::decode_scan`].
    pub(crate) fn decode_scan_with<H: ScanHooks>(
        &mut self,
        data: &[u8],
    ) -> Result<DecodedMeta, MjpegError> {
        // Park an empty scratch where the persistent header storage
        // lives, and walk into the real one (D-1): `JpegInfo::empty`
        // allocates nothing, the moves are pointer-sized struct
        // shuffles, and the walk refills the persistent Vecs in place.
        // Splitting the borrow this way lets the scan body below read
        // `info` freely while the context's planes/tables are
        // mutated through `self`.
        let mut info = std::mem::replace(&mut self.headers, JpegInfo::empty());
        let decoded = self.scan_into::<H>(&mut info, data);
        self.headers = info;
        decoded
    }

    /// The exact output byte count for the frame last walked by
    /// [`DecodeContext::decode_scan`] (the pooled caller sizes its
    /// slot with this before filling).
    ///
    /// # Errors
    /// [`MjpegError::LimitExceeded`] when the total overflows the
    /// pixel-cap arithmetic (the same check the assembly runs).
    pub(crate) fn frame_len(&self) -> Result<usize, MjpegError> {
        frame_total_for(&self.headers).map(|(_, total)| total)
    }

    /// Assemble the frame last walked by [`DecodeContext::decode_scan`]
    /// into `out` (the pooled handoff's second half). `out.len()` must
    /// equal [`DecodeContext::frame_len`] — every byte of `out` is
    /// overwritten (rows crop or fill for I420; every row composes for
    /// RGBA), so a reused pooled slot can never leak a stale pixel.
    ///
    /// # Errors
    /// Typed [`MjpegError`] when `out.len()` disagrees with the walked
    /// geometry (a caller bug, surfaced loudly — never a partial
    /// frame).
    pub(crate) fn assemble_frame(&self, out: &mut [u8]) -> Result<(), MjpegError> {
        assemble_frame_into(&self.headers, &self.planes, out)
    }

    /// The scan body behind [`DecodeContext::decode_scan`]: `info` is
    /// the context's own (persistent) header scratch, walked here and
    /// read for the rest of the frame. Generic over the stage hooks
    /// (the W7-b profiling seam): the production entry instantiates
    /// [`FullStages`], whose trait impls are `#[inline]` forwardings
    /// to the real stages, so the monomorphized production walk is
    /// codegen-identical to a non-generic body (pinned by output
    /// equality against `decode_scan` in the tests below and by the
    /// committed decode-reference digests).
    fn scan_into<H: ScanHooks>(
        &mut self,
        info: &mut JpegInfo,
        data: &[u8],
    ) -> Result<DecodedMeta, MjpegError> {
        parser::parse_into(info, data)?;
        let info = &*info;

        // Component-count semantics: JFIF grayscale (1) or YCbCr (3).
        // 2- and 4-component streams (e.g. Adobe CMYK/YCCK) are outside
        // the honest baseline subset.
        match info.components.len() {
            1 | 3 => {}
            other => {
                return Err(super::error::unsupported(format!(
                    "{other} frame components (JFIF grayscale=1 / YCbCr=3 only)"
                )));
            }
        }

        // Limits (fuzz-safety memory bound).
        let w = u64::from(info.width);
        let h = u64::from(info.height);
        if w.saturating_mul(h) > MAX_PIXELS {
            return Err(MjpegError::LimitExceeded {
                limit: media_types::error::LimitKind::Memory,
                budget: MAX_PIXELS,
                attempted: w.saturating_mul(h),
            });
        }

        // Geometry (§A.1.1): MCU grid from the maximum sampling factors.
        let h_max = info.components.iter().map(|c| c.h).max().unwrap_or(1) as usize;
        let v_max = info.components.iter().map(|c| c.v).max().unwrap_or(1) as usize;
        let w = info.width as usize;
        let h = info.height as usize;
        let mcu_w = w.div_ceil(8 * h_max);
        let mcu_h = h.div_ceil(8 * v_max);

        self.prepare_planes(info, mcu_w, mcu_h, w, h)?;
        self.prepare_tables(info)?;

        // Scan walk.
        let entropy = data.get(info.entropy_offset..).unwrap_or(&[]);
        let mut reader = BitReader::new(entropy);
        let mut dc_pred = [0i64; 4];
        self.cursors.clear();
        self.cursors.resize(info.components.len(), 0);
        let mut concealed = false;
        let mut cause: Option<ConcealCause> = None;
        let restart_interval = info.restart_interval as usize;
        let mut restart_count: u32 = 0;

        'scan: for my in 0..mcu_h {
            for mx in 0..mcu_w {
                let mcu_index = my * mcu_w + mx;
                if restart_interval > 0
                    && mcu_index > 0
                    && mcu_index.is_multiple_of(restart_interval)
                {
                    let expected = (restart_count & 7) as u8;
                    if reader.take_restart(expected).is_err() {
                        concealed = true;
                        cause = Some(ConcealCause::RestartMismatch);
                        break 'scan;
                    }
                    restart_count += 1;
                    dc_pred = [0; 4];
                }
                for scan_comp in &info.scan {
                    let comp: ComponentInfo = info.components[scan_comp.sof_index];
                    let dc_table = match &self.dc_tables[scan_comp.td as usize] {
                        Some(table) => table,
                        None => {
                            return Err(super::error::corrupt_entropy("mjpeg.huffman.no_dc_table"));
                        }
                    };
                    let ac_table = match &self.ac_tables[scan_comp.ta as usize] {
                        Some(table) => table,
                        None => {
                            return Err(super::error::corrupt_entropy("mjpeg.huffman.no_ac_table"));
                        }
                    };
                    let qt = &info.quant_tables[comp.quant_table as usize];
                    for by in 0..comp.v as usize {
                        for bx in 0..comp.h as usize {
                            let mut block = [0i16; 64];
                            let pred = &mut dc_pred[scan_comp.sof_index.min(3)];
                            match decode_block(&mut reader, dc_table, ac_table, pred, &mut block) {
                                Ok(()) => {
                                    let plane = &mut self.planes[scan_comp.sof_index];
                                    let col = mx * comp.h as usize + bx;
                                    let row = my * comp.v as usize + by;
                                    let mut samples = [0u8; 64];
                                    H::transform(&block, qt, &mut samples);
                                    H::store(plane, &samples, col, row);
                                    self.cursors[scan_comp.sof_index] += 1;
                                }
                                Err(stop) => {
                                    // Entropy damage: conceal from here on
                                    // (module docs: mid-gray, observable).
                                    concealed = true;
                                    cause = Some(match stop {
                                        Stop::Exhausted => match reader.stop() {
                                            Some(ReaderStop::Marker(code)) => {
                                                ConcealCause::PrematureMarker(code)
                                            }
                                            _ => ConcealCause::EndOfData,
                                        },
                                        Stop::InvalidCode => ConcealCause::InvalidCode,
                                    });
                                    break 'scan;
                                }
                            }
                        }
                    }
                }
            }
        }

        if concealed {
            for scan_comp in &info.scan {
                let index = scan_comp.sof_index;
                let cursor = self.cursors[index];
                self.planes[index].gray_blocks(cursor);
            }
        }

        Ok(DecodedMeta {
            width: info.width,
            height: info.height,
            format: choose_format(info, h_max, v_max),
            concealed,
            cause,
            field_polarity: info.field_polarity,
        })
    }

    /// The decoded frame's pixel bytes (layout per the returned
    /// metadata's `format`) until the next decode or
    /// [`DecodeContext::take_output`] — the in-place read point for
    /// callers that consume the frame without an ownership handoff
    /// (currently the reuse tests; every production consumer takes
    /// the buffer into the frame).
    #[cfg(test)]
    pub(crate) fn output(&self) -> &[u8] {
        &self.out
    }

    /// Move the decoded frame's pixel bytes out (the one allocation
    /// per frame on the contract path: the frame's own buffer).
    pub(crate) fn take_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    /// (Re)build the padded planes when the geometry key changed;
    /// reuse the existing buffers (without clearing — see module
    /// docs) when it did not. Capacity is retained across geometry
    /// *changes* too (a resize reuses the allocation; the retained
    /// capacity stays bounded by the MAX_PIXELS-derived plane caps).
    fn prepare_planes(
        &mut self,
        info: &JpegInfo,
        mcu_w: usize,
        mcu_h: usize,
        w: usize,
        h: usize,
    ) -> Result<(), MjpegError> {
        let mut sampling = [(0u8, 0u8); 4];
        for (slot, comp) in sampling.iter_mut().zip(info.components.iter()) {
            *slot = (comp.h, comp.v);
        }
        let key = PlaneKey {
            sampling,
            mcu_w,
            mcu_h,
            width: w,
            height: h,
        };
        if self.plane_key == Some(key) {
            return Ok(());
        }
        if self.planes.len() != info.components.len() {
            self.planes = (0..info.components.len())
                .map(|_| Plane {
                    data: Vec::new(),
                    stride: 0,
                    block_cols: 0,
                    block_rows: 0,
                    valid_w: 0,
                    valid_h: 0,
                })
                .collect();
        }
        for (plane, comp) in self.planes.iter_mut().zip(info.components.iter()) {
            let block_cols = mcu_w * comp.h as usize;
            let block_rows = mcu_h * comp.v as usize;
            let stride = block_cols
                .checked_mul(8)
                .ok_or_else(|| MjpegError::LimitExceeded {
                    limit: media_types::error::LimitKind::Memory,
                    budget: MAX_PIXELS,
                    attempted: u64::try_from(block_cols.saturating_mul(8)).unwrap_or(u64::MAX),
                })?;
            let rows = block_rows * 8;
            let valid_w = (w * comp.h as usize).div_ceil(h_max_of(info));
            let valid_h = (h * comp.v as usize).div_ceil(v_max_of(info));
            let size = stride
                .checked_mul(rows)
                .ok_or_else(|| MjpegError::LimitExceeded {
                    limit: media_types::error::LimitKind::Memory,
                    budget: MAX_PIXELS,
                    attempted: u64::try_from(rows).unwrap_or(u64::MAX).saturating_mul(1),
                })?;
            if u64::try_from(size).unwrap_or(u64::MAX) > MAX_PIXELS * 4 {
                return Err(MjpegError::LimitExceeded {
                    limit: media_types::error::LimitKind::Memory,
                    budget: MAX_PIXELS * 4,
                    attempted: u64::try_from(size).unwrap_or(u64::MAX),
                });
            }
            plane.data.clear();
            plane.data.resize(size, 0);
            plane.stride = stride;
            plane.block_cols = block_cols;
            plane.block_rows = block_rows;
            plane.valid_w = valid_w;
            plane.valid_h = valid_h;
        }
        self.plane_key = Some(key);
        self.plane_builds += 1;
        Ok(())
    }

    /// (Re)build the Huffman decoders when the DHT payload changed;
    /// reuse the built tables (LUTs included) when it did not. The
    /// per-frame compare ([`DhtKey::matches`]) allocates nothing;
    /// `DhtKey`'s payload snapshot happens only on a change.
    fn prepare_tables(&mut self, info: &JpegInfo) -> Result<(), MjpegError> {
        if self.dht_key.as_ref().is_some_and(|key| key.matches(info)) {
            return Ok(());
        }
        // V-3 (memory-audit): present tables rebuild **in place** —
        // `HuffmanDecoder::rebuild` overwrites the derived arrays and
        // reuses the HUFFVAL storage, so a stream whose encoder
        // re-emits a varying DHT every frame allocates nothing here
        // after the first build. Absent tables still drop to `None`
        // (the previous semantics — a scan may only reference a
        // declared table).
        for td in 0..4usize {
            if info.huff_present[0][td] {
                refresh_table(
                    &mut self.dc_tables[td],
                    &info.huff_bits[0][td],
                    &info.huff_vals[0][td],
                )
                .map_err(|_| super::error::corrupt_entropy("mjpeg.huffman.bad_table"))?;
            } else {
                self.dc_tables[td] = None;
            }
            if info.huff_present[1][td] {
                refresh_table(
                    &mut self.ac_tables[td],
                    &info.huff_bits[1][td],
                    &info.huff_vals[1][td],
                )
                .map_err(|_| super::error::corrupt_entropy("mjpeg.huffman.bad_table"))?;
            } else {
                self.ac_tables[td] = None;
            }
        }
        match &mut self.dht_key {
            Some(key) => key.update_from(info),
            None => self.dht_key = Some(DhtKey::of(info)),
        }
        self.table_builds += 1;
        Ok(())
    }
}

/// Build-or-rebuild one decoder slot in place (V-3): an existing
/// decoder reuses its buffers (`HuffmanDecoder::rebuild`); a fresh
/// slot allocates once (first build per stream).
fn refresh_table(
    slot: &mut Option<HuffmanDecoder>,
    bits: &[u8; 17],
    vals: &[u8],
) -> Result<(), Stop> {
    match slot.as_mut() {
        Some(table) => table.rebuild(bits, vals),
        None => {
            *slot = Some(HuffmanDecoder::new(bits, vals)?);
            Ok(())
        }
    }
}

fn h_max_of(info: &JpegInfo) -> usize {
    info.components.iter().map(|c| c.h).max().unwrap_or(1) as usize
}

fn v_max_of(info: &JpegInfo) -> usize {
    info.components.iter().map(|c| c.v).max().unwrap_or(1) as usize
}

/// The pooled-output slot rotation depth (W6-02 / V-4; the PCM
/// decoder's `CONVERT_SLOTS` and `pipeline.rs`'s `OUT_SLOTS`
/// precedent, lifted to the video handoff): how many pooled
/// [`SharedBytes`] frame buffers the contract decoder keeps. Four
/// covers the decoded frame still held downstream (sink, presenter,
/// renderer) plus slack — a pipeline that hands frames off
/// asynchronously never starves the pool into the allocate path.
/// Bounded memory: at most `FRAME_SLOTS · current-frame-bytes`
/// (the pool re-keys on exact length, so a geometry change evicts
/// rather than accumulates).
pub(crate) const FRAME_SLOTS: usize = 4;

/// Fill the next pooled frame slot through `fill` (exactly `len`
/// bytes) and hand out a clone (refcount 2: pool + the decoded
/// frame; the clone the sink saw returns the slot to refcount 1 when
/// its buffer drops, making it refillable in place). Warm path: an
/// in-rotation slot of exactly `len` bytes whose `get_mut` succeeds
/// (the pool is the sole owner). Cold path (first frames of a
/// geometry, an all-slots-busy rotation): allocate a fresh slot and
/// enter it into the round-robin (bounded at [`FRAME_SLOTS`], then
/// evicting). The PCM `pooled_convert_slot` pattern verbatim in
/// shape; the fill is fallible because the video assembly validates
/// its destination length (a caller bug surfaces as a typed error,
/// never a panic or a partial frame).
pub(crate) fn pooled_frame_slot(
    slots: &mut Vec<SharedBytes>,
    next: &mut usize,
    len: usize,
    fill: impl FnOnce(&mut [u8]) -> Result<(), MjpegError>,
) -> Result<SharedBytes, MjpegError> {
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != len {
            continue;
        }
        if let Some(bytes) = slots[idx].get_mut() {
            fill(bytes)?;
            *next = (idx + 1) % count;
            return Ok(slots[idx].clone());
        }
        // The slot still has downstream owners — try the next one.
    }
    // Cold path: allocate, fill in place, then enter the rotation.
    let mut fresh = SharedBytes::from_vec(vec![0; len]);
    match fresh.get_mut() {
        Some(bytes) => fill(bytes)?,
        None => {
            // A freshly built `SharedBytes` has refcount 1 by
            // construction; `get_mut` failing here is impossible —
            // surface it loudly instead of fabricating an empty
            // frame.
            return Err(super::error::corrupt_entropy(
                "mjpeg.output.fresh_slot_not_exclusive",
            ));
        }
    }
    if count < FRAME_SLOTS {
        slots.push(fresh.clone());
        *next = (count + 1) % FRAME_SLOTS;
    } else {
        let evict = *next;
        slots[evict] = fresh.clone();
        *next = (evict + 1) % FRAME_SLOTS;
    }
    Ok(fresh)
}

/// The output format + exact byte count for a walked `JpegInfo`.
///
/// # Errors
/// [`MjpegError::LimitExceeded`] when the buffer arithmetic
/// overflows the pixel cap (the fuzz-safety bound).
fn frame_total_for(info: &JpegInfo) -> Result<(OutputFormat, usize), MjpegError> {
    let h_max = h_max_of(info);
    let v_max = v_max_of(info);
    let w = info.width as usize;
    let h = info.height as usize;
    let format = choose_format(info, h_max, v_max);
    let total = match format {
        OutputFormat::I420 => i420_total(w, h)?,
        OutputFormat::Rgba8 => w
            .checked_mul(h)
            .and_then(|px| px.checked_mul(4))
            .ok_or_else(|| limit_error(w * h * 4))?,
    };
    Ok((format, total))
}

/// Assemble the walked frame's pixels from `planes` into `out`
/// (the shared body of the stateless `Vec` path and the pooled-slot
/// contract path — identical bytes by construction, pinned by the
/// differential tests).
///
/// # Errors
/// A typed corrupt-structure error when `out.len()` is not the exact
/// [`frame_total_for`] length (surfaced loudly; `out` is left
/// untouched in that case).
fn assemble_frame_into(
    info: &JpegInfo,
    planes: &[Plane],
    out: &mut [u8],
) -> Result<(), MjpegError> {
    let h_max = h_max_of(info);
    let v_max = v_max_of(info);
    let w = info.width as usize;
    let h = info.height as usize;
    let (format, total) = frame_total_for(info)?;
    if out.len() != total {
        return Err(super::error::corrupt_entropy(
            "mjpeg.output.length_mismatch",
        ));
    }
    match format {
        OutputFormat::I420 => assemble_i420_into(out, planes, w, h),
        OutputFormat::Rgba8 => assemble_rgba_into(out, planes, info, h_max, v_max, w, h),
    }
    Ok(())
}

/// Receive-and-extend an amplitude value of `s` bits (§F.2.2.1 EXTEND).
fn receive_extend(reader: &mut BitReader<'_>, s: u32) -> Result<i32, Stop> {
    let Some(bits) = reader.read_bits(s) else {
        return Err(Stop::Exhausted);
    };
    let v = bits as i32;
    if s == 0 {
        return Ok(0);
    }
    if v < (1 << (s - 1)) {
        Ok(v - (1 << s) + 1)
    } else {
        Ok(v)
    }
}

/// Decode one structure unit: one 8×8 block of one component
/// (§F.2.2.2 / F.2.2.3). `block` arrives zeroed.
fn decode_block(
    reader: &mut BitReader<'_>,
    dc_table: &HuffmanDecoder,
    ac_table: &HuffmanDecoder,
    dc_pred: &mut i64,
    block: &mut [i16; 64],
) -> Result<(), Stop> {
    let size = dc_table.decode(reader)?;
    let diff = if size == 0 {
        0
    } else {
        receive_extend(reader, u32::from(size))?
    };
    *dc_pred = dc_pred.saturating_add(i64::from(diff));
    // The DC predictor is clamped into i16 for storage; hostile
    // cumulative diffs cannot overflow (values far above the legit
    // ±2047 band saturate in the IDCT clip anyway).
    block[0] = (*dc_pred).clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16;
    let mut k = 1usize;
    while k < 64 {
        let rs = ac_table.decode(reader)?;
        let run = (rs >> 4) as usize;
        let size = (rs & 0x0f) as u32;
        if size == 0 {
            if run == 15 {
                // ZRL: sixteen zero coefficients.
                k += 16;
                continue;
            }
            break; // EOB
        }
        k += run;
        if k > 63 {
            return Err(Stop::InvalidCode);
        }
        let value = receive_extend(reader, size)?;
        if let Some(slot) = block.get_mut(ZIGZAG[k]) {
            *slot = value.clamp(-32768, 32767) as i16;
        }
        k += 1;
    }
    Ok(())
}

/// Decode one complete JPEG image with a fresh one-shot context (the
/// stateless entry point — fuzz target and free-function callers).
pub(crate) fn decode(data: &[u8]) -> Result<DecodedImage, MjpegError> {
    let mut context = DecodeContext::new();
    let meta = context.decode(data)?;
    Ok(DecodedImage {
        width: meta.width,
        height: meta.height,
        format: meta.format,
        data: context.take_output(),
        concealed: meta.concealed,
        cause: meta.cause,
        field_polarity: meta.field_polarity,
    })
}

/// The output format for this sampling: I420 for 4:2:0 and grayscale
/// (neutral chroma), RGBA for the rest.
fn choose_format(info: &JpegInfo, h_max: usize, v_max: usize) -> OutputFormat {
    if info.components.len() == 1 {
        return OutputFormat::I420;
    }
    let luma = &info.components[0];
    let chroma_420 = info.components[1..].iter().all(|c| c.h == 1 && c.v == 1);
    if h_max == 2 && v_max == 2 && luma.h == 2 && luma.v == 2 && chroma_420 {
        OutputFormat::I420
    } else {
        OutputFormat::Rgba8
    }
}

/// I420 buffer length: `Y ‖ Cb ‖ Cr` with chroma at ⌈w/2⌉×⌈h/2⌉.
fn i420_total(w: usize, h: usize) -> Result<usize, MjpegError> {
    let cw = w.div_ceil(2);
    let ch = h.div_ceil(2);
    let luma_len = w.checked_mul(h).ok_or_else(|| limit_error(w * h))?;
    let chroma_len = cw.checked_mul(ch).ok_or_else(|| limit_error(cw * ch))?;
    luma_len
        .checked_add(
            chroma_len
                .checked_mul(2)
                .ok_or_else(|| limit_error(chroma_len))?,
        )
        .ok_or_else(|| limit_error(luma_len))
}

/// Crop one plane's valid region directly into `dst` (one row copy —
/// no intermediate buffer, no second copy). The row count is
/// `dst.len() / out_w`. Rows/samples outside the plane's data read
/// as neutral 128 (the cold-path convention of the RGBA composer;
/// unreachable for the plane geometries the I420 path is selected
/// for — luma and chroma `valid` extents always cover their output
/// regions there).
fn crop_plane_into(dst: &mut [u8], plane: &Plane, out_w: usize) {
    // 5.X1b fast path: tightly packed rows (stride == out_w) with the
    // valid region covering the whole destination — one bulk copy
    // instead of one `copy_from_slice` per row. Byte-identical to the
    // per-row loop by construction (same source span, contiguous).
    let rows = dst.len() / out_w;
    if plane.valid_w >= out_w
        && plane.stride == out_w
        && plane.valid_h >= rows
        && out_w > 0
        && rows > 0
    {
        let end = out_w
            .checked_mul(rows)
            .and_then(|len| plane.data.get(..len).map(|_| len));
        if let Some(len) = end {
            dst.copy_from_slice(&plane.data[..len]);
            return;
        }
    }
    for (y, dst_row) in dst.chunks_exact_mut(out_w).enumerate() {
        if y >= plane.valid_h {
            dst_row.fill(128);
            continue;
        }
        let start = y * plane.stride;
        let end = start + out_w.min(plane.valid_w);
        match plane.data.get(start..end) {
            Some(row) => dst_row.copy_from_slice(row),
            None => dst_row.fill(128),
        }
    }
}

/// I420 assembly in place: `Y ‖ Cb ‖ Cr` at their output offsets,
/// cropped straight from the padded planes. Grayscale carries
/// neutral (constant-128) chroma planes — a documented
/// representation, not fabricated data.
fn assemble_i420_into(out: &mut [u8], planes: &[Plane], w: usize, h: usize) {
    let cw = w.div_ceil(2);
    let ch = h.div_ceil(2);
    let luma_len = w * h;
    let chroma_len = cw * ch;
    if let Some(dst) = out.get_mut(..luma_len) {
        crop_plane_into(dst, &planes[0], w);
    }
    match planes.len() {
        1 => {
            if let Some(dst) = out.get_mut(luma_len..) {
                dst.fill(128);
            }
        }
        _ => {
            if let Some(dst) = out.get_mut(luma_len..luma_len + chroma_len) {
                crop_plane_into(dst, &planes[1], cw);
            }
            if let Some(dst) = out.get_mut(luma_len + chroma_len..) {
                crop_plane_into(dst, &planes[2], cw);
            }
        }
    }
}

/// RGBA assembly via the upsample + convert row composer, in place.
fn assemble_rgba_into(
    out: &mut [u8],
    planes: &[Plane],
    info: &JpegInfo,
    h_max: usize,
    v_max: usize,
    w: usize,
    h: usize,
) {
    let geom = |index: usize| ComponentGeom {
        h: info.components.get(index).map_or(1, |c| c.h as usize),
        v: info.components.get(index).map_or(1, |c| c.v as usize),
        valid_w: planes.get(index).map_or(1, |p| p.valid_w),
        valid_h: planes.get(index).map_or(1, |p| p.valid_h),
    };
    let chroma = if planes.len() == 1 {
        None
    } else {
        Some((
            PlaneView {
                data: &planes[1].data,
                stride: planes[1].stride,
            },
            PlaneView {
                data: &planes[2].data,
                stride: planes[2].stride,
            },
        ))
    };
    let composer = RgbComposer::new(
        PlaneView {
            data: &planes[0].data,
            stride: planes[0].stride,
        },
        chroma,
        [geom(0), geom(1), geom(2)],
        h_max,
        v_max,
    );
    for y in 0..h {
        let row_start = y * w * 4;
        let Some(row) = out.get_mut(row_start..row_start + w * 4) else {
            break;
        };
        composer.compose_row(y, w, row);
    }
}

fn limit_error(attempted: usize) -> MjpegError {
    MjpegError::LimitExceeded {
        limit: media_types::error::LimitKind::Memory,
        budget: MAX_PIXELS,
        attempted: u64::try_from(attempted).unwrap_or(u64::MAX),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration-level tests of `decode` live in tests/ (corpus +
    // synthetic JPEGs); the unit tests here target the geometry,
    // assembly and context-reuse helpers with hand-built inputs.

    fn plane_of(bytes: &[u8], w: usize) -> Plane {
        let h = bytes.len() / w;
        Plane {
            data: bytes.to_vec(),
            stride: w,
            block_cols: w.div_ceil(8),
            block_rows: h.div_ceil(8),
            valid_w: w,
            valid_h: h,
        }
    }

    #[test]
    fn crop_into_trims_padding() {
        // A 10x2 valid region inside a 16-wide padded plane.
        let mut data = vec![9u8; 16 * 2];
        data[10..16].fill(255); // padding marker bytes
        data[16 + 10..16 + 16].fill(255);
        let plane = Plane {
            data,
            stride: 16,
            block_cols: 2,
            block_rows: 1,
            valid_w: 10,
            valid_h: 2,
        };
        let mut cropped = [0u8; 20];
        crop_plane_into(&mut cropped, &plane, 10);
        assert!(cropped.iter().all(|&b| b == 9), "{cropped:?}");
    }

    #[test]
    fn crop_into_fills_neutral_past_the_data() {
        // Rows beyond the valid height (unreachable for real I420
        // geometry, defensive here) read as neutral 128, and a short
        // destination is prefix-safe.
        let plane = plane_of(&[7u8; 16], 16); // stride 16, one valid row
        let mut dst = [0u8; 32];
        crop_plane_into(&mut dst, &plane, 8);
        assert_eq!(&dst[..8], &[7u8; 8], "the valid row copies");
        assert_eq!(&dst[8..], &[128u8; 24], "rows past valid_h neutral");
    }

    #[test]
    fn gray_blocks_fills_only_the_tail() {
        // 2x1 blocks: conceal from block 1 on.
        let mut plane = plane_of(&[7u8; 16], 16);
        plane.gray_blocks(1);
        assert_eq!(&plane.data[0..8], &[7u8; 8], "block 0 untouched");
        assert_eq!(&plane.data[8..16], &[128u8; 8], "block 1 gray");
    }

    #[test]
    fn receive_extend_decodes_negative_and_positive() {
        // s=2: values 0..1 → -3..-2; 2..3 → 2..3.
        let mut r = BitReader::new(&[0b00_10_11_00]);
        assert_eq!(receive_extend(&mut r, 2).ok(), Some(-3));
        assert_eq!(receive_extend(&mut r, 2).ok(), Some(2));
        assert_eq!(receive_extend(&mut r, 2).ok(), Some(3));
    }

    #[test]
    fn pixel_cap_rejects_oversize_frames() {
        // A complete header set declaring 8193×8193 (8 MiP + 1): the
        // parse succeeds, the pixel cap fires.
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
        // DHT: DC table 0 with one symbol; AC table 0 with one symbol.
        for class in [0x00u8, 0x10] {
            let mut dht = vec![class];
            dht.extend(&[1u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            dht.push(0x00);
            seg(&mut jpeg, 0xc4, &dht);
        }
        // SOF0: 8193×8193, one component, h1v1, Tq 0.
        let mut sof = vec![8u8];
        sof.extend_from_slice(&[0x20, 0x01, 0x20, 0x01, 1, 1, 0x11, 0]);
        seg(&mut jpeg, 0xc0, &sof);
        // SOS: one component, Td=0 Ta=0.
        seg(&mut jpeg, 0xda, &[1, 1, 0x00, 0, 63, 0]);
        jpeg.extend_from_slice(&[0x00, 0xff, 0xd9]);
        let e = match decode(&jpeg) {
            Err(e) => e,
            Ok(_) => unreachable!("oversize frame must be rejected"),
        };
        match e {
            MjpegError::LimitExceeded { attempted, .. } => {
                assert_eq!(attempted, 8193 * 8193);
            }
            other => unreachable!("expected LimitExceeded, got {other:?}"),
        }
    }

    // --- ED-FIX-1 (C4): decode-context reuse, bit-exact ---

    /// Hand-build a complete tiny baseline JPEG (the marker-segment
    /// pattern of the fuzz tests) with selectable geometry and DHT
    /// payload. Entropy data is all zero bits: every block decodes as
    /// DC-diff 0 + immediate EOB (a flat mid-gray frame) — the point
    /// here is state reuse, not content.
    fn tiny_jpeg(
        width: u16,
        height: u16,
        components: &[(u8, u8, u8)],
        dc_symbols: &[u8],
    ) -> Vec<u8> {
        let mut jpeg = vec![0xff, 0xd8];
        let seg = |jpeg: &mut Vec<u8>, marker: u8, payload: &[u8]| {
            jpeg.push(0xff);
            jpeg.push(marker);
            jpeg.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
            jpeg.extend_from_slice(payload);
        };
        // DQT: Tq=0, all 1s.
        let mut dqt = vec![0u8];
        dqt.extend(std::iter::repeat_n(1u8, 64));
        seg(&mut jpeg, 0xdb, &dqt);
        // DHT: DC table 0 carries `dc_symbols` as one code per symbol
        // (1 bit each); AC table 0 is a single 1-bit EOB.
        let mut dht = vec![0x00u8];
        let mut bits = [0u8; 16];
        bits[0] = dc_symbols.len() as u8;
        dht.extend_from_slice(&bits);
        dht.extend_from_slice(dc_symbols);
        seg(&mut jpeg, 0xc4, &dht);
        let mut dht = vec![0x10u8];
        dht.extend_from_slice(&[1u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        dht.push(0x00);
        seg(&mut jpeg, 0xc4, &dht);
        // SOF0: precision 8, then Y (height) then X (width) per §B.2.2.
        let mut sof = vec![8u8];
        sof.extend_from_slice(&height.to_be_bytes());
        sof.extend_from_slice(&width.to_be_bytes());
        sof.push(components.len() as u8);
        for &(id, hv, _tq) in components {
            sof.push(id);
            sof.push(hv);
            sof.push(0); // Tq 0 (the single quant table above)
        }
        seg(&mut jpeg, 0xc0, &sof);
        // SOS: all components, Td=0 Ta=0.
        let mut sos = vec![components.len() as u8];
        for (id, _hv, _tq) in components {
            sos.push(*id);
            sos.push(0x00);
        }
        sos.extend_from_slice(&[0, 63, 0]);
        seg(&mut jpeg, 0xda, &sos);
        // Entropy: enough zero bytes for every block (2 bits each:
        // DC size 0 + EOB) — w·h·3/256 bytes worst case, so w·h/32
        // plus slack always suffices; anything undecoded conceals
        // identically anyway.
        jpeg.extend(std::iter::repeat_n(
            0u8,
            usize::from(width) * usize::from(height) / 32 + 16,
        ));
        jpeg.extend_from_slice(&[0xff, 0xd9]);
        jpeg
    }

    #[test]
    fn context_reuse_is_bit_exact_and_reallocates_only_on_change() {
        // Three geometries interleaved through ONE context: every
        // output must equal the stateless decode bit-for-bit, the
        // plane set rebuilds only per geometry change, and the DHT
        // set rebuilds only when the tables change. The gray
        // 4:2:0 / 4:4:4 / grayscale triple also crosses the I420
        // (grayscale) and RGBA (4:4:4) output paths.
        let gray = tiny_jpeg(16, 16, &[(1, 0x11, 0)], &[0x00]);
        let yuv420 = tiny_jpeg(16, 16, &[(1, 0x22, 0), (2, 0x11, 0), (3, 0x11, 0)], &[0x00]);
        let yuv444 = tiny_jpeg(24, 16, &[(1, 0x11, 0), (2, 0x11, 0), (3, 0x11, 0)], &[0x00]);
        // Same geometry as `yuv420`, different DC table payload
        // (two 1-bit codes vs one): same decode (entropy is all
        // zeros — the first code is 0 → DC size 0 in both), new key.
        let yuv420_alt_tables = tiny_jpeg(
            16,
            16,
            &[(1, 0x22, 0), (2, 0x11, 0), (3, 0x11, 0)],
            &[0x00, 0x00],
        );

        let mut ctx = DecodeContext::new();
        let reference: Vec<DecodedImage> = [&gray, &yuv420, &yuv444, &yuv420_alt_tables]
            .iter()
            .map(|jpeg| match decode(jpeg) {
                Ok(image) => image,
                Err(e) => unreachable!("stateless decode: {e:?}"),
            })
            .collect();

        // Sequence: gray, 420, 444, 420 (geometry back), 420-alt
        // (tables change), 444, 444 (steady), gray.
        let sequence = [
            (&gray, 0usize),
            (&yuv420, 1),
            (&yuv444, 2),
            (&yuv420, 1),
            (&yuv420_alt_tables, 3),
            (&yuv444, 2),
            (&yuv444, 2),
            (&gray, 0),
        ];
        for (jpeg, want) in sequence {
            let meta = match ctx.decode(jpeg) {
                Ok(meta) => meta,
                Err(e) => unreachable!("context decode: {e:?}"),
            };
            let expected = &reference[want];
            assert_eq!(ctx.output(), expected.data.as_slice(), "output equality");
            assert_eq!(meta.width, expected.width);
            assert_eq!(meta.height, expected.height);
            assert_eq!(meta.format, expected.format);
            assert_eq!(meta.concealed, expected.concealed);
        }
        // Geometry sequence: gray → 420 → 444 → 420(reuse) →
        // 420-alt(reuse) → 444 → 444(reuse) → gray. The 420/444
        // flip-flops must NOT reallocate (capacity retention), so
        // the rebuild counter counts key *changes*: builds at
        // entries 1, 2, 3, 4, 6 and 8 = 6 builds.
        assert_eq!(
            ctx.plane_builds, 6,
            "plane rebuilds only on geometry change"
        );
        // Tables: first build (entry 1), the alt-tables frame
        // (entry 5), and the restore to the original tables on the
        // next frame (entry 6) = 3.
        assert_eq!(ctx.table_builds, 3, "table rebuilds only on DHT change");

        // And the stateless path (fresh context per call) agrees with
        // the reused-context outputs — already asserted above via the
        // reference decodes.
    }

    #[test]
    fn dht_key_compare_is_content_sensitive_without_rebuilding_steady_state() {
        // W2-06: the per-frame table check is an allocation-free
        // structural compare (`DhtKey::matches`). Two streams whose
        // DC tables share the exact BITS histogram but carry
        // DIFFERENT HUFFVAL symbols must be treated as different
        // table sets (a bits-only compare would wrongly reuse), and
        // repeated frames of the same set must not rebuild.
        let symbol_zero = tiny_jpeg(16, 16, &[(1, 0x11, 0)], &[0x00]);
        let symbol_five = tiny_jpeg(16, 16, &[(1, 0x11, 0)], &[0x05]);
        let reference = [
            match decode(&symbol_zero) {
                Ok(image) => image,
                Err(e) => unreachable!("stateless decode: {e:?}"),
            },
            match decode(&symbol_five) {
                Ok(image) => image,
                Err(e) => unreachable!("stateless decode: {e:?}"),
            },
        ];
        // Sanity: the different symbol really does decode differently
        // (DC size 5 vs 0 on an all-zero entropy stream).
        assert_ne!(reference[0].data, reference[1].data);

        // zero, zero, five, five, zero: build on 1, rebuild on 3,
        // rebuild on 5 — nothing in between (content compare, not
        // identity or bits-only).
        let sequence = [
            (&symbol_zero, 0usize),
            (&symbol_zero, 0),
            (&symbol_five, 1),
            (&symbol_five, 1),
            (&symbol_zero, 0),
        ];
        let mut ctx = DecodeContext::new();
        for (jpeg, want) in sequence {
            let meta = match ctx.decode(jpeg) {
                Ok(meta) => meta,
                Err(e) => unreachable!("context decode: {e:?}"),
            };
            assert_eq!(ctx.output(), reference[want].data.as_slice());
            assert_eq!(meta.concealed, reference[want].concealed);
        }
        assert_eq!(
            ctx.table_builds, 3,
            "rebuild only on payload change (HUFFVAL content included)"
        );
    }

    #[test]
    fn header_error_does_not_poison_the_persistent_scratch() {
        // W2-06: the marker walk now reuses the context's persistent
        // `JpegInfo` scratch. A structurally damaged frame errors out
        // mid-walk (partial scratch state); the very next clean frame
        // through the same context must be bit-identical to a fresh
        // decode — the pre-walk reset is a correctness property.
        let clean = tiny_jpeg(16, 16, &[(1, 0x11, 0)], &[0x00]);
        let mut damaged = clean.clone();
        // Cut inside the SOF segment (layout: SOI 2 + DQT 69 + DHT 22
        // + DHT 22 = 115, SOF runs 115..128): quant + both Huffman
        // tables are already in the scratch when the walk fails —
        // real partial state for the reset to clear.
        damaged.truncate(120);
        assert!(damaged.len() < clean.len());

        let mut ctx = DecodeContext::new();
        assert!(
            ctx.decode(&damaged).is_err(),
            "mid-header truncation is a typed error"
        );
        let meta = match ctx.decode(&clean) {
            Ok(meta) => meta,
            Err(e) => unreachable!("clean re-decode: {e:?}"),
        };
        assert!(!meta.concealed, "clean frame is clean");

        let fresh = match decode(&clean) {
            Ok(image) => image,
            Err(e) => unreachable!("stateless reference: {e:?}"),
        };
        assert_eq!(ctx.output(), fresh.data.as_slice());
    }

    #[test]
    fn concealment_does_not_leak_across_frames() {
        // A truncated frame conceals its tail mid-gray; the very next
        // clean frame through the same context must be bit-identical
        // to a fresh decode (no stale gray, no stale plane bytes).
        let clean = tiny_jpeg(32, 32, &[(1, 0x22, 0), (2, 0x11, 0), (3, 0x11, 0)], &[0x00]);
        let mut damaged = clean.clone();
        // Cut inside the entropy region (SOF/SOS headers intact).
        damaged.truncate(damaged.len() * 3 / 4);

        let mut ctx = DecodeContext::new();
        let meta = match ctx.decode(&damaged) {
            Ok(meta) => meta,
            Err(e) => unreachable!("concealable damage decodes: {e:?}"),
        };
        assert!(meta.concealed, "truncation conceals");
        let clean_meta = match ctx.decode(&clean) {
            Ok(meta) => meta,
            Err(e) => unreachable!("clean re-decode: {e:?}"),
        };
        assert!(!clean_meta.concealed, "clean frame is clean");

        let fresh = match decode(&clean) {
            Ok(image) => image,
            Err(e) => unreachable!("stateless reference: {e:?}"),
        };
        assert_eq!(ctx.output(), fresh.data.as_slice());
    }

    #[test]
    fn take_output_hands_off_the_buffer() {
        let jpeg = tiny_jpeg(16, 16, &[(1, 0x11, 0)], &[0x00]);
        let mut ctx = DecodeContext::new();
        let _meta = match ctx.decode(&jpeg) {
            Ok(meta) => meta,
            Err(e) => unreachable!("decode: {e:?}"),
        };
        let taken = ctx.take_output();
        assert!(!taken.is_empty());
        assert!(ctx.output().is_empty(), "buffer moved out");
        // The context still decodes afterwards (fresh buffer).
        let _meta = match ctx.decode(&jpeg) {
            Ok(meta) => meta,
            Err(e) => unreachable!("decode after take: {e:?}"),
        };
        assert_eq!(ctx.output(), taken.as_slice());
    }

    // --- W6-02: pooled-slot handoff (V-4) ---

    #[test]
    fn pooled_slot_refill_is_bit_exact_and_stale_byte_safe() {
        // Two frames whose decoded bytes differ everywhere (flat
        // 128 vs flat ~122), decoded through the pooled path back to
        // back: the second decode must equal the stateless reference
        // EXACTLY — a single byte the assembly failed to overwrite
        // would survive as frame A's pixel and fail this compare (the
        // stale-slot-poisoning proof the module docs promise).
        let flat_mid = tiny_jpeg(16, 16, &[(1, 0x11, 0)], &[0x00]);
        // DC size 6, all-zero entropy → receive_extend(6 bits of 0)
        // = -63: a distinctly darker flat frame.
        let flat_dark = tiny_jpeg(16, 16, &[(1, 0x11, 0)], &[0x06]);
        let reference_mid = match decode(&flat_mid) {
            Ok(image) => image,
            Err(e) => unreachable!("stateless mid: {e:?}"),
        };
        let reference_dark = match decode(&flat_dark) {
            Ok(image) => image,
            Err(e) => unreachable!("stateless dark: {e:?}"),
        };
        assert_ne!(
            reference_mid.data, reference_dark.data,
            "the pair must differ for stale bytes to be observable"
        );

        let mut ctx = DecodeContext::new();
        let mut slots: Vec<SharedBytes> = Vec::new();
        let mut next = 0usize;
        let decode_pooled = |ctx: &mut DecodeContext,
                             slots: &mut Vec<SharedBytes>,
                             next: &mut usize,
                             jpeg: &[u8]|
         -> SharedBytes {
            match ctx.decode_scan(jpeg) {
                Ok(_) => {}
                Err(e) => unreachable!("scan: {e:?}"),
            }
            let len = match ctx.frame_len() {
                Ok(len) => len,
                Err(e) => unreachable!("frame_len: {e:?}"),
            };
            match pooled_frame_slot(slots, next, len, |bytes| ctx.assemble_frame(bytes)) {
                Ok(slot) => slot,
                Err(e) => unreachable!("pooled fill: {e:?}"),
            }
        };

        // Frame A fills the cold slot; frame B must overwrite ALL of
        // it (no zeroing happens on the warm path). All three handles
        // stay retained (downstream ownership), so each decode finds
        // its predecessors busy and the pool grows one slot per
        // retained frame — the PCM pool's documented cold behavior.
        let a = decode_pooled(&mut ctx, &mut slots, &mut next, &flat_mid);
        assert_eq!(a.as_slice(), reference_mid.data.as_slice());
        assert_eq!(slots.len(), 1);
        let b = decode_pooled(&mut ctx, &mut slots, &mut next, &flat_dark);
        assert_eq!(
            b.as_slice(),
            reference_dark.data.as_slice(),
            "warm-slot refill leaves no stale pixel"
        );
        assert_eq!(slots.len(), 2, "a retained → a fresh slot for b");
        // And a third, back to A's content, through the warm pool.
        let a2 = decode_pooled(&mut ctx, &mut slots, &mut next, &flat_mid);
        assert_eq!(a2.as_slice(), reference_mid.data.as_slice());
        assert_eq!(slots.len(), 3, "a+b retained → a third slot");
        // Releasing the downstream owners returns slot 0 to the
        // rotation: the next decode refills in place instead of
        // growing the pool.
        drop(a);
        drop(b);
        drop(a2);
        let a3 = decode_pooled(&mut ctx, &mut slots, &mut next, &flat_mid);
        assert_eq!(a3.as_slice(), reference_mid.data.as_slice());
        assert_eq!(
            slots.len(),
            3,
            "released slots re-enter the rotation, not the allocator"
        );
    }

    #[test]
    fn pooled_slots_rekey_on_geometry_and_stay_bounded() {
        // Alternating geometries (different output lengths) plus a
        // downstream owner that RETAINS frames: the pool re-keys on
        // exact length (cold path per new length), never grows past
        // FRAME_SLOTS, and every handed-out frame is bit-exact vs the
        // stateless reference.
        let small = tiny_jpeg(16, 16, &[(1, 0x11, 0)], &[0x00]);
        let large = tiny_jpeg(32, 32, &[(1, 0x11, 0)], &[0x00]);
        let ref_small = match decode(&small) {
            Ok(image) => image,
            Err(e) => unreachable!("stateless small: {e:?}"),
        };
        let ref_large = match decode(&large) {
            Ok(image) => image,
            Err(e) => unreachable!("stateless large: {e:?}"),
        };

        let mut ctx = DecodeContext::new();
        let mut slots: Vec<SharedBytes> = Vec::new();
        let mut next = 0usize;
        let mut retained: Vec<SharedBytes> = Vec::new();
        for round in 0..12 {
            let (jpeg, reference) = if round % 2 == 0 {
                (&small, &ref_small)
            } else {
                (&large, &ref_large)
            };
            match ctx.decode_scan(jpeg) {
                Ok(_) => {}
                Err(e) => unreachable!("scan: {e:?}"),
            }
            let len = match ctx.frame_len() {
                Ok(len) => len,
                Err(e) => unreachable!("frame_len: {e:?}"),
            };
            let slot = match pooled_frame_slot(&mut slots, &mut next, len, |bytes| {
                ctx.assemble_frame(bytes)
            }) {
                Ok(slot) => slot,
                Err(e) => unreachable!("pooled fill: {e:?}"),
            };
            assert_eq!(slot.as_slice(), reference.data.as_slice());
            retained.push(slot); // downstream keeps every frame
            assert!(
                slots.len() <= FRAME_SLOTS,
                "pool bounded at FRAME_SLOTS ({} entries)",
                slots.len()
            );
        }
        // With every frame retained the pool entered its full
        // rotation; dropping the downstream owners returns the slots
        // to refillable state (the next decode reuses, not grows).
        let before = slots.len();
        retained.clear();
        match ctx.decode_scan(&small) {
            Ok(_) => {}
            Err(e) => unreachable!("scan: {e:?}"),
        }
        let len = match ctx.frame_len() {
            Ok(len) => len,
            Err(e) => unreachable!("frame_len: {e:?}"),
        };
        let slot = match pooled_frame_slot(&mut slots, &mut next, len, |bytes| {
            ctx.assemble_frame(bytes)
        }) {
            Ok(slot) => slot,
            Err(e) => unreachable!("pooled fill: {e:?}"),
        };
        assert_eq!(slot.as_slice(), ref_small.data.as_slice());
        assert_eq!(slots.len(), before, "release returns slots to rotation");
    }

    #[test]
    fn staged_full_scan_is_bit_exact_vs_the_production_path() {
        // W7-b profiling seam: `decode_scan_with::<FullStages>` (the
        // generic walk the staged arms share) must be bit-identical to
        // `decode_scan` (the production instantiation) on every
        // geometry/output-format the decoder can produce, clean and
        // concealed — the seam's license to exist.
        let gray = tiny_jpeg(16, 16, &[(1, 0x11, 0)], &[0x00]);
        let yuv420 = tiny_jpeg(16, 16, &[(1, 0x22, 0), (2, 0x11, 0), (3, 0x11, 0)], &[0x00]);
        let yuv422 = tiny_jpeg(24, 16, &[(1, 0x21, 0), (2, 0x11, 0), (3, 0x11, 0)], &[0x00]);
        // 32×32 (not 16×16) so the 3/4 cut lands inside the entropy
        // region — headers intact, the tail conceals (the
        // `concealment_does_not_leak_across_frames` precedent).
        let mut truncated = tiny_jpeg(32, 32, &[(1, 0x22, 0), (2, 0x11, 0), (3, 0x11, 0)], &[0x00]);
        truncated.truncate(truncated.len() * 3 / 4); // concealed tail
        for jpeg in [&gray, &yuv420, &yuv422, &truncated] {
            let mut production = DecodeContext::new();
            let mut staged = DecodeContext::new();
            let a = match production.decode_scan(jpeg) {
                Ok(meta) => meta,
                Err(e) => unreachable!("production scan: {e:?}"),
            };
            let b = match staged.decode_scan_with::<FullStages>(jpeg) {
                Ok(meta) => meta,
                Err(e) => unreachable!("staged scan: {e:?}"),
            };
            assert_eq!(
                (a.width, a.height, a.format, a.concealed),
                (b.width, b.height, b.format, b.concealed)
            );
            let len = match production.frame_len() {
                Ok(len) => len,
                Err(e) => unreachable!("frame_len: {e:?}"),
            };
            let mut out_a = vec![0u8; len];
            let mut out_b = vec![0u8; len];
            match production.assemble_frame(&mut out_a) {
                Ok(()) => {}
                Err(e) => unreachable!("assemble a: {e:?}"),
            }
            match staged.assemble_frame(&mut out_b) {
                Ok(()) => {}
                Err(e) => unreachable!("assemble b: {e:?}"),
            }
            assert_eq!(out_a, out_b, "staged-full output diverged");
        }
    }

    #[test]
    fn assemble_frame_rejects_a_wrong_length_destination() {
        // The length contract is loud: a destination that disagrees
        // with the walked geometry is a typed error, never a partial
        // frame and never a panic.
        let jpeg = tiny_jpeg(16, 16, &[(1, 0x11, 0)], &[0x00]);
        let mut ctx = DecodeContext::new();
        match ctx.decode_scan(&jpeg) {
            Ok(_) => {}
            Err(e) => unreachable!("scan: {e:?}"),
        }
        let len = match ctx.frame_len() {
            Ok(len) => len,
            Err(e) => unreachable!("frame_len: {e:?}"),
        };
        let mut too_short = vec![0u8; len - 1];
        assert!(ctx.assemble_frame(&mut too_short).is_err());
        let mut too_long = vec![0u8; len + 1];
        assert!(ctx.assemble_frame(&mut too_long).is_err());
        let mut exact = vec![0u8; len];
        match ctx.assemble_frame(&mut exact) {
            Ok(()) => {}
            Err(e) => unreachable!("exact length assembles: {e:?}"),
        }
        let reference = match decode(&jpeg) {
            Ok(image) => image,
            Err(e) => unreachable!("stateless reference: {e:?}"),
        };
        assert_eq!(exact, reference.data);
    }

    /// Insert an AVI1 APP0 (payload `AVI1\0` + polarity byte) right
    /// after the SOI of a `tiny_jpeg` frame.
    fn with_avi1(jpeg: &[u8], polarity: u8) -> Vec<u8> {
        let mut out = Vec::with_capacity(jpeg.len() + 10);
        out.extend_from_slice(&jpeg[..2]);
        out.extend_from_slice(&[0xff, 0xe0, 0x00, 0x08]);
        out.extend_from_slice(b"AVI1\0");
        out.push(polarity);
        out.extend_from_slice(&jpeg[2..]);
        out
    }

    #[test]
    fn avi1_interleaved_frame_is_metadata_only_bit_exact() {
        // Polarity 2 (both fields, lines interleaved — the form
        // interlaced content takes): the marker classifies the frame,
        // it changes no sample. The pixel bytes must equal the
        // marker-free decode, and the polarity must surface on the
        // metadata (card 5.H3's order-preserving guarantee: stream
        // row order IS field order, so a top-field-first source stays
        // top-field-first in the output).
        let plain = tiny_jpeg(16, 16, &[(1, 0x22, 0), (2, 0x11, 0), (3, 0x11, 0)], &[0x00]);
        let marked = with_avi1(&plain, 2);
        let a = match decode(&plain) {
            Ok(image) => image,
            Err(e) => unreachable!("plain decode: {e:?}"),
        };
        let b = match decode(&marked) {
            Ok(image) => image,
            Err(e) => unreachable!("AVI1 decode: {e:?}"),
        };
        assert_eq!(a.field_polarity, None, "plain frame is progressive");
        assert_eq!(b.field_polarity, Some(FieldPolarity::Interleaved));
        assert_eq!(b.data, a.data, "the AVI1 marker is metadata only");
        assert_eq!((b.width, b.height), (a.width, a.height));
        assert!(!b.concealed, "a well-formed interleaved field frame");
    }

    #[test]
    fn avi1_single_field_decodes_at_field_height() {
        // Polarity 0 (first field only): the SOF height IS the field
        // height — the picture decodes at its own (half-frame)
        // dimensions, flagged with its polarity for the presentation
        // layer (module docs: weaving is compositing, not decoding).
        let field_jpeg = tiny_jpeg(16, 8, &[(1, 0x11, 0)], &[0x00]);
        let marked = with_avi1(&field_jpeg, 0);
        let image = match decode(&marked) {
            Ok(image) => image,
            Err(e) => unreachable!("AVI1 field decode: {e:?}"),
        };
        assert_eq!((image.width, image.height), (16, 8));
        assert_eq!(image.field_polarity, Some(FieldPolarity::FirstField));
        assert!(!image.concealed);
        // And polarity 1 (second field) is the same shape of picture.
        let marked = with_avi1(&field_jpeg, 1);
        let image = match decode(&marked) {
            Ok(image) => image,
            Err(e) => unreachable!("AVI1 field decode: {e:?}"),
        };
        assert_eq!((image.width, image.height), (16, 8));
        assert_eq!(image.field_polarity, Some(FieldPolarity::SecondField));
    }

    #[test]
    fn restart_boundaries_crossing_mcu_rows_decode_bit_exact() {
        // Card 5.H3 edge: restart intervals whose boundaries land
        // mid-MCU-row. 24x16 grayscale = a 3x2 MCU grid; DRI=2 puts
        // restarts before MCU 2 (a row boundary) and MCU 4 (mid-row)
        // — both crossing classes in one stream. Each MCU carries one
        // 0x00 entropy byte (2 real bits + byte padding), then the
        // RST marker; the decode must equal the restart-free stream
        // bit-for-bit.
        let plain = tiny_jpeg(24, 16, &[(1, 0x11, 0)], &[0x00]);
        // Rebuild with DRI and interleaved RST markers: headers up to
        // (not including) SOS, then DRI (tables may precede SOS
        // anywhere per §B.2), then SOS, then the restart-interleaved
        // entropy tail instead of the plain zero run.
        let entropy_len = 24 * 16 / 32 + 16;
        let sos_start = plain.len() - entropy_len - 2 - 10; // minus EOI + SOS seg
        assert_eq!(&plain[sos_start..sos_start + 2], &[0xff, 0xda]);
        let mut jpeg = Vec::with_capacity(plain.len() + 16);
        jpeg.extend_from_slice(&plain[..sos_start]);
        jpeg.extend_from_slice(&[0xff, 0xdd, 0x00, 0x04, 0x00, 0x02]);
        jpeg.extend_from_slice(&plain[sos_start..sos_start + 10]);
        // Entropy: MCU0, MCU1 | RST0 | MCU2, MCU3 | RST1 | MCU4, MCU5.
        jpeg.extend_from_slice(&[0x00, 0x00, 0xff, 0xd0]);
        jpeg.extend_from_slice(&[0x00, 0x00, 0xff, 0xd1]);
        jpeg.extend_from_slice(&[0x00, 0x00, 0xff, 0xd9]);
        let restarted = match decode(&jpeg) {
            Ok(image) => image,
            Err(e) => unreachable!("restart decode: {e:?}"),
        };
        let reference = match decode(&plain) {
            Ok(image) => image,
            Err(e) => unreachable!("plain decode: {e:?}"),
        };
        assert!(!restarted.concealed, "all restarts present and numbered");
        assert_eq!(restarted.data, reference.data);
        assert_eq!(restarted.format, reference.format);
    }

    #[test]
    fn odd_geometry_420_decodes_without_panic_or_padding_leak() {
        // Card 5.H3 edge: odd width AND height under 4:2:0 (87x63 —
        // chroma 44x32). The I420 output must be exactly w*h +
        // 2*ceil(w/2)*ceil(h/2) bytes with no padding columns or rows
        // leaking from the padded planes (the crop path's contract).
        let jpeg = tiny_jpeg(87, 63, &[(1, 0x22, 0), (2, 0x11, 0), (3, 0x11, 0)], &[0x00]);
        let image = match decode(&jpeg) {
            Ok(image) => image,
            Err(e) => unreachable!("odd 420 decode: {e:?}"),
        };
        assert_eq!((image.width, image.height), (87, 63));
        assert_eq!(image.format, OutputFormat::I420);
        let cw = 87usize.div_ceil(2);
        let ch = 63usize.div_ceil(2);
        assert_eq!(image.data.len(), 87 * 63 + 2 * cw * ch);
        // Flat mid-gray everywhere (zero-entropy stream): any leaked
        // plane padding would still be 0 from the fresh allocation —
        // so also decode a second frame through the SAME context
        // after a different-geometry frame, proving the reuse path
        // crops stale bytes away (write a big block first).
        let big = tiny_jpeg(96, 64, &[(1, 0x22, 0), (2, 0x11, 0), (3, 0x11, 0)], &[0x00]);
        let mut ctx = DecodeContext::new();
        ctx.decode(&big).unwrap_or_else(|e| unreachable!("{e:?}"));
        let meta = ctx.decode(&jpeg).unwrap_or_else(|e| unreachable!("{e:?}"));
        assert_eq!(meta.format, OutputFormat::I420);
        let out = ctx.output();
        assert_eq!(out.len(), 87 * 63 + 2 * cw * ch);
        assert!(out.iter().all(|&b| b == 128), "flat gray, no stale bytes");
    }
}
