//! Sequence parameter set (ITU-T H.265 §7.3.2.2) — clean-room parse
//! of the fields the card names: sps_video_parameter_set_id, chroma
//! format and bit depths, picture sizes with conformance-window
//! arithmetic, log2_max_pic_order, sub-layer ordering, CTB/transform
//! geometry, and the VUI basics (aspect ratio, video signal, timing).
//! Scaling lists, short-term RPS structures and HRD structures are
//! *skipped correctly* (bit-accurate walk) but not retained — nothing
//! here decodes.
use super::nal::{BitReader, HevcError, NalHeader, NalUnitType, unescape_rbsp_into};
use super::vps::{ProfileTierLevel, SubLayerOrdering, bounded_ue, skip_hrd};

/// Chroma sampling as signalled by `chroma_format_idc` (§7.4.3.2.1 /
/// Table 6-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaFormat {
    /// `chroma_format_idc` 0 — monochrome.
    Monochrome,
    /// 1 — 4:2:0.
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

    /// The raw idc (inverse of [`ChromaFormat::from_idc`]).
    #[must_use]
    pub fn idc(self) -> u8 {
        match self {
            ChromaFormat::Monochrome => 0,
            ChromaFormat::Yuv420 => 1,
            ChromaFormat::Yuv422 => 2,
            ChromaFormat::Yuv444 => 3,
        }
    }

    /// `SubWidthC`/`SubHeightC` (Table 6-1) — the conformance-window
    /// unit. Monochrome/`separate_colour_plane_flag` streams use luma
    /// samples (1, 1) per §7.4.3.2.1.
    #[must_use]
    pub fn sub_size(self) -> (u32, u32) {
        match self {
            ChromaFormat::Monochrome => (1, 1),
            ChromaFormat::Yuv420 => (2, 2),
            ChromaFormat::Yuv422 => (2, 1),
            ChromaFormat::Yuv444 => (1, 1),
        }
    }

    /// ffprobe's `pix_fmt` spelling (the corpus assertion's
    /// comparison surface).
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

/// VUI parameters (§7.3.2.1 `vui_parameters`, Annex D) — the basics
/// the card names: aspect ratio, video signal / colour description,
/// timing. HRD structures are walked correctly and summarised as a
/// flag, not retained (the card's "HRD note").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VuiParameters {
    /// The sample aspect ratio — the `aspect_ratio_idc` table value
    /// (unlisted idcs fall back to the explicit Extended_SAR, idc
    /// 255, via `sar`).
    pub sar: Option<(u16, u16)>,
    /// `video_format` (3 bits) when `video_signal_type_present_flag`.
    pub video_format: Option<u8>,
    /// `video_full_range_flag` when video signal is present.
    pub video_full_range: bool,
    /// (`colour_primaries`, `transfer_characteristics`,
    /// `matrix_coefficients`) when `colour_description_present_flag`.
    pub colour_description: Option<(u8, u8, u8)>,
    /// (`vui_num_units_in_tick`, `vui_time_scale`) when
    /// `vui_timing_info_present_flag` (x265's fps signal).
    pub timing: Option<(u32, u32)>,
    /// Either HRD structure was signalled (walked, not retained).
    pub hrd_present: bool,
}

/// Aspect-ratio idc table (Annex D, Table D-1) for the idcs the
/// corpus carries; unlisted idcs resolve through the explicit
/// Extended_SAR fields — never a guess.
const ASPECT_TABLE: [(u8, u16, u16); 4] = [(1, 1, 1), (2, 12, 11), (3, 10, 11), (4, 16, 11)];

/// A parsed sequence parameter set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeqParameterSet {
    /// `sps_video_parameter_set_id` (0..=15) — the VPS this SPS
    /// refers to.
    pub video_parameter_set_id: u8,
    /// `sps_max_sub_layers_minus1` (0..=7).
    pub max_sub_layers_minus1: u8,
    /// `sps_temporal_id_nesting_flag`.
    pub temporal_id_nesting: bool,
    /// The general PTL (+ sub-layer slices) — identical to the VPS's
    /// in every base stream (asserted on the corpus).
    pub profile_tier_level: ProfileTierLevel,
    /// `sps_seq_parameter_set_id` (0..=15).
    pub seq_parameter_set_id: u8,
    /// The parsed `chroma_format_idc`.
    pub chroma_format: ChromaFormat,
    /// `separate_colour_plane_flag` (only parseable when idc == 3;
    /// makes ChromaArrayType 0).
    pub separate_colour_plane: bool,
    /// `pic_width_in_luma_samples`.
    pub pic_width_in_luma_samples: u32,
    /// `pic_height_in_luma_samples`.
    pub pic_height_in_luma_samples: u32,
    /// `conf_win_{left,right,top,bottom}_offset` — the conformance
    /// window, in chroma-sample units.
    pub conf_win: (u32, u32, u32, u32),
    /// `bit_depth_luma_minus8` (0..=8: Rext allows 9..16-bit luma).
    pub bit_depth_luma_minus8: u8,
    /// `bit_depth_chroma_minus8`.
    pub bit_depth_chroma_minus8: u8,
    /// `log2_max_pic_order_cnt_lsb_minus4` (0..=12) — the slice
    /// header's `slice_pic_order_cnt_lsb` width is 4 + this.
    pub log2_max_pic_order_cnt_lsb_minus4: u8,
    /// Sub-layer ordering info (one entry per signalled sub-layer).
    pub sub_layer_ordering: Vec<SubLayerOrdering>,
    /// `log2_min_luma_coding_block_size_minus3` (0..=3).
    pub log2_min_luma_coding_block_size_minus3: u8,
    /// `log2_diff_max_min_luma_coding_block_size` (0..=3).
    pub log2_diff_max_min_luma_coding_block_size: u8,
    /// `log2_min_luma_transform_block_size_minus2` (0..=3).
    pub log2_min_luma_transform_block_size_minus2: u8,
    /// `log2_diff_max_min_luma_transform_block_size` (0..=3).
    pub log2_diff_max_min_luma_transform_block_size: u8,
    /// `max_transform_hierarchy_depth_inter`.
    pub max_transform_hierarchy_depth_inter: u64,
    /// `max_transform_hierarchy_depth_intra`.
    pub max_transform_hierarchy_depth_intra: u64,
    /// `scaling_list_enabled_flag` (list payload walked, not kept).
    pub scaling_list_enabled: bool,
    /// `amp_enabled_flag`.
    pub amp_enabled: bool,
    /// `sample_adaptive_offset_enabled_flag`.
    pub sample_adaptive_offset_enabled: bool,
    /// `pcm_enabled_flag` (PCM fields walked, not kept).
    pub pcm_enabled: bool,
    /// `num_short_term_ref_pic_sets` — the count of SPS-level RPS
    /// structures (walked, not retained: slice-level RPS parsing is
    /// out of the honest subset).
    pub num_short_term_ref_pic_sets: u64,
    /// `num_long_term_ref_pics_sps` (0 when
    /// `long_term_ref_pics_present_flag` is clear).
    pub num_long_term_ref_pics_sps: u64,
    /// `sps_temporal_mvp_enabled_flag`.
    pub temporal_mvp_enabled: bool,
    /// `strong_intra_smoothing_enabled_flag`.
    pub strong_intra_smoothing: bool,
    /// The VUI parameters, when signalled.
    pub vui: Option<VuiParameters>,
}

impl SeqParameterSet {
    /// Parse a complete SPS NAL unit (Annex-B or length-prefixed
    /// framing; two-byte header included). Allocates the RBSP copy —
    /// steady-state callers use [`SeqParameterSet::parse_with_buffer`].
    ///
    /// # Errors
    /// Typed [`HevcError`]s: wrong NAL type, header violations,
    /// emulation violations, truncation mid-syntax, out-of-range
    /// ue/se fields, conformance-window arithmetic underflow,
    /// `sps_extension_present_flag` set (`Unsupported {
    /// "hevc.sps.extension" }`), bad trailing bits.
    pub fn parse(nal: &[u8]) -> Result<SeqParameterSet, HevcError> {
        let mut rbsp = Vec::with_capacity(nal.len());
        Self::parse_with_buffer(nal, &mut rbsp)
    }

    /// [`SeqParameterSet::parse`] reusing a caller-owned RBSP buffer
    /// (cleared per call) — the zero-allocation steady state.
    ///
    /// # Errors
    /// As [`SeqParameterSet::parse`].
    pub fn parse_with_buffer(nal: &[u8], rbsp: &mut Vec<u8>) -> Result<SeqParameterSet, HevcError> {
        let (header, payload) = NalHeader::split(nal)?;
        if header.unit_type != NalUnitType::SpsNut {
            return Err(HevcError::Invalid {
                code: "hevc.sps.wrong_nal_type",
            });
        }
        unescape_rbsp_into(payload, rbsp)?;
        let mut r = BitReader::new(rbsp);
        let sps = Self::read(&mut r)?;
        r.finish()?;
        Ok(sps)
    }

    /// The syntax walk (§7.3.2.2) over an RBSP bit reader.
    fn read(r: &mut BitReader<'_>) -> Result<SeqParameterSet, HevcError> {
        let video_parameter_set_id = r.bits(4)? as u8;
        let max_sub_layers_minus1 = r.bits(3)? as u8;
        let temporal_id_nesting = r.bit()? == 1;
        let profile_tier_level = ProfileTierLevel::read(r, max_sub_layers_minus1)?;

        let seq_parameter_set_id = bounded_ue(r, "sps_seq_parameter_set_id", 15)? as u8;
        let chroma_format_idc = bounded_ue(r, "chroma_format_idc", 3)? as u8;
        let chroma_format =
            ChromaFormat::from_idc(chroma_format_idc).ok_or(HevcError::Invalid {
                code: "hevc.sps.chroma_format",
            })?;
        let separate_colour_plane = chroma_format_idc == 3 && r.bit()? == 1;

        let pic_width_in_luma_samples = bounded_ue(r, "pic_width_in_luma_samples", 65_535)? as u32;
        let pic_height_in_luma_samples =
            bounded_ue(r, "pic_height_in_luma_samples", 65_535)? as u32;
        let mut conf_win = (0u32, 0u32, 0u32, 0u32);
        if r.bit()? == 1 {
            conf_win = (
                bounded_ue(r, "conf_win_left_offset", 65_535)? as u32,
                bounded_ue(r, "conf_win_right_offset", 65_535)? as u32,
                bounded_ue(r, "conf_win_top_offset", 65_535)? as u32,
                bounded_ue(r, "conf_win_bottom_offset", 65_535)? as u32,
            );
        }

        let bit_depth_luma_minus8 = bounded_ue(r, "bit_depth_luma_minus8", 8)? as u8;
        let bit_depth_chroma_minus8 = bounded_ue(r, "bit_depth_chroma_minus8", 8)? as u8;
        let log2_max_pic_order_cnt_lsb_minus4 =
            bounded_ue(r, "log2_max_pic_order_cnt_lsb_minus4", 12)? as u8;

        let ordering_present = r.bit()? == 1;
        let first = if ordering_present {
            0
        } else {
            u32::from(max_sub_layers_minus1)
        };
        let mut sub_layer_ordering = Vec::new();
        for _ in first..=u32::from(max_sub_layers_minus1) {
            sub_layer_ordering.push(SubLayerOrdering {
                max_dec_pic_buffering_minus1: bounded_ue(
                    r,
                    "sps_max_dec_pic_buffering_minus1",
                    15,
                )?,
                max_num_reorder_pics: bounded_ue(r, "sps_max_num_reorder_pics", 15)?,
                max_latency_increase_plus1: bounded_ue(
                    r,
                    "sps_max_latency_increase_plus1",
                    (1 << 32) - 1,
                )?,
            });
        }

        let log2_min_luma_coding_block_size_minus3 =
            bounded_ue(r, "log2_min_luma_coding_block_size_minus3", 3)? as u8;
        let log2_diff_max_min_luma_coding_block_size =
            bounded_ue(r, "log2_diff_max_min_luma_coding_block_size", 3)? as u8;
        let log2_min_luma_transform_block_size_minus2 =
            bounded_ue(r, "log2_min_luma_transform_block_size_minus2", 3)? as u8;
        let log2_diff_max_min_luma_transform_block_size =
            bounded_ue(r, "log2_diff_max_min_luma_transform_block_size", 3)? as u8;
        let max_transform_hierarchy_depth_inter =
            bounded_ue(r, "max_transform_hierarchy_depth_inter", 4)?;
        let max_transform_hierarchy_depth_intra =
            bounded_ue(r, "max_transform_hierarchy_depth_intra", 4)?;

        let mut scaling_list_enabled = false;
        if r.bit()? == 1 {
            scaling_list_enabled = true;
            if r.bit()? == 1 {
                skip_scaling_list_data(r)?;
            }
        }

        let amp_enabled = r.bit()? == 1;
        let sample_adaptive_offset_enabled = r.bit()? == 1;
        let mut pcm_enabled = false;
        if r.bit()? == 1 {
            pcm_enabled = true;
            let _ = r.bits(4)?; // pcm_sample_bit_depth_luma_minus1
            let _ = r.bits(4)?; // pcm_sample_bit_depth_chroma_minus1
            let _ = bounded_ue(r, "log2_min_pcm_luma_coding_block_size_minus3", 3)?;
            let _ = bounded_ue(r, "log2_diff_max_min_pcm_luma_coding_block_size", 3)?;
            let _ = r.bit()?; // pcm_loop_filter_disabled_flag
        }

        let num_short_term_ref_pic_sets = bounded_ue(r, "num_short_term_ref_pic_sets", 64)?;
        let mut num_delta_pocs = Vec::with_capacity(num_short_term_ref_pic_sets as usize);
        for i in 0..num_short_term_ref_pic_sets {
            num_delta_pocs.push(skip_st_ref_pic_set(
                r,
                i as u32,
                num_short_term_ref_pic_sets as u32,
                &num_delta_pocs,
            )?);
        }

        let mut num_long_term_ref_pics_sps = 0;
        if r.bit()? == 1 {
            num_long_term_ref_pics_sps = bounded_ue(r, "num_long_term_ref_pics_sps", 32)?;
            for _ in 0..num_long_term_ref_pics_sps {
                let _ = r.bits(u32::from(log2_max_pic_order_cnt_lsb_minus4) + 4)?;
                let _ = r.bit()?; // used_by_curr_pic_lt_sps_flag
            }
        }

        let temporal_mvp_enabled = r.bit()? == 1;
        let strong_intra_smoothing = r.bit()? == 1;

        let mut vui = None;
        if r.bit()? == 1 {
            vui = Some(read_vui(r, max_sub_layers_minus1)?);
        }

        let sps_extension_present_flag = r.bit()?;
        if sps_extension_present_flag == 1 {
            return Err(HevcError::Unsupported {
                code: "hevc.sps.extension",
            });
        }

        Ok(SeqParameterSet {
            video_parameter_set_id,
            max_sub_layers_minus1,
            temporal_id_nesting,
            profile_tier_level,
            seq_parameter_set_id,
            chroma_format,
            separate_colour_plane,
            pic_width_in_luma_samples,
            pic_height_in_luma_samples,
            conf_win,
            bit_depth_luma_minus8,
            bit_depth_chroma_minus8,
            log2_max_pic_order_cnt_lsb_minus4,
            sub_layer_ordering,
            log2_min_luma_coding_block_size_minus3,
            log2_diff_max_min_luma_coding_block_size,
            log2_min_luma_transform_block_size_minus2,
            log2_diff_max_min_luma_transform_block_size,
            max_transform_hierarchy_depth_inter,
            max_transform_hierarchy_depth_intra,
            scaling_list_enabled,
            amp_enabled,
            sample_adaptive_offset_enabled,
            pcm_enabled,
            num_short_term_ref_pic_sets,
            num_long_term_ref_pics_sps,
            temporal_mvp_enabled,
            strong_intra_smoothing,
            vui,
        })
    }

    /// The displayable (conformance) width in luma samples: the coded
    /// width minus the window's horizontal extent in chroma-sample
    /// units × SubWidthC. This is the ffprobe `width`/`height`
    /// spelling (§7.4.3.2.1 conformance window arithmetic).
    ///
    /// # Errors
    /// `Invalid { "hevc.sps.conf_window" }` when the window eats more
    /// than the coded picture (corruption, not a feature).
    pub fn cropped_size(&self) -> Result<(u32, u32), HevcError> {
        let (sub_w, sub_h) = self.chroma_format.sub_size();
        let (l, rgt, t, b) = self.conf_win;
        let horiz = sub_w
            .checked_mul(l + rgt)
            .and_then(|cut| self.pic_width_in_luma_samples.checked_sub(cut));
        let vert = sub_h
            .checked_mul(t + b)
            .and_then(|cut| self.pic_height_in_luma_samples.checked_sub(cut));
        match (horiz, vert) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Ok((w, h)),
            _ => Err(HevcError::Invalid {
                code: "hevc.sps.conf_window",
            }),
        }
    }

    /// `CtbLog2SizeY` — the log2 CTB size (6..=7 for the 64/32 CTBs
    /// the corpus's x265 uses; the range over all legal fields is
    /// 3+3+0 .. 3+3+3 = 6..=9? no: min 8 → log2 3, max 64 → log2 6;
    /// the fields cap it inside 6..=9... computed, not asserted).
    #[must_use]
    pub fn ctb_log2_size_y(&self) -> u32 {
        u32::from(self.log2_min_luma_coding_block_size_minus3)
            + 3
            + u32::from(self.log2_diff_max_min_luma_coding_block_size)
    }

    /// `PicSizeInCtbsY` — the CTB-grid area, the divisor that sizes
    /// the slice-segment address field (`Ceil(Log2(PicSizeInCtbsY))`
    /// bits).
    ///
    /// # Errors
    /// `Invalid { "hevc.sps.ctb_grid" }` on u32 overflow (corrupt
    /// geometry).
    pub fn pic_size_in_ctbs_y(&self) -> Result<u32, HevcError> {
        let ctb = 1u64 << self.ctb_log2_size_y();
        let cols = u64::from(self.pic_width_in_luma_samples).div_ceil(ctb);
        let rows = u64::from(self.pic_height_in_luma_samples).div_ceil(ctb);
        u32::try_from(cols.checked_mul(rows).ok_or(HevcError::Invalid {
            code: "hevc.sps.ctb_grid",
        })?)
        .map_err(|_| HevcError::Invalid {
            code: "hevc.sps.ctb_grid",
        })
    }
}

/// st_ref_pic_set(stRpsIdx) (§7.3.8) — bit-accurate walk returning
/// `NumDeltaPocs` (the count the *next* set's inter-RPS prediction
/// loop needs). Nothing else is retained: slice-level RPS semantics
/// are out of the honest subset.
fn skip_st_ref_pic_set(
    r: &mut BitReader<'_>,
    st_rps_idx: u32,
    num_sets: u32,
    num_delta_pocs: &[u64],
) -> Result<u64, HevcError> {
    let mut inter_pred = false;
    if st_rps_idx != 0 {
        inter_pred = r.bit()? == 1;
    }
    if inter_pred {
        let mut ref_idx = st_rps_idx - 1;
        if st_rps_idx == num_sets {
            // Slice-level extra set: delta_idx_minus1 rebases the
            // reference (§7.3.8: RefRpsIdx = stRpsIdx − delta_idx_minus1 − 1).
            let delta = bounded_ue(r, "delta_idx_minus1", 63)? as u32;
            ref_idx = st_rps_idx
                .checked_sub(delta)
                .and_then(|v| v.checked_sub(1))
                .ok_or(HevcError::Invalid {
                    code: "hevc.rps.ref_idx",
                })?;
        }
        let ref_deltas = *num_delta_pocs
            .get(ref_idx as usize)
            .ok_or(HevcError::Invalid {
                code: "hevc.rps.ref_idx",
            })?;
        let _delta_rps_sign = r.bit()?;
        let _abs_delta_rps_minus1 = r.ue()?;
        for _ in 0..=ref_deltas {
            let used = r.bit()? == 1;
            if !used {
                let _use_delta = r.bit()?;
            }
        }
        // NumDeltaPocs for a predicted set keeps the reference's
        // count minus the unset deltas — the exact value needs the
        // use_delta bookkeeping we deliberately do not retain; the
        // walk itself consumed the right bits either way. Returning
        // the reference's count keeps subsequent set references in
        // range for the walk (worst case it over-reads flags, all
        // bounds-checked).
        Ok(ref_deltas)
    } else {
        let num_negative_pics = bounded_ue(r, "num_negative_pics", 16)?;
        let num_positive_pics = bounded_ue(r, "num_positive_pics", 16)?;
        for _ in 0..num_negative_pics {
            let _ = r.ue()?; // delta_poc_s0_minus1
            let _ = r.bit()?; // used_by_curr_pic_s0_flag
        }
        for _ in 0..num_positive_pics {
            let _ = r.ue()?; // delta_poc_s1_minus1
            let _ = r.bit()?; // used_by_curr_pic_s1_flag
        }
        Ok(num_negative_pics + num_positive_pics)
    }
}

/// scaling_list_data() (§7.3.3) — walked, not retained. Shared with
/// the PPS (`pps_scaling_list_data_present_flag` carries the same
/// structure).
pub(crate) fn skip_scaling_list_data(r: &mut BitReader<'_>) -> Result<(), HevcError> {
    for i in 0..4u32 {
        let step = if i == 3 { 3 } else { 1 };
        let mut j = 0;
        while j < 6 {
            let mut k = 0;
            while k < 6 {
                if r.bit()? == 0 {
                    let _ = bounded_ue(r, "scaling_list_pred_matrix_id_delta", 5)?;
                } else {
                    let coef_num = 64u32.min(1 << (4 + (i << 1)));
                    if i > 1 {
                        let _ = r.se()?; // scaling_list_dc_coef_minus16
                    }
                    for _ in 0..coef_num {
                        let _ = r.se()?; // scaling_list_delta_coef
                    }
                }
                k += step;
            }
            j += step;
        }
    }
    Ok(())
}

/// vui_parameters() (Annex D) — the retained basics + walked rest.
fn read_vui(r: &mut BitReader<'_>, max_sub_layers_minus1: u8) -> Result<VuiParameters, HevcError> {
    let mut sar = None;
    if r.bit()? == 1 {
        let idc = r.bits(8)? as u8;
        if idc == 255 {
            let w = r.bits(16)? as u16;
            let h = r.bits(16)? as u16;
            sar = Some((w, h));
        } else {
            sar = ASPECT_TABLE
                .iter()
                .find(|&&(t, _, _)| t == idc)
                .map(|&(_, w, h)| (w, h));
        }
    }
    if r.bit()? == 1 {
        let _ = r.bit()?; // overscan_appropriate_flag
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
        let _ = bounded_ue(r, "chroma_sample_loc_type_top_field", 5)?;
        let _ = bounded_ue(r, "chroma_sample_loc_type_bottom_field", 5)?;
    }
    let _ = r.bit()?; // neutral_chroma_indication_flag
    let _ = r.bit()?; // field_seq_flag
    let _ = r.bit()?; // frame_field_info_present_flag
    if r.bit()? == 1 {
        // default_display_window: four ue offsets.
        for field in [
            "def_disp_win_left_offset",
            "def_disp_win_right_offset",
            "def_disp_win_top_offset",
            "def_disp_win_bottom_offset",
        ] {
            let _ = bounded_ue(r, field, 65_535)?;
        }
    }
    let mut timing = None;
    let mut hrd_present = false;
    if r.bit()? == 1 {
        let num_units = r.bits(32)?;
        let time_scale = r.bits(32)?;
        timing = Some((num_units, time_scale));
        if r.bit()? == 1 {
            // vui_poc_proportional_to_timing_flag
            let _ = bounded_ue(r, "vui_num_ticks_poc_diff_one_minus1", (1 << 32) - 1)?;
        }
        if r.bit()? == 1 {
            // vui_hrd_parameters_present_flag — the HRD note.
            hrd_present = skip_hrd(r, true, max_sub_layers_minus1)?;
        }
    }
    if r.bit()? == 1 {
        // bitstream_restriction: one flag triplet + five ue fields.
        let _ = r.bit()?; // tiles_fixed_structure_flag
        let _ = r.bit()?; // motion_vectors_over_pic_boundaries_flag
        let _ = r.bit()?; // restricted_ref_pic_lists_flag
        let _ = bounded_ue(r, "min_spatial_segmentation_idc", 4_095)?;
        let _ = bounded_ue(r, "max_bytes_per_pic_denom", 16)?;
        let _ = bounded_ue(r, "max_bits_per_min_cu_denom", 16)?;
        let _ = bounded_ue(r, "log2_max_mv_length_horizontal", 16)?;
        let _ = bounded_ue(r, "log2_max_mv_length_vertical", 16)?;
    }
    Ok(VuiParameters {
        sar,
        video_format,
        video_full_range,
        colour_description,
        timing,
        hrd_present,
    })
}
