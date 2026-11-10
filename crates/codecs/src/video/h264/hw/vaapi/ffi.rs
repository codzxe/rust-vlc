//! VA-API C-ABI declarations (card 7.B1) — hand-rolled, no `-sys`
//! crate (AGENTS.md rule 2: zero new dependencies).
//!
//! **Provenance and fidelity.** Every type, constant and signature
//! below is mirrored field-for-field from the *public VA-API
//! specification headers* of **libva 2.22.0** — the exact
//! `libva.so.2` installed on this build host (`libva.so.2.2200.0`,
//! `ldconfig -p`), fetched from the upstream public spec tarball
//! (tag `2.22.0`; MIT license). VA-API freezes this ABI: the
//! H.264 decode buffer structs have been byte-stable since libva
//! 0.31 (upstream appends, never reorders — the `va_reserved`
//! padding fields exist exactly for that promise). Compile-time
//! size assertions below pin the layouts.
//!
//! Scope: exactly the surface an H.264 decode session + capability
//! probe needs — the `vaInitialize`/`vaCreate*` family, the picture
//! pipeline (`vaBeginPicture`/`vaRenderPicture`/`vaEndPicture`),
//! synchronization, teardown, and the three H.264 decode buffer
//! structs. Everything else of VA-API is deliberately NOT declared.
//!
//! # Safety posture
//!
//! Module-level `#[allow(unsafe_code)]` (the audited OS-service
//! exception, MP §9 / doc 01 §2.7; ADR-001 §5 classifies VA-API as
//! an E2 OS-service binding). Nothing here *calls* anything: this
//! file only declares the ABI shapes; the call sites live in
//! [`super::library`] (symbol resolution) and [`super::session`]
//! (session lifecycle), each with invariants written at the call.
//!
//! `[VERIFY]` compile-verified via `cargo check -p codecs` (native
//! x86_64-unknown-linux-gnu) and exercised against the real
//! `libva.so.2` only through the dlopen symbol table in
//! [`super::library`]; driver-mediated decode is NOT runtime-tested
//! in this sandbox (no GPU).
//!
//! **Layout provenance (r3 fix, task 7.w23a):** the size pins and
//! the status constants below were re-verified field-for-field
//! against the actual libva 2.22.0 release headers (the spec
//! tarball's `va/va.h`) — three `VA_STATUS_*` values were wrong in
//! the r2 draft (INVALID_DISPLAY 2→3, UNSUPPORTED_PROFILE 14→12,
//! UNIMPLEMENTED 3→20) and `VASliceParameterBufferH264` is 3128
//! bytes, not 3124 (the 3 alignment pad bytes C inserts before the
//! 4-byte-aligned `RefPicList0` array after the 9 single-byte slice
//! fields — offset 25 rounds to 28).

// SAFETY-SCOPED MODULE: raw VA-API ABI surface (libva 2.22.0 spec).
#![allow(unsafe_code)]

use std::ffi::{c_char, c_int, c_uint, c_void};

/// `VADisplay` — opaque driver context (we hold the pointer, never
/// deref it).
pub type VADisplay = *mut c_void;

/// `VAGenericID` — `unsigned int` handle namespace shared by
/// config/context/surface/buffer ids.
pub type VAGenericID = c_uint;

/// `VAConfigID`.
pub type VAConfigID = VAGenericID;
/// `VAContextID`.
pub type VAContextID = VAGenericID;
/// `VASurfaceID`.
pub type VASurfaceID = VAGenericID;
/// `VABufferID`.
pub type VABufferID = VAGenericID;

/// `VAStatus` — `int` return of every VA call.
pub type VAStatus = c_int;

/// The `VA_STATUS_*` values this lane branches on (2.22.0 `va.h`,
/// re-verified against the release header in this tree's r3 fix).
pub const VA_STATUS_SUCCESS: VAStatus = 0;
pub const VA_STATUS_ERROR_OPERATION_FAILED: VAStatus = 1;
pub const VA_STATUS_ERROR_INVALID_DISPLAY: VAStatus = 3;
pub const VA_STATUS_ERROR_UNIMPLEMENTED: VAStatus = 20;
pub const VA_STATUS_ERROR_UNSUPPORTED_PROFILE: VAStatus = 12;
/// `VA_STATUS_ERROR_DECODING_ERROR` (0x17) — the driver's bitstream
/// damage verdict (mapped to the typed Decode channel by the
/// session's `va_op_error`).
pub const VA_STATUS_ERROR_DECODING_ERROR: VAStatus = 23;

/// Render `VA_STATUS_*` as its spec spelling (diagnostics only).
#[must_use = "pure formatting"]
pub fn va_status_text(status: VAStatus) -> String {
    match status {
        VA_STATUS_SUCCESS => "VA_STATUS_SUCCESS".into(),
        VA_STATUS_ERROR_OPERATION_FAILED => "VA_STATUS_ERROR_OPERATION_FAILED".into(),
        VA_STATUS_ERROR_INVALID_DISPLAY => "VA_STATUS_ERROR_INVALID_DISPLAY".into(),
        VA_STATUS_ERROR_UNIMPLEMENTED => "VA_STATUS_ERROR_UNIMPLEMENTED".into(),
        VA_STATUS_ERROR_UNSUPPORTED_PROFILE => "VA_STATUS_ERROR_UNSUPPORTED_PROFILE".into(),
        VA_STATUS_ERROR_DECODING_ERROR => "VA_STATUS_ERROR_DECODING_ERROR".into(),
        other => format!("VA_STATUS_{other:#x}"),
    }
}

/// `VAProfile` — decode profile selector (2.22.0 `va.h` enum; the
/// H.264 slice this lane needs, plus `None`).
pub type VAProfile = c_int;

/// `VAProfileNone` (= −1).
pub const VA_PROFILE_NONE: VAProfile = -1;
/// `VAProfileH264Main` (= 6).
pub const VA_PROFILE_H264_MAIN: VAProfile = 6;
/// `VAProfileH264High` (= 7).
pub const VA_PROFILE_H264_HIGH: VAProfile = 7;
/// `VAProfileH264ConstrainedBaseline` (= 13).
pub const VA_PROFILE_H264_CONSTRAINED_BASELINE: VAProfile = 13;

/// `VAEntrypoint` (2.22.0 `va.h` enum).
pub type VAEntrypoint = c_int;

/// `VAEntrypointVLD` (= 1) — the slice-based decode entrypoint.
pub const VA_ENTRYPOINT_VLD: VAEntrypoint = 1;

/// `VAConfigAttribType` (2.22.0 `va.h`; the selectors read by the
/// probe/session).
pub type VAConfigAttribType = c_uint;

/// `VAConfigAttribRTFormat` (= 0).
pub const VA_CONFIG_ATTRIB_RT_FORMAT: VAConfigAttribType = 0;
/// `VAConfigAttribMaxPictureWidth` (= 18).
pub const VA_CONFIG_ATTRIB_MAX_PICTURE_WIDTH: VAConfigAttribType = 18;
/// `VAConfigAttribMaxPictureHeight` (= 19).
pub const VA_CONFIG_ATTRIB_MAX_PICTURE_HEIGHT: VAConfigAttribType = 19;

/// `VA_RT_FORMAT_YUV420` (4:2:0 8-bit) — the surface format this
/// lane negotiates.
pub const VA_RT_FORMAT_YUV420: c_uint = 0x0000_0001;

/// `VABufferType` (2.22.0 `va.h` enum; the decode slice).
pub type VABufferType = c_uint;

/// `VAPictureParameterBufferType` (= 0).
pub const VA_PICTURE_PARAMETER_BUFFER_TYPE: VABufferType = 0;
/// `VAIQMatrixBufferType` (= 1).
pub const VA_IQ_MATRIX_BUFFER_TYPE: VABufferType = 1;
/// `VASliceParameterBufferType` (= 4).
pub const VA_SLICE_PARAMETER_BUFFER_TYPE: VABufferType = 4;
/// `VASliceDataBufferType` (= 5).
pub const VA_SLICE_DATA_BUFFER_TYPE: VABufferType = 5;

/// `VASliceDataFlag` `VA_SLICE_DATA_FLAG_ALL` (= 0: the whole
/// slice is in the buffer — always this lane's case).
pub const VA_SLICE_DATA_FLAG_ALL: c_uint = 0;

/// `VA_INVALID_ID` — the "no such handle" sentinel.
pub const VA_INVALID_ID: VAGenericID = 0xFFFF_FFFF;

/// `VAConfigAttrib` (by value, 2.22.0 `va.h`).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VAConfigAttrib {
    /// `type` (the `VAConfigAttribType` selector).
    pub kind: VAConfigAttribType,
    /// OR'd value flags for the attribute.
    pub value: u32,
}

// Compile-time ABI layout pins (C layout with natural alignment;
// these sizes are part of the frozen ABI — drivers index into the
// structs by offset). Any field drift fails the build on every
// target this lane compiles for.
const _: () = assert!(std::mem::size_of::<VAPictureH264>() == 36);
const _: () = assert!(std::mem::size_of::<VAConfigAttrib>() == 8);

// -------------------------------------------------------------------------
// Function-pointer signatures (resolved by dlsym in `library.rs`).
// Every signature is the exact 2.22.0 prototype.
// -------------------------------------------------------------------------

/// `VAStatus vaInitialize(VADisplay dpy, int *major_version, int *minor_version)`.
pub type VaInitializeFn = unsafe extern "C" fn(
    dpy: VADisplay,
    major_version: *mut c_int,
    minor_version: *mut c_int,
) -> VAStatus;
/// `VAStatus vaTerminate(VADisplay dpy)`.
pub type VaTerminateFn = unsafe extern "C" fn(dpy: VADisplay) -> VAStatus;
/// `const char *vaQueryVendorString(VADisplay dpy)`.
pub type VaQueryVendorStringFn = unsafe extern "C" fn(dpy: VADisplay) -> *const c_char;
/// `VAStatus vaQueryConfigProfiles(VADisplay dpy, VAProfile *profile_list, int *num_profiles)`.
pub type VaQueryConfigProfilesFn = unsafe extern "C" fn(
    dpy: VADisplay,
    profile_list: *mut VAProfile,
    num_profiles: *mut c_int,
) -> VAStatus;
/// `VAStatus vaQueryConfigEntrypoints(VADisplay dpy, VAProfile profile, VAEntrypoint *entrypoint_list, int *num_entrypoints)`.
pub type VaQueryConfigEntrypointsFn = unsafe extern "C" fn(
    dpy: VADisplay,
    profile: VAProfile,
    entrypoint_list: *mut VAEntrypoint,
    num_entrypoints: *mut c_int,
) -> VAStatus;
/// `int vaMaxNumProfiles(VADisplay dpy)`.
pub type VaMaxNumProfilesFn = unsafe extern "C" fn(dpy: VADisplay) -> c_int;
/// `int vaMaxNumEntrypoints(VADisplay dpy)`.
pub type VaMaxNumEntrypointsFn = unsafe extern "C" fn(dpy: VADisplay) -> c_int;
/// `VAStatus vaCreateConfig(VADisplay dpy, VAProfile profile, VAEntrypoint entrypoint, VAConfigAttrib *attrib_list, int num_attribs, VAConfigID *config_id)`.
pub type VaCreateConfigFn = unsafe extern "C" fn(
    dpy: VADisplay,
    profile: VAProfile,
    entrypoint: VAEntrypoint,
    attrib_list: *mut VAConfigAttrib,
    num_attribs: c_int,
    config_id: *mut VAConfigID,
) -> VAStatus;
/// `VAStatus vaDestroyConfig(VADisplay dpy, VAConfigID config_id)`.
pub type VaDestroyConfigFn =
    unsafe extern "C" fn(dpy: VADisplay, config_id: VAConfigID) -> VAStatus;
/// `VAStatus vaCreateSurfaces(VADisplay dpy, unsigned int format, unsigned int width, unsigned int height, VASurfaceID *surfaces, unsigned int num_surfaces, VASurfaceAttrib *attrib_list, unsigned int num_attribs)`.
///
/// `VASurfaceAttrib` is always passed as NULL from this lane (the
/// default-attribute form), so that parameter is declared
/// `*mut c_void` — ABI-identical for a null pass.
pub type VaCreateSurfacesFn = unsafe extern "C" fn(
    dpy: VADisplay,
    format: c_uint,
    width: c_uint,
    height: c_uint,
    surfaces: *mut VASurfaceID,
    num_surfaces: c_uint,
    attrib_list: *mut c_void,
    num_attribs: c_uint,
) -> VAStatus;
/// `VAStatus vaDestroySurfaces(VADisplay dpy, VASurfaceID *surfaces, int num_surfaces)`.
pub type VaDestroySurfacesFn = unsafe extern "C" fn(
    dpy: VADisplay,
    surfaces: *mut VASurfaceID,
    num_surfaces: c_int,
) -> VAStatus;
/// `VAStatus vaCreateContext(VADisplay dpy, VAConfigID config_id, int picture_width, int picture_height, int flag, VASurfaceID *render_targets, int num_render_targets, VAContextID *context)`.
pub type VaCreateContextFn = unsafe extern "C" fn(
    dpy: VADisplay,
    config_id: VAConfigID,
    picture_width: c_int,
    picture_height: c_int,
    flag: c_int,
    render_targets: *mut VASurfaceID,
    num_render_targets: c_int,
    context: *mut VAContextID,
) -> VAStatus;
/// `VAStatus vaDestroyContext(VADisplay dpy, VAContextID context)`.
pub type VaDestroyContextFn =
    unsafe extern "C" fn(dpy: VADisplay, context: VAContextID) -> VAStatus;
/// `VAStatus vaCreateBuffer(VADisplay dpy, VAContextID context, VABufferType type, unsigned int size, unsigned int num_elements, void *data, VABufferID *buf_id)`.
pub type VaCreateBufferFn = unsafe extern "C" fn(
    dpy: VADisplay,
    context: VAContextID,
    kind: VABufferType,
    size: c_uint,
    num_elements: c_uint,
    data: *mut c_void,
    buf_id: *mut VABufferID,
) -> VAStatus;
/// `VAStatus vaDestroyBuffer(VADisplay dpy, VABufferID buffer_id)`.
pub type VaDestroyBufferFn =
    unsafe extern "C" fn(dpy: VADisplay, buffer_id: VABufferID) -> VAStatus;
/// `VAStatus vaBeginPicture(VADisplay dpy, VAContextID context, VASurfaceID render_target)`.
pub type VaBeginPictureFn = unsafe extern "C" fn(
    dpy: VADisplay,
    context: VAContextID,
    render_target: VASurfaceID,
) -> VAStatus;
/// `VAStatus vaRenderPicture(VADisplay dpy, VAContextID context, VABufferID *buffers, int num_buffers)`.
pub type VaRenderPictureFn = unsafe extern "C" fn(
    dpy: VADisplay,
    context: VAContextID,
    buffers: *mut VABufferID,
    num_buffers: c_int,
) -> VAStatus;
/// `VAStatus vaEndPicture(VADisplay dpy, VAContextID context)`.
pub type VaEndPictureFn = unsafe extern "C" fn(dpy: VADisplay, context: VAContextID) -> VAStatus;
/// `VAStatus vaSyncSurface(VADisplay dpy, VASurfaceID render_target)`.
pub type VaSyncSurfaceFn =
    unsafe extern "C" fn(dpy: VADisplay, render_target: VASurfaceID) -> VAStatus;

/// `VADisplay vaGetDisplayDRM(int fd)` — from `libva-drm`
/// (`va/drm/va_drm.h`, 2.22.0): the DRM render-node display
/// constructor.
pub type VaGetDisplayDrmFn = unsafe extern "C" fn(fd: c_int) -> VADisplay;

// -------------------------------------------------------------------------
// H.264 decode buffer structs (2.22.0 va.h, field-for-field). The
// `va_reserved` padding fields are REQUIRED by the ABI (drivers
// may read them; the spec says they must be zero).
// -------------------------------------------------------------------------

/// `VAPictureH264` — one DPB entry.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VAPictureH264 {
    /// The surface carrying the picture (`VA_INVALID_ID` for an
    /// unused slot).
    pub picture_id: VASurfaceID,
    /// `frame_num` (or `LongTermFrameIdx` for long-term refs).
    pub frame_idx: u32,
    /// `VA_PICTURE_H264_*` flag set.
    pub flags: u32,
    /// Top field order count.
    pub top_field_order_cnt: i32,
    /// Bottom field order count.
    pub bottom_field_order_cnt: i32,
    /// ABI padding — must be zero.
    pub va_reserved: [u32; 4],
}

/// `VA_PICTURE_H264_INVALID` (`0x1`).
pub const VA_PICTURE_H264_INVALID: u32 = 0x0000_0001;
/// `VA_PICTURE_H264_SHORT_TERM_REFERENCE` (`0x8`).
pub const VA_PICTURE_H264_SHORT_TERM_REFERENCE: u32 = 0x0000_0008;
/// `VA_PICTURE_H264_LONG_TERM_REFERENCE` (`0x10`).
pub const VA_PICTURE_H264_LONG_TERM_REFERENCE: u32 = 0x0000_0010;

impl VAPictureH264 {
    /// The spec's "empty slot" entry: invalid id, all-zero counts.
    #[must_use = "constructs the ABI empty slot"]
    pub fn empty() -> VAPictureH264 {
        VAPictureH264 {
            picture_id: VA_INVALID_ID,
            frame_idx: 0,
            flags: VA_PICTURE_H264_INVALID,
            top_field_order_cnt: 0,
            bottom_field_order_cnt: 0,
            va_reserved: [0; 4],
        }
    }
}

/// `VAPictureParameterBufferH264` — the per-picture parameter
/// buffer (one per picture, before any slice buffer).
///
/// The two bitfield unions are represented as raw `u32` halves
/// with pure pack helpers ([`seq_fields`], [`pic_fields`]) — the
/// exact spec bit orders.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VAPictureParameterBufferH264 {
    /// The picture being decoded.
    pub curr_pic: VAPictureH264,
    /// The DPB's reference pictures (unused slots = `empty()`).
    pub reference_frames: [VAPictureH264; 16],
    /// `pic_width_in_mbs_minus1`.
    pub picture_width_in_mbs_minus1: u16,
    /// `pic_height_in_mbs_minus1`.
    pub picture_height_in_mbs_minus1: u16,
    /// `bit_depth_luma_minus8`.
    pub bit_depth_luma_minus8: u8,
    /// `bit_depth_chroma_minus8`.
    pub bit_depth_chroma_minus8: u8,
    /// `num_ref_frames`.
    pub num_ref_frames: u8,
    /// `seq_fields` bitfield union word (see [`seq_fields`]).
    pub seq_fields: u32,
    /// Deprecated FMO field — zero (the parser refuses FMO
    /// upstream).
    pub num_slice_groups_minus1: u8,
    /// Deprecated FMO field — zero.
    pub slice_group_map_type: u8,
    /// Deprecated FMO field — zero.
    pub slice_group_change_rate_minus1: u16,
    /// `pic_init_qp_minus26`.
    pub pic_init_qp_minus26: i8,
    /// `pic_init_qs_minus26`.
    pub pic_init_qs_minus26: i8,
    /// `chroma_qp_index_offset`.
    pub chroma_qp_index_offset: i8,
    /// `second_chroma_qp_index_offset` — exact as the doubled
    /// value: our PPS parser accepts only streams without the
    /// High-profile extension block that would carry it separately
    /// (the trailing-bits check refuses those).
    pub second_chroma_qp_index_offset: i8,
    /// `pic_fields` bitfield union word (see [`pic_fields`]).
    pub pic_fields: u32,
    /// `frame_num`.
    pub frame_num: u16,
    /// ABI padding — must be zero.
    pub va_reserved: [u32; 8],
}

/// The `seq_fields` bit layout (spec order, LSB first):
/// `chroma_format_idc:2`, `residual_colour_transform_flag:1`,
/// `gaps_in_frame_num_value_allowed_flag:1`, `frame_mbs_only_flag:1`,
/// `mb_adaptive_frame_field_flag:1`, `direct_8x8_inference_flag:1`,
/// `MinLumaBiPredSize8x8:1`, `log2_max_frame_num_minus4:4`,
/// `pic_order_cnt_type:2`, `log2_max_pic_order_cnt_lsb_minus4:4`,
/// `delta_pic_order_always_zero_flag:1`.
pub mod seq_fields {
    /// Pack the spec's `seq_fields.bits` into the union word.
    #[must_use = "pure bit packing"]
    #[allow(clippy::too_many_arguments)]
    pub fn pack(
        chroma_format_idc: u32,
        residual_colour_transform_flag: bool,
        gaps_in_frame_num_value_allowed_flag: bool,
        frame_mbs_only_flag: bool,
        mb_adaptive_frame_field_flag: bool,
        direct_8x8_inference_flag: bool,
        min_luma_bi_pred_size8x8: bool,
        log2_max_frame_num_minus4: u32,
        pic_order_cnt_type: u32,
        log2_max_pic_order_cnt_lsb_minus4: u32,
        delta_pic_order_always_zero_flag: bool,
    ) -> u32 {
        (chroma_format_idc & 0x3)
            | u32::from(residual_colour_transform_flag) << 2
            | u32::from(gaps_in_frame_num_value_allowed_flag) << 3
            | u32::from(frame_mbs_only_flag) << 4
            | u32::from(mb_adaptive_frame_field_flag) << 5
            | u32::from(direct_8x8_inference_flag) << 6
            | u32::from(min_luma_bi_pred_size8x8) << 7
            | (log2_max_frame_num_minus4 & 0xF) << 8
            | (pic_order_cnt_type & 0x3) << 12
            | (log2_max_pic_order_cnt_lsb_minus4 & 0xF) << 14
            | u32::from(delta_pic_order_always_zero_flag) << 18
    }
}

/// The `pic_fields` bit layout (spec order, LSB first):
/// `entropy_coding_mode_flag:1`, `weighted_pred_flag:1`,
/// `weighted_bipred_idc:2`, `transform_8x8_mode_flag:1`,
/// `field_pic_flag:1`, `constrained_intra_pred_flag:1`,
/// `pic_order_present_flag:1`, `deblocking_filter_control_present_flag:1`,
/// `redundant_pic_cnt_present_flag:1`, `reference_pic_flag:1`.
pub mod pic_fields {
    /// Pack the spec's `pic_fields.bits` into the union word.
    #[must_use = "pure bit packing"]
    #[allow(clippy::too_many_arguments)]
    pub fn pack(
        entropy_coding_mode_flag: bool,
        weighted_pred_flag: bool,
        weighted_bipred_idc: u32,
        transform_8x8_mode_flag: bool,
        field_pic_flag: bool,
        constrained_intra_pred_flag: bool,
        pic_order_present_flag: bool,
        deblocking_filter_control_present_flag: bool,
        redundant_pic_cnt_present_flag: bool,
        reference_pic_flag: bool,
    ) -> u32 {
        u32::from(entropy_coding_mode_flag)
            | u32::from(weighted_pred_flag) << 1
            | (weighted_bipred_idc & 0x3) << 2
            | u32::from(transform_8x8_mode_flag) << 4
            | u32::from(field_pic_flag) << 5
            | u32::from(constrained_intra_pred_flag) << 6
            | u32::from(pic_order_present_flag) << 7
            | u32::from(deblocking_filter_control_present_flag) << 8
            | u32::from(redundant_pic_cnt_present_flag) << 9
            | u32::from(reference_pic_flag) << 10
    }
}

/// `VAIQMatrixBufferH264` — the quantization matrices.
///
/// **Honesty note:** the landed parser walks but does not retain
/// scaling lists (sps.rs `skip_scaling_list`), and every PPS our
/// parser accepts carries no scaling-matrix extension (the
/// trailing-bits check refuses them). For those streams the
/// default (flat, all-16) lists are *exact* per §7.3.2.1.1
/// defaults — not a guess. A stream whose SPS signals
/// `seq_scaling_matrix_present_flag` is refused typed by the
/// session (`h264.hw.vaapi.scaling_list`) rather than decoded with
/// different quantization than the bitstream asked for
/// `[VERIFY: driver-visible; follow-up = retain scaling lists in
/// the parser]`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VAIQMatrixBufferH264 {
    /// 4x4 lists, raster order (6 lists of 16).
    pub scaling_list4x4: [[u8; 16]; 6],
    /// 8x8 lists, raster order (2 lists of 64).
    pub scaling_list8x8: [[u8; 64]; 2],
    /// ABI padding — must be zero.
    pub va_reserved: [u32; 4],
}

impl VAIQMatrixBufferH264 {
    /// The spec-default (flat) matrices: every list all 16s.
    #[must_use = "constructs the spec-default matrices"]
    pub fn flat_default() -> VAIQMatrixBufferH264 {
        VAIQMatrixBufferH264 {
            scaling_list4x4: [[16; 16]; 6],
            scaling_list8x8: [[16; 64]; 2],
            va_reserved: [0; 4],
        }
    }
}

/// `VASliceParameterBufferH264` — one per slice.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VASliceParameterBufferH264 {
    /// Bytes of slice data in the slice-data buffer.
    pub slice_data_size: u32,
    /// Byte offset of the slice's NAL unit in the data buffer.
    pub slice_data_offset: u32,
    /// `VA_SLICE_DATA_FLAG_ALL`.
    pub slice_data_flag: u32,
    /// Bit offset from the NAL header byte to the start of slice
    /// data: `8 + header_bits` over the RBSP (the spec's exact
    /// definition; emulation-prevention bytes stay in the data
    /// buffer).
    pub slice_data_bit_offset: u16,
    /// `first_mb_in_slice`.
    pub first_mb_in_slice: u16,
    /// `slice_type` code (Table 7-7, 0..=9).
    pub slice_type: u8,
    /// `direct_spatial_mv_pred_flag` (B slices).
    pub direct_spatial_mv_pred_flag: u8,
    /// `num_ref_idx_l0_active_minus1`.
    pub num_ref_idx_l0_active_minus1: u8,
    /// `num_ref_idx_l1_active_minus1`.
    pub num_ref_idx_l1_active_minus1: u8,
    /// `cabac_init_idc`.
    pub cabac_init_idc: u8,
    /// `slice_qp_delta`.
    pub slice_qp_delta: i8,
    /// `disable_deblocking_filter_idc`.
    pub disable_deblocking_filter_idc: u8,
    /// `slice_alpha_c0_offset_div2`.
    pub slice_alpha_c0_offset_div2: i8,
    /// `slice_beta_offset_div2`.
    pub slice_beta_offset_div2: i8,
    /// RefPicList0 (32 spec slots).
    pub ref_pic_list0: [VAPictureH264; 32],
    /// RefPicList1 (32 spec slots).
    pub ref_pic_list1: [VAPictureH264; 32],
    /// `luma_log2_weight_denom`.
    pub luma_log2_weight_denom: u8,
    /// `chroma_log2_weight_denom`.
    pub chroma_log2_weight_denom: u8,
    /// `luma_weight_l0_flag` (bit i = entry i).
    pub luma_weight_l0_flag: u8,
    /// `luma_weight_l0[32]`.
    pub luma_weight_l0: [i16; 32],
    /// `luma_offset_l0[32]`.
    pub luma_offset_l0: [i16; 32],
    /// `chroma_weight_l0_flag` (bit i = entry i).
    pub chroma_weight_l0_flag: u8,
    /// `chroma_weight_l0[32][2]`.
    pub chroma_weight_l0: [[i16; 2]; 32],
    /// `chroma_offset_l0[32][2]`.
    pub chroma_offset_l0: [[i16; 2]; 32],
    /// `luma_weight_l1_flag`.
    pub luma_weight_l1_flag: u8,
    /// `luma_weight_l1[32]`.
    pub luma_weight_l1: [i16; 32],
    /// `luma_offset_l1[32]`.
    pub luma_offset_l1: [i16; 32],
    /// `chroma_weight_l1_flag`.
    pub chroma_weight_l1_flag: u8,
    /// `chroma_weight_l1[32][2]`.
    pub chroma_weight_l1: [[i16; 2]; 32],
    /// `chroma_offset_l1[32][2]`.
    pub chroma_offset_l1: [[i16; 2]; 32],
    /// ABI padding — must be zero.
    pub va_reserved: [u32; 4],
}

impl VASliceParameterBufferH264 {
    /// The base slice parameter: spec "absent" defaults, both
    /// ref lists fully invalid, weights neutral.
    #[must_use = "constructs the base slice parameter"]
    pub fn empty() -> VASliceParameterBufferH264 {
        VASliceParameterBufferH264 {
            slice_data_size: 0,
            slice_data_offset: 0,
            slice_data_flag: VA_SLICE_DATA_FLAG_ALL,
            slice_data_bit_offset: 0,
            first_mb_in_slice: 0,
            slice_type: 0,
            direct_spatial_mv_pred_flag: 0,
            num_ref_idx_l0_active_minus1: 0,
            num_ref_idx_l1_active_minus1: 0,
            cabac_init_idc: 0,
            slice_qp_delta: 0,
            disable_deblocking_filter_idc: 0,
            slice_alpha_c0_offset_div2: 0,
            slice_beta_offset_div2: 0,
            ref_pic_list0: [VAPictureH264::empty(); 32],
            ref_pic_list1: [VAPictureH264::empty(); 32],
            luma_log2_weight_denom: 0,
            chroma_log2_weight_denom: 0,
            luma_weight_l0_flag: 0,
            luma_weight_l0: [0; 32],
            luma_offset_l0: [0; 32],
            chroma_weight_l0_flag: 0,
            chroma_weight_l0: [[0; 2]; 32],
            chroma_offset_l0: [[0; 2]; 32],
            luma_weight_l1_flag: 0,
            luma_weight_l1: [0; 32],
            luma_offset_l1: [0; 32],
            chroma_weight_l1_flag: 0,
            chroma_weight_l1: [[0; 2]; 32],
            chroma_offset_l1: [[0; 2]; 32],
            va_reserved: [0; 4],
        }
    }
}

// ABI size pins for the three H.264 decode buffers (C layout,
// natural alignment — see the module docs; 672 / 240 / 3128 are
// the frozen 2.22.0 sizes; the slice-parameter pin includes the 3
// pad bytes C inserts before the 4-aligned `RefPicList0` arrays).
const _: () = assert!(std::mem::size_of::<VAPictureParameterBufferH264>() == 672);
const _: () = assert!(std::mem::size_of::<VAIQMatrixBufferH264>() == 240);
const _: () = assert!(std::mem::size_of::<VASliceParameterBufferH264>() == 3128);
