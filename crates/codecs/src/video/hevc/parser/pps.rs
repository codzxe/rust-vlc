//! Picture parameter set (ITU-T H.265 §7.3.2.3) — clean-room parse
//! of the card's field set: ids, the tile-grid basics, the
//! loop-filter bits, and the init QP. Reference-list defaults and
//! the deblocking/entropy fine print are retained where the slice
//! header's minimal subset needs them.
use super::nal::{BitReader, HevcError, NalHeader, NalUnitType, unescape_rbsp_into};
use super::vps::bounded_ue;

/// A parsed picture parameter set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PicParameterSet {
    /// `pps_pic_parameter_set_id` (0..=63).
    pub pic_parameter_set_id: u8,
    /// `pps_seq_parameter_set_id` (0..=15) — the SPS this PPS pairs
    /// with.
    pub seq_parameter_set_id: u8,
    /// `dependent_slice_segments_enabled_flag` — gates
    /// `dependent_slice_segment_flag` in slice headers.
    pub dependent_slice_segments_enabled: bool,
    /// `output_flag_present_flag` — gates `pic_output_flag`.
    pub output_flag_present: bool,
    /// `num_extra_slice_header_bits` (0..=7).
    pub num_extra_slice_header_bits: u8,
    /// `sign_data_hiding_enabled_flag`.
    pub sign_data_hiding_enabled: bool,
    /// `cabac_init_present_flag`.
    pub cabac_init_present: bool,
    /// `num_ref_idx_l0_default_active_minus1` (0..=15).
    pub num_ref_idx_l0_default_active_minus1: u64,
    /// `num_ref_idx_l1_default_active_minus1` (0..=15).
    pub num_ref_idx_l1_default_active_minus1: u64,
    /// `init_qp_minus26` — `pps_init_qp_minus26` (the final QP bound
    /// is level/depth-dependent; kept verbatim).
    pub init_qp_minus26: i32,
    /// `constrained_intra_pred_flag`.
    pub constrained_intra_pred: bool,
    /// `transform_skip_enabled_flag`.
    pub transform_skip_enabled: bool,
    /// `cu_qp_delta_enabled_flag`.
    pub cu_qp_delta_enabled: bool,
    /// `diff_cu_qp_delta_depth` (when enabled; else 0).
    pub diff_cu_qp_delta_depth: u64,
    /// `pps_cb_qp_offset`.
    pub cb_qp_offset: i32,
    /// `pps_cr_qp_offset`.
    pub cr_qp_offset: i32,
    /// `pps_slice_chroma_qp_offsets_present_flag`.
    pub slice_chroma_qp_offsets_present: bool,
    /// `weighted_pred_flag`.
    pub weighted_pred: bool,
    /// `weighted_bipred_flag`.
    pub weighted_bipred: bool,
    /// `transquant_bypass_enabled_flag`.
    pub transquant_bypass_enabled: bool,
    /// `tiles_enabled_flag`.
    pub tiles_enabled: bool,
    /// `entropy_coding_sync_enabled_flag` (WPP).
    pub entropy_coding_sync_enabled: bool,
    /// `num_tile_columns_minus1` (when tiled; else 0 → one column).
    pub num_tile_columns_minus1: u64,
    /// `num_tile_rows_minus1` (when tiled; else 0).
    pub num_tile_rows_minus1: u64,
    /// `uniform_spacing_flag` (when tiled; column/row width payloads
    /// are walked, not retained).
    pub uniform_spacing: bool,
    /// `loop_filter_across_tiles_enabled_flag` (when tiled).
    pub loop_filter_across_tiles_enabled: bool,
    /// `pps_loop_filter_across_slices_enabled_flag`.
    pub loop_filter_across_slices_enabled: bool,
    /// `deblocking_filter_control_present_flag`.
    pub deblocking_filter_control_present: bool,
    /// `deblocking_filter_override_enabled_flag` (when control is
    /// present).
    pub deblocking_filter_override_enabled: bool,
    /// `pps_deblocking_filter_disabled_flag` (when control is
    /// present).
    pub deblocking_filter_disabled: bool,
    /// (`pps_beta_offset_div2`, `pps_tc_offset_div2`) when the PPS
    /// enables deblocking with explicit offsets.
    pub deblocking_offsets: Option<(i32, i32)>,
    /// `pps_scaling_list_data_present_flag` (payload walked).
    pub scaling_list_data_present: bool,
    /// `lists_modification_present_flag`.
    pub lists_modification_present: bool,
    /// `log2_parallel_merge_level_minus2` (0..=4).
    pub log2_parallel_merge_level_minus2: u64,
    /// `slice_segment_header_extension_present_flag`.
    pub slice_segment_header_extension_present: bool,
}

impl PicParameterSet {
    /// Parse a complete PPS NAL unit (Annex-B or length-prefixed
    /// framing; two-byte header included). Allocates the RBSP copy —
    /// steady-state callers use [`PicParameterSet::parse_with_buffer`].
    ///
    /// # Errors
    /// Typed [`HevcError`]s: wrong NAL type, header violations,
    /// emulation violations, truncation mid-syntax, out-of-range
    /// fields, `pps_extension_present_flag` set (`Unsupported {
    /// "hevc.pps.extension" }`), bad trailing bits.
    pub fn parse(nal: &[u8]) -> Result<PicParameterSet, HevcError> {
        let mut rbsp = Vec::with_capacity(nal.len());
        Self::parse_with_buffer(nal, &mut rbsp)
    }

    /// [`PicParameterSet::parse`] reusing a caller-owned RBSP buffer
    /// (cleared per call) — the zero-allocation steady state.
    ///
    /// # Errors
    /// As [`PicParameterSet::parse`].
    pub fn parse_with_buffer(nal: &[u8], rbsp: &mut Vec<u8>) -> Result<PicParameterSet, HevcError> {
        let (header, payload) = NalHeader::split(nal)?;
        if header.unit_type != NalUnitType::PpsNut {
            return Err(HevcError::Invalid {
                code: "hevc.pps.wrong_nal_type",
            });
        }
        unescape_rbsp_into(payload, rbsp)?;
        let mut r = BitReader::new(rbsp);
        let pps = Self::read(&mut r)?;
        r.finish()?;
        Ok(pps)
    }

    /// The syntax walk (§7.3.2.3) over an RBSP bit reader.
    fn read(r: &mut BitReader<'_>) -> Result<PicParameterSet, HevcError> {
        let pic_parameter_set_id = bounded_ue(r, "pps_pic_parameter_set_id", 63)? as u8;
        let seq_parameter_set_id = bounded_ue(r, "pps_seq_parameter_set_id", 15)? as u8;
        let dependent_slice_segments_enabled = r.bit()? == 1;
        let output_flag_present = r.bit()? == 1;
        let num_extra_slice_header_bits = r.bits(3)? as u8;
        let sign_data_hiding_enabled = r.bit()? == 1;
        let cabac_init_present = r.bit()? == 1;
        let num_ref_idx_l0_default_active_minus1 =
            bounded_ue(r, "num_ref_idx_l0_default_active_minus1", 15)?;
        let num_ref_idx_l1_default_active_minus1 =
            bounded_ue(r, "num_ref_idx_l1_default_active_minus1", 15)?;
        let init_qp_minus26 = r.se()? as i32;
        let constrained_intra_pred = r.bit()? == 1;
        let transform_skip_enabled = r.bit()? == 1;
        let cu_qp_delta_enabled = r.bit()? == 1;
        let diff_cu_qp_delta_depth = if cu_qp_delta_enabled {
            bounded_ue(r, "diff_cu_qp_delta_depth", 4)?
        } else {
            0
        };
        let cb_qp_offset = r.se()? as i32;
        let cr_qp_offset = r.se()? as i32;
        let slice_chroma_qp_offsets_present = r.bit()? == 1;
        let weighted_pred = r.bit()? == 1;
        let weighted_bipred = r.bit()? == 1;
        let transquant_bypass_enabled = r.bit()? == 1;
        let tiles_enabled = r.bit()? == 1;
        let entropy_coding_sync_enabled = r.bit()? == 1;

        let mut num_tile_columns_minus1 = 0;
        let mut num_tile_rows_minus1 = 0;
        let mut uniform_spacing = false;
        let mut loop_filter_across_tiles_enabled = false;
        if tiles_enabled {
            num_tile_columns_minus1 = bounded_ue(r, "num_tile_columns_minus1", 30)?;
            num_tile_rows_minus1 = bounded_ue(r, "num_tile_rows_minus1", 30)?;
            uniform_spacing = r.bit()? == 1;
            if !uniform_spacing {
                for _ in 0..num_tile_columns_minus1 {
                    let _ = r.ue()?; // column_width_minus1
                }
                for _ in 0..num_tile_rows_minus1 {
                    let _ = r.ue()?; // row_height_minus1
                }
            }
            loop_filter_across_tiles_enabled = r.bit()? == 1;
        }

        let loop_filter_across_slices_enabled = r.bit()? == 1;
        let mut deblocking_filter_control_present = false;
        let mut deblocking_filter_override_enabled = false;
        let mut deblocking_filter_disabled = false;
        let mut deblocking_offsets = None;
        if r.bit()? == 1 {
            deblocking_filter_control_present = true;
            deblocking_filter_override_enabled = r.bit()? == 1;
            deblocking_filter_disabled = r.bit()? == 1;
            if !deblocking_filter_disabled {
                deblocking_offsets = Some((r.se()? as i32, r.se()? as i32));
            }
        }

        let mut scaling_list_data_present = false;
        if r.bit()? == 1 {
            scaling_list_data_present = true;
            super::sps::skip_scaling_list_data(r)?;
        }

        let lists_modification_present = r.bit()? == 1;
        let log2_parallel_merge_level_minus2 =
            bounded_ue(r, "log2_parallel_merge_level_minus2", 4)?;
        let slice_segment_header_extension_present = r.bit()? == 1;

        let pps_extension_present_flag = r.bit()?;
        if pps_extension_present_flag == 1 {
            return Err(HevcError::Unsupported {
                code: "hevc.pps.extension",
            });
        }

        Ok(PicParameterSet {
            pic_parameter_set_id,
            seq_parameter_set_id,
            dependent_slice_segments_enabled,
            output_flag_present,
            num_extra_slice_header_bits,
            sign_data_hiding_enabled,
            cabac_init_present,
            num_ref_idx_l0_default_active_minus1,
            num_ref_idx_l1_default_active_minus1,
            init_qp_minus26,
            constrained_intra_pred,
            transform_skip_enabled,
            cu_qp_delta_enabled,
            diff_cu_qp_delta_depth,
            cb_qp_offset,
            cr_qp_offset,
            slice_chroma_qp_offsets_present,
            weighted_pred,
            weighted_bipred,
            transquant_bypass_enabled,
            tiles_enabled,
            entropy_coding_sync_enabled,
            num_tile_columns_minus1,
            num_tile_rows_minus1,
            uniform_spacing,
            loop_filter_across_tiles_enabled,
            loop_filter_across_slices_enabled,
            deblocking_filter_control_present,
            deblocking_filter_override_enabled,
            deblocking_filter_disabled,
            deblocking_offsets,
            scaling_list_data_present,
            lists_modification_present,
            log2_parallel_merge_level_minus2,
            slice_segment_header_extension_present,
        })
    }
}
