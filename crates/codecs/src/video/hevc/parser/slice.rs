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
use super::pps::PicParameterSet;
use super::sps::SeqParameterSet;

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
        let slice_pic_parameter_set_id =
            super::vps::bounded_ue(r, "slice_pic_parameter_set_id", 63)? as u8;

        let mut dependent_slice_segment = None;
        let mut slice_segment_address = None;
        if !first_slice_segment_in_pic {
            if pps.dependent_slice_segments_enabled {
                dependent_slice_segment = Some(r.bit()? == 1);
            }
            if dependent_slice_segment != Some(true) {
                slice_segment_address = Some(r.bits(sps.slice_segment_address_bits.clamp(1, 32))?);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::hevc::parser::nal::tests::Bits;
    use crate::video::hevc::parser::nal::{AnnexBNals, NalHeader, NalUnitType};

    /// A vanilla seam: 8-bit POC lsb, no colour planes, no extra
    /// header bits, dependent segments + output flag enabled so the
    /// gated fields fire.
    fn seam() -> (SliceSegmentSpsInfo, SliceSegmentPpsInfo) {
        (
            SliceSegmentSpsInfo {
                log2_max_pic_order_cnt_lsb: 8,
                separate_colour_plane: false,
                slice_segment_address_bits: 6,
            },
            SliceSegmentPpsInfo {
                dependent_slice_segments_enabled: true,
                output_flag_present: true,
                num_extra_slice_header_bits: 0,
            },
        )
    }

    #[test]
    fn idr_first_slice_reads_the_honest_prefix() {
        // IDR_W_RADL (19), first segment: first flag, IRAP
        // no_output flag, PPS id 0, slice_type I — and *no* POC lsb
        // (IDR defines POC = 0, §7.4.8).
        let mut b = Bits::new();
        b.flag(true); // first_slice_segment_in_pic
        b.flag(false); // no_output_of_prior_pics (IRAP-gated)
        b.ue(0); // slice_pic_parameter_set_id
        b.ue(2); // slice_type = I
        let nal = b.nal(19);
        let (sps, pps) = seam();
        let h = SliceSegmentHeader::parse(&nal, &sps, &pps).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(h.unit_type, NalUnitType::IdrWRadl);
        assert!(h.first_slice_segment_in_pic);
        assert_eq!(h.no_output_of_prior_pics, Some(false));
        assert_eq!(h.slice_pic_parameter_set_id, 0);
        assert_eq!(h.slice_type, Some(SliceType::I));
        assert_eq!(h.slice_pic_order_cnt_lsb, None);
        assert_eq!(h.dependent_slice_segment, None);
        assert_eq!(h.slice_segment_address, None);
    }

    #[test]
    fn trail_r_b_slice_reads_poc_and_gated_flags() {
        // TRAIL_R (1), first, PPS id 3, B slice, output flag set,
        // colour plane 2, POC lsb 0x5a over 8 bits.
        let mut b = Bits::new();
        b.flag(true); // first
        b.ue(3); // pps id
        b.ue(0); // slice_type = B
        b.flag(true); // pic_output_flag (PPS-gated)
        b.put(2, 2); // colour_plane_id (SPS-gated)
        b.put(8, 0x5a); // slice_pic_order_cnt_lsb
        let nal = b.nal(1);
        let (mut sps, pps) = seam();
        sps.separate_colour_plane = true;
        let h = SliceSegmentHeader::parse(&nal, &sps, &pps).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(h.slice_pic_parameter_set_id, 3);
        assert_eq!(h.slice_type, Some(SliceType::B));
        assert_eq!(h.pic_output_flag, Some(true));
        assert_eq!(h.colour_plane_id, Some(2));
        assert_eq!(h.slice_pic_order_cnt_lsb, Some(0x5a));
        assert_eq!(h.no_output_of_prior_pics, None, "non-IRAP reads none");
    }

    #[test]
    fn non_first_independent_segment_reads_ctb_address() {
        // Spec order: first flag, PPS id ue, then (non-first) the
        // dependent flag + CTB address, then the shared prefix.
        let mut b = Bits::new();
        b.flag(false); // not first
        b.ue(0); // slice_pic_parameter_set_id
        b.flag(false); // dependent_slice_segment_flag (PPS-gated)
        b.put(6, 9); // slice_segment_address over ceil(log2 PicCtbs)
        b.ue(1); // slice_type = P
        b.flag(false); // pic_output_flag (PPS-gated)
        b.put(8, 0x21); // POC lsb
        let nal = b.nal(1);
        let (sps, pps) = seam();
        let h = SliceSegmentHeader::parse(&nal, &sps, &pps).unwrap_or_else(|e| unreachable!("{e}"));
        assert!(!h.first_slice_segment_in_pic);
        assert_eq!(h.dependent_slice_segment, Some(false));
        assert_eq!(h.slice_segment_address, Some(9));
        assert_eq!(h.slice_type, Some(SliceType::P));
        assert_eq!(h.slice_pic_order_cnt_lsb, Some(0x21));
    }

    #[test]
    fn dependent_segment_stops_before_the_shared_syntax() {
        // Dependent segments carry no address/type/POC at all — the
        // prefix boundary moves up for them (§7.3.6.1).
        let mut b = Bits::new();
        b.flag(false);
        b.flag(true); // dependent
        let nal = b.nal(1);
        let (sps, pps) = seam();
        let h = SliceSegmentHeader::parse(&nal, &sps, &pps).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(h.dependent_slice_segment, Some(true));
        assert_eq!(h.slice_segment_address, None);
        assert_eq!(h.slice_type, None);
        assert_eq!(h.slice_pic_order_cnt_lsb, None);
    }

    #[test]
    fn extra_slice_header_bits_and_cra_nuance() {
        // CRA_NUT (21): IRAP flag read, POC lsb *is* coded (only IDR
        // types define POC 0), plus PPS reserved flag bits first.
        // Local seam: 2 extra header bits, no output flag.
        let mut b = Bits::new();
        b.flag(true);
        b.flag(true); // no_output_of_prior_pics (IRAP-gated)
        b.ue(0);
        b.put(2, 0b11); // num_extra_slice_header_bits = 2 reserved
        b.ue(2); // I
        b.put(8, 0x7f); // POC lsb present on CRA
        let nal = b.nal(21);
        let (sps, mut pps) = seam();
        pps.num_extra_slice_header_bits = 2;
        pps.output_flag_present = false;
        let h = SliceSegmentHeader::parse(&nal, &sps, &pps).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(h.unit_type, NalUnitType::CraNut);
        assert_eq!(h.no_output_of_prior_pics, Some(true));
        assert_eq!(h.slice_pic_order_cnt_lsb, Some(0x7f));
    }

    #[test]
    fn typed_error_paths() {
        let (sps, pps) = seam();
        // Non-VCL NAL: refused before any syntax walk.
        let vps = Bits::new().nal(32);
        assert_eq!(
            SliceSegmentHeader::parse(&vps, &sps, &pps),
            Err(HevcError::Invalid {
                code: "hevc.slice.not_vcl"
            })
        );
        // slice_pic_parameter_set_id out of range (64 > 63). The
        // IRAP no_output flag is read before the ue.
        let mut b = Bits::new();
        b.flag(true);
        b.flag(false);
        b.ue(64);
        assert!(matches!(
            SliceSegmentHeader::parse(&b.nal(19), &sps, &pps),
            Err(HevcError::OutOfRange {
                field: "slice_pic_parameter_set_id",
                ..
            })
        ));
        // slice_type out of range (5 > 2).
        let mut b = Bits::new();
        b.flag(true);
        b.flag(false);
        b.ue(0);
        b.ue(5);
        assert!(matches!(
            SliceSegmentHeader::parse(&b.nal(19), &sps, &pps),
            Err(HevcError::OutOfRange {
                field: "slice_type",
                ..
            })
        ));
        // Truncated mid-syntax: only the first flag survives.
        let mut b = Bits::new();
        b.flag(true);
        b.flag(false);
        let mut nal = b.nal(19);
        nal.truncate(3); // header + 1 byte of RBSP
        assert!(matches!(
            SliceSegmentHeader::parse(&nal, &sps, &pps),
            Err(HevcError::Truncated { .. })
        ));
    }

    #[test]
    fn seam_derivation_matches_the_0100_sps() {
        // The real 0100 SPS: 1024x768, 32×32 CTBs → 32×24 = 192
        // CTBs → 10 address bits; log2_max_poc_lsb = 8 (both facts
        // cross-checked against an independent bit-level walk).
        let nal = crate::video::hevc::parser::nal::tests::SPS_0100;
        let sps = crate::video::hevc::parser::sps::SeqParameterSet::parse(&nal)
            .unwrap_or_else(|e| unreachable!("{e}"));
        let seam = SliceSegmentSpsInfo::from_sps(&sps).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(seam.log2_max_pic_order_cnt_lsb, 8);
        assert!(!seam.separate_colour_plane);
        assert_eq!(seam.slice_segment_address_bits, 10);
        let pps_nal = crate::video::hevc::parser::nal::tests::PPS_0100;
        let pps = crate::video::hevc::parser::pps::PicParameterSet::parse(&pps_nal)
            .unwrap_or_else(|e| unreachable!("{e}"));
        let pps_seam = SliceSegmentPpsInfo::from_pps(&pps);
        assert!(!pps_seam.dependent_slice_segments_enabled);
        assert!(!pps_seam.output_flag_present);
        assert_eq!(pps_seam.num_extra_slice_header_bits, 0);
    }

    #[test]
    fn ceil_log2_table() {
        assert_eq!(ceil_log2(1), 0);
        assert_eq!(ceil_log2(2), 1);
        assert_eq!(ceil_log2(3), 2);
        assert_eq!(ceil_log2(4), 2);
        assert_eq!(ceil_log2(192), 8);
        assert_eq!(ceil_log2(256), 8);
    }

    #[test]
    fn annexb_walk_of_vcl_and_parameter_sets_classifies() {
        // Smoke: the two-byte header split + is_vcl agree with the
        // slice parser's refusal of parameter-set NALs.
        let mut stream = Vec::new();
        stream.extend_from_slice(&[0x00, 0x00, 0x01]);
        stream.extend(Bits::new().nal(32));
        stream.extend_from_slice(&[0x00, 0x00, 0x01]);
        stream.extend(Bits::new().nal(1));
        let types: Vec<_> = AnnexBNals::new(&stream)
            .filter_map(|n| NalHeader::split(n).ok().map(|(h, _)| h.unit_type))
            .collect();
        assert_eq!(types.len(), 2);
        assert!(!types[0].is_vcl());
        assert!(types[1].is_vcl());
    }
}
