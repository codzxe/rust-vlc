//! The VideoToolbox H.264 decode session (card 7.B1, issue #41):
//! the [`H264HwDecoder`](super::H264HwDecoder) implementation for
//! the macOS lane.
//!
//! # Runtime honesty (read this first)
//!
//! This sandbox is a Linux host with **no macOS SDK and no Apple
//! hardware**: every OS call below is **compile-verified only**, via
//! `cargo check --target aarch64-apple-darwin` — **runtime hardware
//! behavior NOT exercised — `[VERIFY]`** (the same label the lane's
//! [`ffi`] module carries for its ABI declarations and key strings).
//! The host-testable pure halves of this lane are deliberately
//! OS-neutral and live in [`super::super::feed`] (the AVCC sample
//! form, the CMTime nanosecond round trip) — those run in this
//! host's suite; nothing in this file does (a `#[cfg(test)]` module
//! here would never compile in a gate, so none is written — that is
//! the honest shape, not an omission to fix later).
//!
//! # Session shape
//!
//! [`VideoToolboxSession`] is minted by the bridge as **unopened**
//! (the bridge's job is only "this accelerator is mine"). The first
//! [`configure`](H264HwDecoder::configure) performs the whole
//! bring-up — format description off the raw SPS/PPS NALs, the
//! hardware-required decoder specification, the destination pixel
//! buffer attributes, the decompression session — so every failure
//! lands in the **typed** `Configure` error channel, never a panic,
//! never a fake session. A re-configure tears the session down and
//! rebuilds it.
//!
//! # The decode pipeline (one picture)
//!
//! ```text
//! feed::avcc_sample_into(au.units) ─▶ CMBlockBuffer (self-allocating,
//!   bytes copied in) ─▶ CMSampleBufferCreateReady ─▶ output PTS
//!   (CMTime @ 10⁹) ─▶ VTDecompressionSessionDecodeFrame (async flag)
//!   ─▶ callback retains CVPixelBuffer → shared queue ─▶ drain():
//!   pool slot acquire + emit ─▶ H264HwFrame{surface: pool handle}
//! ```
//!
//! The asynchronous decode contract maps exactly onto the
//! [`H264HwDecoder`] shape: frames surface via the output callback
//! during this call, a later call, or at
//! [`flush`](H264HwDecoder::flush)
//! (`VTDecompressionSessionWaitForAsynchronousFrames`); the drain
//! step moves whatever surfaced into the call's return value.
//!
//! # Hardware-required honesty (ADR-001)
//!
//! The decoder specification carries `RequireHardwareDecode = true`
//! (the exported `kCFBooleanTrue` singleton, linked from
//! CoreFoundation): without it VideoToolbox may silently serve its
//! *software* H.264 decoder, which would break ADR-001's truthful
//! active-decoder report from the inside. A machine without a
//! hardware VT decoder therefore fails session creation — the typed
//! `h264.hw.videotoolbox.session` `Configure` error — never a
//! silent software fallback.
//!
//! # Typed refusals (honest gaps, never guesses)
//!
//! | stream feature | code | why |
//! |---|---|---|
//! | field pictures (PAFF/MBAFF, `frame_mbs_only` clear) | `h264.hw.videotoolbox.field_picture` | the seam's per-frame `interlaced` reporting is unexercised; same typed refusal as the VA-API lane (consistency over unverifiable capability) |
//! | profile ≠ Baseline/Main/High, or chroma ≠ 4:2:0 8-bit | `h264.hw.videotoolbox.profile` | VideoToolbox exposes no H.264 Hi10P/4:2:2 decoder; refuse typed instead of letting the OS error dangle |
//! | MVC (prefix/subset-SPS NALs) | `h264.hw.videotoolbox.mvc` | single-view lane |
//! | a NAL exceeding the AVCC `u32` length space | `h264.hw.videotoolbox.sample` | the feed refuses to truncate — bounded-input honesty |
//! | surfaced frames beyond the pool cap | `h264.hw.videotoolbox.pool_exhausted` | bounded back-pressure (D-2/D-3), never an unbounded retain |
//!
//! # Safety posture
//!
//! Module-level `#[allow(unsafe_code)]` (ADR-001 §5: VideoToolbox /
//! CoreMedia / CoreVideo / CoreFoundation are the E2 OS-service
//! binding; MP §9 / doc 01 §2.7 audited exception). Every block
//! documents its invariants; the one struct crossing the boundary
//! by value (`CMTime`) is size-pinned in [`ffi`].

// SAFETY-SCOPED MODULE: the Apple framework C ABIs ffi.rs declares
// (VideoToolbox / CoreMedia / CoreVideo / CoreFoundation).
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::sync::Arc;
use std::sync::Mutex;

use super::super::capability::{HwAvailability, HwBackend, HwUnavailableReason};
use super::super::feed;
use super::super::frame_pool::{HwSurfacePool, MAX_SURFACES};
use super::super::{
    H264HwAccessUnit, H264HwDecoded, H264HwDecoder, H264HwError, H264HwFrame, H264HwPps, H264HwSps,
    H264HwStreamInfo,
};
use super::ffi;
use crate::video::h264::parser::nal::NalUnitType;
use crate::video::h264::parser::pps::PicParameterSet;
use crate::video::h264::parser::sps::SeqParameterSet;
use media_types::Stamp;

/// Surfaces beyond the SPS's declared DPB (present-queue headroom —
/// mirrors the VA-API lane's guidance "ref frames plus a few").
const SURFACES_BEYOND_DPB: usize = 6;

/// Minimum emitted-surface pool depth (intra-only streams still
/// need decode + emit headroom).
const MIN_SURFACES: usize = 4;

/// `kVTVideoDecoderSpecification_RequireHardwareDecode` — the
/// CFString's content per VideoToolbox's public header docs
/// `[VERIFY: no macOS SDK in this sandbox to diff against]`.
const KEY_REQUIRE_HARDWARE_DECODE: &str = "RequireHardwareDecode";
/// `kCVPixelBufferPixelFormatTypeKey` (content per CoreVideo's
/// public header docs — `[VERIFY]` as above).
const KEY_PIXEL_FORMAT_TYPE: &str = "PixelFormatType";
/// `kCVPixelBufferWidthKey` (`[VERIFY]` as above).
const KEY_WIDTH: &str = "Width";
/// `kCVPixelBufferHeightKey` (`[VERIFY]` as above).
const KEY_HEIGHT: &str = "Height";

// ===========================================================================
// Pure helpers (compile-verified here; the wire forms + the stamp
// round trip they ride are host-tested in feed.rs).
// ===========================================================================

/// The profiles this lane serves: Baseline (66), Extended (88) →
/// Main superset, Main (77), High (100) — 8-bit 4:2:0 only
/// (VideoToolbox exposes no H.264 Hi10P/4:2:2 decoder; separate
/// colour planes (idc 3) are equally out).
fn vt_serves(sps: &SeqParameterSet) -> bool {
    matches!(sps.profile_idc, 66 | 77 | 88 | 100)
        && sps.chroma_format_idc.unwrap_or(1) <= 1
        && sps.bit_depth_luma_minus8 == 0
        && sps.bit_depth_chroma_minus8 == 0
        && !sps.separate_colour_plane
}

/// Coded (macroblock-aligned) size — the destination-surface
/// negotiation dimensions (display cropping is the present side's
/// business, carried separately). `None` on arithmetic overflow —
/// typed upstream, never a truncating cast.
fn coded_size(sps: &SeqParameterSet) -> Option<(u32, u32)> {
    if !sps.frame_mbs_only {
        return None; // field-picture streams are refused typed before this
    }
    let width = (sps.pic_width_in_mbs_minus1.checked_add(1)?) * 16;
    let height = (sps.pic_height_in_map_units_minus1.checked_add(1)?) * 16;
    Some((width, height))
}

/// Emitted-surface pool depth: the SPS's declared DPB bound plus
/// present headroom, floored at [`MIN_SURFACES`], capped by the
/// pool's hard [`MAX_SURFACES`] (bounded by construction).
fn pool_capacity(sps: &SeqParameterSet) -> usize {
    (usize::from(sps.max_num_ref_frames) + SURFACES_BEYOND_DPB).clamp(MIN_SURFACES, MAX_SURFACES)
}

/// Pack a stamp into the `CMTime` the sample carries (timescale
/// 10⁹ — the exact base [`feed::stamp_nanos`] round-trips). A
/// missing stamp becomes the invalid `CMTime` (no flags), which
/// [`stamp_of`] reads back as `Stamp::Missing`.
fn cm_time_of(stamp: Stamp) -> ffi::CMTime {
    match feed::stamp_nanos(stamp) {
        Some(nanos) => ffi::CMTime {
            value: nanos,
            timescale: feed::NANO_TIMESCALE,
            flags: ffi::KCM_TIME_FLAGS_VALID,
            epoch: 0,
        },
        None => ffi::CMTime {
            value: 0,
            timescale: 0,
            flags: 0,
            epoch: 0,
        },
    }
}

/// Unpack a `CMTime` back into a stamp (the inverse of
/// [`cm_time_of`]): anything that is not the valid 10⁹ form is
/// `Stamp::Missing` — never a guessed time.
fn stamp_of(time: &ffi::CMTime) -> Stamp {
    if time.flags & ffi::KCM_TIME_FLAGS_VALID != 0 && time.timescale == feed::NANO_TIMESCALE {
        feed::nanos_stamp(time.value)
    } else {
        Stamp::Missing
    }
}

/// The SPS/PPS pair a configure uses, by reference into the stream
/// info (lowest-id PPS → its SPS — the decode-side "active"
/// convention, the VA-API lane's rule).
fn pick_parameter_sets(stream: &H264HwStreamInfo) -> Option<(&H264HwSps, &H264HwPps)> {
    let pps = stream
        .pps
        .iter()
        .min_by_key(|p| p.pps.pic_parameter_set_id)?;
    let sps = stream
        .sps
        .iter()
        .find(|s| s.sps.seq_parameter_set_id == pps.pps.seq_parameter_set_id)?;
    Some((sps, pps))
}

// ===========================================================================
// CF object helpers (configure-time only — not hot paths).
// ===========================================================================

/// An empty mutable CFDictionary with the documented default
/// (`kCFType…`) callbacks and `capacity` as the entry hint
/// (Create rule: the caller releases).
///
/// `None` on allocation failure — typed upstream, never a guess.
unsafe fn new_dictionary(capacity: isize) -> Option<ffi::CFMutableDictionaryRef> {
    // SAFETY: the documented NULL-callbacks form selects the
    // kCFType callbacks; capacity is the caller's entry count.
    let dict = unsafe {
        ffi::CFDictionaryCreateMutable(
            std::ptr::null(),
            capacity,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    (!dict.is_null()).then_some(dict)
}

/// A runtime-created UTF-8 CFString key (Create rule). CFString
/// equality is by contents, so runtime-created keys match the
/// framework constants' values — the lane links no `kVT…` data
/// symbols for strings (see `ffi`'s ABI decisions).
unsafe fn cf_string(key: &str) -> Option<ffi::CFStringRef> {
    // Interior-NUL keys cannot exist in this lane (four ASCII
    // constants); the None arm is the honest refusal, not a guess.
    let c_key = key.as_bytes();
    let mut buf = [0u8; 64];
    if c_key.len() >= buf.len() || c_key.contains(&0) {
        return None;
    }
    buf[..c_key.len()].copy_from_slice(c_key);
    // SAFETY: buf is NUL-terminated (zeroed beyond the copy) and
    // outlives the call; the encoding constant is the ABI value.
    let s = unsafe {
        ffi::CFStringCreateWithCString(
            std::ptr::null(),
            buf.as_ptr().cast(),
            ffi::KCF_STRING_ENCODING_UTF8,
        )
    };
    (!s.is_null()).then_some(s)
}

/// A `CFNumber` from an `i64` (Create rule; `SInt64` type).
unsafe fn cf_number_i64(value: i64) -> Option<ffi::CFNumberRef> {
    let v = value;
    // SAFETY: &v is a valid, aligned i64 for the call; the number
    // type constant is the ABI value.
    let n = unsafe {
        ffi::CFNumberCreate(
            std::ptr::null(),
            ffi::KCF_NUMBER_SINT64_TYPE,
            &raw const v as *const c_void,
        )
    };
    (!n.is_null()).then_some(n)
}

/// Set `key = kCFBooleanTrue` (the exported singleton). `false`
/// only on CF allocation failure — typed upstream.
unsafe fn set_true(dict: ffi::CFMutableDictionaryRef, key: &str) -> bool {
    // SAFETY: both objects are Create-rule locals released here;
    // the dictionary retains its own references (default callbacks).
    unsafe {
        let Some(k) = cf_string(key) else {
            return false;
        };
        ffi::CFDictionarySetValue(dict, k, ffi::KCF_BOOLEAN_TRUE);
        ffi::CFRelease(k);
        true
    }
}

/// Set `key = value` (an `i64` CFNumber). `false` only on CF
/// allocation failure — typed upstream.
unsafe fn set_i64(dict: ffi::CFMutableDictionaryRef, key: &str, value: i64) -> bool {
    // SAFETY: as set_true — Create-rule locals, dictionary-owned
    // retains.
    unsafe {
        let Some(k) = cf_string(key) else {
            return false;
        };
        let Some(v) = cf_number_i64(value) else {
            ffi::CFRelease(k);
            return false;
        };
        ffi::CFDictionarySetValue(dict, k, v);
        ffi::CFRelease(k);
        ffi::CFRelease(v);
        true
    }
}

/// The decoder specification: `RequireHardwareDecode = true` — the
/// lane's ADR-001 honesty (no silent VT software fallback). Returns
/// a Create-rule dictionary reference.
fn decoder_specification() -> Option<ffi::CFDictionaryRef> {
    // SAFETY: the helpers' local retains are released inside; the
    // returned dictionary is the sole owner of its entries.
    unsafe {
        let dict = new_dictionary(1)?;
        if !set_true(dict, KEY_REQUIRE_HARDWARE_DECODE) {
            ffi::CFRelease(dict.cast());
            return None;
        }
        Some(dict.cast())
    }
}

/// The destination pixel-buffer attributes: `'420v'` (the NV12-form
/// biplanar surface the presentation view promises) plus the coded
/// dimensions. Returns a Create-rule dictionary reference.
fn destination_attributes(width: u32, height: u32) -> Option<ffi::CFDictionaryRef> {
    // SAFETY: as decoder_specification — helper-local retains
    // released inside; the dictionary owns its entries.
    unsafe {
        let dict = new_dictionary(3)?;
        let ok = set_i64(
            dict,
            KEY_PIXEL_FORMAT_TYPE,
            i64::from(ffi::KCV_PIXEL_FORMAT_420V),
        ) && set_i64(dict, KEY_WIDTH, i64::from(width))
            && set_i64(dict, KEY_HEIGHT, i64::from(height));
        if !ok {
            ffi::CFRelease(dict.cast());
            return None;
        }
        Some(dict.cast())
    }
}

/// Release one retained pixel buffer (null-tolerant no-op) — every
/// buffer the callback retained crosses this exactly once (pool
/// teardown, back-pressure rollback, or the present-side release).
unsafe fn release_buffer(buffer: ffi::CVPixelBufferRef) {
    if !buffer.is_null() {
        // SAFETY: the handle was retained exactly once for this
        // slot/queue entry and is not held elsewhere by this lane.
        unsafe { ffi::CVPixelBufferRelease(buffer) };
    }
}

// ===========================================================================
// The callback channel.
// ===========================================================================

/// One frame the output callback surfaced: the retained pixel
/// buffer plus the geometry and stamp the callback could see.
struct CallbackFrame {
    /// The output surface (retained by the callback; released by
    /// the session's drain/teardown paths — never dereferenced as
    /// Rust data, only passed back to CoreVideo).
    buffer: ffi::CVPixelBufferRef,
    /// The buffer's `(width, height)` as CoreVideo reported it.
    size: (u32, u32),
    /// The output presentation timestamp (from the sample's
    /// `CMTime`; `Missing` when it was not the valid 10⁹ form).
    pts: Stamp,
}

// SAFETY: the buffer is a ref-counted CF object the callback
// retained for this queue (single owner chain: callback push →
// session drain → present side). CF objects are safe to move
// across threads while ref-counted; the pointer is only ever
// passed back to CoreVideo calls.
unsafe impl Send for CallbackFrame {}

/// The callback↔session channel (the refcon target). The queue is
/// bounded by the decode pipeline's own back-pressure: the session
/// drains it every decode/flush call and refuses typed when the
/// emitted-surface pool is full, so it never grows past one
/// pipeline latency's worth of frames.
pub(super) struct Shared {
    /// Frames surfaced by the callback, awaiting the next drain.
    queue: Mutex<Vec<CallbackFrame>>,
}

impl Shared {
    fn empty() -> Shared {
        Shared {
            queue: Mutex::new(Vec::new()),
        }
    }
}

/// The output callback VT invokes per decoded frame (installed at
/// session create; the refcon is the session's `Arc<Shared>`).
///
/// Failure frames (`status != noErr`) surface nothing — the decode
/// call's own status carries that truth upstream; a null image
/// buffer equally so.
///
/// # Safety (the refcon lifetime invariant)
///
/// The session invalidates the VT session in its `Drop` **before**
/// the `Arc<Shared>` drops, and
/// `VTDecompressionSessionInvalidate` is documented synchronous
/// with the last output callback — so `refcon` is always a live
/// `&Shared` here.
pub(super) unsafe extern "C" fn on_frame(
    refcon: *mut c_void,
    _source_frame_refcon: *mut c_void,
    status: ffi::OSStatus,
    _info_flags: ffi::VTDecodeInfoFlags,
    image_buffer: ffi::CVImageBufferRef,
    presentation_time_stamp: *mut ffi::CMTime,
    _presentation_duration: *mut ffi::CMTime,
) {
    // SAFETY: the refcon invariant above (session-outlives-callback
    // ordering, enforced by the Drop path).
    let shared = unsafe { &*(refcon.cast::<Shared>()) };
    if status != ffi::NO_ERR || image_buffer.is_null() {
        return;
    }
    // SAFETY: the image buffer is live for the callback; Retain
    // mints the reference this lane releases exactly once.
    let buffer = unsafe { ffi::CVPixelBufferRetain(image_buffer) };
    if buffer.is_null() {
        return;
    }
    // SAFETY: the retained handle is a valid pixel buffer.
    let (width, height) = unsafe {
        (
            ffi::CVPixelBufferGetWidth(buffer),
            ffi::CVPixelBufferGetHeight(buffer),
        )
    };
    // SAFETY: the callback's timestamp pointer is either null
    // (→ Missing) or a valid CMTime VT filled in (→ the inverse
    // packaging of cm_time_of).
    let pts = unsafe { presentation_time_stamp.as_ref() }.map_or(Stamp::Missing, stamp_of);
    let frame = CallbackFrame {
        buffer,
        size: (
            u32::try_from(width).unwrap_or(u32::MAX),
            u32::try_from(height).unwrap_or(u32::MAX),
        ),
        pts,
    };
    // The lock body cannot panic; the poisoned arm keeps the extern
    // boundary panic-free regardless (an extern "C" unwind is an
    // abort — never allowed here).
    let mut queue = match shared.queue.lock() {
        Ok(queue) => queue,
        Err(poisoned) => poisoned.into_inner(),
    };
    queue.push(frame);
}

// ===========================================================================
// The session.
// ===========================================================================

/// The VideoToolbox decode session (unopened until its first
/// `configure` — the bridge mints it, the seam feeds it).
///
/// `Send` (the [`H264HwDecoder`] contract): every raw handle below
/// is owned exclusively by this session and only ever passed back
/// to the framework calls on the owning thread; the shared queue
/// is a `Mutex` (the callback and the session take it in turn).
pub struct VideoToolboxSession {
    /// The format description (Create-rule; rebuilt per stream
    /// generation).
    format_description: ffi::CMVideoFormatDescriptionRef,
    /// The decompression session (Create-rule; `null` until the
    /// first successful configure, dead after invalidate).
    session: ffi::VTDecompressionSessionRef,
    /// The callback channel — declared before the pool/emitted
    /// bookkeeping it feeds. (Drop invalidates the session first;
    /// the Arc itself may then drop in any field order.)
    shared: Arc<Shared>,
    /// The bounded emitted-surface pool (slot → retained pixel
    /// buffer).
    pool: HwSurfacePool<ffi::CVPixelBufferRef>,
    /// Handles handed to the codec layer (the present side's
    /// release hook is later-wave work; teardown walks this list).
    emitted: Vec<u64>,
    /// Coded dimensions of the current stream.
    coded: (u32, u32),
    /// Display dimensions of the current stream (SPS crop applied
    /// by the parser's caller).
    display_size: Option<(u32, u32)>,
    /// The active SPS snapshot.
    sps: Option<SeqParameterSet>,
    /// The active PPS snapshot.
    pps: Option<PicParameterSet>,
    /// Scratch AVCC sample buffer (cleared per picture; grown once
    /// — zero steady-state allocation, D-1).
    sample: Vec<u8>,
}

// SAFETY: the non-Send members are the opaque framework handles
// (format description, session, retained pixel buffers) — each is
// owned exclusively by this session, created and destroyed by it
// alone, and only ever passed back to framework calls (single
// owner, no aliasing). VideoToolbox session handles are documented
// valid across threads for the owning client; moving the session
// to another thread races with nothing.
unsafe impl Send for VideoToolboxSession {}

impl VideoToolboxSession {
    /// An unopened session (the bridge's mint).
    #[must_use = "the bridge's session mint"]
    pub fn new() -> VideoToolboxSession {
        VideoToolboxSession {
            format_description: std::ptr::null(),
            session: std::ptr::null_mut(),
            shared: Arc::new(Shared::empty()),
            pool: HwSurfacePool::new(0),
            emitted: Vec::new(),
            coded: (0, 0),
            display_size: None,
            sps: None,
            pps: None,
            sample: Vec::new(),
        }
    }

    /// Tear the stream generation down: invalidate the session
    /// (synchronous with the last callback — the refcon `Arc`
    /// outlives every invocation), release both Create-rule
    /// handles, then release every retained pixel buffer (queued
    /// and emitted alike). After this the session is unopened
    /// again, ready for the next configure.
    fn teardown(&mut self) {
        if !self.session.is_null() {
            // SAFETY: live handle, its final use; Invalidate is the
            // documented drain-and-stop primitive.
            unsafe { ffi::VTDecompressionSessionInvalidate(self.session) };
            // SAFETY: Create-rule handle, invalidated above.
            unsafe { ffi::CFRelease(self.session.cast()) };
            self.session = std::ptr::null_mut();
        }
        if !self.format_description.is_null() {
            // SAFETY: Create-rule handle; this lane's sole owner.
            unsafe { ffi::CFRelease(self.format_description.cast()) };
            self.format_description = std::ptr::null();
        }
        // Undrained callback frames: release their retains.
        let queued = std::mem::take(
            &mut *self
                .shared
                .queue
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        for frame in queued {
            // SAFETY: exactly-once release of the callback's retain.
            unsafe { release_buffer(frame.buffer) };
        }
        // Emitted slots: release each retained buffer, free the slot.
        for handle in self.emitted.drain(..) {
            if let Some(buffer) = self.pool.resource(handle).copied() {
                // SAFETY: exactly-once release of the callback's
                // retain (the slot held it until this line).
                unsafe { release_buffer(buffer) };
            }
            self.pool.release(handle);
        }
        self.pool.drain();
        self.sps = None;
        self.pps = None;
        self.display_size = None;
    }

    /// Move every callback-surfaced frame into the decoded payload.
    /// All-or-nothing against the pool: if every surfaced frame
    /// cannot be held, the whole drain rolls back (each buffer
    /// released, each slot freed) and the typed back-pressure
    /// error is the answer — frames are never partially dropped.
    fn drain(&mut self) -> Result<H264HwDecoded, H264HwError> {
        let queued = std::mem::take(
            &mut *self
                .shared
                .queue
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        if queued.is_empty() {
            return Ok(H264HwDecoded::default());
        }
        let mut handles = Vec::with_capacity(queued.len());
        for frame in &queued {
            match self.pool.acquire(frame.buffer) {
                Ok(handle) => handles.push(handle),
                Err(_) => {
                    // Roll back: the acquired holds release their
                    // buffers and slots; the never-acquired tail
                    // (including the frame that found no slot)
                    // releases its buffers.
                    for handle in &handles {
                        if let Some(buffer) = self.pool.resource(*handle).copied() {
                            // SAFETY: the slot's exactly-once retain.
                            unsafe { release_buffer(buffer) };
                        }
                        self.pool.release(*handle);
                    }
                    for frame in queued.iter().skip(handles.len()) {
                        // SAFETY: the queue's exactly-once retain.
                        unsafe { release_buffer(frame.buffer) };
                    }
                    return Err(H264HwError::Decode {
                        code: "h264.hw.videotoolbox.pool_exhausted",
                    });
                }
            }
        }
        let mut out = H264HwDecoded::default();
        for (frame, handle) in queued.into_iter().zip(handles) {
            // Acquire left the slot Decoding; emitted is what it
            // becomes the moment the frame leaves this call.
            self.pool.emit(handle);
            self.emitted.push(handle);
            out.frames.push(H264HwFrame {
                surface: handle,
                size: frame.size,
                // Field pictures are refused typed at configure —
                // every frame this lane emits is progressive.
                interlaced: false,
                pts: frame.pts,
            });
        }
        Ok(out)
    }
}

impl Default for VideoToolboxSession {
    fn default() -> Self {
        VideoToolboxSession::new()
    }
}

impl Drop for VideoToolboxSession {
    fn drop(&mut self) {
        self.teardown();
    }
}

impl H264HwDecoder for VideoToolboxSession {
    fn backend_id(&self) -> &str {
        HwBackend::VideoToolbox.backend_id()
    }

    fn configure(&mut self, stream: &H264HwStreamInfo) -> Result<(), H264HwError> {
        // 1. The stream needs an SPS+PPS pair (active-pair rule).
        let (sps_entry, pps_entry) = pick_parameter_sets(stream).ok_or(H264HwError::Configure {
            code: "h264.hw.videotoolbox.no_parameter_sets",
        })?;
        let sps = sps_entry.sps.clone();
        let pps = pps_entry.pps.clone();
        // 2. Typed refusals (the honest gaps — see module docs).
        if !sps.frame_mbs_only {
            return Err(H264HwError::Configure {
                code: "h264.hw.videotoolbox.field_picture",
            });
        }
        if !vt_serves(&sps) {
            return Err(H264HwError::Configure {
                code: "h264.hw.videotoolbox.profile",
            });
        }
        let (width, height) = coded_size(&sps).ok_or(H264HwError::Configure {
            code: "h264.hw.videotoolbox.geometry",
        })?;
        // 3. Rebuild the stream generation from scratch.
        self.teardown();
        // 4. Format description off the raw parameter-set NALs —
        //    the seam's feed form is exactly the API's input form
        //    (complete NALs, emulation prevention kept).
        let pointers = [sps_entry.nal.as_ptr(), pps_entry.nal.as_ptr()];
        let sizes = [sps_entry.nal.len(), pps_entry.nal.len()];
        let mut format_description: ffi::CMVideoFormatDescriptionRef = std::ptr::null();
        // SAFETY: the pointer/size arrays address the live stream
        // info's NAL vectors for the call; the length is the AVCC
        // constant this lane's samples use; the out-pointer is a
        // live local.
        let status = unsafe {
            ffi::CMVideoFormatDescriptionCreateFromH264ParameterSets(
                std::ptr::null(),
                pointers.len(),
                pointers.as_ptr(),
                sizes.as_ptr(),
                feed::AVCC_LENGTH_SIZE as i32,
                &mut format_description,
            )
        };
        if status != ffi::NO_ERR || format_description.is_null() {
            return Err(H264HwError::Configure {
                code: "h264.hw.videotoolbox.format_description",
            });
        }
        // 5. Decoder spec (hardware-required) + destination attrs.
        let Some(spec) = decoder_specification() else {
            // SAFETY: Create-rule handle from step 4, sole owner.
            unsafe { ffi::CFRelease(format_description.cast()) };
            return Err(H264HwError::Configure {
                code: "h264.hw.videotoolbox.specification",
            });
        };
        let Some(attributes) = destination_attributes(width, height) else {
            // SAFETY: Create-rule handles, sole owner (the spec
            // dictionary was not passed to any framework call yet).
            unsafe {
                ffi::CFRelease(spec.cast());
                ffi::CFRelease(format_description.cast());
            }
            return Err(H264HwError::Configure {
                code: "h264.hw.videotoolbox.specification",
            });
        };
        // 6. The decompression session, with the output callback
        //    installed (refcon = this session's shared channel).
        let record = ffi::VTDecompressionOutputCallbackRecord {
            decompressionOutputCallback: Some(on_frame),
            decompressionOutputRefCon: Arc::as_ptr(&self.shared).cast_mut().cast(),
        };
        let mut session: ffi::VTDecompressionSessionRef = std::ptr::null_mut();
        // SAFETY: format_description is the live description from
        // step 4; spec/attributes are the live dictionaries above;
        // the record's refcon is the shared-state Arc this session
        // outlives (Drop invalidates before the Arc drops); the
        // out-pointer is a live local.
        let status = unsafe {
            ffi::VTDecompressionSessionCreate(
                std::ptr::null(),
                format_description,
                spec,
                attributes,
                &record,
                &mut session,
            )
        };
        // The session retained its own copies: release this lane's
        // dictionary references either way.
        // SAFETY: Create-rule handles from step 5, sole owner.
        unsafe {
            ffi::CFRelease(spec.cast());
            ffi::CFRelease(attributes.cast());
        }
        if status != ffi::NO_ERR || session.is_null() {
            // SAFETY: Create-rule handle from step 4, sole owner.
            unsafe { ffi::CFRelease(format_description.cast()) };
            return Err(H264HwError::Configure {
                code: "h264.hw.videotoolbox.session",
            });
        }
        self.format_description = format_description;
        self.session = session;
        self.pool = HwSurfacePool::new(pool_capacity(&sps));
        self.coded = (width, height);
        self.display_size = stream.display_size;
        self.sps = Some(sps);
        self.pps = Some(pps);
        self.emitted.clear();
        Ok(())
    }

    fn decode(&mut self, au: &H264HwAccessUnit) -> Result<H264HwDecoded, H264HwError> {
        if self.session.is_null() || self.sps.is_none() {
            return Err(H264HwError::Decode {
                code: "h264.hw.videotoolbox.not_configured",
            });
        }
        // MVC NALs are out of scope (single-view lane).
        if au
            .units
            .iter()
            .any(|u| matches!(u.unit_type, NalUnitType::Prefix | NalUnitType::SubsetSps))
        {
            return Err(H264HwError::Decode {
                code: "h264.hw.videotoolbox.mvc",
            });
        }
        let Some(first_vcl) = au.first_vcl() else {
            // Parameter-set-only / SEI-only AU: nothing to decode.
            return Ok(H264HwDecoded::default());
        };
        if first_vcl.slice.field_pic_flag {
            return Err(H264HwError::Decode {
                code: "h264.hw.videotoolbox.field_picture",
            });
        }
        // 1. The AVCC sample (every NAL of the picture, be32
        //    length prefixes) into the reusable scratch buffer —
        //    `None` only when a NAL exceeds the u32 length space
        //    (refused typed; never a truncating cast).
        feed::avcc_sample_into(au.units.iter().map(|u| u.nal.as_slice()), &mut self.sample).ok_or(
            H264HwError::Decode {
                code: "h264.hw.videotoolbox.sample",
            },
        )?;
        // 2. The self-allocating block buffer + the byte copy that
        //    decouples the sample from the scratch buffer.
        let mut block: ffi::CMBlockBufferRef = std::ptr::null_mut();
        // SAFETY: the documented self-allocating form (memory block
        // null, default allocator, no flags); data_length is the
        // sample size; the out-pointer is a live local.
        let status = unsafe {
            ffi::CMBlockBufferCreateWithMemoryBlock(
                std::ptr::null(),
                std::ptr::null_mut(),
                self.sample.len(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                self.sample.len(),
                0,
                &mut block,
            )
        };
        if status != ffi::NO_ERR || block.is_null() {
            return Err(H264HwError::Decode {
                code: "h264.hw.videotoolbox.block_buffer",
            });
        }
        // SAFETY: block was created above with data_length room at
        // offset 0; the source is the live scratch sample.
        let status = unsafe {
            ffi::CMBlockBufferReplaceDataBytes(
                self.sample.as_ptr().cast(),
                block,
                0,
                self.sample.len(),
            )
        };
        if status != ffi::NO_ERR {
            // SAFETY: Create-rule handle from step 2, sole owner.
            unsafe { ffi::CFRelease(block.cast()) };
            return Err(H264HwError::Decode {
                code: "h264.hw.videotoolbox.block_buffer",
            });
        }
        // 3. The ready sample buffer (one sample, one size entry,
        //    no timing array — the PTS is attached next).
        let size_entry = self.sample.len();
        let mut sample: ffi::CMSampleBufferRef = std::ptr::null_mut();
        // SAFETY: block is the live block buffer from step 2 (the
        // sample buffer retains it); the format description is the
        // live one from configure; counts and the size array match
        // the one-sample form; the out-pointer is a live local.
        let status = unsafe {
            ffi::CMSampleBufferCreateReady(
                std::ptr::null(),
                block,
                self.format_description,
                1,
                0,
                std::ptr::null(),
                1,
                &size_entry,
                &mut sample,
            )
        };
        // SAFETY: Create-rule handle from step 2; the sample buffer
        // holds its own retain of the block from this call on.
        unsafe { ffi::CFRelease(block.cast()) };
        if status != ffi::NO_ERR || sample.is_null() {
            return Err(H264HwError::Decode {
                code: "h264.hw.videotoolbox.sample_buffer",
            });
        }
        // 4. The output presentation timestamp (the pure stamp →
        //    nanosecond → CMTime packaging of feed.rs).
        // SAFETY: sample is the live sample buffer from step 3.
        let status = unsafe {
            ffi::CMSampleBufferSetOutputPresentationTimestamp(sample, cm_time_of(au.pts))
        };
        if status != ffi::NO_ERR {
            // SAFETY: Create-rule handle from step 3, sole owner.
            unsafe { ffi::CFRelease(sample.cast()) };
            return Err(H264HwError::Decode {
                code: "h264.hw.videotoolbox.sample_buffer",
            });
        }
        // 5. Submit (asynchronous: frames surface via the callback
        //    during this call, a later call, or at flush).
        // SAFETY: the session is live (checked above, not
        // invalidated); sample is the live buffer from step 3; no
        // per-frame refcon and no info out-pointer are wanted.
        let status = unsafe {
            ffi::VTDecompressionSessionDecodeFrame(
                self.session,
                sample,
                ffi::KVT_DECODE_FRAME_ENABLE_ASYNC,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        // VideoToolbox retains what it needs for the duration of
        // the decode; this lane's reference ends here.
        // SAFETY: Create-rule handle from step 3, sole owner.
        unsafe { ffi::CFRelease(sample.cast()) };
        if status != ffi::NO_ERR {
            return Err(H264HwError::Decode {
                code: "h264.hw.videotoolbox.decode_frame",
            });
        }
        // 6. Whatever surfaced, hand over.
        self.drain()
    }

    fn flush(&mut self) -> Result<H264HwDecoded, H264HwError> {
        if self.session.is_null() {
            // Unopened: nothing was ever submitted — the honest
            // empty drain, not a typed error.
            return Ok(H264HwDecoded::default());
        }
        // SAFETY: the session is live; the wait is the documented
        // drain of every in-flight frame through the callback.
        let status = unsafe { ffi::VTDecompressionSessionWaitForAsynchronousFrames(self.session) };
        if status != ffi::NO_ERR {
            return Err(H264HwError::Decode {
                code: "h264.hw.videotoolbox.flush",
            });
        }
        self.drain()
    }
}

// ===========================================================================
// The lane probe.
// ===========================================================================

/// The lane probe: `VTIsHardwareDecodeSupported(kCMVideoCodecType_H264)`
/// (macOS 10.13+). A yes answers
/// [`HwAvailability::Available`] and carries the probe detail (the
/// accelerator mints off it); a no is the typed
/// `DriverIncompatible` truth — VideoToolbox is present on every
/// macOS build, so a "no" means the machine's VideoToolbox has no
/// hardware H.264 decoder, which is a driver/hardware fact, not an
/// absence of the service.
pub fn probe() -> (HwAvailability, Option<String>) {
    // SAFETY: the documented pure capability query — no session, no
    // side effects; the codec-type constant is the ABI value.
    let supported = unsafe { ffi::VTIsHardwareDecodeSupported(ffi::KCM_VIDEO_CODEC_TYPE_H264) };
    if supported != 0 {
        let detail = "VideoToolbox hardware H.264 decode supported".to_string();
        (
            HwAvailability::Available {
                backend: HwBackend::VideoToolbox,
                detail: detail.clone(),
            },
            Some(detail),
        )
    } else {
        (
            HwAvailability::Unavailable {
                backend: HwBackend::VideoToolbox,
                reason: HwUnavailableReason::DriverIncompatible(
                    "VTIsHardwareDecodeSupported answered false".into(),
                ),
            },
            None,
        )
    }
}
