//! Slice segment header (ITU-T H.265 §7.3.6.1) — the **minimal
//! honest subset** of card 5.G1: first-slice flag, IRAP
//! `no_output_of_prior_pics_flag`, PPS id, dependent flag +
//! segment address, slice type, output flag, colour plane id, and
//! `slice_pic_order_cnt_lsb`.
//!
//! # What is deliberately NOT parsed (recorded, not smoothed)
//!
//! Everything after `slice_pic_order_cnt_lsb`: the short-term
//! reference-picture-set machinery (`short_term_ref_pic_set_sps_flag`
//! / slice-level `st_ref_pic_set()`), long-term RPS, temporal-MVP
//! candidates, SAO, deblocking, entry-point offsets and the whole
//! coded slice data. A full RPS parse needs retained SPS RPS state
//! this card intentionally does not model — the [`SliceSegmentHeader`]
//! stops at the honest boundary instead of guessing, and no
//! `rbsp_trailing_bits` check is attempted (the header is a prefix,
//! not the whole RBSP).
use super::nal::{BitReader, HevcError, NalHeader, unescape_rbsp_into};
use super::sps::SeqParameterSet;
use super::pps::PicParameterSet;

/// The SPS facts the slice-segment prefix needs (the minimal seam,
/// mirroring card 5.F1b's `SliceSpsInfo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceSegmentSpsInfo {
    /// `log2_max_pic_order_cnt_lsb` = minus4 + 4: the
    /// `slice_pic_order_cnt_lsb` read width.
    pub log2_max_pic_order_cnt_lsb: u32,
    /// `separate_colour_plane_flag` — gates `colour_plane_id`.
    pub separate_colour_plane: bool,
    /// `Ceil(Log2(PicSizeInCtbsY))`: the `slice_segment_address`
    /// read width.
    pub slice_segment_address_bits: u32,
}

impl SliceSegmentSpsInfo {
    /// Derive the seam from a parsed SPS.
    ///
    /// # Errors
    /// Propagates the SPS's CTB-grid arithmetic error.
    /// [`SeqParameterSet::pic_size_in_ctbs_y`]).
    pub fn from_sps(sps: &SeqParameterSet) -> Result<SliceSegmentSpsInfo, HevcError> {
        let pic_size = sps.pic_size_in_ctbs_y()?;
        Ok(SliceSegmentSpsInfo {
            log2_max_pic_order_cnt_lsb: u32::from(sps.log2_max_pic_order_cnt_lsb_minus4) + 4,
            separate_colour_plane: sps.separate_colour_plane,
            slice_segment_address_bits: ceil_log2(u64::from(pic_size)),
        })
    }
}

/// `Ceil(Log2(v))` for v ≥ 1 (bits to index v values).
fn ceil_log2(v: u64) -> u32 {
    let floor = 63 - v.leading_zeros();
    if v.count_ones() == 1 {
        floor
    } else {
        floor + 1
    }
}

/// The PPS facts the slice-segment prefix needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceSegmentPpsInfo {
    /// `dependent_slice_segments_enabled_flag` — gates
    /// `dependent_slice_segment_flag`.
    pub dependent_slice_segments_enabled: bool,
    /// `output_flag_present_flag` — gates `pic_output_flag`.
    pub output_flag_present: bool,
    /// `num_extra_slice_header_bits` — reserved flag bits read before
    /// `slice_type`.
    pub num_extra_slice_header_bits: u8,
}

impl SliceSegmentPpsInfo {
    /// Derive the seam from a parsed PPS.
    #[must_use]
    pub fn from_pps(pps: &PicParameterSet) -> SliceSegmentPpsInfo {
        SliceSegmentPpsInfo {
            dependent_slice_segments_enabled: pps.dependent_slice_segments_enabled,
            output_flag_present: pps.output_flag_present,
            num_extra_slice_header_bits: pps.num_extra_slice_header_bits,
        }
    }
}

/// `slice_type` (§7.3.6.1): 0 = B, 1 = P, 2 = I.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceType {
    /// 0 — B slice.
    B,
    /// 1 — P slice.
    P,
    /// 2 — I slice.
    I,
}

impl SliceType {
    /// From the raw ue value (0..=2; other values are spec-invalid
    /// and map to `None`).
    #[must_use]
    pub fn from_id(id: u64) -> Option<SliceType> {
        match id {
            0 => Some(SliceType::B),
            1 => Some(SliceType::P),
            2 => Some(SliceType::I),
            _ => None,
        }
    }

    /// The raw ue value.
    #[must_use]
    pub fn id(self) -> u64 {
        match self {
            SliceType::B => 0,
            SliceType::P => 1,
            SliceType::I => 2,
        }
    }
}

/// A parsed (prefix of a) slice segment header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceSegmentHeader {
    /// The NAL unit's type (carried into the header logic: IRAP
    /// types read `no_output_of_prior_pics_flag`, IDR types skip the
    /// POC lsb).
    pub unit_type: super::nal::NalUnitType,
    /// `nuh_layer_id` / `nuh_temporal_id_plus1` from the NAL header.
    pub layer_id: u8,
    pub temporal_id_plus1: u8,
    /// `first_slice_segment_in_pic_flag`.
    pub first_slice_segment_in_pic: bool,
    /// `no_output_of_prior_pics_flag` — only read for IRAP NAL types
    /// (16..=23).
    pub no_output_of_prior_pics: Option<bool>,
    /// `slice_pic_parameter_set_id` (0..=63).
    pub slice_pic_parameter_set_id: u8,
    /// `dependent_slice_segment_flag` — only read when the PPS
    /// enables it and the segment is not the first in its picture.
    pub dependent_slice_segment: Option<bool>,
    /// `slice_segment_address` — only read for non-first segments
    /// (CTB-grid index).
    pub slice_segment_address: Option<u32>,
    /// `slice_type` — only read for independent segments.
    pub slice_type: Option<SliceType>,
    /// `pic_output_flag` — only when the PPS carries
    /// `output_flag_present_flag`.
    pub pic_output_flag: Option<bool>,
    /// `colour_plane_id` — only when the SPS is
    /// `separate_colour_plane`.
    pub colour_plane_id: Option<u8>,
    /// `slice_pic_order_cnt_lsb` — only for non-IDR pictures (IDR
    /// types define POC = 0; §7.4.8).
    pub slice_pic_order_cnt_lsb: Option<u32>,
}

impl SliceSegmentHeader {
    /// Parse the minimal slice-segment-header prefix of a complete
    /// VCL NAL unit (two-byte header included), using the SPS/PPS
    /// seams. See the module docs for the honest-subset boundary.
    ///
    /// # Errors
    /// Typed [`HevcError`]s: non-VCL NAL type, header violations,
    /// emulation violations, truncation mid-syntax, out-of-range
    /// `slice_pic_parameter_set_id`/`slice_type`.
    pub fn parse(
        nal: &[u8],
        sps: &SliceSegmentSpsInfo,
        pps: &SliceSegmentPpsInfo,
    ) -> Result<SliceSegmentHeader, HevcError> {
        let (header, payload) = NalHeader::split(nal)?;
        if !header.unit_type.is_vcl() {
            return Err(HevcError::Invalid {
                code: "hevc.slice.not_vcl",
            });
        }
        let mut rbsp = Vec::with_capacity(nal.len());
        unescape_rbsp_into(payload, &mut rbsp)?;
        let mut r = BitReader::new(&rbsp);
        Self::read(&mut r, header, sps, pps)
    }

    /// The syntax walk (§7.3.6.1 prefix) over an RBSP bit reader.
    fn read(
        r: &mut BitReader<'_>,
        header: NalHeader,
        sps: &SliceSegmentSpsInfo,
        pps: &SliceSegmentPpsInfo,
    ) -> Result<SliceSegmentHeader, HevcError> {
        let first_slice_segment_in_pic = r.bit()? == 1;
        let no_output_of_prior_pics = if header.unit_type.is_irap() {
            Some(r.bit()? == 1)
        } else {
            None
        };
        let slice_pic_parameter_set_id = super::vps::bounded_ue(
            r,
            "slice_pic_parameter_set_id",
            63,
        )? as u8;

        let mut dependent_slice_segment = None;
        let mut slice_segment_address = None;
        if !first_slice_segment_in_pic {
            if pps.dependent_slice_segments_enabled {
                dependent_slice_segment = Some(r.bit()? == 1);
            }
            if dependent_slice_segment != Some(true) {
                slice_segment_address = Some(
                    r.bits(sps.slice_segment_address_bits
                        .clamp(1, 32))?,
                );
            }
        }

        let mut slice_type = None;
        let mut pic_output_flag = None;
        let mut colour_plane_id = None;
        let mut slice_pic_order_cnt_lsb = None;
        if dependent_slice_segment != Some(true) {
            for _ in 0..pps.num_extra_slice_header_bits {
                let _ = r.bit()?; // slice_reserved_flag[i]
            }
            let raw_type = slice_type_raw(r)?;
            slice_type = SliceType::from_id(raw_type);
            if pps.output_flag_present {
                pic_output_flag = Some(r.bit()? == 1);
            }
            if sps.separate_colour_plane {
                colour_plane_id = Some(r.bits(2)? as u8);
            }
            // IDR_W_RADL (19) and IDR_N_LP (20) define POC = 0 — no
            // lsb is coded. Every other VCL type codes it.
            if !matches!(
                header.unit_type,
                super::nal::NalUnitType::IdrWRadl | super::nal::NalUnitType::IdrNLp
            ) {
                slice_pic_order_cnt_lsb =
                    Some(r.bits(sps.log2_max_pic_order_cnt_lsb.clamp(1, 32))?);
            }
        }

        Ok(SliceSegmentHeader {
            unit_type: header.unit_type,
            layer_id: header.layer_id,
            temporal_id_plus1: header.temporal_id_plus1,
            first_slice_segment_in_pic,
            no_output_of_prior_pics,
            slice_pic_parameter_set_id,
            dependent_slice_segment,
            slice_segment_address,
            slice_type,
            pic_output_flag,
            colour_plane_id,
            slice_pic_order_cnt_lsb,
        })
    }
}

/// Helper: read the raw `slice_type` ue and range-check it.
fn slice_type_raw(r: &mut BitReader<'_>) -> Result<u64, HevcError> {
    let v = r.ue()?;
    if v > 2 {
        return Err(HevcError::OutOfRange {
            field: "slice_type",
            value: v,
            max: 2,
        });
    }
    Ok(v)
}
