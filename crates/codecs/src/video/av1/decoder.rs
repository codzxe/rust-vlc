//! The rav1d-backed AV1 decoder core (card 5.E1): the engine seam
//! and the [`codec_api::Decoder`] implementation.
//!
//! # The unsafe seam (and why it exists)
//!
//! rav1d 1.1.0's only public Rust API is its `dav1d_*` C-ABI surface
//! (`pub unsafe extern "C" fn` in `rav1d::src::lib`; the Rust-native
//! `rav1d_open`/`rav1d_send_data`/`rav1d_get_picture` are
//! `pub(crate)` — verified in the record, R6). This module is the
//! repo's second sanctioned `unsafe` exception (the MJPEG SIMD
//! kernel, `video/mjpeg/idct_simd.rs`, is the first): the workspace
//! `unsafe_code = "deny"` lint is re-allowed **for this file only**,
//! every `unsafe` block carries a `SAFETY` comment, and the seam is
//! kept to the narrow operations the ABI forces on us (raw picture
//! plane walks, an Arc handle, zeroed FFI structs of
//! nullable-pointer fields). Everything else — OBU validation, error
//! taxonomy, output framing, allocation pooling — is safe Rust in
//! this module and [`super::obu`].
//!
//! # Engine admission (the record, not hope)
//!
//! rav1d is admitted **asm-off**
//! (`docs/dependencies/records/rav1d-1.1.0.md`): the `asm` feature
//! is default-on and compiles shipped `.S`/`.asm` through
//! `cc`/`nasm-rs`; our pin turns it off, the record's sabotage build
//! proves `build.rs` inert, and the two `bitdepth_*` features that
//! remain are pure-Rust DSP gates. No dav1d, no C, anywhere in the
//! graph.
//!
//! # Packet model and damage philosophy
//!
//! One packet = one temporal unit (the IVF demuxer's contract;
//! Matroska WebM/MP4 `av01` delivery re-frames the same way).
//! [`super::obu::parse_temporal_unit`] validates framing *before*
//! the engine sees bytes; rav1d owns the bitstream proper. A frame
//! that fails inside rav1d is **per-packet damage**: the error is
//! counted, the drain continues, and the call returns
//! [`DecodeOutcome::Recovered`](codec_api::DecodeOutcome::Recovered)
//! with [`RecoverReason::CorruptInput`](codec_api::RecoverReason) —
//! the decoder resyncs at the next TU (dav1d's own resync
//! semantics). Structural damage our framing rejects surfaces as a
//! typed `ErrorKind::Corrupt` per doc 03 §4.5(d).
//!
//! # Presentation-time association (the honest way)
//!
//! rav1d emits pictures in presentation order, which can lag the
//! decode order AV1 allows; it carries no clock of ours. Each fed TU
//! stamps its `Dav1dDataProps::timestamp` with a monotonically
//! increasing counter; rav1d propagates the props to the picture it
//! produced, and a bounded pending-map (counter → packet pts) restores
//! the exact packet-time. Unmatched entries (damaged frames) age out
//! at the cap.
//!
//! # v0 output boundary (typed, not silent)
//!
//! media-types' pixel vocabulary is 8-bit today: this decoder emits
//! **8-bit 4:2:0 as [`PixelFormat::I420`]** and refuses anything
//! else (monochrome `I400`, `I422`/`I444`, 10/12-bit) with a typed
//! `av1.layout`/`av1.bitdepth` `Unsupported` error — the 10-bit
//! fixture in the differential suite pins that refusal. A 16-bit
//! planar media-types format is the recorded follow-up seam.
//!
//! # Allocation discipline (D-1)
//!
//! Output frames refill pooled [`SharedBytes`] slots (the MJPEG
//! pattern): steady-state decodes reuse a small rotation instead of
//! allocating per frame. The rav1d C ABI forces one `Dav1dData`
//! buffer per packet (`dav1d_data_create` + one copy) — recorded as
//! the ABI's cost, an engine-side seam to revisit when rav1d grows a
//! Rust-native `&[u8]` entry point.

// The second sanctioned `unsafe` exception (module docs): the rav1d
// C-ABI seam — the engine's only public surface in 1.1.0.
#![allow(unsafe_code)]

use super::obu;
use codec_api::prelude::*;
use media_types::frame::{
    ColorDescription, ColorMatrix, ColorPrimaries, ColorRange, FrameStorage, TransferCharacteristics,
    VideoFrame,
};
use media_types::packet::SharedBytes;
use media_types::{EngineError, ErrorKind, PipelineStage, Stamp};
use rav1d::include::dav1d::common::Dav1dDataProps;
use rav1d::include::dav1d::dav1d::{Dav1dContext, Dav1dSettings};
use rav1d::include::dav1d::data::Dav1dData;
use rav1d::include::dav1d::headers::{
    DAV1D_COLOR_PRI_BT2020, DAV1D_COLOR_PRI_BT470BG, DAV1D_COLOR_PRI_BT470M, DAV1D_COLOR_PRI_BT601,
    DAV1D_COLOR_PRI_BT709, DAV1D_MC_BT2020_CL, DAV1D_MC_BT2020_NCL, DAV1D_MC_BT601,
    DAV1D_PIXEL_LAYOUT_I420, DAV1D_TRC_HLG, DAV1D_TRC_SMPTE2084, DAV1D_TRC_SRGB,
};
use rav1d::include::dav1d::picture::Dav1dPicture;
use rav1d::src::lib::{
    dav1d_close, dav1d_data_create, dav1d_data_unref, dav1d_default_settings, dav1d_flush,
    dav1d_get_picture, dav1d_open, dav1d_picture_unref, dav1d_send_data,
};
use std::collections::VecDeque;
use std::ffi::c_int;
use std::ptr::NonNull;

/// Pooled output slots (the MJPEG `FRAME_SLOTS` posture): enough for
/// the sink to hold a few frames without forcing steady-state
/// allocation, bounded so a stalled consumer cannot grow us.
const FRAME_SLOTS: usize = 4;

/// Pending TU counters kept for pts restoration (bounded: damaged
/// frames that never emit must not grow the map; the oldest entry is
/// dropped on overflow, degrading that frame's pts to `Missing`).
const PENDING_CAP: usize = 1024;

/// Defensive cap on EAGAIN ping-pong during one feed (cannot happen
/// in the per-packet-drain flow; a violation is a stall, not a
/// spin).
const SEND_RETRY_CAP: usize = 64;

/// `ENOMEM` on every platform we ship (Linux, Windows, macOS, the
/// BSDs — errno 12 everywhere; rav1d returns it negated on picture
/// allocation failure).
const ERRNO_ENOMEM: c_int = 12;

/// Platform-correct `EAGAIN` test without a libc edge: Rust's own
/// `ErrorKind` mapping classifies the raw os error code for us
/// (errno values differ across platforms; the mapping does not).
fn is_eagain(code: c_int) -> bool {
    std::io::Error::from_raw_os_error(code).kind() == std::io::ErrorKind::WouldBlock
}

/// Platform-correct `EINVAL` test (same trick).
fn is_einval(code: c_int) -> bool {
    std::io::Error::from_raw_os_error(code).kind() == std::io::ErrorKind::InvalidInput
}

/// The rav1d context handle: one owned `Dav1dContext` (an opaque Arc
/// handle), closed exactly once on drop.
struct Rav1dHandle {
    ctx: Option<Dav1dContext>,
}

// SAFETY: `Dav1dContext` is `RawArc<Rav1dContext>` — an Arc handle
// that is `!Send` only because raw pointers are. rav1d upstream
// asserts `unsafe impl Send/Sync for Rav1dContext` (1.1.0's
// src/internal.rs: the context is Arc/Mutex/Atomic plumbing), so
// moving the single owned handle across threads is sound for exactly
// the same reason `Arc<Rav1dContext>: Send` would be.
unsafe impl Send for Rav1dHandle {}

impl Rav1dHandle {
    /// Open a decoder with `n_threads` helper threads (the codec-api
    /// `ThreadBudget`; rav1d derives its frame-thread count from it —
    /// 1 means fully synchronous decode, the memory-conservative
    /// default).
    fn open(n_threads: i32) -> Result<Rav1dHandle, EngineError> {
        // SAFETY: `Dav1dSettings` is a `#[repr(C)]` struct of
        // integers, bitflag-width ints, an allocator of
        // nullable-pointer callbacks and a logger of
        // nullable-pointer fields — all-zero is a valid (allocator-
        // and logger-less) instance, immediately overwritten in full
        // by `dav1d_default_settings` below. Nothing is dropped by
        // the overwrite (no `Drop` impls anywhere in the struct).
        let mut settings: Dav1dSettings = unsafe { std::mem::zeroed() };
        // SAFETY: `settings` is an initialized, aligned local of the
        // exact pointee type `dav1d_default_settings` writes.
        unsafe { dav1d_default_settings(NonNull::from(&mut settings)) };
        settings.n_threads = n_threads;
        settings.max_frame_delay = 0; // derive from n_threads
        settings.apply_grain = 1; // final pixels, the ffmpeg default
        // Silence the default stderr logger (Rav1dSettings::default
        // maps to `Some(Stderr)` — the record's R6): a library
        // component must not write to the process's stderr.
        //
        // SAFETY: `Dav1dLogger` is two nullable-pointer `Option`
        // fields; all-zero is their `None`/`None` niche encoding, and
        // a `None` callback converts to "no logging" in rav1d's
        // `TryFrom<Dav1dSettings>` (verified in source).
        settings.logger = unsafe { std::mem::zeroed() };
        let mut ctx_out: Option<Dav1dContext> = None;
        // SAFETY: both pointers are valid, aligned locals of the
        // exact pointee types; `dav1d_open` only writes through them.
        let r = unsafe {
            dav1d_open(Some(NonNull::from(&mut ctx_out)), Some(NonNull::from(&settings)))
        };
        if r.0 != 0 {
            return Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Decoder { cause: media_types::error::DecodeCause::BadState },
                media_types::error::ErrorContext::new("av1.init.engine"),
            ));
        }
        let ctx = ctx_out.take().unwrap_or_else(|| {
            unreachable!("dav1d_open success guarantees a context handle")
        });
        Ok(Rav1dHandle { ctx: Some(ctx) })
    }

    /// The context handle for an FFI call (Copy — the C ABI treats
    /// it as a borrowed reference for the call's duration).
    fn get(&self) -> Dav1dContext {
        match self.ctx {
            Some(ctx) => ctx,
            // Cleared only during Drop; every caller holds `&self`
            // of a live engine.
            None => unreachable!("context cleared only during Drop"),
        }
    }
}

impl Drop for Rav1dHandle {
    fn drop(&mut self) {
        if let Some(ctx) = self.ctx.take() {
            let mut slot = Some(ctx);
            // SAFETY: `slot` is a valid local of the exact pointee
            // type `dav1d_close` reads and nulls; the handle came
            // from `dav1d_open` and is closed exactly once (the
            // `Option` dance makes the take-once explicit).
            unsafe { dav1d_close(Some(NonNull::from(&mut slot))) };
        }
    }
}

/// The AV1 decoder behind the codec-api [`Decoder`] contract.
///
/// Stateful (AV1 is inter-frame), but damage-bounded per packet: a
/// corrupt TU is typed and skipped, the engine resyncs, the next TU
/// decodes. `flush` drains the reordering delay and resets the
/// pending-time map.
pub struct Av1Decoder {
    /// The configured stream generation (None until `configure`).
    generation: Option<GenerationId>,
    /// The rav1d engine (opened at configure; init failure typed).
    engine: Option<Rav1dHandle>,
    /// Pending TU counters awaiting their picture (module docs).
    pending: VecDeque<(u64, Stamp)>,
    /// Monotonic TU counter stamped into `Dav1dDataProps::timestamp`.
    tu_counter: u64,
    /// Pooled output slots (the MJPEG pattern, module docs).
    frame_slots: Vec<SharedBytes>,
    /// Round-robin cursor into `frame_slots`.
    slot_next: usize,
}

impl Default for Av1Decoder {
    fn default() -> Self {
        Av1Decoder {
            generation: None,
            engine: None,
            pending: VecDeque::new(),
            tu_counter: 0,
            frame_slots: Vec::new(),
            slot_next: 0,
        }
    }
}

impl Av1Decoder {
    /// A fresh decoder, unconfigured.
    pub fn new() -> Av1Decoder {
        Av1Decoder::default()
    }

    /// The codec descriptors this decoder accepts (the IVF demuxer's
    /// honest `av1` name; `av01` accepted as the fourcc spelling).
    fn accepts(descriptor: &str) -> bool {
        matches!(descriptor, "av1" | "av01")
    }

    /// Fold accumulated per-call damage into an outcome (module
    /// docs: counted, resynced, never fatal).
    fn take_damage(outcome: &mut DecodeOutcome, damage: u32) {
        if damage > 0 {
            let mut skipped = damage;
            if let DecodeOutcome::Recovered { skipped: s, .. } = *outcome {
                skipped = skipped.saturating_add(s);
            }
            *outcome = DecodeOutcome::Recovered { skipped, reason: RecoverReason::CorruptInput };
        }
    }

    /// Stamp this TU's counter and pts into the pending map (with
    /// the module-doc aging policy).
    fn push_pending(&mut self, counter: u64, pts: Stamp) {
        if self.pending.len() == PENDING_CAP {
            self.pending.pop_front();
        }
        self.pending.push_back((counter, pts));
    }

    /// Pop the pts stamped for the picture's TU counter
    /// (presentation order may differ from feed order, so search;
    /// bounded by `PENDING_CAP`).
    fn pop_pending(&mut self, counter: u64) -> Stamp {
        if let Some(i) = self.pending.iter().position(|&(c, _)| c == counter) {
            match self.pending.remove(i) {
                Some((_, pts)) => pts,
                None => Stamp::Missing,
            }
        } else {
            Stamp::Missing
        }
    }

    /// Feed one TU's bytes to the engine, draining pictures (the
    /// dav1d send/get discipline: EAGAIN on send means drain first).
    ///
    /// Returns the emitted-frame count; damage accrues in `damage`.
    fn feed_tu(
        &mut self,
        counter: u64,
        bytes: &[u8],
        out: &mut dyn FrameSink,
        generation: GenerationId,
        damage: &mut u32,
    ) -> Result<usize, EngineError> {
        let ctx = match &self.engine {
            Some(engine) => engine.get(),
            None => unreachable!("feed only runs with an engine"),
        };
        let len = bytes.len();
        let mut data = Dav1dData::default();
        // SAFETY: `data` is a valid local of the exact pointee type;
        // on success `dav1d_data_create` installs an owned `sz`-byte
        // buffer in it and returns the writable pointer (a null
        // return is the allocation-failure path we check).
        let ptr = unsafe { dav1d_data_create(Some(NonNull::from(&mut data)), len) };
        if ptr.is_null() {
            return Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::ResourceLimit {
                    limit: media_types::error::LimitKind::Memory,
                    budget: 0,
                    attempted: len as u64,
                },
                media_types::error::ErrorContext::new("av1.feed.alloc"),
            ));
        }
        // SAFETY: `ptr` is the `len`-byte buffer just created and
        // owned by `data` (or the engine once sent); `bytes` and it
        // cannot overlap (a fresh heap allocation).
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, len) };
        // The props ride with the data into the picture (module
        // docs): stamp the TU counter.
        data.m = Dav1dDataProps { timestamp: counter as i64, ..Default::default() };

        let result = self.feed_loop(ctx, &mut data, out, generation, damage);
        if data.r#ref.is_some() {
            // The engine did not take the buffer (error path, or a
            // stall): release our reference — `Dav1dData` is a C
            // struct with no `Drop`, an unreleased `r#ref` would
            // leak the allocation.
            //
            // SAFETY: `data` is the valid local that owns the
            // reference `dav1d_data_create` installed and that no
            // successful send has taken.
            unsafe { dav1d_data_unref(Some(NonNull::from(&mut data))) };
        }
        result
    }

    /// The send/retry core of [`Self::feed_tu`] (split so the buffer
    /// cleanup above always runs).
    fn feed_loop(
        &mut self,
        ctx: Dav1dContext,
        data: &mut Dav1dData,
        out: &mut dyn FrameSink,
        generation: GenerationId,
        damage: &mut u32,
    ) -> Result<usize, EngineError> {
        let mut emitted = 0usize;
        for _ in 0..SEND_RETRY_CAP {
            // SAFETY: `ctx` is the live handle (borrowed for the
            // call); `data` is a valid pointee of the exact type the
            // ABI reads and rewrites.
            let r = unsafe { dav1d_send_data(Some(ctx), Some(NonNull::from(&mut *data))) };
            if r.0 == 0 {
                emitted += self.drain_pictures(out, generation, damage)?;
                return Ok(emitted);
            }
            if is_eagain(r.0) {
                // Engine input is still pending: drain pictures so it
                // can consume, then retry.
                emitted += self.drain_pictures(out, generation, damage)?;
                continue;
            }
            return Err(classify_engine_error(r.0, "av1.feed.rejected"));
        }
        Err(EngineError::new(
            PipelineStage::Decode,
            ErrorKind::Decoder { cause: media_types::error::DecodeCause::BadState },
            media_types::error::ErrorContext::new("av1.feed.stalled"),
        ))
    }

    /// Drain every ready picture (until EAGAIN), copying each into a
    /// pooled slot and pushing it to `out`.
    fn drain_pictures(
        &mut self,
        out: &mut dyn FrameSink,
        generation: GenerationId,
        damage: &mut u32,
    ) -> Result<usize, EngineError> {
        let ctx = match &self.engine {
            Some(engine) => engine.get(),
            None => unreachable!("drain only runs with an engine"),
        };
        let mut emitted = 0usize;
        loop {
            let mut pic = Dav1dPicture::default();
            // SAFETY: `ctx` is the live handle; `pic` is a valid,
            // aligned local of the exact pointee type the ABI writes
            // on success.
            let r = unsafe { dav1d_get_picture(Some(ctx), Some(NonNull::from(&mut pic))) };
            if r.0 == 0 {
                // Copy out (and unref) before propagating any emit
                // error, so the engine never leaks a picture
                // reference on the unsupported-layout path.
                let emit = self.emit_picture(&pic, out, generation);
                // SAFETY: `pic` was filled by `dav1d_get_picture`
                // above and not yet unref'd; the ABI takes the
                // picture's references back through this pointer.
                unsafe { dav1d_picture_unref(Some(NonNull::from(&mut pic))) };
                emit?;
                emitted += 1;
                continue;
            }
            if is_eagain(r.0) {
                return Ok(emitted);
            }
            if is_einval(r.0) {
                return Err(EngineError::new(
                    PipelineStage::Decode,
                    ErrorKind::Decoder { cause: media_types::error::DecodeCause::BadState },
                    media_types::error::ErrorContext::new("av1.picture.einval"),
                ));
            }
            if r.0 == -ERRNO_ENOMEM {
                return Err(EngineError::new(
                    PipelineStage::Decode,
                    ErrorKind::ResourceLimit {
                        limit: media_types::error::LimitKind::Memory,
                        budget: 0,
                        attempted: 0,
                    },
                    media_types::error::ErrorContext::new("av1.picture.alloc"),
                ));
            }
            // Bitstream damage on one frame (EGeneric/EIO/ERANGE/…):
            // rav1d latches the error for exactly one get call; the
            // next TU resyncs. Count and keep draining.
            *damage += 1;
        }
    }

    /// Copy one rav1d picture into a pooled slot and push the
    /// [`Frame::Video`] to `out`.
    fn emit_picture(
        &mut self,
        pic: &Dav1dPicture,
        out: &mut dyn FrameSink,
        generation: GenerationId,
    ) -> Result<(), EngineError> {
        let w = pic.p.w;
        let h = pic.p.h;
        if w <= 0 || h <= 0 {
            return Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Corrupt {
                    detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
                },
                media_types::error::ErrorContext::new("av1.picture.geometry"),
            ));
        }
        // v0 output boundary (module docs): 8-bit 4:2:0 only —
        // everything else is a typed refusal, never a misread.
        let unsupported = |code: &'static str| {
            EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Unsupported {
                    format: Some(media_types::error::FormatInfo {
                        name: Some("av1".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                media_types::error::ErrorContext::new(code),
            )
        };
        if pic.p.layout != DAV1D_PIXEL_LAYOUT_I420 {
            return Err(unsupported("av1.layout"));
        }
        if pic.p.bpc != 8 {
            return Err(unsupported("av1.bitdepth"));
        }
        let (w, h) = (w as usize, h as usize);
        let y_stride = pic.stride[0];
        let uv_stride = pic.stride[1];
        // The default allocator only produces positive strides
        // (aligned width + optional 64-byte pad); a negative stride
        // would need bottom-up plane walks — refused typed, never
        // misread.
        if y_stride <= 0 || uv_stride <= 0 {
            return Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Unsupported {
                    format: Some(media_types::error::FormatInfo {
                        name: Some("av1".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                media_types::error::ErrorContext::new("av1.stride"),
            ));
        }
        let (y_stride, uv_stride) = (y_stride as usize, uv_stride as usize);
        let cw = w.div_ceil(2);
        let ch = h.div_ceil(2);
        let len = w * h + 2 * cw * ch;
        let [y, u, v] = pic.data;
        let (Some(y), Some(u), Some(v)) = (y, u, v) else {
            return Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Corrupt {
                    detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
                },
                media_types::error::ErrorContext::new("av1.picture.no_planes"),
            ));
        };
        let color = picture_color(pic);
        let counter = u64::try_from(pic.m.timestamp.max(0)).unwrap_or(u64::MAX);
        let pts = self.pop_pending(counter);

        let data = pooled_frame_slot(&mut self.frame_slots, &mut self.slot_next, len, |bytes| {
            // SAFETY: the three plane pointers come from a live
            // `Dav1dPicture` filled by `dav1d_get_picture` and are
            // valid until `dav1d_picture_unref` (which the caller
            // performs only after we return). Per the allocator
            // contract each plane holds `stride × aligned_h ≥
            // stride × h` bytes, and we read `w ≤ stride` bytes per
            // row for `h` rows — every read is in-bounds.
            unsafe {
                copy_plane(bytes, y.cast().as_ptr(), h, w, y_stride);
                copy_plane(&mut bytes[w * h..], u.cast().as_ptr(), ch, cw, uv_stride);
                copy_plane(
                    &mut bytes[w * h + cw * ch..],
                    v.cast().as_ptr(),
                    ch,
                    cw,
                    uv_stride,
                );
            }
        })?;

        let frame = Frame::Video(VideoFrame {
            format: media_types::PixelFormat::I420,
            size: (w as u32, h as u32),
            data: FrameStorage::Cpu(data),
            pts,
            color,
            interlaced: false,
            generation,
        });
        out.accept(frame)
    }
}

/// Copy `rows × row_bytes` from a strided plane into tightly packed
/// `dst`.
///
/// # Safety
/// `src` must point to a plane of at least `(rows − 1) · stride +
/// row_bytes` readable bytes (the caller establishes the lifetime
/// and the `row_bytes ≤ stride` bound).
unsafe fn copy_plane(
    dst: &mut [u8],
    src: *const u8,
    rows: usize,
    row_bytes: usize,
    stride: usize,
) {
    for row in 0..rows {
        // SAFETY: caller-guaranteed readable `row_bytes` at the
        // row start (`src + row·stride`, within the plane), and the
        // arithmetic stays in-bounds of the allocation.
        //
        // SAFETY (the unsafe-op-in-unsafe-fn discipline, edition
        // 2024): this unsafe fn's whole contract is the caller's
        // bounds guarantee above.
        let src_row = unsafe { src.add(row * stride) };
        let dst_row = &mut dst[row * row_bytes..(row + 1) * row_bytes];
        // SAFETY: `src_row` (caller-bound) and `dst_row` (an exact
        // disjoint slice) cannot overlap.
        unsafe { std::ptr::copy_nonoverlapping(src_row, dst_row.as_mut_ptr(), row_bytes) };
    }
}

/// Map the picture's sequence-header colorimetry onto the
/// media-types vocabulary (unsignaled → the conservative BT.709
/// limited default, the module-doc assumption; unmappable exotic
/// values likewise — the renderer owns the fallback rendering).
fn picture_color(pic: &Dav1dPicture) -> ColorDescription {
    // SAFETY: `seq_hdr`, when present, is an owned `RawArc` target
    // installed by `dav1d_get_picture`; it stays alive until
    // `dav1d_picture_unref` (which the caller performs only after
    // this function returns) — a shared read is sound, and the
    // struct is plain `#[repr(C)]` data.
    let seq = pic.seq_hdr.map(|p| unsafe { p.as_ref() });
    let Some(seq) = seq else {
        return ColorDescription::BT709_LIMITED;
    };
    let primaries = match seq.pri {
        DAV1D_COLOR_PRI_BT709 => ColorPrimaries::Bt709,
        DAV1D_COLOR_PRI_BT601 | DAV1D_COLOR_PRI_BT470M | DAV1D_COLOR_PRI_BT470BG => {
            ColorPrimaries::Bt601
        }
        DAV1D_COLOR_PRI_BT2020 => ColorPrimaries::Bt2020,
        _ => ColorPrimaries::Bt709,
    };
    let transfer = match seq.trc {
        DAV1D_TRC_SRGB => TransferCharacteristics::Srgb,
        DAV1D_TRC_SMPTE2084 => TransferCharacteristics::Pq,
        DAV1D_TRC_HLG => TransferCharacteristics::Hlg,
        _ => TransferCharacteristics::Bt709,
    };
    let matrix = match seq.mtrx {
        DAV1D_MC_BT601 => ColorMatrix::Bt601,
        DAV1D_MC_BT2020_NCL | DAV1D_MC_BT2020_CL => ColorMatrix::Bt2020Ncl,
        _ => ColorMatrix::Bt709,
    };
    let range = if seq.color_range != 0 { ColorRange::Full } else { ColorRange::Limited };
    ColorDescription { primaries, transfer, matrix, range }
}

/// Classify a non-EAGAIN engine return from the feed path.
fn classify_engine_error(code: c_int, site: &'static str) -> EngineError {
    if code == -ERRNO_ENOMEM {
        EngineError::new(
            PipelineStage::Decode,
            ErrorKind::ResourceLimit {
                limit: media_types::error::LimitKind::Memory,
                budget: 0,
                attempted: 0,
            },
            media_types::error::ErrorContext::new("av1.feed.alloc"),
        )
    } else if is_einval(code) {
        EngineError::new(
            PipelineStage::Decode,
            ErrorKind::Decoder { cause: media_types::error::DecodeCause::BadState },
            media_types::error::ErrorContext::new(site),
        )
    } else {
        // The bitstream itself was rejected by the engine (a bad
        // sequence header, an unparseable OBU chain our framing pass
        // tolerated) — per-frame damage, the next TU resyncs.
        EngineError::new(
            PipelineStage::Decode,
            ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
            },
            media_types::error::ErrorContext::new(site),
        )
    }
}

/// Pooled output-slot rotation (the MJPEG `pooled_frame_slot`
/// pattern): steady-state decodes refill a slot in place; a slot
/// still owned downstream falls through to the next, and only an
/// all-busy rotation allocates.
fn pooled_frame_slot(
    slots: &mut Vec<SharedBytes>,
    next: &mut usize,
    len: usize,
    fill: impl FnOnce(&mut [u8]),
) -> Result<SharedBytes, EngineError> {
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != len {
            continue;
        }
        if let Some(bytes) = slots[idx].get_mut() {
            fill(bytes);
            *next = (idx + 1) % count;
            return Ok(slots[idx].clone());
        }
        // The slot still has downstream owners — try the next one.
    }
    let mut fresh = SharedBytes::from_vec(vec![0; len]);
    match fresh.get_mut() {
        Some(bytes) => fill(bytes),
        None => {
            // A freshly built `SharedBytes` has refcount 1 by
            // construction; `get_mut` failing here is impossible —
            // surface it loudly instead of fabricating an empty
            // frame.
            return Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Corrupt {
                    detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
                },
                media_types::error::ErrorContext::new("av1.output.fresh_slot_not_exclusive"),
            ));
        }
    }
    if slots.len() < FRAME_SLOTS {
        slots.push(fresh.clone());
    }
    *next = 0;
    Ok(fresh)
}

impl Decoder for Av1Decoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        if !Self::accepts(cfg.codec.as_str()) {
            return Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Unsupported {
                    format: Some(media_types::error::FormatInfo {
                        name: Some(cfg.codec.to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                media_types::error::ErrorContext::new("av1.codec_descriptor"),
            ));
        }
        let threads = i32::try_from(cfg.threads.threads()).unwrap_or(1);
        // Re-configuration on a fresh generation after a seek: drop
        // the old engine (its reference frames are stale) and open a
        // clean one — the honest flush, since rav1d's references
        // cannot be selectively discarded.
        self.engine = Some(Rav1dHandle::open(threads)?);
        self.pending.clear();
        self.frame_slots.clear();
        self.slot_next = 0;
        self.tu_counter = 0;
        self.generation = Some(cfg.generation);
        Ok(())
    }

    /// Decode one packet (one temporal unit).
    ///
    /// Per-frame isolation (doc 03 §4.5(d)): a typed `Corrupt`
    /// error or a `Recovered` outcome damaged this packet only —
    /// rav1d resyncs at the next TU (its OBU parser scans forward
    /// for frame boundaries), so the next packet decodes normally.
    #[allow(clippy::missing_errors_doc)]
    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        let Some(generation) = self.generation else {
            return Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Decoder { cause: media_types::error::DecodeCause::BadState },
                media_types::error::ErrorContext::new("av1.not_configured"),
            ));
        };
        if self.engine.is_none() {
            return Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Decoder { cause: media_types::error::DecodeCause::BadState },
                media_types::error::ErrorContext::new("av1.no_engine"),
            ));
        }
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        // A discontinuity (seek landed mid-stream): the engine's
        // reference frames are stale — flush it before feeding.
        if packet.discontinuity {
            if let Some(engine) = &self.engine {
                // SAFETY: `engine.get()` is the live handle; the ABI
                // only touches engine state.
                unsafe { dav1d_flush(engine.get()) };
            }
            self.pending.clear();
        }
        // Frame the TU first — typed, byte-precise damage before the
        // engine sees anything.
        let tu = obu::parse_temporal_unit(packet.data.as_slice())?;
        let counter = self.tu_counter;
        if tu.may_produce_picture {
            self.push_pending(counter, packet.pts);
        }
        self.tu_counter = counter.wrapping_add(1);
        let mut damage = 0u32;
        let emitted =
            self.feed_tu(counter, packet.data.as_slice(), out, packet.generation, &mut damage)?;
        let mut outcome = if emitted > 0 {
            DecodeOutcome::Emitted { frames: emitted as u32 }
        } else {
            DecodeOutcome::NeedMoreInput
        };
        Self::take_damage(&mut outcome, damage);
        Ok(outcome)
    }

    fn flush(&mut self, out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        let Some(generation) = self.generation else {
            return Err(EngineError::new(
                PipelineStage::Decode,
                ErrorKind::Decoder { cause: media_types::error::DecodeCause::BadState },
                media_types::error::ErrorContext::new("av1.not_configured"),
            ));
        };
        let ctx = match &self.engine {
            Some(engine) => engine.get(),
            None => {
                return Err(EngineError::new(
                    PipelineStage::Decode,
                    ErrorKind::Decoder { cause: media_types::error::DecodeCause::BadState },
                    media_types::error::ErrorContext::new("av1.no_engine"),
                ));
            }
        };
        // End-of-stream signal: feeding an empty `Dav1dData` sets the
        // engine's drain flag (its documented EOS idiom), after
        // which `dav1d_get_picture` yields the reordering delay.
        let mut eos = Dav1dData::default();
        // SAFETY: `ctx` is the live handle; `eos` is a valid local
        // of the exact pointee type (all-None fields — the empty-
        // data form the ABI defines).
        unsafe { dav1d_send_data(Some(ctx), Some(NonNull::from(&mut eos))) };
        let mut damage = 0u32;
        let emitted = self.drain_pictures(out, generation, &mut damage)?;
        self.pending.clear();
        let mut outcome = DecodeOutcome::Emitted { frames: emitted as u32 };
        Self::take_damage(&mut outcome, damage);
        if emitted > 0 || damage > 0 {
            Ok(outcome)
        } else {
            Ok(DecodeOutcome::Drained)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_gate_accepts_the_delivery_vocabulary() {
        assert!(Av1Decoder::accepts("av1"));
        assert!(Av1Decoder::accepts("av01"));
        assert!(!Av1Decoder::accepts("vp9"));
        assert!(!Av1Decoder::accepts(""));
    }

    #[test]
    fn eagain_classification_is_platform_correct() {
        // The errno value behind WouldBlock on this platform, found
        // through std's own mapping (Linux 11, macOS/BSD 35).
        let code = raw_would_block();
        assert!(is_eagain(-code));
        assert!(!is_eagain(-22));
        assert!(is_einval(-22));
        assert!(!is_einval(-code));
    }

    /// This platform's `EAGAIN` raw value, via std's own mapping.
    fn raw_would_block() -> c_int {
        for code in 1..200 {
            if std::io::Error::from_raw_os_error(code).kind() == std::io::ErrorKind::WouldBlock {
                return code;
            }
        }
        unreachable!("some errno maps to WouldBlock on every supported platform")
    }

    #[test]
    fn damage_folds_into_recovered() {
        let mut outcome = DecodeOutcome::Emitted { frames: 2 };
        Av1Decoder::take_damage(&mut outcome, 1);
        assert_eq!(
            outcome,
            DecodeOutcome::Recovered { skipped: 1, reason: RecoverReason::CorruptInput }
        );
        Av1Decoder::take_damage(&mut outcome, 3);
        assert_eq!(
            outcome,
            DecodeOutcome::Recovered { skipped: 4, reason: RecoverReason::CorruptInput }
        );
        // Zero damage leaves outcomes untouched.
        let mut drained = DecodeOutcome::Drained;
        Av1Decoder::take_damage(&mut drained, 0);
        assert_eq!(drained, DecodeOutcome::Drained);
    }

    #[test]
    fn pending_map_bounded_and_lifo_tolerant() {
        let mut decoder = Av1Decoder::new();
        for i in 0..PENDING_CAP {
            decoder.push_pending(i as u64, Stamp::Missing);
        }
        decoder.push_pending(PENDING_CAP as u64, Stamp::Missing);
        assert_eq!(decoder.pending.len(), PENDING_CAP);
        // Out-of-order (presentation vs feed) resolution works.
        let pts = media_types::time::Timestamp::from_nanos(42);
        decoder.push_pending(9999, Stamp::Present(pts));
        assert_eq!(decoder.pop_pending(9999), Stamp::Present(pts));
        assert_eq!(decoder.pop_pending(9999), Stamp::Missing);
    }
}
