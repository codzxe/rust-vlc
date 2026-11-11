//! Sequence parameter set (ITU-T H.264 §7.3.2.1.1) — clean-room
//! parse of the fields the card names: profile/level identity,
//! chroma/colour-plane/bit-depth geometry, frame-number and
//! picture-order-count parameters, reference-frame budget, coded and
//! cropped picture sizes, and the VUI basics (aspect ratio, video
//! signal, timing). Scaling lists and HRD structures are *skipped
//! correctly* (bit-accurate walk) but not retained — nothing here
//! decodes.
use super::nal::{BitReader, H264Error, NalHeader, NalUnitType, unescape_rbsp_into};

/// Chroma sampling as signalled by `chroma_format_idc` (§7.4.2.1.1,
/// incl. the 4:2:0 default when the profile does not signal one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaFormat {
    /// `chroma_format_idc` 0 — monochrome.
    Monochrome,
    /// 1 — 4:2:0 (also the implicit default).
    Yuv420,
    /// 2 — 4:2:2.
    Yuv422,
    /// 3 — 4:4:4.
    Yuv444,
}

impl ChromaFormat {
    /// From a raw `chroma_format_idc` (0..=3).
    #[must_use]
    pub fn from_idc(idc: u8) -> Option<ChromaFormat> {
        match idc {
            0 => Some(ChromaFormat::Monochrome),
            1 => Some(ChromaFormat::Yuv420),
            2 => Some(ChromaFormat::Yuv422),
            3 => Some(ChromaFormat::Yuv444),
            _ => None,
        }
    }

    /// `SubWidthC`/`SubHeightC` (Table 6-1); `None` for monochrome
    /// (no chroma plane exists).
    #[must_use]
    pub fn sub_size(self) -> Option<(u32, u32)> {
        match self {
            ChromaFormat::Monochrome => None,
            ChromaFormat::Yuv420 => Some((2, 2)),
            ChromaFormat::Yuv422 => Some((2, 1)),
            ChromaFormat::Yuv444 => Some((1, 1)),
        }
    }

    /// ffprobe's `pix_fmt` spelling for 8-bit streams (the corpus
    /// assertion's comparison surface).
    #[must_use]
    pub fn pix_fmt_name(self, full_range: bool, bit_depth: u8) -> String {
        let depth = if bit_depth > 8 {
            format!("{bit_depth}le")
        } else if full_range {
            "j".to_string()
        } else {
            String::new()
        };
        match self {
            ChromaFormat::Monochrome => format!("gray{depth}"),
            ChromaFormat::Yuv420 => format!("yuv{depth}420p"),
            ChromaFormat::Yuv422 => format!("yuv{depth}422p"),
            ChromaFormat::Yuv444 => format!("yuv{depth}444p"),
        }
    }
}

/// VUI parameters (§7.3.2.1.1 `vui_parameters`) — the basics the card
/// names: aspect ratio, video signal / colour description, timing.
/// HRD and bitstream-restriction structures are walked correctly and
/// summarised as flags, not retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VuiParameters {
    /// `aspect_ratio_idc` when `aspect_ratio_info_present_flag` is
    /// set (255 = Extended_SAR, see `sar`).
    pub aspect_ratio_idc: Option<u8>,
    /// The sample aspect ratio — either the `aspect_ratio_idc` table
    /// value (when a known idc) or the explicit `sar_width/height`.
    pub sar: Option<(u16, u16)>,
    /// `video_format` (3 bits) when `video_signal_type_present_flag`.
    pub video_format: Option<u8>,
    /// `video_full_range_flag` when video signal is present.
    pub video_full_range: bool,
    /// (`colour_primaries`, `transfer_characteristics`,
    /// `matrix_coefficients`) when `colour_description_present_flag`.
    pub colour_description: Option<(u8, u8, u8)>,
    /// (`num_units_in_tick`, `time_scale`, `fixed_frame_rate_flag`)
    /// when `timing_info_present_flag`.
    pub timing: Option<(u32, u32, bool)>,
    /// Either HRD structure was signalled.
    pub hrd_present: bool,
    /// `pic_struct_present_flag`.
    pub pic_struct_present: bool,
    /// `max_dec_frame_buffering` when bitstream restrictions are
    /// signalled with a sane value.
    pub max_dec_frame_buffering: Option<u32>,
}

/// Aspect-ratio idc table (Annex E, Table E-1) for the idcs the
/// corpus carries; unlisted idcs resolve to `None` (the caller falls
/// back to the explicit Extended_SAR or reports unknown — never a
/// guess).
const ASPECT_TABLE: [(u8, u16, u16); 4] = [(1, 1, 1), (2, 12, 11), (3, 10, 11), (4, 16, 11)];

/// A parsed sequence parameter set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeqParameterSet {
    /// `profile_idc`.
    pub profile_idc: u8,
    /// The constraint-set flag byte (`constraint_set0..5` in bits
    /// 7..2, two reserved bits below).
    pub constraint_flags: u8,
    /// `level_idc` (e.g. 13 → level 1.3; ffprobe reports it as `13`).
    pub level_idc: u8,
    /// `seq_parameter_set_id` (0..=31).
    pub seq_parameter_set_id: u8,
    /// `chroma_format_idc` — `None` when the profile did not signal
    /// it (implicit 4:2:0).
    pub chroma_format_idc: Option<u8>,
    /// `separate_colour_plane_flag` (only parseable when idc == 3).
    pub separate_colour_plane: bool,
    /// `bit_depth_luma_minus8` (0 when not signalled).
    pub bit_depth_luma_minus8: u8,
    /// `bit_depth_chroma_minus8` (0 when not signalled).
    pub bit_depth_chroma_minus8: u8,
    /// `log2_max_frame_num_minus4` (0..=12).
    pub log2_max_frame_num_minus4: u8,
    /// `pic_order_cnt_type` (0..=2).
    pub pic_order_cnt_type: u8,
    /// `log2_max_pic_order_cnt_lsb_minus4` (type 0 only).
    pub log2_max_pic_order_cnt_lsb_minus4: Option<u8>,
    /// Type-1 POC cycle offsets (`offset_for_ref_frame` list).
    pub poc_cycle_offsets: Vec<i32>,
    /// `max_num_ref_frames` (0..=16).
    pub max_num_ref_frames: u8,
    /// `gaps_in_frame_num_value_allowed_flag`.
    pub gaps_in_frame_num_allowed: bool,
    /// `pic_width_in_mbs_minus1`.
    pub pic_width_in_mbs_minus1: u32,
    /// `pic_height_in_map_units_minus1`.
    pub pic_height_in_map_units_minus1: u32,
    /// `frame_mbs_only_flag`.
    pub frame_mbs_only: bool,
    /// `mb_adaptive_frame_field_flag` (interlaced-only).
    pub mb_adaptive_frame_field: bool,
    /// `direct_8x8_inference_flag`.
    pub direct_8x8_inference: bool,
    /// `frame_crop_{left,right,top,bottom}_offset`.
    pub crop: Option<(u32, u32, u32, u32)>,
    /// The VUI parameters, when signalled.
    pub vui: Option<VuiParameters>,
}

impl SeqParameterSet {
    /// Parse a complete SPS NAL unit (Annex-B or AVC-framed; header
    /// byte included). Allocates the RBSP copy — steady-state callers
    /// use [`SeqParameterSet::parse_with_buffer`].
    ///
    /// # Errors
    /// Typed [`H264Error`]s: wrong NAL type, forbidden bit, emulation
    /// violations, truncation mid-syntax, out-of-range ue/se fields,
    /// crop/dimension arithmetic overflow, bad trailing bits.
    pub fn parse(nal: &[u8]) -> Result<SeqParameterSet, H264Error> {
        let mut rbsp = Vec::with_capacity(nal.len());
        Self::parse_with_buffer(nal, &mut rbsp)
    }

    /// [`SeqParameterSet::parse`] reusing a caller-owned RBSP buffer
    /// (cleared per call) — the zero-allocation steady state.
    ///
    /// # Errors
    /// As [`SeqParameterSet::parse`].
    pub fn parse_with_buffer(nal: &[u8], rbsp: &mut Vec<u8>) -> Result<SeqParameterSet, H264Error> {
        let (header, payload) = NalHeader::split(nal)?;
        if header.unit_type != NalUnitType::Sps {
            return Err(H264Error::Invalid {
                code: "h264.sps.wrong_nal_type",
            });
        }
        unescape_rbsp_into(payload, rbsp)?;
        let mut r = BitReader::new(rbsp);
        let sps = Self::read(&mut r)?;
        r.finish()?;
        Ok(sps)
    }

    /// The syntax walk (§7.3.2.1.1) over an RBSP bit reader.
    fn read(r: &mut BitReader<'_>) -> Result<SeqParameterSet, H264Error> {
        let profile_idc = r.bits(8)? as u8;
        let constraint_flags = r.bits(8)? as u8;
        let level_idc = r.bits(8)? as u8;

        let ue_limited =
            |r: &mut BitReader<'_>, field: &'static str, max: u64| -> Result<u64, H264Error> {
                let v = r.ue()?;
                if v > max {
                    return Err(H264Error::OutOfRange {
                        field,
                        value: v,
                        max,
                    });
                }
                Ok(v)
            };

        let seq_parameter_set_id = ue_limited(r, "seq_parameter_set_id", 31)? as u8;

        let mut chroma_format_idc = None;
        let mut separate_colour_plane = false;
        let mut bit_depth_luma_minus8 = 0u8;
        let mut bit_depth_chroma_minus8 = 0u8;
        if matches!(
            profile_idc,
            100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135
        ) {
            let idc = ue_limited(r, "chroma_format_idc", 3)? as u8;
            if idc == 3 {
                separate_colour_plane = r.bit()? == 1;
            }
            bit_depth_luma_minus8 = ue_limited(r, "bit_depth_luma_minus8", 6)? as u8;
            bit_depth_chroma_minus8 = ue_limited(r, "bit_depth_chroma_minus8", 6)? as u8;
            let _qpprime = r.bit()?; // qpprime_y_zero_transform_bypass_flag
            if r.bit()? == 1 {
                // seq_scaling_matrix_present: walk (not retain) the
                // scaling lists — 4:4:4 signals 12, everything else 8.
                let count = if chroma_or_idc(idc, separate_colour_plane) == 3 {
                    12
                } else {
                    8
                };
                for i in 0..count {
                    if r.bit()? == 1 {
                        skip_scaling_list(r, if i < 6 { 16 } else { 64 })?;
                    }
                }
            }
            chroma_format_idc = Some(idc);
        }

        let log2_max_frame_num_minus4 = ue_limited(r, "log2_max_frame_num_minus4", 12)? as u8;
        let pic_order_cnt_type = ue_limited(r, "pic_order_cnt_type", 2)? as u8;
        let mut log2_max_pic_order_cnt_lsb_minus4 = None;
        let mut poc_cycle_offsets = Vec::new();
        match pic_order_cnt_type {
            0 => {
                log2_max_pic_order_cnt_lsb_minus4 =
                    Some(ue_limited(r, "log2_max_pic_order_cnt_lsb_minus4", 12)? as u8);
            }
            1 => {
                let _always_zero = r.bit()?; // delta_pic_order_always_zero_flag
                let _non_ref = r.se()?;
                let _top_to_bottom = r.se()?;
                let cycles = ue_limited(r, "num_ref_frames_in_pic_order_cnt_cycle", 256)?;
                poc_cycle_offsets.reserve(cycles as usize);
                for _ in 0..cycles {
                    let off = r.se()?;
                    if off < i32::MIN as i64 || off > i32::MAX as i64 {
                        return Err(H264Error::OutOfRange {
                            field: "offset_for_ref_frame",
                            value: off.unsigned_abs(),
                            max: i32::MAX as u64,
                        });
                    }
                    poc_cycle_offsets.push(off as i32);
                }
            }
            _ => {}
        }

        let max_num_ref_frames = ue_limited(r, "max_num_ref_frames", 16)? as u8;
        let gaps_in_frame_num_allowed = r.bit()? == 1;
        let pic_width_in_mbs_minus1 = r.ue()? as u32;
        let pic_height_in_map_units_minus1 = r.ue()? as u32;
        if u64::from(pic_width_in_mbs_minus1) > 65_535
            || u64::from(pic_height_in_map_units_minus1) > 65_535
        {
            // 65536+ macroblocks per axis exceeds every spec level's
            // frame size bound by orders of magnitude — corruption.
            return Err(H264Error::OutOfRange {
                field: "pic_size_in_mbs",
                value: u64::from(pic_width_in_mbs_minus1.max(pic_height_in_map_units_minus1)),
                max: 65_535,
            });
        }
        let frame_mbs_only = r.bit()? == 1;
        let mb_adaptive_frame_field = if frame_mbs_only { false } else { r.bit()? == 1 };
        let direct_8x8_inference = r.bit()? == 1;
        let crop = if r.bit()? == 1 {
            Some((
                r.ue()? as u32,
                r.ue()? as u32,
                r.ue()? as u32,
                r.ue()? as u32,
            ))
        } else {
            None
        };
        let vui = if r.bit()? == 1 {
            Some(read_vui(r)?)
        } else {
            None
        };

        Ok(SeqParameterSet {
            profile_idc,
            constraint_flags,
            level_idc,
            seq_parameter_set_id,
            chroma_format_idc,
            separate_colour_plane,
            bit_depth_luma_minus8,
            bit_depth_chroma_minus8,
            log2_max_frame_num_minus4,
            pic_order_cnt_type,
            log2_max_pic_order_cnt_lsb_minus4,
            poc_cycle_offsets,
            max_num_ref_frames,
            gaps_in_frame_num_allowed,
            pic_width_in_mbs_minus1,
            pic_height_in_map_units_minus1,
            frame_mbs_only,
            mb_adaptive_frame_field,
            direct_8x8_inference,
            crop,
            vui,
        })
    }

    /// The effective chroma format (`separate_colour_plane` forces
    /// `ChromaArrayType == 0`; an unsignalled idc is the 4:2:0
    /// default).
    #[must_use]
    pub fn chroma_format(&self) -> ChromaFormat {
        let idc = if self.separate_colour_plane {
            0
        } else {
            self.chroma_format_idc.unwrap_or(1)
        };
        ChromaFormat::from_idc(idc).unwrap_or(ChromaFormat::Yuv420)
    }

    /// The frame-height interlace factor: 1 (progressive,
    /// `frame_mbs_only`) or 2 (field/adaptive pictures).
    fn height_factor(&self) -> u32 {
        u32::from(!self.frame_mbs_only) + 1
    }

    /// Coded size in luma pixels (`PicWidthInMbs·16` ×
    /// `FrameHeightInMbs·16`), checked arithmetic.
    #[must_use]
    pub fn coded_size(&self) -> Option<(u32, u32)> {
        let w = (u64::from(self.pic_width_in_mbs_minus1) + 1)
            .checked_mul(16)?
            .min(u64::from(u32::MAX)) as u32;
        let h = (u64::from(self.pic_height_in_map_units_minus1) + 1)
            .checked_mul(16)?
            .checked_mul(u64::from(self.height_factor()))?
            .min(u64::from(u32::MAX)) as u32;
        Some((w, h))
    }

    /// `MaxFrameNum = 2^(log2_max_frame_num_minus4 + 4)` (§7.4.3) —
    /// the `frame_num` wrap modulus. The parser bounds the exponent
    /// to 12, so the shift is ≤ 16 and the value ≤ 65536 (always in
    /// `u32` range for a parsed SPS).
    ///
    /// 7.B1 wiring (task 7.w23a): the VA-API DPB/POC reconstruction
    /// (`hw/vaapi/session.rs`) consumes this — pure derivation, kept
    /// beside the field it derives from.
    #[must_use = "pure derivation"]
    pub fn max_frame_num(&self) -> u32 {
        1u32 << (u32::from(self.log2_max_frame_num_minus4) + 4)
    }

    /// `MaxPicOrderCntLsb = 2^(log2_max_pic_order_cnt_lsb_minus4 + 4)`
    /// (§7.4.3) — the POC-LSB wrap modulus for `pic_order_cnt_type ==
    /// 0` streams. Non-type-0 SPSs carry no exponent (`None` → the
    /// minimum modulus 16, unused by those streams' POC arithmetic).
    #[must_use = "pure derivation"]
    pub fn max_pic_order_cnt_lsb(&self) -> u32 {
        1u32 << (u32::from(self.log2_max_pic_order_cnt_lsb_minus4.unwrap_or(0)) + 4)
    }

    /// Visible (post-cropping) size — the width/height a container
    /// and ffprobe report. `None` when crop offsets exceed the coded
    /// size or arithmetic overflows (corruption, not a feature).
    #[must_use]
    pub fn display_size(&self) -> Option<(u32, u32)> {
        let (w, h) = self.coded_size()?;
        let (mut crop_x, mut crop_y) = match self.chroma_format().sub_size() {
            // ChromaArrayType == 0: crop units are luma samples with
            // the interlace factor on Y only (§7.4.2.1.1).
            None => (1, self.height_factor()),
            Some((sub_w, sub_h)) => (sub_w, sub_h * self.height_factor()),
        };
        if self.separate_colour_plane {
            crop_x = 1;
            crop_y = self.height_factor();
        }
        let (l, rr, t, b) = self.crop.unwrap_or((0, 0, 0, 0));
        let dx = crop_x.checked_mul(l.checked_add(rr)?)?;
        let dy = crop_y.checked_mul(t.checked_add(b)?)?;
        if u64::from(dx) > u64::from(w) || u64::from(dy) > u64::from(h) {
            return None;
        }
        Some((w - dx, h - dy))
    }

    /// The signalled sample aspect ratio (idc table or Extended_SAR).
    #[must_use]
    pub fn aspect_ratio(&self) -> Option<(u16, u16)> {
        let vui = self.vui.as_ref()?;
        if let Some(sar) = vui.sar {
            return Some(sar);
        }
        let idc = vui.aspect_ratio_idc?;
        ASPECT_TABLE
            .iter()
            .find(|&&(i, _, _)| i == idc)
            .map(|&(_, w, h)| (w, h))
    }

    /// ffprobe's `profile` spelling for the parsed identity
    /// (constraint-set flags disambiguate Constrained/Intra
    /// variants); unknown ids report the raw number.
    #[must_use]
    pub fn profile_name(&self) -> String {
        let cs1 = self.constraint_flags & 0x40 != 0;
        let cs3 = self.constraint_flags & 0x10 != 0;
        match self.profile_idc {
            66 => if cs1 {
                "Constrained Baseline"
            } else {
                "Baseline"
            }
            .to_string(),
            77 => "Main".to_string(),
            88 => "Extended".to_string(),
            100 => "High".to_string(),
            110 => if cs3 { "High 10 Intra" } else { "High 10" }.to_string(),
            122 => if cs3 {
                "High 4:2:2 Intra"
            } else {
                "High 4:2:2"
            }
            .to_string(),
            244 => if cs3 {
                "High 4:4:4 Intra"
            } else {
                "High 4:4:4 Predictive"
            }
            .to_string(),
            44 => "CAVLC 4:4:4".to_string(),
            83 => "Scalable Baseline".to_string(),
            86 => "Scalable High".to_string(),
            118 => "Stereo High".to_string(),
            128 => "Multiview High".to_string(),
            other => format!("{other}"),
        }
    }
}

/// `ChromaArrayType` for the scaling-list count decision.
fn chroma_or_idc(idc: u8, separate: bool) -> u8 {
    if separate { 0 } else { idc }
}

/// Walk one `scaling_list` (§7.3.2.1.1.1) without retaining it.
fn skip_scaling_list(r: &mut BitReader<'_>, size: u8) -> Result<(), H264Error> {
    let mut last_scale: i64 = 8;
    let mut next_scale: i64 = 8;
    for _ in 0..size {
        if next_scale != 0 {
            let delta = r.se()?;
            next_scale = (last_scale + delta).rem_euclid(256);
            if next_scale != 0 {
                last_scale = next_scale;
            }
        }
    }
    Ok(())
}

/// Walk the `vui_parameters` syntax (§7.3.2.1.1), retaining the card's
/// basics and correctly skipping HRD/restriction internals.
fn read_vui(r: &mut BitReader<'_>) -> Result<VuiParameters, H264Error> {
    let mut aspect_ratio_idc = None;
    let mut sar = None;
    if r.bit()? == 1 {
        let idc = r.bits(8)? as u8;
        aspect_ratio_idc = Some(idc);
        if idc == 255 {
            let w = r.bits(16)? as u16;
            let h = r.bits(16)? as u16;
            if w != 0 && h != 0 {
                sar = Some((w, h));
            }
        } else if let Some(&(_, w, h)) = ASPECT_TABLE.iter().find(|&&(i, _, _)| i == idc) {
            sar = Some((w, h));
        }
    }
    if r.bit()? == 1 {
        let _overscan_appropriate = r.bit()?;
    }
    let mut video_format = None;
    let mut video_full_range = false;
    let mut colour_description = None;
    if r.bit()? == 1 {
        video_format = Some(r.bits(3)? as u8);
        video_full_range = r.bit()? == 1;
        if r.bit()? == 1 {
            colour_description = Some((r.bits(8)? as u8, r.bits(8)? as u8, r.bits(8)? as u8));
        }
    }
    if r.bit()? == 1 {
        let _top = r.ue()?;
        let _bottom = r.ue()?; // chroma_sample_loc_type_top/bottom
    }
    let mut timing = None;
    if r.bit()? == 1 {
        let num_units_in_tick = r.bits(32)?;
        let time_scale = r.bits(32)?;
        let fixed = r.bit()? == 1;
        if num_units_in_tick != 0 && time_scale != 0 {
            timing = Some((num_units_in_tick, time_scale, fixed));
        }
    }
    let mut hrd_present = false;
    if r.bit()? == 1 {
        skip_hrd(r)?;
        hrd_present = true;
    }
    if r.bit()? == 1 {
        skip_hrd(r)?;
        hrd_present = true;
    }
    if hrd_present {
        let _low_delay = r.bit()?;
    }
    let pic_struct_present = r.bit()? == 1;
    let mut max_dec_frame_buffering = None;
    if r.bit()? == 1 {
        let _mvs_over_bounds = r.bit()?;
        let _max_bytes = r.ue()?;
        let _max_bits = r.ue()?;
        let _log2_max_mv_h = r.ue()?;
        let _log2_max_mv_v = r.ue()?;
        let _max_reorder = r.ue()?;
        let buffering = r.ue()?;
        if buffering <= 16 {
            max_dec_frame_buffering = Some(buffering as u32);
        }
    }
    Ok(VuiParameters {
        aspect_ratio_idc,
        sar,
        video_format,
        video_full_range,
        colour_description,
        timing,
        hrd_present,
        pic_struct_present,
        max_dec_frame_buffering,
    })
}

/// Walk one `hrd_parameters` structure (sub-clause of VUI): the
/// CPB scheduling table. Skipped bit-accurately, never retained.
fn skip_hrd(r: &mut BitReader<'_>) -> Result<(), H264Error> {
    let cpb_count = r.ue()?;
    if cpb_count > 31 {
        return Err(H264Error::OutOfRange {
            field: "cpb_cnt_minus1",
            value: cpb_count,
            max: 31,
        });
    }
    let _bit_rate_scale = r.bits(4)?;
    let _cpb_size_scale = r.bits(4)?;
    for _ in 0..=cpb_count {
        let _bit_rate_value = r.ue()?;
        let _cpb_size_value = r.ue()?;
        let _cbr = r.bit()?;
    }
    let _initial_removal_len = r.bits(5)?;
    let _cpb_removal_len = r.bits(5)?;
    let _dpb_output_len = r.bits(5)?;
    let _time_offset_len = r.bits(5)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::h264::parser::nal::AvcNals;

    /// A real x264-encoded SPS/PPS pair (extracted from the corpus's
    /// 0019-raw-video.h264: High@1.3, 64×48, 4:2:0) — hand-verified
    /// against the raw dump `00 00 00 01 67 64 00 0d f3 65 11 ec 05
    /// b2 00 00 03 00 02 00 00 03 00 28 1e 28 53 2c 80`-shaped bytes.
    const REAL_SPS: &[u8] = &[
        0x67, 0x64, 0x00, 0x0d, 0xf3, 0x65, 0x11, 0xec, 0x05, 0xb2, 0x00, 0x00, 0x03, 0x00, 0x02,
        0x00, 0x00, 0x03, 0x00, 0x28, 0x1e, 0x28, 0x53, 0x2c,
    ];
    const REAL_PPS: &[u8] = &[0x68, 0xce, 0x3c, 0x80];

    #[test]
    fn real_corpus_sps_parses_field_exact() {
        // Field values verified against ffmpeg's `trace_headers` on
        // the same file (the bit-exact oracle for this fixture).
        let sps = SeqParameterSet::parse(REAL_SPS).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(sps.profile_idc, 100);
        assert_eq!(sps.level_idc, 13);
        assert_eq!(sps.profile_name(), "High");
        assert_eq!(sps.seq_parameter_set_id, 0);
        // A gray High-profile encode: chroma_format_idc 0 (4:0:0),
        // verified against trace_headers. ffmpeg's default builds
        // render gray H.264 as 4:2:0 with neutral chroma, which is
        // why ffprobe calls this stream `yuvj420p` (see the corpus
        // test's `ffprobe_visible_pix_fmt`).
        assert_eq!(sps.chroma_format_idc, Some(0));
        assert_eq!(sps.chroma_format(), ChromaFormat::Monochrome);
        assert_eq!(
            sps.chroma_format()
                .pix_fmt_name(sps.vui.as_ref().is_some_and(|v| v.video_full_range), 8),
            "grayj"
        );
        assert!(!sps.separate_colour_plane);
        assert_eq!(sps.bit_depth_luma_minus8, 0);
        assert_eq!(sps.bit_depth_chroma_minus8, 0);
        assert_eq!(sps.log2_max_frame_num_minus4, 0);
        assert_eq!(sps.pic_order_cnt_type, 0);
        assert_eq!(sps.log2_max_pic_order_cnt_lsb_minus4, Some(2));
        assert_eq!(sps.max_num_ref_frames, 4);
        assert!(!sps.gaps_in_frame_num_allowed);
        assert_eq!(sps.pic_width_in_mbs_minus1, 3); // 4 mbs → 64 px
        assert_eq!(sps.pic_height_in_map_units_minus1, 2); // 48 px
        assert!(sps.frame_mbs_only);
        assert!(sps.direct_8x8_inference);
        assert_eq!(sps.crop, None);
        assert_eq!(sps.coded_size(), Some((64, 48)));
        assert_eq!(sps.display_size(), Some((64, 48)));
        assert_eq!(sps.chroma_format(), ChromaFormat::Monochrome);
        assert_eq!(sps.aspect_ratio(), Some((1, 1)));
        // x264's VUI: full-range flag (ffprobe reports yuvj420p) and
        // 20 fps timing (num_units_in_tick 1, time_scale 20).
        let vui = sps.vui.as_ref().unwrap_or_else(|| unreachable!("vui"));
        assert!(vui.video_full_range);
        assert_eq!(vui.video_format, Some(5)); // unspecified
        assert_eq!(vui.timing, Some((1, 20, false)));
        assert_eq!(vui.max_dec_frame_buffering, Some(4));
        // Our honest pix_fmt for 4:0:0 + full range is `grayj`;
        // ffprobe's `yuvj420p` for the same bytes is ffmpeg's
        // neutral-chroma rendering view (documented in the corpus
        // test's `ffprobe_visible_pix_fmt`).
        assert_eq!(
            sps.chroma_format().pix_fmt_name(vui.video_full_range, 8),
            "grayj"
        );
    }

    #[test]
    fn real_corpus_pps_pairs_with_the_sps() {
        // This tiny encode runs CAVLC even inside High profile
        // (entropy_coding_mode_flag 0 — verified vs trace_headers;
        // High permits CAVLC, the decoder-side requirement is only
        // that CABAC be *supported*).
        let pps = super::super::pps::PicParameterSet::parse(REAL_PPS)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(pps.pic_parameter_set_id, 0);
        assert_eq!(pps.seq_parameter_set_id, 0);
        assert!(!pps.entropy_coding_mode); // CAVLC
        assert!(!pps.bottom_field_pic_order_in_frame_present);
        assert_eq!(pps.num_slice_groups_minus1, 0);
        assert_eq!(pps.num_ref_idx_l0_default_active_minus1, 0);
        assert_eq!(pps.num_ref_idx_l1_default_active_minus1, 0);
        assert!(!pps.weighted_pred);
        assert_eq!(pps.weighted_bipred_idc, 0);
        assert_eq!(pps.pic_init_qp_minus26, 0);
        assert_eq!(pps.pic_init_qs_minus26, 0);
        assert_eq!(pps.chroma_qp_index_offset, 0);
        assert!(pps.deblocking_filter_control_present);
        assert!(!pps.constrained_intra_pred);
        assert!(!pps.redundant_pic_cnt_present);
    }

    #[test]
    fn truncation_forbidden_and_wrong_type_are_typed() {
        // Every prefix-truncation of the real SPS fails typed (the
        // RBSP runs out mid-syntax or loses its stop bit).
        for cut in 1..REAL_SPS.len() {
            assert!(
                SeqParameterSet::parse(&REAL_SPS[..cut]).is_err(),
                "cut at {cut} must not parse cleanly"
            );
        }
        let mut forbidden = REAL_SPS.to_vec();
        forbidden[0] = 0xe7; // forbidden_zero_bit set on an SPS id
        assert_eq!(
            SeqParameterSet::parse(&forbidden),
            Err(H264Error::ForbiddenZeroBit)
        );
        // The PPS NAL offered to the SPS parser: wrong NAL type.
        assert_eq!(
            SeqParameterSet::parse(REAL_PPS),
            Err(H264Error::Invalid {
                code: "h264.sps.wrong_nal_type"
            })
        );
        // A 1-byte SPS NAL: header parses, syntax truncates.
        assert!(matches!(
            SeqParameterSet::parse(&[0x67]),
            Err(H264Error::Truncated { .. })
        ));
    }

    /// Minimal one-shot SPS bit builder for the crafted-syntax tests
    /// (ue packing per §9.1: `v+1` in binary, prefixed by one fewer
    /// zero than its bit length).
    struct SpsBits(Vec<bool>);

    impl SpsBits {
        fn new() -> SpsBits {
            SpsBits(Vec::new())
        }
        fn byte(&mut self, byte: u8) -> &mut Self {
            for i in (0..8).rev() {
                self.0.push(byte & (1 << i) != 0);
            }
            self
        }
        fn flag(&mut self, on: bool) -> &mut Self {
            self.0.push(on);
            self
        }
        fn ue(&mut self, v: u64) -> &mut Self {
            let payload = v + 1;
            let len = 64 - payload.leading_zeros() as usize;
            for _ in 0..len - 1 {
                self.0.push(false);
            }
            for i in (0..len).rev() {
                self.0.push(payload & (1 << i) != 0);
            }
            self
        }
        /// se(v) (§9.2): positive v is codeNum 2v−1, non-positive is
        /// codeNum −2v (k=1→+1, k=2→−1, k=3→+2, …).
        fn se(&mut self, v: i64) -> &mut Self {
            let code_num = if v > 0 {
                u64::try_from(2 * v - 1).unwrap_or(u64::MAX)
            } else {
                u64::try_from(-2 * v).unwrap_or(u64::MAX)
            };
            self.ue(code_num)
        }
        fn nal(&mut self) -> Vec<u8> {
            self.0.push(true); // rbsp stop bit
            let mut bytes = vec![0x67u8]; // SPS NAL header
            for chunk in self.0.chunks(8) {
                let mut byte = 0u8;
                for (i, &b) in chunk.iter().enumerate() {
                    if b {
                        byte |= 1 << (7 - i);
                    }
                }
                bytes.push(byte);
            }
            bytes
        }
    }

    #[test]
    fn invalid_sps_id_is_out_of_range() {
        // profile 100 triggers the high-profile branch; the first ue
        // (sps id) encodes 32, one above the 0..=31 spec range.
        let bytes = SpsBits::new().byte(100).byte(0).byte(13).ue(32).nal();
        assert_eq!(
            SeqParameterSet::parse(&bytes),
            Err(H264Error::OutOfRange {
                field: "seq_parameter_set_id",
                value: 32,
                max: 31
            })
        );
    }

    #[test]
    fn emulation_damage_in_sps_is_typed() {
        // 00 00 03 with a bad suffix injected mid-SPS payload.
        let mut damaged = REAL_SPS.to_vec();
        damaged.extend_from_slice(&[0x00, 0x00, 0x03, 0x99, 0x00]);
        assert!(matches!(
            SeqParameterSet::parse(&damaged),
            Err(H264Error::Emulation { .. })
        ));
    }

    #[test]
    fn crop_arithmetic_matches_spec_crop_units() {
        // 4:4:4 (idc 3) with crop (1,2,3,4): CropUnit 1×1 → coded
        // 16×16 displays as 16−3 × 16−7 = 13×9.
        let bytes = SpsBits::new()
            .byte(100) // profile High
            .byte(0)
            .byte(13)
            .ue(0) // sps id
            .ue(3) // chroma_format_idc = 3 (4:4:4)
            .flag(false) // separate_colour_plane
            .ue(0) // bit_depth_luma_minus8
            .ue(0) // bit_depth_chroma_minus8
            .flag(false) // qpprime
            .flag(false) // scaling matrix absent
            .ue(0) // log2_max_frame_num_minus4
            .ue(0) // pic_order_cnt_type = 0
            .ue(0) // log2_max_poc_lsb_minus4
            .ue(1) // max_num_ref_frames
            .flag(false) // gaps
            .ue(0) // width 1 mb
            .ue(0) // height 1 map unit
            .flag(true) // frame_mbs_only
            .flag(true) // direct_8x8_inference
            .flag(true) // frame_cropping
            .ue(1) // left
            .ue(2) // right
            .ue(3) // top
            .ue(4) // bottom
            .flag(false) // vui absent
            .nal();
        let sps = SeqParameterSet::parse(&bytes).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(sps.chroma_format(), ChromaFormat::Yuv444);
        assert_eq!(sps.coded_size(), Some((16, 16)));
        assert_eq!(sps.display_size(), Some((13, 9)));
        // Crop larger than the coded frame is corruption, typed.
        let bad = SpsBits::new()
            .byte(100)
            .byte(0)
            .byte(13)
            .ue(0)
            .ue(3)
            .flag(false)
            .ue(0)
            .ue(0)
            .flag(false)
            .flag(false)
            .ue(0)
            .ue(0)
            .ue(0)
            .ue(1)
            .flag(false)
            .ue(0)
            .ue(0)
            .flag(true)
            .flag(true)
            .flag(true)
            .ue(9) // left
            .ue(9) // right — 18 > 16 luma samples
            .ue(0)
            .ue(0)
            .flag(false)
            .nal();
        let sps = SeqParameterSet::parse(&bad).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(sps.display_size(), None);
    }

    #[test]
    fn poc_type_1_cycle_offsets_parse_and_bound() {
        // pic_order_cnt_type 1: delta flag, two se offsets, a cycle
        // count, then the per-reference se cycle list (all retained).
        let bytes = SpsBits::new()
            .byte(66) // Baseline: no chroma signalling branch
            .byte(0)
            .byte(13)
            .ue(0) // sps id
            .ue(0) // log2_max_frame_num_minus4 → 16-bit frame_num
            .ue(1) // pic_order_cnt_type = 1
            .flag(true) // delta_pic_order_always_zero
            .se(3) // offset_for_non_ref_pic
            .se(-2) // offset_for_top_to_bottom
            .ue(3) // num_ref_frames_in_pic_order_cnt_cycle
            .se(1)
            .se(-1)
            .se(5) // the cycle offsets
            .ue(2) // max_num_ref_frames
            .flag(false) // gaps
            .ue(0) // width 1 mb
            .ue(0) // height 1 map unit
            .flag(true) // frame_mbs_only
            .flag(true) // direct_8x8_inference
            .flag(false) // no crop
            .flag(false) // no vui
            .nal();
        let sps = SeqParameterSet::parse(&bytes).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(sps.pic_order_cnt_type, 1);
        assert_eq!(sps.log2_max_pic_order_cnt_lsb_minus4, None);
        assert_eq!(sps.poc_cycle_offsets, vec![1, -1, 5]);
        assert_eq!(sps.max_num_ref_frames, 2);
        assert_eq!(sps.display_size(), Some((16, 16)));
        // Exp-Golomb width caps |se(v)| at 2^31−1 (a 31-zero prefix is
        // the reader's limit), so the i32 bound on
        // `offset_for_ref_frame` is enforced structurally; anything
        // larger is refused by the typed ue-width error, never a wrap.
        let big = SpsBits::new()
            .byte(66)
            .byte(0)
            .byte(13)
            .ue(0)
            .ue(0)
            .ue(1) // poc type 1
            .flag(false)
            .se(0)
            .se(0)
            .ue(1) // one cycle entry…
            .se(2_147_483_650) // …needs a 33-bit code: typed refusal
            .ue(0)
            .flag(false)
            .ue(0)
            .ue(0)
            .flag(true)
            .flag(true)
            .flag(false)
            .flag(false)
            .nal();
        assert!(matches!(
            SeqParameterSet::parse(&big),
            Err(H264Error::Invalid {
                code: "h264.ue.overflow"
            })
        ));
    }

    #[test]
    fn avc_length_prefixed_entry_form_parses_the_same_sps() {
        // Frame the real SPS AVC-style and parse through the splitter:
        // both entry forms must reach the identical SPS structure.
        let mut avc = Vec::new();
        avc.extend_from_slice(
            &u32::try_from(REAL_SPS.len())
                .unwrap_or_default()
                .to_be_bytes(),
        );
        avc.extend_from_slice(REAL_SPS);
        let mut nals = AvcNals::new(&avc, 4).unwrap_or_else(|e| unreachable!("{e}"));
        let nal = nals
            .next()
            .unwrap_or_else(|| unreachable!("one NAL"))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert!(nals.next().is_none());
        assert_eq!(
            SeqParameterSet::parse(nal),
            SeqParameterSet::parse(REAL_SPS)
        );
    }
}
