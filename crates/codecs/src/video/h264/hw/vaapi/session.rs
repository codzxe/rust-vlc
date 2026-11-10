//! The VA-API H.264 decode session (card 7.B1): the
//! [`H264HwDecoder`](super::H264HwDecoder) implementation for the
//! Linux lane.
//!
//! # Runtime honesty (read this first)
//!
//! This sandbox has **no GPU** (`/dev/dri` absent — the probe's
//! typed `NoDeviceNode` path is what actually runs here). The whole
//! driver-mediated pipeline below (`vaCreateSurfaces` …
//! `vaSyncSurface`) is therefore **compile-verified + unit-verified
//! in its pure parts only; runtime hardware behavior NOT exercised
//! — `[VERIFY]`**. Everything the host *can* test honestly is
//! tested: profile negotiation mapping, POC arithmetic, DPB
//! reference-list construction (§8.2.4), buffer field packing —
//! pure functions, golden-pinned; the lane's probe failure path is
//! live (the typed no-hardware outcome this host really produces).
//!
//! # Session shape
//!
//! [`VaApiSession`] is minted by the bridge as **unopened**: the
//! bridge's job is only "this accelerator is mine" (downcast). The
//! first [`configure`](H264HwDecoder::configure) performs the whole
//! device bring-up — `dlopen` libva + libva-drm, open the DRM node,
//! `vaGetDisplayDRM`, `vaInitialize`, profile/entrypoint query,
//! `vaCreateConfig` — so every failure lands in the **typed**
//! `Configure` error channel, never a panic, never a fake session.
//! A re-configure (new stream generation) tears the per-stream
//! objects (context/surfaces/config) down to the display and
//! rebuilds them.
//!
//! # The buffer pipeline (one picture)
//!
//! ```text
//! pick free surface ─▶ vaBeginPicture(surface)
//!      vaCreateBuffer × {picture-parameter, IQ-matrix,
//!                        per-slice: slice-parameter, slice-data}
//!      vaRenderPicture(all ids) ─▶ vaEndPicture ─▶ vaSyncSurface
//! emit H264HwFrame{surface: pool handle} ─▶ DPB bookkeeping (§8.2.5)
//! ```
//!
//! The DPB bookkeeping is the honest bridge between the *parser*
//! (headers retained, no decoder state) and the *driver* (wants
//! filled `VAPictureH264` DPB slots + ref lists): the session
//! tracks `(surface, frame_num, POC, short/long-term)` per decoded
//! reference picture and reconstructs list0/list1 per §8.2.4
//! (initial order + the parsed `ref_pic_list_modification` ops).
//!
//! # Typed refusals (honest gaps, never guesses)
//!
//! | stream feature | code | why |
//! |---|---|---|
//! | field pictures (PAFF/interlaced) | `h264.hw.vaapi.field_picture` | per-field DPB/POC not modeled |
//! | `pic_order_cnt_type == 1` | `h264.hw.vaapi.poc_type1` | SPS parser does not retain `offset_for_non_ref_pic` (sps.rs follow-up) |
//! | MVC (prefix/subset-SPS NALs, view-index ops) | `h264.hw.vaapi.mvc` | single-view lane |
//! | MMCO op 6 (MVC long-term) | `h264.hw.vaapi.mmco6` | MVC-only form |
//! | ref-list op naming an absent picture | `h264.hw.vaapi.ref_list` | damage: refuse, never mispredict |
//! | scaling lists | — (ffi.rs honesty note) | not detectable until the parser retains the flag; flat defaults used, `[VERIFY]` |
//!
//! §8.2.4.3's `picNumX` modular arithmetic is implemented per the
//! spec's `( CurrPicNum + differenceOfPicNums ) % MaxFrameNum` form
//! **plus** the wrapped-`PicNum` comparison space — the exact
//! two-form reading is `[VERIFY: no conformance corpus ran through
//! this path; differential-vs-oracle is the admission gate]`.
//!
//! # Safety posture
//!
//! Module-level `#[allow(unsafe_code)]` (ADR-001 §5: VA-API is the
//! E2 OS-service binding; MP §9 / doc 01 §2.7 audited exception).
//! Every call site documents its invariants; the struct families
//! crossing the boundary are the 2.22.0 frozen-ABI buffers of
//! [`super::ffi`] (size-pinned there).

// SAFETY-SCOPED MODULE: VA-API driver calls (libva 2.22.0 ABI).
#![allow(unsafe_code)]

use std::fs::File;
use std::os::fd::AsRawFd;

use super::super::capability::{HwAvailability, HwBackend, HwUnavailableReason};
use super::super::frame_pool::{HwSurfacePool, MAX_SURFACES};
use super::super::{
    H264HwAccessUnit, H264HwDecoded, H264HwDecoder, H264HwError, H264HwFrame, H264HwStreamInfo,
};
use super::device::{DriDiscovery, DriNode, DriScanner};
use super::ffi::{
    self, VA_CONFIG_ATTRIB_RT_FORMAT, VA_ENTRYPOINT_VLD, VA_INVALID_ID,
    VA_PICTURE_H264_LONG_TERM_REFERENCE, VA_PICTURE_H264_SHORT_TERM_REFERENCE,
    VA_PROFILE_H264_CONSTRAINED_BASELINE, VA_PROFILE_H264_HIGH, VA_PROFILE_H264_MAIN,
    VA_RT_FORMAT_YUV420, VA_STATUS_SUCCESS, VABufferID, VAConfigAttrib, VAIQMatrixBufferH264,
    VAPictureH264, VAPictureParameterBufferH264, VASliceParameterBufferH264, VASurfaceID,
    va_status_text,
};
use super::library::{VaDecodeCalls, VaLibrary};
use crate::video::h264::parser::nal::NalUnitType;
use crate::video::h264::parser::pps::PicParameterSet;
use crate::video::h264::parser::slice::{
    DecRefPicMarking, MmcoOp, PicOrderSyntax, PredWeightEntry, RefPicListModOp, SliceHeader,
    SliceKind, next_frame_num_offset, pic_order_cnt_type2,
};
use crate::video::h264::parser::sps::SeqParameterSet;

/// Reference-frame slots VA-API's picture parameter carries (the
/// frozen 2.22.0 `reference_frames` array width).
const VA_DPB_SLOTS: usize = 16;

/// Per-list reference slots of the slice parameter (frozen 32).
const VA_REF_LIST_SLOTS: usize = 32;

/// Extra surfaces beyond the DPB (driver scratch + present queue —
/// VA-API's own guidance is "num_ref_frames plus a few").
const SURFACES_BEYOND_DPB: usize = 6;

/// Minimum surface count (intra-only streams still need decode +
/// emit headroom).
const MIN_SURFACES: usize = 4;

// ===========================================================================
// Pure negotiation + reconstruction logic (host-testable, no FFI).
// ===========================================================================

/// The VA-API profile an SPS maps onto (`None` = a profile this
/// lane does not serve — extended/MVC/High-10/etc.).
fn va_profile_for(sps: &SeqParameterSet) -> Option<ffi::VAProfile> {
    // Baseline + constraint_set1_flag (bit 6) = Constrained
    // Baseline, the hardware-universal intersection.
    if sps.profile_idc == 66 && (sps.constraint_flags & 0x40 != 0) {
        return Some(VA_PROFILE_H264_CONSTRAINED_BASELINE);
    }
    match sps.profile_idc {
        66 | 88 => Some(VA_PROFILE_H264_MAIN), // Baseline/Extended → Main superset
        77 => Some(VA_PROFILE_H264_MAIN),
        100 => Some(VA_PROFILE_H264_HIGH),
        _ => None,
    }
}

/// The folded slice-type code (Table 7-7; drivers take `t % 5`).
fn slice_type_code(header: &SliceHeader) -> u8 {
    match header.slice_type.kind {
        SliceKind::P => 0,
        SliceKind::B => 1,
        SliceKind::I => 2,
        SliceKind::Sp => 3,
        SliceKind::Si => 4,
    }
}

/// One tracked decoded picture (the session-side DPB entry). The
/// wrapped `PicNum` is derived per-use (it is relative to the
/// current picture, §8.2.3), so only the raw facts are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DpbPicture {
    /// The VA surface carrying the picture.
    pub(crate) surface: VASurfaceID,
    /// `frame_num` at decode time (raw, unwrapped).
    pub(crate) frame_num: u32,
    /// Long-term (`LongTermFrameIdx` in `long_term_frame_idx`).
    pub(crate) long_term: bool,
    /// `LongTermFrameIdx` (long-term entries only).
    pub(crate) long_term_frame_idx: u32,
    /// The picture's POC (frame pictures: the picture's POC).
    pub(crate) poc: i32,
}

/// §8.2.3 frame `PicNum` in wrapped `i64` space (relative to the
/// current picture's `frame_num`): entries decoded in a previous
/// `frame_num` cycle wrap negative.
fn pic_num_of(entry: &DpbPicture, current_frame_num: u32, max_frame_num: u32) -> i64 {
    let raw = i64::from(entry.frame_num);
    if entry.frame_num > current_frame_num {
        raw - i64::from(max_frame_num)
    } else {
        raw
    }
}

/// Why a reference-list build failed (typed, never a guessed list).
#[derive(Debug, Clone, PartialEq, Eq)]
enum RefListError {
    /// `modification_of_pic_nums_idc` 4/5 (MVC view ops).
    MvcViewOp,
    /// The op's target picture is not in the DPB (damage).
    MissingTarget { pic_num: u32 },
}

/// Build one reference list (§8.2.4): initial order (short-term by
/// `PicNum` descending, then long-term by `LongTermPicNum`
/// ascending), then the parsed reordering ops applied in sequence.
fn build_ref_list(
    dpb: &[DpbPicture],
    current_frame_num: u32,
    max_frame_num: u32,
    ops: &[RefPicListModOp],
    max_entries: usize,
) -> Result<Vec<VAPictureH264>, RefListError> {
    // Initial order.
    let mut remaining: Vec<DpbPicture> = dpb.to_vec();
    remaining.sort_by(|a, b| match (a.long_term, b.long_term) {
        (false, false) => pic_num_of(b, current_frame_num, max_frame_num).cmp(&pic_num_of(
            a,
            current_frame_num,
            max_frame_num,
        )),
        (true, true) => a.long_term_frame_idx.cmp(&b.long_term_frame_idx),
        (false, true) => std::cmp::Ordering::Less,
        (true, false) => std::cmp::Ordering::Greater,
    });
    let mut list: Vec<DpbPicture> = Vec::with_capacity(remaining.len());
    let mut position = 0usize;
    for op in ops {
        match op {
            // idc 4/5: MVC view indices — out of lane scope.
            RefPicListModOp::ShortTermByViewIdx { .. } => {
                return Err(RefListError::MvcViewOp);
            }
            // idc 2: a long-term picture by LongTermPicNum.
            RefPicListModOp::LongTermByPicNum { long_term_pic_num } => {
                let found = remaining
                    .iter()
                    .position(|p| p.long_term && p.long_term_frame_idx == *long_term_pic_num);
                match found {
                    Some(index) => splice_in(&mut list, &mut position, remaining.remove(index)),
                    None => {
                        return Err(RefListError::MissingTarget {
                            pic_num: *long_term_pic_num,
                        });
                    }
                }
            }
            // idc 0/1: a short-term picture by the §8.2.4.3.2
            // picNumX derivation (possibly re-flagged long-term).
            short_term_op => {
                let abs_diff = match short_term_op {
                    RefPicListModOp::ShortTermByPicNum {
                        abs_diff_pic_num_minus1,
                    }
                    | RefPicListModOp::ShortTermToLongTermByPicNum {
                        abs_diff_pic_num_minus1,
                    } => *abs_diff_pic_num_minus1,
                    // Unreachable: both remaining variants bound
                    // above; kept total instead of panicking.
                    _ => 0,
                };
                let difference = u64::from(abs_diff) + 1;
                // The spec form: (CurrPicNum + diff) % MaxFrameNum.
                let modulo = (u64::from(current_frame_num) + difference) % u64::from(max_frame_num);
                // The wrapped-space form: CurrPicNum − diff (the
                // older-picture direction). A real target matches
                // at least one; accepting either is the two-form
                // reading documented in the module header
                // [VERIFY].
                let negative = i64::from(current_frame_num) - difference as i64;
                let found = remaining.iter().position(|p| {
                    if p.long_term {
                        return false;
                    }
                    let pic_num = pic_num_of(p, current_frame_num, max_frame_num);
                    pic_num == negative
                        || (i64::from(p.frame_num) == modulo as i64
                            && p.frame_num > current_frame_num)
                });
                match found {
                    Some(index) => {
                        let mut entry = remaining.remove(index);
                        if matches!(
                            short_term_op,
                            RefPicListModOp::ShortTermToLongTermByPicNum { .. }
                        ) {
                            entry.long_term = true;
                            entry.long_term_frame_idx = u32::try_from(modulo).unwrap_or(0);
                        }
                        splice_in(&mut list, &mut position, entry);
                    }
                    None => {
                        return Err(RefListError::MissingTarget {
                            pic_num: u32::try_from(modulo).unwrap_or(u32::MAX),
                        });
                    }
                }
            }
        }
    }
    // The untouched initial order tails the list (§8.2.4.3.6).
    list.extend(remaining);
    list.truncate(max_entries.min(VA_REF_LIST_SLOTS));
    Ok(list.into_iter().map(|p| va_picture_of(&p)).collect())
}

/// §8.2.4.3 "insert at the current position, shift the rest".
fn splice_in(list: &mut Vec<DpbPicture>, position: &mut usize, entry: DpbPicture) {
    let at = (*position).min(list.len());
    list.insert(at, entry);
    *position = at + 1;
}

/// A DPB entry's `VAPictureH264` projection (flags + frame_idx per
/// §8.2.2: `frame_idx` carries `LongTermFrameIdx` for long-term
/// entries, else `frame_num`).
fn va_picture_of(p: &DpbPicture) -> VAPictureH264 {
    VAPictureH264 {
        picture_id: p.surface,
        frame_idx: if p.long_term {
            p.long_term_frame_idx
        } else {
            p.frame_num
        },
        flags: if p.long_term {
            VA_PICTURE_H264_LONG_TERM_REFERENCE
        } else {
            VA_PICTURE_H264_SHORT_TERM_REFERENCE
        },
        top_field_order_cnt: p.poc,
        bottom_field_order_cnt: p.poc,
        va_reserved: [0; 4],
    }
}

/// Coded picture size in pixels (`None` on geometry overflow).
fn coded_size(sps: &SeqParameterSet) -> Option<(u32, u32)> {
    let width = sps
        .pic_width_in_mbs_minus1
        .checked_add(1)?
        .checked_mul(16)?;
    let map_units = sps.pic_height_in_map_units_minus1.checked_add(1)?;
    let height = map_units
        .checked_mul(if sps.frame_mbs_only { 1 } else { 2 })?
        .checked_mul(16)?;
    Some((width, height))
}

// ===========================================================================
// The session.
// ===========================================================================

/// The VA-API H.264 session (one per open stream; minted by
/// [`super::VaApiBridge`]).
///
/// Device bring-up is lazy (first `configure`); teardown is
/// `Drop`-ordered in reverse (context → surfaces → config →
/// `vaTerminate` → fd → dlopen handles).
///
/// `Send` (the [`H264HwDecoder`] contract): the one raw pointer
/// member is the opaque `VADisplay`, held exclusively by this
/// session and only ever passed back to libva calls on the owning
/// thread. libva's public API is thread-safe per display
/// (internally locked; the 2.22.0 `va.h` contract), and the fn
/// pointers of [`VaLibrary`] are plain code addresses — moving the
/// session across threads creates no aliasing (single owner) and
/// no data race (libva-internal locking).
pub struct VaApiSession {
    /// The DRM node this session serves (from the accelerator).
    node: DriNode,
    /// Fixture root (tests only; production scans the real `/dev`).
    root: Option<std::path::PathBuf>,
    /// Loaded symbol table — `None` until first configure.
    lib: Option<VaLibrary>,
    /// The `VADisplay` (opaque driver context; never dereferenced).
    display: ffi::VADisplay,
    /// The open DRM node fd (outlives the display).
    fd: Option<File>,
    /// `(major, minor)` as `vaInitialize` reported.
    va_version: (i32, i32),
    /// Current config id (`VA_INVALID_ID` when torn down).
    config: ffi::VAConfigID,
    /// Current context id.
    context: ffi::VAContextID,
    /// Coded size of the current stream.
    coded: (u32, u32),
    /// Display size of the current stream.
    display_size: Option<(u32, u32)>,
    /// The bounded surface pool (slot → `VASurfaceID`).
    pool: HwSurfacePool<VASurfaceID>,
    /// Every created surface id (destroyed as one array).
    surfaces: Vec<VASurfaceID>,
    /// Round-robin cursor for free-surface picks.
    surface_cursor: usize,
    /// The active SPS snapshot.
    sps: Option<SeqParameterSet>,
    /// The active PPS snapshot.
    pps: Option<PicParameterSet>,
    /// The session DPB (reference tracking, §8.2).
    dpb: Vec<DpbPicture>,
    /// POC state: previous reference frame_num.
    prev_ref_frame_num: u32,
    /// POC state: previous reference FrameNumOffset (type 2).
    frame_num_offset: u32,
    /// POC state: previous reference POC lsb + msb (type 0).
    prev_ref_poc_lsb: u32,
    prev_ref_poc_msb: u32,
}

// SAFETY: the only non-Send member is the opaque `VADisplay` raw
// pointer (see the struct docs above): it is created and destroyed
// by this session alone (single owner, no aliasing) and every use
// goes through libva calls, which are internally locked per display
// (the 2.22.0 `va.h` thread-safety contract). Moving the session to
// another thread therefore races with nothing.
unsafe impl Send for VaApiSession {}

impl H264HwDecoder for VaApiSession {
    fn backend_id(&self) -> &str {
        HwBackend::VaApi.backend_id()
    }

    fn configure(&mut self, stream: &H264HwStreamInfo) -> Result<(), H264HwError> {
        // 1. Device bring-up (first configure only; typed failures).
        self.ensure_display()?;
        // 2. The stream needs an SPS+PPS pair.
        let (sps, pps) = pick_parameter_sets(stream).ok_or(H264HwError::Configure {
            code: "h264.hw.vaapi.no_parameter_sets",
        })?;
        // 3. Typed refusals the session cannot decode honestly.
        if !sps.frame_mbs_only {
            return Err(H264HwError::Configure {
                code: "h264.hw.vaapi.field_picture",
            });
        }
        if sps.pic_order_cnt_type == 1 {
            return Err(H264HwError::Configure {
                code: "h264.hw.vaapi.poc_type1",
            });
        }
        let Some((coded_width, coded_height)) = coded_size(&sps) else {
            return Err(H264HwError::Configure {
                code: "h264.hw.vaapi.geometry",
            });
        };
        // 4. Profile negotiation against the driver's real table.
        let profile = va_profile_for(&sps).ok_or(H264HwError::Configure {
            code: "h264.hw.vaapi.profile",
        })?;
        self.ensure_profile(profile)?;
        // 5. (Re)build the per-stream objects.
        self.teardown_stream();
        self.create_stream_objects(profile, &sps, coded_width, coded_height)?;
        self.display_size = stream.display_size;
        self.sps = Some(sps);
        self.pps = Some(pps);
        self.dpb.clear();
        self.prev_ref_frame_num = 0;
        self.frame_num_offset = 0;
        self.prev_ref_poc_lsb = 0;
        self.prev_ref_poc_msb = 0;
        self.surface_cursor = 0;
        Ok(())
    }

    fn decode(&mut self, au: &H264HwAccessUnit) -> Result<H264HwDecoded, H264HwError> {
        let (sps, pps) = match (&self.sps, &self.pps) {
            (Some(sps), Some(pps)) => (sps.clone(), pps.clone()),
            _ => {
                return Err(H264HwError::Decode {
                    code: "h264.hw.vaapi.not_configured",
                });
            }
        };
        // MVC NALs are out of scope (single-view lane).
        if au
            .units
            .iter()
            .any(|u| matches!(u.unit_type, NalUnitType::Prefix | NalUnitType::SubsetSps))
        {
            return Err(H264HwError::Decode {
                code: "h264.hw.vaapi.mvc",
            });
        }
        let Some(first_vcl) = au.first_vcl() else {
            // Parameter-set-only / SEI-only AU: nothing to decode.
            return Ok(H264HwDecoded::default());
        };
        let header = first_vcl.slice.clone();
        if header.field_pic_flag {
            return Err(H264HwError::Decode {
                code: "h264.hw.vaapi.field_picture",
            });
        }
        // Copy snapshot of the picture-pipeline fn pointers: taking
        // &VaLibrary here would pin &self across the &mut-self pool /
        // DPB / POC updates below (E0502).
        let calls = self
            .lib
            .as_ref()
            .ok_or(H264HwError::Decode {
                code: "h264.hw.vaapi.not_configured",
            })?
            .decode_calls();
        // Pick a free surface (bounded, typed — never a grow).
        let surface_id = self.next_free_surface().ok_or(H264HwError::Decode {
            code: "h264.hw.vaapi.pool_exhausted",
        })?;
        let handle = self
            .pool
            .acquire(surface_id)
            .map_err(|_| H264HwError::Decode {
                code: "h264.hw.vaapi.pool_exhausted",
            })?;
        // SAFETY: display/context are the live ids created (and not
        // yet destroyed) by configure; surface_id is pool-owned.
        let status = unsafe { (calls.begin_picture)(self.display, self.context, surface_id) };
        if status != VA_STATUS_SUCCESS {
            let _ = self.pool.release(handle);
            return Err(va_op_error("h264.hw.vaapi.begin_picture", status));
        }
        // ---- picture parameter + IQ matrix ----
        let max_frame_num = sps.max_frame_num();
        let poc = self.picture_poc(&sps, &header, first_vcl.nal.idr_pic_flag)?;
        let curr_pic = VAPictureH264 {
            picture_id: surface_id,
            frame_idx: header.frame_num,
            flags: if first_vcl.nal.nal_ref_idc != 0 {
                VA_PICTURE_H264_SHORT_TERM_REFERENCE
            } else {
                0
            },
            top_field_order_cnt: poc,
            bottom_field_order_cnt: poc,
            va_reserved: [0; 4],
        };
        let pic_param = picture_parameter(&sps, &pps, &header, curr_pic, &self.dpb);
        let iq = VAIQMatrixBufferH264::flat_default();
        // ---- slice parameters + the shared slice-data buffer ----
        let mut slice_params: Vec<VASliceParameterBufferH264> = Vec::new();
        let mut slice_data: Vec<u8> = Vec::new();
        let max_entries = usize::from(sps.max_num_ref_frames.max(1)).min(VA_REF_LIST_SLOTS);
        for unit in &au.units {
            let Some(vcl) = &unit.vcl else { continue };
            if vcl.slice.field_pic_flag {
                let _ = self.pool.release(handle);
                return Err(H264HwError::Decode {
                    code: "h264.hw.vaapi.field_picture",
                });
            }
            let (list0, list1) = self.ref_lists(&self.dpb, &vcl.slice, max_entries)?;
            let offset = u32::try_from(slice_data.len()).unwrap_or(u32::MAX);
            let data_size = u32::try_from(unit.nal.len()).unwrap_or(u32::MAX);
            slice_data.extend_from_slice(&unit.nal);
            slice_params.push(slice_parameter(
                &pps,
                &vcl.slice,
                vcl.nal.nal_ref_idc,
                offset,
                data_size,
                list0,
                list1,
            ));
        }
        // ---- create + render the buffers ----
        // Order per the ABI's own usage: picture-parameter,
        // IQ-matrix, then per slice: slice-parameter, slice-data
        // (the data buffer is shared; its id is passed per slice).
        let mut ids: Vec<VABufferID> = Vec::with_capacity(2 + slice_params.len() * 2);
        self.push_buffer(
            &calls,
            &mut ids,
            ffi::VA_PICTURE_PARAMETER_BUFFER_TYPE,
            std::ptr::from_ref(&pic_param).cast(),
            size_of_u32::<VAPictureParameterBufferH264>(),
        )
        .inspect_err(|_| {
            let _ = self.pool.release(handle);
        })?;
        self.push_buffer(
            &calls,
            &mut ids,
            ffi::VA_IQ_MATRIX_BUFFER_TYPE,
            std::ptr::from_ref(&iq).cast(),
            size_of_u32::<VAIQMatrixBufferH264>(),
        )
        .inspect_err(|_| {
            let _ = self.pool.release(handle);
        })?;
        let sp_size = size_of_u32::<VASliceParameterBufferH264>();
        for sp in &slice_params {
            self.push_buffer(
                &calls,
                &mut ids,
                ffi::VA_SLICE_PARAMETER_BUFFER_TYPE,
                std::ptr::from_ref(sp).cast(),
                sp_size,
            )
            .inspect_err(|_| {
                let _ = self.pool.release(handle);
            })?;
            self.push_buffer(
                &calls,
                &mut ids,
                ffi::VA_SLICE_DATA_BUFFER_TYPE,
                slice_data.as_ptr().cast(),
                u32::try_from(slice_data.len()).unwrap_or(u32::MAX),
            )
            .inspect_err(|_| {
                let _ = self.pool.release(handle);
            })?;
        }
        // SAFETY: ids are live create_buffer results, not yet
        // destroyed.
        let status = unsafe {
            (calls.render_picture)(
                self.display,
                self.context,
                ids.as_mut_ptr(),
                ids.len() as i32,
            )
        };
        let render_result = if status != VA_STATUS_SUCCESS {
            Err(va_op_error("h264.hw.vaapi.render_picture", status))
        } else {
            Ok(())
        };
        // Buffers are destroyable after vaEndPicture per the ABI;
        // destroy them regardless of the render outcome (no leak on
        // the error path).
        for id in ids {
            // SAFETY: each id was created on this context and is
            // destroyed exactly once here.
            unsafe { (calls.destroy_buffer)(self.display, id) };
        }
        if let Err(e) = render_result {
            let _ = self.pool.release(handle);
            return Err(e);
        }
        // SAFETY: live context.
        let status = unsafe { (calls.end_picture)(self.display, self.context) };
        if status != VA_STATUS_SUCCESS {
            let _ = self.pool.release(handle);
            return Err(va_op_error("h264.hw.vaapi.end_picture", status));
        }
        // Synchronous decode (the trait's contract).
        // SAFETY: surface_id is pool-owned and was just submitted.
        let status = unsafe { (calls.sync_surface)(self.display, surface_id) };
        if status != VA_STATUS_SUCCESS {
            let _ = self.pool.release(handle);
            return Err(va_op_error("h264.hw.vaapi.sync_surface", status));
        }
        // DPB bookkeeping (§8.2.5) — the entry's wrapped PicNum
        // derivation uses this picture's frame_num, so it must run
        // while the entry set is still the pre-update view.
        self.update_dpb(
            &sps,
            &header,
            first_vcl.nal.nal_ref_idc,
            first_vcl.nal.idr_pic_flag,
            surface_id,
            poc,
            max_frame_num,
        )?;
        // Emit: the slot's resource stays resolvable for the
        // present side until release.
        if !self.pool.emit(handle) {
            let _ = self.pool.release(handle);
            return Err(H264HwError::Decode {
                code: "h264.hw.vaapi.pool_exhausted",
            });
        }
        Ok(H264HwDecoded {
            frames: vec![H264HwFrame {
                surface: handle,
                size: self.display_size.unwrap_or(self.coded),
                interlaced: false,
                pts: au.pts,
            }],
        })
    }

    fn flush(&mut self) -> Result<H264HwDecoded, H264HwError> {
        // The session decodes synchronously (vaSyncSurface per
        // picture), so every submitted picture has already been
        // returned by its decode call: flush has nothing buffered.
        Ok(H264HwDecoded::default())
    }
}

/// `std::mem::size_of` as `u32` (the ABI's size argument).
fn size_of_u32<T>() -> u32 {
    u32::try_from(std::mem::size_of::<T>()).unwrap_or(u32::MAX)
}

impl VaApiSession {
    /// An unopened session bound to `node` (the bridge's mint).
    #[must_use = "the bridge's session mint"]
    pub fn new(node: DriNode) -> VaApiSession {
        VaApiSession {
            node,
            root: None,
            lib: None,
            display: std::ptr::null_mut(),
            fd: None,
            va_version: (0, 0),
            config: VA_INVALID_ID,
            context: VA_INVALID_ID,
            coded: (0, 0),
            display_size: None,
            pool: HwSurfacePool::new(0),
            surfaces: Vec::new(),
            surface_cursor: 0,
            sps: None,
            pps: None,
            dpb: Vec::new(),
            prev_ref_frame_num: 0,
            frame_num_offset: 0,
            prev_ref_poc_lsb: 0,
            prev_ref_poc_msb: 0,
        }
    }

    /// Test seam: bring the device up under a fixture root instead
    /// of `/` (never used in production).
    #[cfg(test)]
    fn with_root(node: DriNode, root: &std::path::Path) -> VaApiSession {
        // Struct-update syntax (`..new(node)`) would move out of a
        // Drop type (E0509) — mutate a fresh session instead.
        let mut session = VaApiSession::new(node);
        session.root = Some(root.to_path_buf());
        session
    }

    /// The VA `(major, minor)` of the initialized display ((0,0)
    /// before the first configure — diagnostics).
    #[must_use = "pure accessor"]
    pub fn va_version(&self) -> (i32, i32) {
        self.va_version
    }

    /// A free surface for the next render target: not held by any
    /// live pool slot (in-flight or emitted) and not referenced by
    /// any DPB entry. `None` = exhausted (the typed back-pressure
    /// condition — never a grow past the cap).
    fn next_free_surface(&mut self) -> Option<VASurfaceID> {
        let count = self.surfaces.len();
        for step in 0..count {
            let index = (self.surface_cursor + step) % count;
            let candidate = self.surfaces[index];
            let held = self.pool.holds_resource(&candidate)
                || self.dpb.iter().any(|p| p.surface == candidate);
            if !held {
                self.surface_cursor = (index + 1) % count;
                return Some(candidate);
            }
        }
        None
    }

    /// First-configure device bring-up: dlopen, open node, display,
    /// vaInitialize. Idempotent (skips when the display is live).
    fn ensure_display(&mut self) -> Result<(), H264HwError> {
        if self.lib.is_some() {
            return Ok(());
        }
        let lib = VaLibrary::load().map_err(|_| H264HwError::Configure {
            code: "h264.hw.vaapi.library",
        })?;
        let scanner = match &self.root {
            Some(root) => DriScanner::with_root(root),
            None => DriScanner::new(),
        };
        let code = "h264.hw.vaapi.device_open";
        let fd = scanner
            .open_node(&self.node)
            .map_err(|e| H264HwError::Configure {
                code: if e.kind() == std::io::ErrorKind::PermissionDenied {
                    "h264.hw.vaapi.permission"
                } else {
                    code
                },
            })?;
        // SAFETY: fd is an owned DRM node fd valid for the call;
        // the returned VADisplay is stored and only ever passed
        // back to libva (never dereferenced by us).
        let display = unsafe { (lib.get_display_drm)(fd.as_raw_fd()) };
        if display.is_null() {
            return Err(H264HwError::Configure {
                code: "h264.hw.vaapi.display",
            });
        }
        let mut major = 0i32;
        let mut minor = 0i32;
        // SAFETY: display just constructed; both out-pointers are
        // valid, aligned i32 locals.
        let status = unsafe { (lib.initialize)(display, &mut major, &mut minor) };
        if status != VA_STATUS_SUCCESS {
            return Err(va_configure_error("h264.hw.vaapi.initialize", status));
        }
        self.va_version = (major, minor);
        self.display = display;
        self.fd = Some(fd);
        self.lib = Some(lib);
        Ok(())
    }

    /// Verify the driver serves `profile` with the VLD entrypoint
    /// (the honest capability check at configure time).
    fn ensure_profile(&self, profile: ffi::VAProfile) -> Result<(), H264HwError> {
        let lib = self.lib.as_ref().ok_or(H264HwError::Configure {
            code: "h264.hw.vaapi.library",
        })?;
        let refuse = || H264HwError::Configure {
            code: "h264.hw.vaapi.profile",
        };
        // SAFETY: display live; the out-vec is bounded by the
        // driver's own max; count receives the driver's value.
        let cap = unsafe { (lib.max_num_profiles)(self.display) };
        if cap <= 0 {
            return Err(refuse());
        }
        let mut profiles = vec![0 as ffi::VAProfile; cap as usize];
        let mut count = 0i32;
        // SAFETY: as above.
        let status =
            unsafe { (lib.query_config_profiles)(self.display, profiles.as_mut_ptr(), &mut count) };
        if status != VA_STATUS_SUCCESS {
            return Err(va_configure_error("h264.hw.vaapi.query_profiles", status));
        }
        if !profiles[..count.clamp(0, cap) as usize].contains(&profile) {
            return Err(refuse());
        }
        // SAFETY: same shape for the entrypoint query.
        let ecap = unsafe { (lib.max_num_entrypoints)(self.display) };
        if ecap <= 0 {
            return Err(refuse());
        }
        let mut entrypoints = vec![0 as ffi::VAEntrypoint; ecap as usize];
        let mut ecount = 0i32;
        // SAFETY: as above.
        let status = unsafe {
            (lib.query_config_entrypoints)(
                self.display,
                profile,
                entrypoints.as_mut_ptr(),
                &mut ecount,
            )
        };
        if status != VA_STATUS_SUCCESS
            || !entrypoints[..ecount.clamp(0, ecap) as usize].contains(&VA_ENTRYPOINT_VLD)
        {
            return Err(refuse());
        }
        Ok(())
    }

    /// Tear the per-stream objects down to the display.
    fn teardown_stream(&mut self) {
        if let Some(lib) = &self.lib
            && !self.display.is_null()
        {
            // SAFETY: context/config/surfaces are the live ids
            // created on this display by create_stream_objects (the
            // guards skip the VA_INVALID_ID / empty no-op cases).
            if self.context != VA_INVALID_ID {
                unsafe { (lib.destroy_context)(self.display, self.context) };
            }
            if self.config != VA_INVALID_ID {
                unsafe { (lib.destroy_config)(self.display, self.config) };
            }
            if !self.surfaces.is_empty() {
                unsafe {
                    (lib.destroy_surfaces)(
                        self.display,
                        self.surfaces.as_mut_ptr(),
                        self.surfaces.len() as i32,
                    )
                };
            }
        }
        self.context = VA_INVALID_ID;
        self.config = VA_INVALID_ID;
        self.surfaces.clear();
        self.pool.drain();
    }

    /// Create config + surfaces + context for the negotiated
    /// profile/geometry.
    fn create_stream_objects(
        &mut self,
        profile: ffi::VAProfile,
        sps: &SeqParameterSet,
        coded_width: u32,
        coded_height: u32,
    ) -> Result<(), H264HwError> {
        let lib = self.lib.as_ref().ok_or(H264HwError::Configure {
            code: "h264.hw.vaapi.library",
        })?;
        // Surface count: DPB + headroom, bounded by the pool cap.
        let dpb_size = usize::from(sps.max_num_ref_frames).clamp(1, VA_DPB_SLOTS);
        let count = (dpb_size + SURFACES_BEYOND_DPB).clamp(MIN_SURFACES, MAX_SURFACES);
        // Config: RT format YUV420 (the 8-bit 4:2:0 surface; other
        // samplings were refused typed by the codec layer's
        // surface_format gate upstream).
        let attrib = VAConfigAttrib {
            kind: VA_CONFIG_ATTRIB_RT_FORMAT,
            value: VA_RT_FORMAT_YUV420,
        };
        let mut config = VA_INVALID_ID;
        // SAFETY: display live; attrib is one fully-initialized
        // struct; config out-pointer valid.
        let status = unsafe {
            (lib.create_config)(
                self.display,
                profile,
                VA_ENTRYPOINT_VLD,
                std::ptr::from_ref(&attrib) as *mut VAConfigAttrib,
                1,
                &mut config,
            )
        };
        if status != VA_STATUS_SUCCESS {
            return Err(va_configure_error("h264.hw.vaapi.create_config", status));
        }
        self.config = config;
        // Surfaces (default attributes — the NULL attrib form).
        let mut surfaces = vec![VA_INVALID_ID; count];
        // SAFETY: display + config live; the out-array is `count`
        // u32 slots; the attrib list is NULL/0 (the documented
        // default-attribute form).
        let status = unsafe {
            (lib.create_surfaces)(
                self.display,
                VA_RT_FORMAT_YUV420,
                coded_width,
                coded_height,
                surfaces.as_mut_ptr(),
                count as u32,
                std::ptr::null_mut(),
                0,
            )
        };
        if status != VA_STATUS_SUCCESS {
            return Err(va_configure_error("h264.hw.vaapi.create_surfaces", status));
        }
        self.surfaces = surfaces;
        self.pool = HwSurfacePool::new(count);
        // Context over the surface set (progressive: flag 0).
        let mut context: ffi::VAContextID = VA_INVALID_ID;
        // SAFETY: display + config live; render_targets is the
        // surface array of live ids, length passed verbatim.
        let status = unsafe {
            (lib.create_context)(
                self.display,
                self.config,
                coded_width as i32,
                coded_height as i32,
                0,
                self.surfaces.as_mut_ptr(),
                self.surfaces.len() as i32,
                &mut context,
            )
        };
        if status != VA_STATUS_SUCCESS {
            return Err(va_configure_error("h264.hw.vaapi.create_context", status));
        }
        self.context = context;
        self.coded = (coded_width, coded_height);
        Ok(())
    }

    /// One `vaCreateBuffer` + id bookkeeping (all invariants here,
    /// once).
    fn push_buffer(
        &self,
        calls: &VaDecodeCalls,
        ids: &mut Vec<VABufferID>,
        kind: ffi::VABufferType,
        bytes: *const u8,
        size: u32,
    ) -> Result<(), H264HwError> {
        let mut id = VA_INVALID_ID;
        // SAFETY: display/context live (configure); `bytes` points
        // at `size` fully-initialized bytes that outlive the call —
        // vaCreateBuffer copies the data into driver-owned storage,
        // so no aliasing survives the call.
        let status = unsafe {
            (calls.create_buffer)(
                self.display,
                self.context,
                kind,
                size,
                1,
                bytes as *mut std::ffi::c_void,
                &mut id,
            )
        };
        if status != VA_STATUS_SUCCESS || id == VA_INVALID_ID {
            return Err(va_op_error("h264.hw.vaapi.create_buffer", status));
        }
        ids.push(id);
        Ok(())
    }

    /// The picture's POC (type 0 wrap state or type 2 arithmetic;
    /// type 1 refused typed at configure).
    fn picture_poc(
        &mut self,
        sps: &SeqParameterSet,
        header: &SliceHeader,
        idr: bool,
    ) -> Result<i32, H264HwError> {
        match (&header.pic_order, sps.pic_order_cnt_type) {
            (PicOrderSyntax::Type0 { lsb, delta_bottom }, 0) => {
                let max = sps.max_pic_order_cnt_lsb();
                let overflow = || H264HwError::Decode {
                    code: "h264.hw.vaapi.poc",
                };
                // §8.2.1.1 wrap arithmetic (frame pictures).
                let msb: i64 = if idr {
                    0
                } else if lsb < &self.prev_ref_poc_lsb
                    && self.prev_ref_poc_lsb.wrapping_sub(*lsb) >= max / 2
                {
                    i64::from(self.prev_ref_poc_msb) + i64::from(max)
                } else if lsb > &self.prev_ref_poc_lsb
                    && lsb.wrapping_sub(self.prev_ref_poc_lsb) > max / 2
                {
                    i64::from(self.prev_ref_poc_msb) - i64::from(max)
                } else {
                    i64::from(self.prev_ref_poc_msb)
                };
                let top = msb
                    .checked_add(i64::from(*lsb))
                    .and_then(|v| v.checked_add(i64::from(delta_bottom.unwrap_or(0))))
                    .ok_or_else(overflow)?;
                i32::try_from(top).map_err(|_| overflow())
            }
            (PicOrderSyntax::Type2, 2) => {
                let value = pic_order_cnt_type2(idr, 1, header.frame_num, self.frame_num_offset)
                    .map_err(|_| H264HwError::Decode {
                        code: "h264.hw.vaapi.poc",
                    })?;
                i32::try_from(value).map_err(|_| H264HwError::Decode {
                    code: "h264.hw.vaapi.poc",
                })
            }
            _ => Err(H264HwError::Decode {
                code: "h264.hw.vaapi.poc",
            }),
        }
    }

    /// Build (list0, list1) for the slice (§8.2.4; I/SI slices get
    /// empty lists).
    fn ref_lists(
        &self,
        dpb: &[DpbPicture],
        header: &SliceHeader,
        max_entries: usize,
    ) -> Result<(Vec<VAPictureH264>, Vec<VAPictureH264>), H264HwError> {
        let current = header.frame_num;
        let build = |ops: &[RefPicListModOp]| -> Result<Vec<VAPictureH264>, H264HwError> {
            let max_frame_num = self.sps.as_ref().map_or(16, SeqParameterSet::max_frame_num);
            build_ref_list(dpb, current, max_frame_num, ops, max_entries).map_err(|e| match e {
                RefListError::MvcViewOp => H264HwError::Decode {
                    code: "h264.hw.vaapi.mvc",
                },
                RefListError::MissingTarget { .. } => H264HwError::Decode {
                    code: "h264.hw.vaapi.ref_list",
                },
            })
        };
        Ok(match header.slice_type.kind {
            SliceKind::I | SliceKind::Si => (Vec::new(), Vec::new()),
            SliceKind::B => (
                build(&header.ref_pic_list_modification.l0)?,
                build(&header.ref_pic_list_modification.l1)?,
            ),
            SliceKind::P | SliceKind::Sp => {
                (build(&header.ref_pic_list_modification.l0)?, Vec::new())
            }
        })
    }

    /// §8.2.5 marking + sliding-window DPB update for the picture
    /// just decoded onto `surface`.
    ///
    /// (8 parameters = the spec's own inputs: the SPS facts, the
    /// slice's marking syntax, the NAL reference flag + IDR flag, the
    /// surface, the POC and the frame-num modulus — collapsing them
    /// would hide the §8.2.5 mapping, so the lint is allowed here.)
    #[allow(clippy::too_many_arguments)]
    fn update_dpb(
        &mut self,
        sps: &SeqParameterSet,
        header: &SliceHeader,
        nal_ref_idc: u8,
        idr: bool,
        surface: VASurfaceID,
        poc: i32,
        max_frame_num: u32,
    ) -> Result<(), H264HwError> {
        let max_refs = usize::from(sps.max_num_ref_frames).min(VA_DPB_SLOTS);
        if idr {
            // §8.2.5.1: every prior picture is dropped.
            self.dpb.clear();
        } else {
            // Sliding window (§8.2.5.3): while the short-term count
            // exceeds the window, drop the smallest-PicNum
            // (oldest) short-term entry.
            while self.dpb.iter().filter(|p| !p.long_term).count() > max_refs {
                let victim = self
                    .dpb
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| !p.long_term)
                    .min_by_key(|(_, p)| pic_num_of(p, header.frame_num, max_frame_num))
                    .map(|(index, _)| index);
                match victim {
                    Some(index) => {
                        self.dpb.remove(index);
                    }
                    None => break,
                }
            }
        }
        let is_reference = nal_ref_idc != 0;
        if is_reference {
            let mut long_term = false;
            let mut long_term_frame_idx = 0u32;
            match &header.dec_ref_pic_marking {
                Some(DecRefPicMarking::Idr {
                    long_term_reference_flag,
                    ..
                }) => {
                    long_term = *long_term_reference_flag;
                }
                Some(DecRefPicMarking::Adaptive(ops)) => {
                    for op in ops {
                        match op {
                            MmcoOp::End => break,
                            MmcoOp::MarkShortTermUnused {
                                difference_of_pic_nums_minus1,
                            } => {
                                let diff = i64::from(*difference_of_pic_nums_minus1) + 1;
                                let target = i64::from(header.frame_num) - diff;
                                self.dpb.retain(|p| {
                                    p.long_term
                                        || pic_num_of(p, header.frame_num, max_frame_num) != target
                                });
                            }
                            MmcoOp::MarkLongTermUnused {
                                long_term_frame_idx,
                            } => {
                                self.dpb.retain(|p| {
                                    !p.long_term || p.long_term_frame_idx != *long_term_frame_idx
                                });
                            }
                            MmcoOp::ShortTermToLongTerm {
                                difference_of_pic_nums_minus1,
                                long_term_frame_idx: idx,
                            } => {
                                let diff = i64::from(*difference_of_pic_nums_minus1) + 1;
                                let target = i64::from(header.frame_num) - diff;
                                for p in &mut self.dpb {
                                    if !p.long_term
                                        && pic_num_of(p, header.frame_num, max_frame_num) == target
                                    {
                                        p.long_term = true;
                                        p.long_term_frame_idx = *idx;
                                    }
                                }
                            }
                            MmcoOp::SetMaxLongTermFrameIdx { .. } => {}
                            MmcoOp::MarkCurrentLongTerm {
                                long_term_frame_idx: idx,
                            } => {
                                long_term = true;
                                long_term_frame_idx = *idx;
                            }
                            MmcoOp::AssignLongTermFrameIdx { .. } => {
                                return Err(H264HwError::Decode {
                                    code: "h264.hw.vaapi.mmco6",
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
            self.dpb.push(DpbPicture {
                surface,
                frame_num: header.frame_num,
                long_term,
                long_term_frame_idx,
                poc,
            });
            // POC/frame-num state advances on reference pictures
            // (§8.2.1's "previous" quantities).
            if let (PicOrderSyntax::Type0 { lsb, .. }, 0) =
                (&header.pic_order, sps.pic_order_cnt_type)
            {
                let max = sps.max_pic_order_cnt_lsb();
                let msb = if idr {
                    0
                } else if *lsb < self.prev_ref_poc_lsb
                    && self.prev_ref_poc_lsb.wrapping_sub(*lsb) >= max / 2
                {
                    self.prev_ref_poc_msb.wrapping_add(max)
                } else if *lsb > self.prev_ref_poc_lsb
                    && lsb.wrapping_sub(self.prev_ref_poc_lsb) > max / 2
                {
                    self.prev_ref_poc_msb.wrapping_sub(max)
                } else {
                    self.prev_ref_poc_msb
                };
                self.prev_ref_poc_lsb = *lsb;
                self.prev_ref_poc_msb = msb;
            }
            self.frame_num_offset = next_frame_num_offset(
                idr,
                header.frame_num,
                self.prev_ref_frame_num,
                self.frame_num_offset,
                max_frame_num,
            )
            .map_err(|_| H264HwError::Decode {
                code: "h264.hw.vaapi.poc",
            })?;
            self.prev_ref_frame_num = header.frame_num;
        }
        Ok(())
    }
}

impl Drop for VaApiSession {
    fn drop(&mut self) {
        self.teardown_stream();
        if let Some(lib) = &self.lib {
            // SAFETY: display was initialized on this lib and no
            // stream objects remain (teardown_stream above).
            if !self.display.is_null() {
                unsafe { (lib.terminate)(self.display) };
            }
        }
        self.fd = None;
        self.lib = None;
    }
}

/// The SPS/PPS pair a configure uses (lowest-id PPS → its SPS —
/// the decode-side "active" convention).
fn pick_parameter_sets(stream: &H264HwStreamInfo) -> Option<(SeqParameterSet, PicParameterSet)> {
    let pps = stream
        .pps
        .iter()
        .min_by_key(|p| p.pps.pic_parameter_set_id)?;
    let sps = stream
        .sps
        .iter()
        .find(|s| s.sps.seq_parameter_set_id == pps.pps.seq_parameter_set_id)?;
    Some((sps.sps.clone(), pps.pps.clone()))
}

/// Fill the picture-parameter buffer (pure; golden-tested).
fn picture_parameter(
    sps: &SeqParameterSet,
    pps: &PicParameterSet,
    header: &SliceHeader,
    curr_pic: VAPictureH264,
    dpb: &[DpbPicture],
) -> VAPictureParameterBufferH264 {
    let mut reference_frames = [VAPictureH264::empty(); VA_DPB_SLOTS];
    for (slot, pic) in dpb.iter().take(VA_DPB_SLOTS).enumerate() {
        reference_frames[slot] = va_picture_of(pic);
    }
    let seq_fields = ffi::seq_fields::pack(
        u32::from(sps.chroma_format_idc.unwrap_or(1)),
        false, // residual_colour_transform_flag (gone from served profiles)
        sps.gaps_in_frame_num_allowed,
        sps.frame_mbs_only,
        sps.mb_adaptive_frame_field,
        sps.direct_8x8_inference,
        false, // MinLumaBiPredSize8x8 (High-only; [VERIFY] see module docs)
        u32::from(sps.log2_max_frame_num_minus4),
        u32::from(sps.pic_order_cnt_type),
        u32::from(sps.log2_max_pic_order_cnt_lsb_minus4.unwrap_or(0)),
        false, // delta_pic_order_always_zero_flag — not retained by the parser (module docs)
    );
    let pic_fields = ffi::pic_fields::pack(
        pps.entropy_coding_mode,
        pps.weighted_pred,
        u32::from(pps.weighted_bipred_idc),
        false, // transform_8x8_mode_flag — not retained; scaling-list note applies
        false, // field_pic_flag (refused typed upstream)
        pps.constrained_intra_pred,
        pps.bottom_field_pic_order_in_frame_present,
        pps.deblocking_filter_control_present,
        pps.redundant_pic_cnt_present,
        false, // reference_pic_flag — carried on curr_pic.flags instead
    );
    VAPictureParameterBufferH264 {
        curr_pic,
        reference_frames,
        picture_width_in_mbs_minus1: u16::try_from(sps.pic_width_in_mbs_minus1).unwrap_or(u16::MAX),
        picture_height_in_mbs_minus1: u16::try_from(sps.pic_height_in_map_units_minus1)
            .unwrap_or(u16::MAX),
        bit_depth_luma_minus8: sps.bit_depth_luma_minus8,
        bit_depth_chroma_minus8: sps.bit_depth_chroma_minus8,
        num_ref_frames: sps.max_num_ref_frames.min(VA_DPB_SLOTS as u8),
        seq_fields,
        num_slice_groups_minus1: 0,
        slice_group_map_type: 0,
        slice_group_change_rate_minus1: 0,
        pic_init_qp_minus26: pps.pic_init_qp_minus26,
        pic_init_qs_minus26: pps.pic_init_qs_minus26,
        chroma_qp_index_offset: pps.chroma_qp_index_offset,
        second_chroma_qp_index_offset: pps.chroma_qp_index_offset, // ffi.rs honesty note
        pic_fields,
        frame_num: u16::try_from(header.frame_num).unwrap_or(u16::MAX),
        va_reserved: [0; 8],
    }
}

/// Fill one slice-parameter buffer (pure; golden-tested).
fn slice_parameter(
    pps: &PicParameterSet,
    header: &SliceHeader,
    _nal_ref_idc: u8,
    slice_data_offset: u32,
    slice_data_size: u32,
    list0: Vec<VAPictureH264>,
    list1: Vec<VAPictureH264>,
) -> VASliceParameterBufferH264 {
    let mut sp = VASliceParameterBufferH264::empty();
    sp.slice_data_size = slice_data_size;
    sp.slice_data_offset = slice_data_offset;
    sp.slice_data_flag = ffi::VA_SLICE_DATA_FLAG_ALL;
    // Bit offset from the NAL start to the first slice-data bit:
    // 8 (header byte) + the RBSP bits the header consumed.
    sp.slice_data_bit_offset = u16::try_from(8 + header.header_bits).unwrap_or(u16::MAX);
    sp.first_mb_in_slice = u16::try_from(header.first_mb_in_slice).unwrap_or(u16::MAX);
    sp.slice_type = slice_type_code(header);
    sp.direct_spatial_mv_pred_flag = u8::from(header.direct_spatial_mv_pred_flag.unwrap_or(false));
    let (l0, l1) = match &header.num_ref_idx_override {
        Some((l0, l1)) => (
            u8::try_from(*l0).unwrap_or(0),
            l1.map(|v| u8::try_from(v).unwrap_or(0)).unwrap_or(0),
        ),
        None => (
            pps.num_ref_idx_l0_default_active_minus1,
            pps.num_ref_idx_l1_default_active_minus1,
        ),
    };
    sp.num_ref_idx_l0_active_minus1 = l0;
    sp.num_ref_idx_l1_active_minus1 = l1;
    sp.cabac_init_idc = u8::try_from(header.cabac_init_idc.unwrap_or(0)).unwrap_or(0);
    sp.slice_qp_delta = i8::try_from(header.slice_qp_delta).unwrap_or(0);
    if let Some(deblock) = &header.deblocking {
        sp.disable_deblocking_filter_idc =
            u8::try_from(deblock.disable_deblocking_filter_idc).unwrap_or(0);
        sp.slice_alpha_c0_offset_div2 =
            i8::try_from(deblock.slice_alpha_c0_offset_div2.unwrap_or(0)).unwrap_or(0);
        sp.slice_beta_offset_div2 =
            i8::try_from(deblock.slice_beta_offset_div2.unwrap_or(0)).unwrap_or(0);
    }
    for (slot, pic) in list0.iter().take(VA_REF_LIST_SLOTS).enumerate() {
        sp.ref_pic_list0[slot] = *pic;
    }
    for (slot, pic) in list1.iter().take(VA_REF_LIST_SLOTS).enumerate() {
        sp.ref_pic_list1[slot] = *pic;
    }
    if let Some(weights) = &header.pred_weight_table {
        sp.luma_log2_weight_denom = u8::try_from(weights.luma_log2_weight_denom).unwrap_or(0);
        sp.chroma_log2_weight_denom =
            u8::try_from(weights.chroma_log2_weight_denom.unwrap_or(0)).unwrap_or(0);
        fill_weights(
            &mut sp.luma_weight_l0_flag,
            &mut sp.luma_weight_l0,
            &mut sp.luma_offset_l0,
            &mut sp.chroma_weight_l0_flag,
            &mut sp.chroma_weight_l0,
            &mut sp.chroma_offset_l0,
            &weights.l0,
        );
        fill_weights(
            &mut sp.luma_weight_l1_flag,
            &mut sp.luma_weight_l1,
            &mut sp.luma_offset_l1,
            &mut sp.chroma_weight_l1_flag,
            &mut sp.chroma_weight_l1,
            &mut sp.chroma_offset_l1,
            &weights.l1,
        );
    }
    sp
}

/// Fill one pred-weight list's arrays (pure helper).
fn fill_weights(
    luma_flag: &mut u8,
    luma_weight: &mut [i16; 32],
    luma_offset: &mut [i16; 32],
    chroma_flag: &mut u8,
    chroma_weight: &mut [[i16; 2]; 32],
    chroma_offset: &mut [[i16; 2]; 32],
    entries: &[PredWeightEntry],
) {
    for (index, entry) in entries.iter().take(VA_REF_LIST_SLOTS).enumerate() {
        if let Some((weight, offset)) = entry.luma {
            *luma_flag |= 1 << index;
            luma_weight[index] = i16::try_from(weight).unwrap_or(0);
            luma_offset[index] = i16::try_from(offset).unwrap_or(0);
        }
        if let Some(chroma) = entry.chroma {
            *chroma_flag |= 1 << index;
            for plane in 0..2 {
                chroma_weight[index][plane] = i16::try_from(chroma[plane].0).unwrap_or(0);
                chroma_offset[index][plane] = i16::try_from(chroma[plane].1).unwrap_or(0);
            }
        }
    }
}

/// Map a VA status to the typed decode error (stable code per op).
fn va_op_error(code: &'static str, status: ffi::VAStatus) -> H264HwError {
    // The spec spelling is recoverable in a debug session; the
    // stable code names the op. [VERIFY: structured status payload
    // when the reporting seam grows one.]
    let _ = va_status_text(status);
    H264HwError::Decode { code }
}

/// Map a VA status to the typed configure error.
fn va_configure_error(code: &'static str, status: ffi::VAStatus) -> H264HwError {
    let _ = va_status_text(status);
    H264HwError::Configure { code }
}

/// The capability probe (card 7.B1's typed negotiation entry).
///
/// Probe order (each failure typed, never a guessed success):
/// device nodes → node open (permission) → libva load → display +
/// initialize → profile query. On success the vendor string is the
/// truthful `detail` of the [`HwAvailability::Available`] outcome.
pub fn probe() -> (HwAvailability, Option<(DriNode, String)>) {
    probe_with(&DriScanner::new())
}

/// The probe against an injectable scanner root (the test seam),
/// additionally returning the discovered node + vendor when (and
/// only when) the outcome is `Available` — the structured facts
/// [`super::VaApiAccelerator`] is minted from.
pub fn probe_with(scanner: &DriScanner) -> (HwAvailability, Option<(DriNode, String)>) {
    let backend = HwBackend::VaApi;
    let nodes = match scanner.discover() {
        DriDiscovery::Found(nodes) if !nodes.is_empty() => nodes,
        // An empty or absent /dev/dri is the same typed truth: no
        // hardware decode device exists.
        DriDiscovery::Found(_) | DriDiscovery::Empty | DriDiscovery::NoDriDirectory => {
            return (
                HwAvailability::Unavailable {
                    backend,
                    reason: HwUnavailableReason::NoDeviceNode,
                },
                None,
            );
        }
        DriDiscovery::Unreadable(_) => {
            // An unreadable /dev/dri yields no usable device — the
            // same typed truth as an absent one (the reason
            // vocabulary has no generic-Io variant).
            return (
                HwAvailability::Unavailable {
                    backend,
                    reason: HwUnavailableReason::NoDeviceNode,
                },
                None,
            );
        }
    };
    // Open the first render node (the scanner guarantees
    // render-first order).
    let Some(node) = nodes.iter().find(|n| n.render).or_else(|| nodes.first()) else {
        return (
            HwAvailability::Unavailable {
                backend,
                reason: HwUnavailableReason::NoDeviceNode,
            },
            None,
        );
    };
    let fd = match scanner.open_node(node) {
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            return (
                HwAvailability::Unavailable {
                    backend,
                    reason: HwUnavailableReason::PermissionDenied,
                },
                None,
            );
        }
        Err(_) => {
            return (
                HwAvailability::Unavailable {
                    backend,
                    reason: HwUnavailableReason::NoDeviceNode,
                },
                None,
            );
        }
        Ok(fd) => fd,
    };
    let Ok(lib) = VaLibrary::load() else {
        return (
            HwAvailability::Unavailable {
                backend,
                reason: HwUnavailableReason::LibraryUnavailable(format!(
                    "dlopen failed for {} (tried {} and {})",
                    node.name,
                    super::library::VA_SONAMES.join(", "),
                    super::library::VA_DRM_SONAMES.join(", ")
                )),
            },
            None,
        );
    };
    // SAFETY: fd is owned + valid; the display pointer is only
    // passed back to libva calls below and never dereferenced.
    let display = unsafe { (lib.get_display_drm)(fd.as_raw_fd()) };
    if display.is_null() {
        return (
            HwAvailability::Unavailable {
                backend,
                reason: HwUnavailableReason::DriverIncompatible(
                    "vaGetDisplayDRM returned NULL".to_string(),
                ),
            },
            None,
        );
    }
    let mut major = 0i32;
    let mut minor = 0i32;
    // SAFETY: display from vaGetDisplayDRM above; out-pointers
    // valid, aligned i32 locals.
    let status = unsafe { (lib.initialize)(display, &mut major, &mut minor) };
    if status != VA_STATUS_SUCCESS {
        return (
            HwAvailability::Unavailable {
                backend,
                reason: HwUnavailableReason::DriverIncompatible(format!(
                    "vaInitialize: {}",
                    va_status_text(status)
                )),
            },
            None,
        );
    }
    // The honest "driver serves H.264 decode" check: one of the
    // three H.264 profiles this lane maps onto.
    // SAFETY: display initialized; bounded out-array + count.
    let cap = unsafe { (lib.max_num_profiles)(display) };
    let mut profile_ok = false;
    if cap > 0 {
        let mut profiles = vec![0 as ffi::VAProfile; cap as usize];
        let mut count = 0i32;
        // SAFETY: as above.
        if unsafe { (lib.query_config_profiles)(display, profiles.as_mut_ptr(), &mut count) }
            == VA_STATUS_SUCCESS
        {
            let served = &profiles[..count.clamp(0, cap) as usize];
            profile_ok = served.iter().any(|p| {
                matches!(
                    *p,
                    VA_PROFILE_H264_MAIN
                        | VA_PROFILE_H264_HIGH
                        | VA_PROFILE_H264_CONSTRAINED_BASELINE
                )
            });
        }
    }
    let vendor = {
        // SAFETY: display initialized; the returned string is
        // driver-owned static storage — read-only, copied
        // immediately, never freed by us. from_ptr stays inside the
        // same unsafe block: the pointer is only valid C-string
        // storage while libva keeps it alive (probe teardown below
        // happens after the copy).
        let raw = unsafe { (lib.query_vendor_string)(display) };
        if raw.is_null() {
            String::from("unknown vendor")
        } else {
            unsafe { std::ffi::CStr::from_ptr(raw) }
                .to_string_lossy()
                .into_owned()
        }
    };
    // Teardown the probe's objects before reporting (the session
    // creates its own).
    // SAFETY: display initialized exactly once above.
    unsafe { (lib.terminate)(display) };
    drop(fd);
    if !profile_ok {
        return (
            HwAvailability::Unavailable {
                backend,
                reason: HwUnavailableReason::DriverIncompatible(format!(
                    "no H.264 decode profile in {vendor}"
                )),
            },
            None,
        );
    }
    let node = node.clone();
    let via = node.name.clone();
    (
        HwAvailability::Available {
            backend,
            detail: format!("VA-API {major}.{minor} via {via}: {vendor}"),
        },
        Some((node, vendor)),
    )
}

// ===========================================================================
// Host-runnable tests: the pure reconstruction logic + the honest
// probe paths. NO test here touches a driver (the sandbox has no
// GPU; the driver-mediated path is [VERIFY] compile-only).
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::h264::parser::slice::SliceType;
    use std::os::unix::fs::PermissionsExt;

    fn dpb_pic(surface: u32, frame_num: u32, poc: i32) -> DpbPicture {
        DpbPicture {
            surface,
            frame_num,
            long_term: false,
            long_term_frame_idx: 0,
            poc,
        }
    }

    #[test]
    fn profile_mapping_covers_the_served_profiles_only() {
        let base = SeqParameterSet {
            profile_idc: 77,
            constraint_flags: 0,
            level_idc: 30,
            seq_parameter_set_id: 0,
            chroma_format_idc: Some(1),
            separate_colour_plane: false,
            bit_depth_luma_minus8: 0,
            bit_depth_chroma_minus8: 0,
            log2_max_frame_num_minus4: 0,
            pic_order_cnt_type: 2,
            log2_max_pic_order_cnt_lsb_minus4: None,
            poc_cycle_offsets: Vec::new(),
            max_num_ref_frames: 1,
            gaps_in_frame_num_allowed: false,
            pic_width_in_mbs_minus1: 3,
            pic_height_in_map_units_minus1: 2,
            frame_mbs_only: true,
            mb_adaptive_frame_field: false,
            direct_8x8_inference: true,
            crop: None,
            vui: None,
        };
        assert_eq!(va_profile_for(&base), Some(VA_PROFILE_H264_MAIN));
        let mut sps = base.clone();
        sps.profile_idc = 100;
        assert_eq!(va_profile_for(&sps), Some(VA_PROFILE_H264_HIGH));
        sps.profile_idc = 66;
        assert_eq!(va_profile_for(&sps), Some(VA_PROFILE_H264_MAIN));
        // Baseline + constraint_set1 → Constrained Baseline.
        sps.constraint_flags = 0x40;
        assert_eq!(
            va_profile_for(&sps),
            Some(VA_PROFILE_H264_CONSTRAINED_BASELINE)
        );
        // Served-nothing: High 10 (110) — an honest typed refusal.
        sps.profile_idc = 110;
        sps.constraint_flags = 0;
        assert_eq!(va_profile_for(&sps), None);
    }

    #[test]
    fn initial_ref_list_orders_short_term_desc_then_long_term_asc() {
        // FrameNums 7, 5, 3 decoded in order; current picture 9.
        let dpb = [
            dpb_pic(11, 7, 14),
            dpb_pic(12, 5, 10),
            dpb_pic(13, 3, 6),
            DpbPicture {
                surface: 14,
                frame_num: 4,
                long_term: true,
                long_term_frame_idx: 2,
                poc: 8,
            },
            DpbPicture {
                surface: 15,
                frame_num: 2,
                long_term: true,
                long_term_frame_idx: 0,
                poc: 4,
            },
        ];
        let list = build_ref_list(&dpb, 9, 16, &[], 32).unwrap_or_else(|e| unreachable!("{e:?}"));
        let surfaces: Vec<u32> = list.iter().map(|p| p.picture_id).collect();
        assert_eq!(surfaces, [11, 12, 13, 15, 14]);
        // Short-term entries carry SHORT_TERM_REFERENCE + frame_num.
        assert_eq!(
            list[0].flags & VA_PICTURE_H264_SHORT_TERM_REFERENCE,
            VA_PICTURE_H264_SHORT_TERM_REFERENCE
        );
        assert_eq!(list[0].frame_idx, 7);
        // Long-term entries carry LONG_TERM_REFERENCE + the idx.
        assert_eq!(
            list[3].flags & VA_PICTURE_H264_LONG_TERM_REFERENCE,
            VA_PICTURE_H264_LONG_TERM_REFERENCE
        );
        assert_eq!(list[3].frame_idx, 0);
    }

    #[test]
    fn frame_num_wrap_demotes_older_cycle_pictures() {
        // MaxFrameNum 16; entries 15 and 2, current frame_num 3:
        // §8.2.3 — entry 15 > 3 wraps (PicNum 15−16 = −1, the
        // PREVIOUS cycle), entry 2 ≤ 3 stays PicNum 2. Descending
        // PicNum puts 2 first: naive frame_num order would be
        // [21, 22] (15 before 2) — the wrap-aware PicNum demotes the
        // older-cycle entry to last. (r3 fix: the r2 draft probed
        // with current frame_num 1, under which BOTH entries wrap
        // (2 > 1 → −14) and the expected [22, 21] was unsatisfiable;
        // the scenario, not the code, was wrong — §8.2.3 reading.)
        let dpb = [dpb_pic(21, 15, 30), dpb_pic(22, 2, 4)];
        let list = build_ref_list(&dpb, 3, 16, &[], 32).unwrap_or_else(|e| unreachable!("{e:?}"));
        let surfaces: Vec<u32> = list.iter().map(|p| p.picture_id).collect();
        assert_eq!(surfaces, [22, 21]);
    }

    #[test]
    fn reorder_ops_reorder_and_reflag() {
        let dpb = [dpb_pic(31, 7, 14), dpb_pic(32, 5, 10), dpb_pic(33, 3, 6)];
        // idc 0: move the short-term picture with PicNum
        // (Curr=9) − 3 = 6… no entry; use diff 4 → PicNum 5
        // (abs_diff_pic_num_minus1 = 3).
        let ops = [RefPicListModOp::ShortTermByPicNum {
            abs_diff_pic_num_minus1: 3,
        }];
        let list = build_ref_list(&dpb, 9, 16, &ops, 32).unwrap_or_else(|e| unreachable!("{e:?}"));
        let surfaces: Vec<u32> = list.iter().map(|p| p.picture_id).collect();
        assert_eq!(surfaces, [32, 31, 33], "PicNum 5 moved to the front");
        // idc 1: same target, re-flagged long-term with
        // LongTermPicNum = picNumX (the wrapped form: 9+4=13 % 16).
        let ops = [RefPicListModOp::ShortTermToLongTermByPicNum {
            abs_diff_pic_num_minus1: 3,
        }];
        let list = build_ref_list(&dpb, 9, 16, &ops, 32).unwrap_or_else(|e| unreachable!("{e:?}"));
        assert_eq!(list[0].picture_id, 32);
        assert_eq!(
            list[0].flags & VA_PICTURE_H264_LONG_TERM_REFERENCE,
            VA_PICTURE_H264_LONG_TERM_REFERENCE
        );
        assert_eq!(list[0].frame_idx, 13);
        // idc 2: a long-term pick by LongTermPicNum.
        let dpb_long = [
            dpb_pic(31, 7, 14),
            DpbPicture {
                surface: 34,
                frame_num: 5,
                long_term: true,
                long_term_frame_idx: 1,
                poc: 10,
            },
        ];
        let ops = [RefPicListModOp::LongTermByPicNum {
            long_term_pic_num: 1,
        }];
        let list =
            build_ref_list(&dpb_long, 9, 16, &ops, 32).unwrap_or_else(|e| unreachable!("{e:?}"));
        assert_eq!(list[0].picture_id, 34);
    }

    #[test]
    fn wrap_encoded_reorder_target_resolves() {
        // CurrPicNum 0, MaxFrameNum 16: referencing PicNum 12 is
        // encoded as abs_diff_pic_num_minus1 = 27 (0+28 % 16 = 12);
        // the entry with frame_num 12 (previous cycle) matches.
        let dpb = [dpb_pic(41, 12, 24)];
        let ops = [RefPicListModOp::ShortTermByPicNum {
            abs_diff_pic_num_minus1: 27,
        }];
        let list = build_ref_list(&dpb, 0, 16, &ops, 32).unwrap_or_else(|e| unreachable!("{e:?}"));
        assert_eq!(list[0].picture_id, 41);
    }

    #[test]
    fn missing_target_and_mvc_ops_are_typed_errors() {
        let dpb = [dpb_pic(51, 7, 14)];
        let ops = [RefPicListModOp::ShortTermByPicNum {
            abs_diff_pic_num_minus1: 40,
        }];
        assert_eq!(
            build_ref_list(&dpb, 9, 16, &ops, 32)
                .err()
                .and_then(|e| match e {
                    RefListError::MissingTarget { pic_num } => Some(pic_num),
                    _ => None,
                }),
            Some(2)
        );
        let ops = [RefPicListModOp::ShortTermByViewIdx {
            abs_diff_view_idx_minus1: 0,
        }];
        assert!(matches!(
            build_ref_list(&dpb, 9, 16, &ops, 32),
            Err(RefListError::MvcViewOp)
        ));
    }

    #[test]
    fn coded_size_is_macroblock_math() {
        let mut sps = SeqParameterSet {
            profile_idc: 77,
            constraint_flags: 0,
            level_idc: 30,
            seq_parameter_set_id: 0,
            chroma_format_idc: Some(1),
            separate_colour_plane: false,
            bit_depth_luma_minus8: 0,
            bit_depth_chroma_minus8: 0,
            log2_max_frame_num_minus4: 0,
            pic_order_cnt_type: 2,
            log2_max_pic_order_cnt_lsb_minus4: None,
            poc_cycle_offsets: Vec::new(),
            max_num_ref_frames: 1,
            gaps_in_frame_num_allowed: false,
            pic_width_in_mbs_minus1: 3,
            pic_height_in_map_units_minus1: 2,
            frame_mbs_only: true,
            mb_adaptive_frame_field: false,
            direct_8x8_inference: true,
            crop: None,
            vui: None,
        };
        assert_eq!(coded_size(&sps), Some((64, 48)));
        // Frame/field coding doubles the map-unit height.
        sps.frame_mbs_only = false;
        assert_eq!(coded_size(&sps), Some((64, 96)));
        // Overflow is a typed None, never a wrap.
        sps.pic_width_in_mbs_minus1 = u32::MAX;
        assert_eq!(coded_size(&sps), None);
    }

    #[test]
    fn slice_parameter_carries_the_parsed_facts() {
        let pps = PicParameterSet {
            pic_parameter_set_id: 0,
            seq_parameter_set_id: 0,
            entropy_coding_mode: true,
            bottom_field_pic_order_in_frame_present: false,
            num_slice_groups_minus1: 0,
            num_ref_idx_l0_default_active_minus1: 0,
            num_ref_idx_l1_default_active_minus1: 0,
            weighted_pred: false,
            weighted_bipred_idc: 0,
            pic_init_qp_minus26: 0,
            pic_init_qs_minus26: 0,
            chroma_qp_index_offset: 0,
            deblocking_filter_control_present: true,
            constrained_intra_pred: false,
            redundant_pic_cnt_present: false,
        };
        let mut header = SliceHeader {
            first_mb_in_slice: 5,
            slice_type: SliceType {
                kind: SliceKind::P,
                all_slices_of_picture: false,
            },
            pic_parameter_set_id: 0,
            colour_plane_id: None,
            frame_num: 9,
            field_pic_flag: false,
            bottom_field_flag: None,
            idr_pic_id: None,
            pic_order: PicOrderSyntax::Type2,
            redundant_pic_cnt: None,
            direct_spatial_mv_pred_flag: None,
            num_ref_idx_override: None,
            ref_pic_list_modification: Default::default(),
            pred_weight_table: None,
            dec_ref_pic_marking: None,
            cabac_init_idc: Some(1),
            slice_qp_delta: -4,
            deblocking: Some(crate::video::h264::parser::slice::DeblockingFilterControl {
                disable_deblocking_filter_idc: 0,
                slice_alpha_c0_offset_div2: Some(2),
                slice_beta_offset_div2: Some(-1),
            }),
            header_bits: 33,
        };
        let sp = slice_parameter(&pps, &header, 1, 0x40, 0x1_000, Vec::new(), Vec::new());
        assert_eq!(sp.slice_data_size, 0x1_000);
        assert_eq!(sp.slice_data_offset, 0x40);
        assert_eq!(sp.slice_data_flag, ffi::VA_SLICE_DATA_FLAG_ALL);
        assert_eq!(sp.slice_data_bit_offset, 41, "8 header bits + 33 RBSP bits");
        assert_eq!(sp.first_mb_in_slice, 5);
        assert_eq!(sp.slice_type, 0, "folded P code");
        assert_eq!(sp.cabac_init_idc, 1);
        assert_eq!(sp.slice_qp_delta, -4);
        assert_eq!(sp.disable_deblocking_filter_idc, 0);
        assert_eq!(sp.slice_alpha_c0_offset_div2, 2);
        assert_eq!(sp.slice_beta_offset_div2, -1);
        assert_eq!(sp.num_ref_idx_l0_active_minus1, 0, "PPS default");
        // The override replaces the PPS default.
        header.num_ref_idx_override = Some((3, None));
        let sp = slice_parameter(&pps, &header, 1, 0, 1, Vec::new(), Vec::new());
        assert_eq!(sp.num_ref_idx_l0_active_minus1, 3);
    }

    #[test]
    fn picture_parameter_packs_the_bitfield_words() {
        let sps = SeqParameterSet {
            profile_idc: 77,
            constraint_flags: 0,
            level_idc: 30,
            seq_parameter_set_id: 0,
            chroma_format_idc: Some(1),
            separate_colour_plane: false,
            bit_depth_luma_minus8: 0,
            bit_depth_chroma_minus8: 0,
            log2_max_frame_num_minus4: 1,
            pic_order_cnt_type: 2,
            log2_max_pic_order_cnt_lsb_minus4: None,
            poc_cycle_offsets: Vec::new(),
            max_num_ref_frames: 2,
            gaps_in_frame_num_allowed: false,
            pic_width_in_mbs_minus1: 3,
            pic_height_in_map_units_minus1: 2,
            frame_mbs_only: true,
            mb_adaptive_frame_field: false,
            direct_8x8_inference: true,
            crop: None,
            vui: None,
        };
        let pps = PicParameterSet {
            pic_parameter_set_id: 0,
            seq_parameter_set_id: 0,
            entropy_coding_mode: true,
            bottom_field_pic_order_in_frame_present: false,
            num_slice_groups_minus1: 0,
            num_ref_idx_l0_default_active_minus1: 0,
            num_ref_idx_l1_default_active_minus1: 0,
            weighted_pred: false,
            weighted_bipred_idc: 0,
            pic_init_qp_minus26: -1,
            pic_init_qs_minus26: 0,
            chroma_qp_index_offset: 2,
            deblocking_filter_control_present: true,
            constrained_intra_pred: false,
            redundant_pic_cnt_present: false,
        };
        let header = SliceHeader {
            first_mb_in_slice: 0,
            slice_type: SliceType {
                kind: SliceKind::I,
                all_slices_of_picture: false,
            },
            pic_parameter_set_id: 0,
            colour_plane_id: None,
            frame_num: 7,
            field_pic_flag: false,
            bottom_field_flag: None,
            idr_pic_id: None,
            pic_order: PicOrderSyntax::Type2,
            redundant_pic_cnt: None,
            direct_spatial_mv_pred_flag: None,
            num_ref_idx_override: None,
            ref_pic_list_modification: Default::default(),
            pred_weight_table: None,
            dec_ref_pic_marking: None,
            cabac_init_idc: None,
            slice_qp_delta: 0,
            deblocking: None,
            header_bits: 0,
        };
        let curr = VAPictureH264 {
            picture_id: 99,
            frame_idx: 7,
            flags: VA_PICTURE_H264_SHORT_TERM_REFERENCE,
            top_field_order_cnt: 14,
            bottom_field_order_cnt: 14,
            va_reserved: [0; 4],
        };
        let dpb = [dpb_pic(77, 5, 10)];
        let pp = picture_parameter(&sps, &pps, &header, curr, &dpb);
        assert_eq!(pp.curr_pic.picture_id, 99);
        assert_eq!(pp.frame_num, 7);
        assert_eq!(pp.num_ref_frames, 2);
        assert_eq!(pp.picture_width_in_mbs_minus1, 3);
        assert_eq!(pp.picture_height_in_mbs_minus1, 2);
        // seq_fields: chroma 1 | frame_mbs_only (bit 4) |
        // direct_8x8 (bit 6) | log2_max_frame_num_minus4=1 (bits 8..12)
        // | pic_order_cnt_type=2 (bits 12..14) — the r2 golden
        // missed the type word the fixture SPS carries.
        assert_eq!(pp.seq_fields, 0x1 | 0x10 | 0x40 | (1 << 8) | (2 << 12));
        // pic_fields: cabac (bit 0) | deblock present (bit 8)
        assert_eq!(pp.pic_fields, 0x1 | 0x100);
        assert_eq!(pp.reference_frames[0].picture_id, 77);
        assert_eq!(pp.reference_frames[1].flags, ffi::VA_PICTURE_H264_INVALID);
        assert_eq!(pp.pic_init_qp_minus26, -1);
        assert_eq!(pp.chroma_qp_index_offset, 2);
        assert_eq!(
            pp.second_chroma_qp_index_offset, 2,
            "see ffi.rs honesty note"
        );
        // Every va_reserved word is zero (the ABI's hard rule).
        assert_eq!(pp.va_reserved, [0; 8]);
    }

    #[test]
    fn probe_reports_no_device_node_on_a_dri_less_root() {
        // The sandbox reality as a fixture: no /dev/dri at all.
        let root = std::env::temp_dir()
            .join("lumen-codecs-hw-vaapi-probe-tests")
            .join(format!("none-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap_or_else(|e| unreachable!("{e}"));
        let (availability, minted) = probe_with(&DriScanner::with_root(&root));
        assert_eq!(
            availability,
            HwAvailability::Unavailable {
                backend: HwBackend::VaApi,
                reason: HwUnavailableReason::NoDeviceNode,
            }
        );
        assert!(
            minted.is_none(),
            "no accelerator is minted off a failed probe"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn probe_reports_permission_denied_on_a_locked_node() {
        if euid_is_root() {
            return; // chmod-000 denial is unprivileged-only (CI note)
        }
        let root = std::env::temp_dir()
            .join("lumen-codecs-hw-vaapi-probe-tests")
            .join(format!("perm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dri = root.join("dev/dri");
        std::fs::create_dir_all(&dri).unwrap_or_else(|e| unreachable!("{e}"));
        std::fs::write(dri.join("renderD128"), b"").unwrap_or_else(|e| unreachable!("{e}"));
        std::fs::set_permissions(
            dri.join("renderD128"),
            std::fs::Permissions::from_mode(0o000),
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        let (availability, minted) = probe_with(&DriScanner::with_root(&root));
        assert_eq!(
            availability,
            HwAvailability::Unavailable {
                backend: HwBackend::VaApi,
                reason: HwUnavailableReason::PermissionDenied,
            }
        );
        assert!(
            minted.is_none(),
            "no accelerator is minted off a failed probe"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn probe_on_the_real_host_is_the_typed_sandbox_outcome() {
        // This host has no /dev/dri (the sandbox reality): the
        // probe's honest answer is the typed NoDeviceNode. On a
        // host WITH a DRI node the probe proceeds to libva —
        // any well-formed outcome is acceptable there (this
        // asserts the no-hardware truth where it is the truth,
        // and shape-only elsewhere).
        let (availability, minted) = probe();
        match availability {
            HwAvailability::Unavailable { reason, .. } => {
                if !std::path::Path::new("/dev/dri").exists() {
                    assert_eq!(reason, HwUnavailableReason::NoDeviceNode);
                }
            }
            HwAvailability::Available { detail, .. } => {
                assert!(!detail.is_empty(), "Available carries the vendor detail");
                assert!(minted.is_some(), "Available mints the accelerator facts");
            }
        }
    }

    #[test]
    fn session_configure_without_parameter_sets_is_typed() {
        let mut session = VaApiSession::with_root(
            DriNode {
                name: "renderD128".into(),
                render: true,
            },
            &std::env::temp_dir(),
        );
        // No /dev/dri under the temp root: even bring-up fails
        // typed (the honest first-failure; the parameter-set check
        // sits behind it). Both codes are typed refusals — the
        // contract under test.
        match <VaApiSession as H264HwDecoder>::configure(&mut session, &H264HwStreamInfo::default())
        {
            Err(H264HwError::Configure { code }) => {
                assert!(
                    code.starts_with("h264.hw.vaapi."),
                    "stable dotted code, got {code}"
                );
            }
            other => unreachable!("must be a typed configure refusal, got {other:?}"),
        }
    }

    #[test]
    fn va_api_lane_backend_label_is_stable() {
        let session = VaApiSession::new(DriNode {
            name: "renderD128".into(),
            render: true,
        });
        assert_eq!(H264HwDecoder::backend_id(&session), "hw.vaapi");
    }

    /// std-only euid check (skip the chmod-000 branch under root).
    fn euid_is_root() -> bool {
        std::fs::read_to_string("/proc/self/status")
            .map(|s| {
                s.lines().any(|line| {
                    let mut fields = line.split_whitespace();
                    fields.next() == Some("Uid:") && fields.next() == Some("0")
                })
            })
            .unwrap_or(false)
    }
}
