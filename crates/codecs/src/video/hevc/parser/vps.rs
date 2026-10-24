//! Video parameter set (ITU-T H.265 §7.3.2.1) and the shared
//! profile_tier_level() (§7.3.3) + hrd_parameters() (Annex C / F.2.2)
//! walkers, clean-room.
//!
//! The VPS is HEVC's outermost parameter set: it fixes the sub-layer
//! count that every profile_tier_level() call in the stream uses, the
//! sub-layer decoding-order info, and (optionally) the stream-level
//! timing. HRD structures are *walked correctly* (bit-accurate) but
//! not retained — a summary flag is recorded (the card's "HRD note").
use super::nal::{BitReader, HevcError, NalHeader, NalUnitType, unescape_rbsp_into};

/// One sub-layer's profile_tier_level slice (§7.3.3): presence flags
/// plus the sub-layer level; the sub-layer profile payload (84 bits)
/// is walked, not retained (base streams leave it absent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubLayerProfileTierLevel {
    /// `sub_layer_profile_present_flag[i]`.
    pub profile_present: bool,
    /// `sub_layer_level_present_flag[i]`.
    pub level_present: bool,
    /// `sub_layer_level_idc[i]` when present.
    pub level_idc: Option<u8>,
}

/// profile_tier_level(profilePresentFlag=1, maxNumSubLayersMinus1) —
/// the 12-byte general PTL plus the per-sub-layer presence bits and
/// levels (§7.3.3). Shared by the VPS and SPS (and mirrored by the
/// `hvcC` record — the corpus assertions cross-check all three).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileTierLevel {
    /// `general_profile_space` (0..=3).
    pub profile_space: u8,
    /// `general_tier_flag` (0 = Main tier, 1 = High tier).
    pub tier: bool,
    /// `general_profile_idc` (0..=31).
    pub profile_idc: u8,
    /// The 32 `general_profile_compatibility_flag`s, MSB = flag 0.
    pub profile_compatibility_flags: u32,
    /// `general_progressive_source_flag`.
    pub progressive_source: bool,
    /// `general_interlaced_source_flag`.
    pub interlaced_source: bool,
    /// `general_non_packed_constraint_flag`.
    pub non_packed_constraint: bool,
    /// `general_frame_only_constraint_flag`.
    pub frame_only_constraint: bool,
    /// The 48-bit constraint-indicator word (the four flags above in
    /// its top bits + the 44 reserved/inbld bits) — the exact value
    /// an `hvcC` record carries, for record-vs-SPS assertions.
    pub constraint_indicator_flags: u64,
    /// `general_level_idc` (30 × level major: 93 = level 9.3 — the
    /// ffprobe `level` spelling).
    pub level_idc: u8,
    /// The per-sub-layer slices (empty for the 1-layer streams that
    /// dominate the corpus).
    pub sub_layers: Vec<SubLayerProfileTierLevel>,
}

impl ProfileTierLevel {
    /// ffprobe's profile spelling for the idcs the corpus carries
    /// (unlisted idcs report verbatim — never a guess).
    #[must_use]
    pub fn profile_name(&self) -> String {
        match self.profile_idc {
            1 => "Main".to_string(),
            2 => "Main 10".to_string(),
            3 => "Main Still Picture".to_string(),
            4 => "Rext".to_string(),
            5 => "SCC".to_string(),
            other => format!("idc {other}"),
        }
    }

    /// The syntax walk (§7.3.3) over an RBSP bit reader.
    pub(crate) fn read(
        r: &mut BitReader<'_>,
        max_sub_layers_minus1: u8,
    ) -> Result<ProfileTierLevel, HevcError> {
        let profile_space = r.bits(2)? as u8;
        let tier = r.bit()? == 1;
        let profile_idc = r.bits(5)? as u8;

        let mut compat: u32 = 0;
        for _ in 0..32 {
            compat = (compat << 1) | r.bit()?;
        }
        let progressive_source = r.bit()? == 1;
        let interlaced_source = r.bit()? == 1;
        let non_packed_constraint = r.bit()? == 1;
        let frame_only_constraint = r.bit()? == 1;
        // The 43 reserved bits + the trailing inbld/reserved bit —
        // walked bit-accurately, retained inside the 48-bit word.
        let mut reserved: u64 = 0;
        for _ in 0..44 {
            reserved = (reserved << 1) | u64::from(r.bit()?);
        }
        let constraint_indicator_flags = (u64::from(progressive_source) << 47)
            | (u64::from(interlaced_source) << 46)
            | (u64::from(non_packed_constraint) << 45)
            | (u64::from(frame_only_constraint) << 44)
            | reserved;
        let level_idc = r.bits(8)? as u8;

        // Sub-layer presence bits + the reserved 2-bit fillers up to
        // sub-layer 7 (only read when any sub-layer exists).
        let mut present = [(false, false); 7];
        for slot in &mut present[..usize::from(max_sub_layers_minus1)] {
            slot.0 = r.bit()? == 1;
            slot.1 = r.bit()? == 1;
        }
        if max_sub_layers_minus1 > 0 {
            for _ in max_sub_layers_minus1..8 {
                let reserved_two = r.bits(2)?;
                if reserved_two != 0 {
                    return Err(HevcError::Invalid {
                        code: "hevc.ptl.reserved",
                    });
                }
            }
        }

        let mut sub_layers = Vec::with_capacity(usize::from(max_sub_layers_minus1));
        for i in 0..usize::from(max_sub_layers_minus1) {
            let (profile_present, level_present) = present[i];
            if profile_present {
                // Sub-layer profile payload: 2+1+5 + 32 + 44 bits,
                // walked (the general layer above is the retained
                // one; sub-layer profiles are multi-layer-stream
                // territory).
                let _space_tier_idc = r.bits(8)?;
                for _ in 0..32 {
                    r.bit()?;
                }
                for _ in 0..44 {
                    r.bit()?;
                }
            }
            let level_idc = if level_present { Some(r.bits(8)? as u8) } else { None };
            sub_layers.push(SubLayerProfileTierLevel {
                profile_present,
                level_present,
                level_idc,
            });
        }

        Ok(ProfileTierLevel {
            profile_space,
            tier,
            profile_idc,
            profile_compatibility_flags: compat,
            progressive_source,
            interlaced_source,
            non_packed_constraint,
            frame_only_constraint,
            constraint_indicator_flags,
            level_idc,
            sub_layers,
        })
    }
}

/// hrd_parameters(commonInfPresentFlag, maxNumSubLayersMinus1)
/// (Annex C / F.2.2) — bit-accurate walk, no retention.
///
/// # Spec-version note (recorded)
///
/// `fixed_pic_rate_within_cvs_flag` is read only when
/// `fixed_pic_rate_general_flag` is 0 (the v1 conditional form — the
/// form ffmpeg's CBS parser and the corpus's x265 encoder implement);
/// `cpb_cnt_minus1` is read unconditionally. This is the divergent
/// spot between H.265 drafts; the chosen form is pinned by the
/// corpus.
pub(crate) fn skip_hrd(
    r: &mut BitReader<'_>,
    common_info_present: bool,
    max_sub_layers_minus1: u8,
) -> Result<bool, HevcError> {
    let mut any_hrd = false;
    let mut sub_pic = false;
    if common_info_present {
        let nal_hrd = r.bit()? == 1;
        let vcl_hrd = r.bit()? == 1;
        any_hrd = nal_hrd || vcl_hrd;
        if any_hrd {
            sub_pic = r.bit()? == 1;
            if sub_pic {
                let _ = r.bits(8)?; // tick_divisor_minus2
                let _ = r.bits(5)?; // du_cpb_removal_delay_increment_length_minus1
                let _ = r.bit()?; // sub_pic_cpb_params_in_pic_timing_sei_flag
                let _ = r.bits(5)?; // dpb_output_delay_du_length_minus1
            }
            let _ = r.bits(4)?; // bit_rate_scale
            let _ = r.bits(4)?; // cpb_size_scale
            if sub_pic {
                let _ = r.bits(4)?; // cpb_size_du_scale
            }
            let _ = r.bits(5)?; // initial_cpb_removal_delay_length_minus1
            let _ = r.bits(5)?; // au_cpb_removal_delay_length_minus1
            let _ = r.bits(5)?; // dpb_output_delay_length_minus1
        }
    }
    for _ in 0..=max_sub_layers_minus1 {
        let fixed_general = r.bit()? == 1;
        let fixed_within = if fixed_general {
            true // inferred 1 (§F.2.2)
        } else {
            r.bit()? == 1
        };
        if fixed_within {
            bounded_ue(r, "elemental_duration_in_tc_minus1", 2047)?;
        } else {
            let _low_delay = r.bit()?;
        }
        let cpb_cnt_minus1 = bounded_ue(r, "cpb_cnt_minus1", 31)?;
        for _ in 0..=cpb_cnt_minus1 {
            let _ = r.ue()?; // bit_rate_value_minus1
            let _ = r.ue()?; // cpb_size_value_minus1
            if sub_pic {
                let _ = r.ue()?; // bit_rate_du_value_minus1
                let _ = r.ue()?; // cpb_size_du_value_minus1
            }
            let _ = r.bit()?; // cbr_flag
        }
    }
    Ok(any_hrd)
}

/// A ue(v) read with a spec-declared bound (helper).
pub(crate) fn bounded_ue(
    r: &mut BitReader<'_>,
    field: &'static str,
    max: u64,
) -> Result<u64, HevcError> {
    let v = r.ue()?;
    if v > max {
        return Err(HevcError::OutOfRange {
            field,
            value: v,
            max,
        });
    }
    Ok(v)
}

/// One sub-layer's decoding-order info (`vps/sps_max_dec_pic_buffering_minus1[i]`,
/// `..._max_num_reorder_pics[i]`, `..._max_latency_increase_plus1[i]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubLayerOrdering {
    /// `max_dec_pic_buffering_minus1` (0..=15 sane streams; the spec
    /// bound is level-dependent — 15 is the highest declared value
    /// any level permits).
    pub max_dec_pic_buffering_minus1: u64,
    /// `max_num_reorder_pics`.
    pub max_num_reorder_pics: u64,
    /// `max_latency_increase_plus1`.
    pub max_latency_increase_plus1: u64,
}

/// A parsed video parameter set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoParameterSet {
    /// `vps_video_parameter_set_id` (0..=15).
    pub video_parameter_set_id: u8,
    /// `vps_base_layer_internal_flag`.
    pub base_layer_internal: bool,
    /// `vps_base_layer_available_flag`.
    pub base_layer_available: bool,
    /// `vps_max_layers_minus1` (0..=62).
    pub max_layers_minus1: u8,
    /// `vps_max_sub_layers_minus1` (0..=7) — governs every
    /// profile_tier_level() in the stream.
    pub max_sub_layers_minus1: u8,
    /// `vps_temporal_id_nesting_flag`.
    pub temporal_id_nesting: bool,
    /// The general PTL (+ sub-layer slices).
    pub profile_tier_level: ProfileTierLevel,
    /// The sub-layer ordering info (one entry per sub-layer from the
    /// first signalled one up; the list always covers sub-layer
    /// `max_sub_layers_minus1`).
    pub sub_layer_ordering: Vec<SubLayerOrdering>,
    /// `vps_max_layer_id` (0..=63).
    pub max_layer_id: u8,
    /// `vps_num_layer_sets_minus1` (0 in every base stream; layer-set
    /// membership bits are walked, not retained).
    pub num_layer_sets_minus1: u64,
    /// (`vps_num_units_in_tick`, `vps_time_scale`,
    /// `vps_poc_proportional_to_timing_flag`) when timing info is
    /// present.
    pub timing: Option<(u32, u32, bool)>,
    /// The HRD note: at least one `hrd_parameters()` structure was
    /// signalled in the VPS (walked, not retained).
    pub hrd_present: bool,
}

impl VideoParameterSet {
    /// Parse a complete VPS NAL unit (Annex-B or length-prefixed
    /// framing; two-byte header included). Allocates the RBSP copy —
    /// steady-state callers use [`VideoParameterSet::parse_with_buffer`].
    ///
    /// # Errors
    /// Typed [`HevcError`]s: wrong NAL type, forbidden bit, temporal
    /// id 0, emulation violations, truncation mid-syntax,
    /// out-of-range fields, `vps_extension_flag` set
    /// (`Unsupported { "hevc.vps.extension" }`), bad trailing bits.
    pub fn parse(nal: &[u8]) -> Result<VideoParameterSet, HevcError> {
        let mut rbsp = Vec::with_capacity(nal.len());
        Self::parse_with_buffer(nal, &mut rbsp)
    }

    /// [`VideoParameterSet::parse`] reusing a caller-owned RBSP
    /// buffer (cleared per call) — the zero-allocation steady state.
    ///
    /// # Errors
    /// As [`VideoParameterSet::parse`].
    pub fn parse_with_buffer(
        nal: &[u8],
        rbsp: &mut Vec<u8>,
    ) -> Result<VideoParameterSet, HevcError> {
        let (header, payload) = NalHeader::split(nal)?;
        if header.unit_type != NalUnitType::VpsNut {
            return Err(HevcError::Invalid {
                code: "hevc.vps.wrong_nal_type",
            });
        }
        unescape_rbsp_into(payload, rbsp)?;
        let mut r = BitReader::new(rbsp);
        let vps = Self::read(&mut r)?;
        r.finish()?;
        Ok(vps)
    }

    /// The syntax walk (§7.3.2.1) over an RBSP bit reader.
    fn read(r: &mut BitReader<'_>) -> Result<VideoParameterSet, HevcError> {
        let video_parameter_set_id = r.bits(4)? as u8;
        let base_layer_internal = r.bit()? == 1;
        let base_layer_available = r.bit()? == 1;
        let max_layers_minus1 = r.bits(6)? as u8;
        let max_sub_layers_minus1 = r.bits(3)? as u8;
        let temporal_id_nesting = r.bit()? == 1;
        let reserved = r.bits(16)?;
        if reserved != 0xffff {
            return Err(HevcError::Invalid {
                code: "hevc.vps.reserved",
            });
        }
        let profile_tier_level = ProfileTierLevel::read(r, max_sub_layers_minus1)?;

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
                    "vps_max_dec_pic_buffering_minus1",
                    15,
                )?,
                max_num_reorder_pics: bounded_ue(r, "vps_max_num_reorder_pics", 15)?,
                max_latency_increase_plus1: bounded_ue(
                    r,
                    "vps_max_latency_increase_plus1",
                    (1 << 32) - 1,
                )?,
            });
        }

        let max_layer_id = r.bits(6)? as u8;
        let num_layer_sets_minus1 = bounded_ue(r, "vps_num_layer_sets_minus1", 1023)?;
        for _ in 1..=num_layer_sets_minus1 {
            for _ in 0..=max_layer_id {
                r.bit()?; // layer_id_included_flag[i][j]
            }
        }

        let mut timing = None;
        let mut hrd_present = false;
        if r.bit()? == 1 {
            // vps_timing_info_present_flag
            let num_units_in_tick = r.bits(32)?;
            let time_scale = r.bits(32)?;
            let poc_proportional = r.bit()? == 1;
            if poc_proportional {
                bounded_ue(r, "vps_num_ticks_poc_diff_one_minus1", (1 << 32) - 1)?;
            }
            timing = Some((num_units_in_tick, time_scale, poc_proportional));
            let num_hrd = bounded_ue(r, "vps_num_hrd_parameters", 1023)?;
            for i in 0..num_hrd {
                bounded_ue(r, "hrd_layer_set_idx", 1023)?;
                let cprms = if i > 0 { r.bit()? == 1 } else { true };
                if skip_hrd(r, cprms, max_sub_layers_minus1)? {
                    hrd_present = true;
                }
            }
        }

        let vps_extension_flag = r.bit()?;
        if vps_extension_flag == 1 {
            // The extension payload would need its own syntax walk;
            // refusing loudly beats mis-parsing the tail.
            return Err(HevcError::Unsupported {
                code: "hevc.vps.extension",
            });
        }

        Ok(VideoParameterSet {
            video_parameter_set_id,
            base_layer_internal,
            base_layer_available,
            max_layers_minus1,
            max_sub_layers_minus1,
            temporal_id_nesting,
            profile_tier_level,
            sub_layer_ordering,
            max_layer_id,
            num_layer_sets_minus1,
            timing,
            hrd_present,
        })
    }
}
