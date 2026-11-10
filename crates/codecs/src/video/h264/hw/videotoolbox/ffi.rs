//! Hand-rolled VideoToolbox / CoreMedia / CoreVideo / CoreFoundation
//! ABI declarations (card 7.B1, issue #41) — the macOS lane's
//! OS-service binding (ADR-001's audited E2 exception; MP §9 /
//! doc 01 §2.7).
//!
//! # Provenance (the honest label)
//!
//! These declarations were written from Apple's **public framework
//! header documentation** (VideoToolbox `VTDecompressionSession.h`,
//! CoreMedia `CMFormatDescription.h` / `CMSampleBuffer.h` /
//! `CMBlockBuffer.h` / `CMTime.h`, CoreVideo `CVPixelBuffer.h`,
//! CoreFoundation `CFDictionary.h` / `CFNumber.h` / `CFString.h`).
//! **No macOS SDK exists in this sandbox to diff them against**
//! (unlike the VA-API lane, whose `ffi.rs` was verified against the
//! installed libva 2.22.0 headers), and **no macOS hardware exists
//! to exercise them** — every call site below is compile-verified
//! only, via `cargo check --target aarch64-apple-darwin`:
//! **runtime behavior NOT exercised — `[VERIFY]`**.
//!
//! # ABI decisions
//!
//! - Every CF/CM/VT handle is an **opaque pointer** (`*const` /
//!   `*mut c_void`): the frameworks are CF-bridged C APIs whose
//!   struct layouts are private; nothing here dereferences one.
//! - `CMTime` is the one struct that crosses the boundary **by
//!   value** — laid out `#[repr(C)]` and size-pinned below (the
//!   packaging of the pure `feed::stamp_nanos` numbers).
//! - The callback record is `#[repr(C)]` with the callback as an
//!   `Option<fn>` (null = no callback is a legal VT state).
//! - CF string keys are created at runtime
//!   (`CFStringCreateWithCString`) instead of linking the exported
//!   `kVT…`/`kCVPixelBuffer…` constants: runtime-created CFStrings
//!   compare equal to the constants (CFString equality is by
//!   contents), so the dictionaries key correctly without binding
//!   more data symbols.
//! - `CFDictionaryCreateMutable` with `NULL` key/value callbacks
//!   selects the documented default `kCFType…CallBacks` (the
//!   keys/values passed are all CF objects).

// SAFETY-SCOPED MODULE (doc 01 §2.7 / MP §9): the Apple framework
// C ABIs are the audited OS-service exception this file declares.
#![allow(unsafe_code)]

use std::ffi::c_char;
use std::ffi::c_void;

// ---- opaque handle types (never dereferenced) ----------------------------

/// `CFAllocatorRef` (`kCFAllocatorDefault` = null).
pub type CFAllocatorRef = *const c_void;
/// `CFStringRef`.
pub type CFStringRef = *const c_void;
/// `CFBooleanRef` (no `CFBooleanCreate` exists — the two
/// `kCFBoolean…` singletons are the only values).
pub type CFBooleanRef = *const c_void;
/// `CFNumberRef`.
pub type CFNumberRef = *const c_void;
/// `CFDictionaryRef`.
pub type CFDictionaryRef = *const c_void;
/// `CFMutableDictionaryRef`.
pub type CFMutableDictionaryRef = *mut c_void;
/// `CFTypeRef` (the generic release/retain argument).
pub type CFTypeRef = *const c_void;
/// `CFIndex` (signed long).
pub type CFIndex = isize;
/// `CFNumberType`.
pub type CFNumberType = CFIndex;
/// `CMItemCount` (signed).
pub type CMItemCount = CFIndex;
/// MacTypes `Boolean`.
pub type Boolean = u8;
/// `OSStatus`.
pub type OSStatus = i32;
/// `FourCharCode`.
pub type FourCharCode = u32;
/// `CMVideoCodecType`.
pub type CMVideoCodecType = FourCharCode;
/// `CMVideoFormatDescriptionRef`.
pub type CMVideoFormatDescriptionRef = *const c_void;
/// `CMBlockBufferRef`.
pub type CMBlockBufferRef = *const c_void;
/// `CMSampleBufferRef`.
pub type CMSampleBufferRef = *const c_void;
/// `CVImageBufferRef` (the callback's output).
pub type CVImageBufferRef = *mut c_void;
/// `CVPixelBufferRef` (the retained output surface).
pub type CVPixelBufferRef = *mut c_void;
/// `VTDecompressionSessionRef`.
pub type VTDecompressionSessionRef = *mut c_void;
/// `VTDecodeInfoFlags`.
pub type VTDecodeInfoFlags = u32;

// ---- constants ------------------------------------------------------------

/// `noErr`.
pub const NO_ERR: OSStatus = 0;
/// `kCFStringEncodingUTF8`.
pub const KCF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
/// `kCFNumberSInt32Type`.
pub const KCF_NUMBER_SINT32_TYPE: CFNumberType = 3;
/// `kCFNumberSInt64Type`.
pub const KCF_NUMBER_SINT64_TYPE: CFNumberType = 4;
/// `kCMVideoCodecType_H264` (`'avc1'`).
pub const KCM_VIDEO_CODEC_TYPE_H264: CMVideoCodecType = 0x6176_6331;
/// `kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange` (`'420v'`) —
/// the NV12-form biplanar surface the presentation view promises
/// (BT.709 limited range, `decode.rs`'s `HW_COLOR`).
pub const KCV_PIXEL_FORMAT_420V: i32 = 0x3432_3076;
/// `kCVPixelFormatType_422YpCbCr8_yuvs` (`'yuvs'`) — the 4:2:2
/// packed form for `PixelFormat::Yuyv422` streams.
pub const KCV_PIXEL_FORMAT_YUVS: i32 = 0x7975_7673;
/// `kCMTimeFlags_Valid` (the only flag this lane sets).
pub const KCM_TIME_FLAGS_VALID: u32 = 1;
/// `kVTDecodeFrame_EnableAsynchronousDecompression` (the decode
/// call returns before the callback; output surfaces on later calls
/// or at `flush` — exactly the `H264HwDecoder` contract).
pub const KVT_DECODE_FRAME_ENABLE_ASYNC: u32 = 1 << 0;

// ---- structs that cross the boundary --------------------------------------

/// CoreMedia's `CMTime` (by value): epoch/value `i64` pairs around
/// a `i32` timescale and `u32` flags. Size-pinned: a layout drift
/// here corrupts every timestamp crossing the boundary.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CMTime {
    /// The value (`epoch × scale` numerator units).
    pub value: i64,
    /// The timescale (ticks per second; this lane uses 10⁹).
    pub timescale: i32,
    /// `kCMTimeFlags_*`.
    pub flags: u32,
    /// The epoch (always 0 here).
    pub epoch: i64,
}

// Size/ABI pins: 8 + (4 + pad) + 8 = 24 bytes, 8-aligned.
const _: () = assert!(std::mem::size_of::<CMTime>() == 24);
const _: () = assert!(std::mem::align_of::<CMTime>() == 8);
const _: () = assert!(std::mem::offset_of!(CMTime, timescale) == 8);
const _: () = assert!(std::mem::offset_of!(CMTime, flags) == 12);
const _: () = assert!(std::mem::offset_of!(CMTime, epoch) == 16);

/// The output callback VT invokes per decoded frame.
///
/// `Option<fn>`: a null callback is a legal `VTDecompressionSessionCreate`
/// argument (sessions without output callbacks exist); this lane
/// always installs [`crate::video::h264::hw::videotoolbox::session::on_frame`].
/// The parameter names mirror the C ABI's (camelCase, as documented
/// in VTDecompressionSession.h) — the lane's session callback
/// implements this exact shape.
#[allow(non_snake_case)]
pub type VTDecompressionOutputCallback = Option<
    unsafe extern "C" fn(
        decompressionOutputRefCon: *mut c_void,
        sourceFrameRefCon: *mut c_void,
        status: OSStatus,
        infoFlags: VTDecodeInfoFlags,
        imageBuffer: CVImageBufferRef,
        presentationTimeStamp: *mut CMTime,
        presentationDuration: *mut CMTime,
    ),
>;

/// `VTDecompressionOutputCallbackRecord`.
#[repr(C)]
#[allow(non_snake_case)]
pub struct VTDecompressionOutputCallbackRecord {
    /// The callback (never null in this lane).
    pub decompressionOutputCallback: VTDecompressionOutputCallback,
    /// The refcon pointer (this lane: `Arc<Shared>` as a raw
    /// pointer; see the session docs for the lifetime invariant).
    pub decompressionOutputRefCon: *mut c_void,
}

const _: () = assert!(std::mem::size_of::<VTDecompressionOutputCallbackRecord>() == 16);

// ---- CoreFoundation -------------------------------------------------------

// SAFETY: the declared ABI of the CoreFoundation framework (the
// reference-counting base every other framework here rides on). All
// handles are opaque; Create-rule ownership is documented per call
// site in the session.
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    /// Release one reference (`CFTypeRef`; never null at this lane's
    /// call sites — checked before every call).
    pub fn CFRelease(cf: CFTypeRef);
    /// Create a CFString from a UTF-8 C string (Create rule: the
    /// caller releases).
    pub fn CFStringCreateWithCString(
        alloc: CFAllocatorRef,
        c_str: *const c_char,
        encoding: u32,
    ) -> CFStringRef;
    /// Create a CFNumber from a typed value pointer (Create rule).
    pub fn CFNumberCreate(
        alloc: CFAllocatorRef,
        the_type: CFNumberType,
        value_ptr: *const c_void,
    ) -> CFNumberRef;
    /// Create an empty mutable CFDictionary (`NULL` callbacks = the
    /// documented `kCFType…` defaults; Create rule).
    pub fn CFDictionaryCreateMutable(
        alloc: CFAllocatorRef,
        capacity: CFIndex,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> CFMutableDictionaryRef;
    /// Set a key/value pair (both CF objects; the dictionary
    /// retains them per the default callbacks).
    pub fn CFDictionarySetValue(
        the_dictionary: CFMutableDictionaryRef,
        key: CFTypeRef,
        value: CFTypeRef,
    );
    /// `kCFBooleanTrue` — the exported singleton. `CFBoolean` has
    /// no create function; the two singletons are linked data
    /// symbols, which is why this one constant is a symbol link
    /// and not a runtime creation like the CFString keys. The
    /// decoder specification's `RequireHardwareDecode` value — the
    /// lane's no-silent-software guarantee (ADR-001).
    #[link_name = "kCFBooleanTrue"]
    pub static KCF_BOOLEAN_TRUE: CFBooleanRef;
}

// ---- CoreVideo ------------------------------------------------------------

// SAFETY: the declared ABI of the CoreVideo framework — the output
// surface's retain/release pair and its geometry queries. The
// buffer handle is opaque; this lane retains exactly once per pool
// slot and releases exactly once on slot drop.
#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
    /// Retain a pixel buffer (returns the same handle).
    pub fn CVPixelBufferRetain(buffer: CVPixelBufferRef) -> CVPixelBufferRef;
    /// Release one pixel-buffer reference.
    pub fn CVPixelBufferRelease(buffer: CVPixelBufferRef);
    /// The buffer's width in pixels.
    pub fn CVPixelBufferGetWidth(buffer: CVPixelBufferRef) -> usize;
    /// The buffer's height in pixels.
    pub fn CVPixelBufferGetHeight(buffer: CVPixelBufferRef) -> usize;
}

// ---- CoreMedia ------------------------------------------------------------

// SAFETY: the declared ABI of the CoreMedia framework — the
// parameter-set → format-description factory, the block/sample
// buffer constructors the decode path builds, and the CMTime
// packaging. All out-pointers are checked for success before use;
// Create-rule ownership is documented per call site in the session.
#[link(name = "CoreMedia", kind = "framework")]
unsafe extern "C" {
    /// Build a `CMVideoFormatDescription` from raw H.264 SPS/PPS
    /// NALs (no start codes, emulation prevention kept — the seam's
    /// exact feed form; `nalUnitHeaderLength` 4 = the AVCC form
    /// `feed::avcc_sample` produces for samples).
    #[allow(non_snake_case)]
    pub fn CMVideoFormatDescriptionCreateFromH264ParameterSets(
        allocator: CFAllocatorRef,
        parameter_set_count: usize,
        parameter_set_pointers: *const *const u8,
        parameter_set_sizes: *const usize,
        nal_unit_header_length: i32,
        format_description_out: *mut CMVideoFormatDescriptionRef,
    ) -> OSStatus;
    /// Create a block buffer that allocates its own `blockLength`
    /// memory (memory block null + default allocator + no flags —
    /// the documented self-allocating form; bytes copied in by
    /// [`CMBlockBufferReplaceDataBytes`]).
    #[allow(non_snake_case)]
    pub fn CMBlockBufferCreateWithMemoryBlock(
        allocator: CFAllocatorRef,
        memory_block: *mut c_void,
        block_length: usize,
        block_allocator: CFAllocatorRef,
        custom_block_source: *const c_void,
        offset_to_data: usize,
        data_length: usize,
        flags: u32,
        block_buffer_out: *mut CMBlockBufferRef,
    ) -> OSStatus;
    /// Copy `data_length` bytes into a block buffer at
    /// `offset_into_destination` (the copy that decouples the
    /// sample from this lane's scratch `Vec`).
    #[allow(non_snake_case)]
    pub fn CMBlockBufferReplaceDataBytes(
        source: *const c_void,
        destination: CMBlockBufferRef,
        offset_into_destination: usize,
        data_length: usize,
    ) -> OSStatus;
    /// Assemble a ready `CMSampleBuffer` (one sample, one total
    /// size entry, no timing array — the presentation timestamp is
    /// attached after via
    /// [`CMSampleBufferSetOutputPresentationTimestamp`]).
    #[allow(non_snake_case)]
    pub fn CMSampleBufferCreateReady(
        allocator: CFAllocatorRef,
        data_buffer: CMBlockBufferRef,
        format_description: CMVideoFormatDescriptionRef,
        num_samples: CMItemCount,
        num_sample_timing_entries: CMItemCount,
        sample_timing_array: *const c_void,
        num_sample_size_entries: CMItemCount,
        sample_size_array: *const usize,
        sample_buffer_out: *mut CMSampleBufferRef,
    ) -> OSStatus;
    /// Attach the output presentation timestamp to a sample buffer
    /// (the CMTime built by the session's pure conversion).
    #[allow(non_snake_case)]
    pub fn CMSampleBufferSetOutputPresentationTimestamp(
        sbuf: CMSampleBufferRef,
        timestamp: CMTime,
    ) -> OSStatus;
}

// ---- VideoToolbox ---------------------------------------------------------

// SAFETY: the declared ABI of the VideoToolbox framework — the
// hardware-support query (the lane probe) and the decompression
// session lifecycle (create / decode / wait / invalidate). The
// session handle is opaque; the callback-record layout is pinned
// above; the refcon lifetime invariant is documented in the session.
#[link(name = "VideoToolbox", kind = "framework")]
unsafe extern "C" {
    /// The capability query: does this machine's VideoToolbox
    /// support **hardware** H.264 decode (the lane probe's one
    /// question; macOS 10.13+).
    #[allow(non_snake_case)]
    pub fn VTIsHardwareDecodeSupported(codec_type: CMVideoCodecType) -> Boolean;
    /// Create a decompression session off a format description,
    /// decoder specification (this lane: `RequireHardwareDecode`)
    /// and destination pixel-buffer attributes (this lane: `'420v'`
    /// or `'yuvs'`, width/height).
    #[allow(non_snake_case)]
    pub fn VTDecompressionSessionCreate(
        allocator: CFAllocatorRef,
        video_format_description: CMVideoFormatDescriptionRef,
        video_decoder_specification: CFDictionaryRef,
        destination_image_buffer_attributes: CFDictionaryRef,
        output_callback: *const VTDecompressionOutputCallbackRecord,
        session_out: *mut VTDecompressionSessionRef,
    ) -> OSStatus;
    /// Submit one sample (asynchronous: frames surface via the
    /// output callback during this or a later call, or at
    /// [`VTDecompressionSessionWaitForAsynchronousFrames`]).
    #[allow(non_snake_case)]
    pub fn VTDecompressionSessionDecodeFrame(
        session: VTDecompressionSessionRef,
        sample_buffer: CMSampleBufferRef,
        decode_flags: u32,
        source_frame_ref_con: *mut c_void,
        info_flags_out: *mut VTDecodeInfoFlags,
    ) -> OSStatus;
    /// Block until every in-flight frame has surfaced through the
    /// callback (the flush primitive).
    #[allow(non_snake_case)]
    pub fn VTDecompressionSessionWaitForAsynchronousFrames(
        session: VTDecompressionSessionRef,
    ) -> OSStatus;
    /// Invalidate the session (synchronous with the last callback —
    /// the Drop-order invariant; the handle is dead afterwards).
    #[allow(non_snake_case)]
    pub fn VTDecompressionSessionInvalidate(session: VTDecompressionSessionRef);
}
