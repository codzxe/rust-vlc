//! Picture parameter set (ITU-T H.264 §7.3.2.2) — clean-room parse of
//! the card's field list: ids, the entropy-coding mode (CAVLC/CABAC —
//! parsed, never claimed as a decode capability), the field-POC
//! bottom flag, slice-group count, reference-index defaults, weighted
//! prediction, QP/QS offsets, and the deblocking/redundant-count
//! flags.
//!
//! # FMO honesty
//!
//! `num_slice_groups_minus1 > 0` (flexible macroblock ordering — the
//! slice-group map topology of §7.3.2.2.1) is parsed up to the count
//! and then refused with a typed `Unsupported`: no real-world encoder
//! in our corpus (or, practically, any x264/ffmpeg-authored stream)
//! emits FMO, and modeling its seven map types buys nothing a
//! hardware-feed parser needs. The refusal is honest and observable,
//! never a silent skip.

use super::nal::{BitReader, H264Error, NalHeader, NalUnitType, unescape_rbsp_into};

/// A parsed picture parameter set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PicParameterSet {
    /// `pic_parameter_set_id` (0..=255).
    pub pic_parameter_set_id: u8,
    /// `seq_parameter_set_id` (0..=31) — the SPS this PPS pairs with.
    pub seq_parameter_set_id: u8,
    /// `entropy_coding_mode_flag`: `true` = CABAC, `false` = CAVLC
    /// (parse only — decoding entropy-coded data is out of scope).
    pub entropy_coding_mode: bool,
    /// `bottom_field_pic_order_in_frame_present_flag` (POC-in-slice
    /// bottom-field signalling).
    pub bottom_field_pic_order_in_frame_present: bool,
    /// `num_slice_groups_minus1` (0 everywhere in practice; > 0 is
    /// refused as FMO-Unsupported).
    pub num_slice_groups_minus1: u32,
    /// `num_ref_idx_l0_default_active_minus1` (0..=31).
    pub num_ref_idx_l0_default_active_minus1: u8,
    /// `num_ref_idx_l1_default_active_minus1` (0..=31).
    pub num_ref_idx_l1_default_active_minus1: u8,
    /// `weighted_pred_flag` (explicit P-slice weighting).
    pub weighted_pred: bool,
    /// `weighted_bipred_idc` (0..=2).
    pub weighted_bipred_idc: u8,
    /// `pic_init_qp_minus26` (−26..=25 → slice QP base 0..=51).
    pub pic_init_qp_minus26: i8,
    /// `pic_init_qs_minus26` (−26..=25, SP/SI slices).
    pub pic_init_qs_minus26: i8,
    /// `chroma_qp_index_offset` (−12..=12, doubled for Cr).
    pub chroma_qp_index_offset: i8,
    /// `deblocking_filter_control_present_flag`.
    pub deblocking_filter_control_present: bool,
    /// `constrained_intra_pred_flag`.
    pub constrained_intra_pred: bool,
    /// `redundant_pic_cnt_present_flag`.
    pub redundant_pic_cnt_present: bool,
}

impl PicParameterSet {
    /// Parse a complete PPS NAL unit (header byte included).
    ///
    /// # Errors
    /// Typed [`H264Error`]s: wrong NAL type, forbidden bit, emulation
    /// violations, truncation, out-of-range ids/QP offsets, FMO
    /// slice groups, bad trailing bits.
    pub fn parse(nal: &[u8]) -> Result<PicParameterSet, H264Error> {
        let mut rbsp = Vec::with_capacity(nal.len());
        Self::parse_with_buffer(nal, &mut rbsp)
    }

    /// [`PicParameterSet::parse`] reusing a caller-owned RBSP buffer.
    ///
    /// # Errors
    /// As [`PicParameterSet::parse`].
    pub fn parse_with_buffer(nal: &[u8], rbsp: &mut Vec<u8>) -> Result<PicParameterSet, H264Error> {
        let (header, payload) = NalHeader::split(nal)?;
        if header.unit_type != NalUnitType::Pps {
            return Err(H264Error::Invalid {
                code: "h264.pps.wrong_nal_type",
            });
        }
        unescape_rbsp_into(payload, rbsp)?;
        let mut r = BitReader::new(rbsp);
        let pps = Self::read(&mut r)?;
        r.finish()?;
        Ok(pps)
    }

    fn read(r: &mut BitReader<'_>) -> Result<PicParameterSet, H264Error> {
        let pic_parameter_set_id = bounded_ue(r, "pic_parameter_set_id", 255)? as u8;
        let seq_parameter_set_id = bounded_ue(r, "seq_parameter_set_id", 31)? as u8;
        let entropy_coding_mode = r.bit()? == 1;
        let bottom_field_pic_order_in_frame_present = r.bit()? == 1;
        let num_slice_groups_minus1 = bounded_ue(r, "num_slice_groups_minus1", 7)?;
        if num_slice_groups_minus1 > 0 {
            // FMO slice-group topology (§7.3.2.2.1): parsed-to-count,
            // then honestly refused (see module docs).
            return Err(H264Error::Unsupported {
                code: "h264.pps.fmo_slice_groups",
            });
        }
        let num_ref_idx_l0_default_active_minus1 =
            bounded_ue(r, "num_ref_idx_l0_default_active_minus1", 31)? as u8;
        let num_ref_idx_l1_default_active_minus1 =
            bounded_ue(r, "num_ref_idx_l1_default_active_minus1", 31)? as u8;
        let weighted_pred = r.bit()? == 1;
        let weighted_bipred_idc = r.bits(2)? as u8;
        let pic_init_qp_minus26 = bounded_se(r, "pic_init_qp_minus26", -26, 25)?;
        let pic_init_qs_minus26 = bounded_se(r, "pic_init_qs_minus26", -26, 25)?;
        let chroma_qp_index_offset = bounded_se(r, "chroma_qp_index_offset", -12, 12)?;
        let deblocking_filter_control_present = r.bit()? == 1;
        let constrained_intra_pred = r.bit()? == 1;
        let redundant_pic_cnt_present = r.bit()? == 1;
        Ok(PicParameterSet {
            pic_parameter_set_id,
            seq_parameter_set_id,
            entropy_coding_mode,
            bottom_field_pic_order_in_frame_present,
            num_slice_groups_minus1: 0,
            num_ref_idx_l0_default_active_minus1,
            num_ref_idx_l1_default_active_minus1,
            weighted_pred,
            weighted_bipred_idc,
            pic_init_qp_minus26: pic_init_qp_minus26 as i8,
            pic_init_qs_minus26: pic_init_qs_minus26 as i8,
            chroma_qp_index_offset: chroma_qp_index_offset as i8,
            deblocking_filter_control_present,
            constrained_intra_pred,
            redundant_pic_cnt_present,
        })
    }

    /// Does this PPS reference an SPS we actually hold? (The wiring
    /// check the hw-feed adapter will run per slice.)
    #[must_use]
    pub fn references(&self, sps_ids: &[u8]) -> bool {
        sps_ids.contains(&self.seq_parameter_set_id)
    }
}

/// A `ue(v)` read with a spec-declared upper bound (card 5.F1 error
/// path: ids and counts outside their ranges are corruption).
fn bounded_ue(r: &mut BitReader<'_>, field: &'static str, max: u64) -> Result<u64, H264Error> {
    let v = r.ue()?;
    if v > max {
        return Err(H264Error::OutOfRange {
            field,
            value: v,
            max,
        });
    }
    Ok(v)
}

/// An `se(v)` read with a declared inclusive range.
fn bounded_se(
    r: &mut BitReader<'_>,
    field: &'static str,
    min: i64,
    max: i64,
) -> Result<i64, H264Error> {
    let v = r.se()?;
    if v < min || v > max {
        return Err(H264Error::OutOfRange {
            field,
            value: v.unsigned_abs(),
            max: max.unsigned_abs(),
        });
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A CABAC High-profile PPS from a real corpus encode
    /// (0014-muxed-av.mkv's avcC; Main-sized, weighted-pred off,
    /// QP offset −2): `68 fb e3 71 a0 9d 80 80 c0 c0`-class bytes
    /// are built below instead from the bit grammar, so the fixture
    /// stays self-describing. See `sps.rs` tests for a real-dump PPS.
    #[test]
    fn crafted_pps_covers_the_full_field_list() {
        // Bits: pps_id ue0 "1", sps_id ue0 "1", entropy 1,
        // bottom 0, slice_groups ue0 "1", l0 ue2 "011",
        // l1 ue0 "1", weighted_pred 1, bipred "10",
        // qp se(-5)=codeNum9 "000101010", qs se(1)=codeNum2 "011",
        // chroma_qp se(0)=codeNum0... (se 0 is codeNum 1 → "010";
        // se −1 is codeNum 2 → "011"), deblock 1, constrained 1,
        // redundant 0, stop.
        let mut bits: Vec<bool> = Vec::new();
        let push_ue = |bits: &mut Vec<bool>, v: u64| {
            let payload = v + 1;
            let len = 64 - payload.leading_zeros() as usize;
            for _ in 0..len - 1 {
                bits.push(false);
            }
            for i in (0..len).rev() {
                bits.push(payload & (1 << i) != 0);
            }
        };
        push_ue(&mut bits, 0); // pps id
        push_ue(&mut bits, 0); // sps id
        bits.push(true); // entropy_coding_mode → CABAC
        bits.push(false); // bottom_field_pic_order
        push_ue(&mut bits, 0); // slice groups
        push_ue(&mut bits, 2); // l0 default active −1 = 2
        push_ue(&mut bits, 0); // l1 default active −1 = 0
        bits.push(true); // weighted_pred
        bits.push(true); // weighted_bipred_idc bit 1
        bits.push(false); // …bit 0 → idc 2
        // se(−5) = codeNum 10 (k even → negative magnitude 5).
        push_ue(&mut bits, 10);
        // se(+1) = codeNum 1 ("1").
        push_ue(&mut bits, 1);
        // chroma_qp se(+3) = codeNum 5 → ue 5.
        push_ue(&mut bits, 5);
        bits.push(true); // deblocking
        bits.push(true); // constrained_intra
        bits.push(false); // redundant_pic_cnt
        bits.push(true); // stop
        let mut bytes = vec![0x68u8];
        for chunk in bits.chunks(8) {
            let mut byte = 0u8;
            for (i, &b) in chunk.iter().enumerate() {
                if b {
                    byte |= 1 << (7 - i);
                }
            }
            bytes.push(byte);
        }
        let pps = PicParameterSet::parse(&bytes).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(pps.pic_parameter_set_id, 0);
        assert_eq!(pps.seq_parameter_set_id, 0);
        assert!(pps.entropy_coding_mode); // CABAC
        assert!(!pps.bottom_field_pic_order_in_frame_present);
        assert_eq!(pps.num_slice_groups_minus1, 0);
        assert_eq!(pps.num_ref_idx_l0_default_active_minus1, 2);
        assert_eq!(pps.num_ref_idx_l1_default_active_minus1, 0);
        assert!(pps.weighted_pred);
        assert_eq!(pps.weighted_bipred_idc, 2);
        assert_eq!(pps.pic_init_qp_minus26, -5);
        assert_eq!(pps.pic_init_qs_minus26, 1);
        assert_eq!(pps.chroma_qp_index_offset, 3);
        assert!(pps.deblocking_filter_control_present);
        assert!(pps.constrained_intra_pred);
        assert!(!pps.redundant_pic_cnt_present);
        // Pairing check against a held SPS table.
        assert!(pps.references(&[0, 1]));
        assert!(!pps.references(&[1, 2]));
    }

    #[test]
    fn error_paths_are_typed() {
        // Wrong NAL type.
        assert_eq!(
            PicParameterSet::parse(&[0x67, 0x80]),
            Err(H264Error::Invalid {
                code: "h264.pps.wrong_nal_type"
            })
        );
        // Forbidden bit.
        assert_eq!(
            PicParameterSet::parse(&[0xe8, 0x80]),
            Err(H264Error::ForbiddenZeroBit)
        );
        // Truncated: header + a lone ue prefix bit is not enough.
        assert!(matches!(
            PicParameterSet::parse(&[0x68]),
            Err(H264Error::Truncated { .. })
        ));
        // sps id above 31: pps_id ue0, then ue(32).
        let mut bits: Vec<bool> = Vec::new();
        let push_ue = |bits: &mut Vec<bool>, v: u64| {
            let payload = v + 1;
            let len = 64 - payload.leading_zeros() as usize;
            for _ in 0..len - 1 {
                bits.push(false);
            }
            for i in (0..len).rev() {
                bits.push(payload & (1 << i) != 0);
            }
        };
        push_ue(&mut bits, 0); // pps id
        push_ue(&mut bits, 32); // sps id — out of range
        bits.push(true); // stop
        let mut bytes = vec![0x68u8];
        for chunk in bits.chunks(8) {
            let mut byte = 0u8;
            for (i, &b) in chunk.iter().enumerate() {
                if b {
                    byte |= 1 << (7 - i);
                }
            }
            bytes.push(byte);
        }
        assert_eq!(
            PicParameterSet::parse(&bytes),
            Err(H264Error::OutOfRange {
                field: "seq_parameter_set_id",
                value: 32,
                max: 31
            })
        );
        // QP offset outside the 0..=51 window: pic_init_qp se(−40).
        let mut bits: Vec<bool> = Vec::new();
        push_ue(&mut bits, 0);
        push_ue(&mut bits, 0);
        bits.push(false); // entropy
        bits.push(false); // bottom
        push_ue(&mut bits, 0); // slice groups
        push_ue(&mut bits, 0); // l0
        push_ue(&mut bits, 0); // l1
        bits.push(false); // weighted pred
        bits.push(false); // bipred bit
        bits.push(false); // bipred bit
        push_ue(&mut bits, 80); // se(−40) = codeNum 80
        bits.push(true); // stop
        let mut bytes = vec![0x68u8];
        for chunk in bits.chunks(8) {
            let mut byte = 0u8;
            for (i, &b) in chunk.iter().enumerate() {
                if b {
                    byte |= 1 << (7 - i);
                }
            }
            bytes.push(byte);
        }
        assert!(matches!(
            PicParameterSet::parse(&bytes),
            Err(H264Error::OutOfRange {
                field: "pic_init_qp_minus26",
                ..
            })
        ));
    }

    #[test]
    fn fmo_slice_groups_are_unsupported_not_silent() {
        // pps_id 0, sps_id 0, then num_slice_groups_minus1 = 1.
        let mut bits: Vec<bool> = Vec::new();
        let push_ue = |bits: &mut Vec<bool>, v: u64| {
            let payload = v + 1;
            let len = 64 - payload.leading_zeros() as usize;
            for _ in 0..len - 1 {
                bits.push(false);
            }
            for i in (0..len).rev() {
                bits.push(payload & (1 << i) != 0);
            }
        };
        push_ue(&mut bits, 0);
        push_ue(&mut bits, 0);
        bits.push(false); // entropy
        bits.push(false); // bottom
        push_ue(&mut bits, 1); // slice groups − 1 = 1 → FMO
        bits.push(true); // stop
        let mut bytes = vec![0x68u8];
        for chunk in bits.chunks(8) {
            let mut byte = 0u8;
            for (i, &b) in chunk.iter().enumerate() {
                if b {
                    byte |= 1 << (7 - i);
                }
            }
            bytes.push(byte);
        }
        assert_eq!(
            PicParameterSet::parse(&bytes),
            Err(H264Error::Unsupported {
                code: "h264.pps.fmo_slice_groups"
            })
        );
    }
}
