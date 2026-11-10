//! H.264 slice-header parser (card 5.F1b; ITU-T H.264 (03/2023)
//! §7.3.3 `slice_header()`, §7.3.3.1–7.3.3.3 sub-syntaxes).
//!
//! Clean-room from the spec text, parse-level only: the header is
//! read in CAVLC **syntax order** up to and including the deblocking
//! filter control — **no entropy-coded macroblock data is touched**
//! (that, and any decode claim, belongs to a later card).
//!
//! # Context seams (the F1a/F1b unification point)
//!
//! A slice header cannot be parsed statelessly: its field widths and
//! optional-field gates come from the *active* SPS and PPS (e.g.
//! `frame_num` is `u(log2_max_frame_num_minus4 + 4)` bits, POC
//! syntax depends on `pic_order_cnt_type`). Card 5.F1a (parallel
//! agent) owns the real SPS/PPS parsers, so this module defines the
//! **minimal projection** it consumes — [`SliceSpsInfo`] and
//! [`SlicePpsInfo`] — as plain data. F1a's parsed parameter sets can
//! populate these at Lead merge; until then callers (and the corpus
//! tests here, via a test-only bootstrap reader) fill them in.
//!
//! # Damage model
//!
//! Every failure is a typed [`SliceHeaderError`]: truncated
//! mid-bitread, out-of-range parameter-set id / slice type, spec-less
//! `memory_management_control_operation` codes, FMO slice groups
//! (honest [`SliceHeaderError::FmoUnsupported`] —
//! `slice_group_change_cycle` needs PPS/SPS fields outside the
//! minimal seam, and FMO is absent from every real corpus stream),
//! and op-count budgets that bound malformed-stream loops. No panic
//! on any input; loops always consume bits, so they terminate.

use core::fmt;

/// MSB-first bit reader over an RBSP slice (§9: `u(n)`, `ue(v)`,
/// `se(v)`).
///
/// Lives here because the slice header is this card's consumer; at
/// Lead merge it should hoist to `h264/parser/bitreader.rs` and serve
/// F1a's SPS/PPS/NAL parsers too (same reading semantics).
#[derive(Debug, Clone, Copy)]
pub struct RbspBitReader<'a> {
    data: &'a [u8],
    bit_pos: usize,
}

/// Typed bit-read failure (truncation or an out-of-range
/// Exp-Golomb code).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitReadError {
    /// The RBSP ended before the field's bits were available.
    Truncated {
        /// Bits the pending read still needed at the point of failure.
        needed: usize,
        /// Bits the RBSP still had at that point.
        available: usize,
    },
    /// An `ue(v)` code with more than 31 leading zeros (cannot be a
    /// spec-range value in any field this parser reads).
    GolombOverflow {
        /// Leading zeros observed before the marker bit.
        leading_zeros: u32,
    },
}

impl fmt::Display for BitReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            BitReadError::Truncated { needed, available } => write!(
                f,
                "rbsp truncated mid-read: needed {needed} more bits, {available} available"
            ),
            BitReadError::GolombOverflow { leading_zeros } => write!(
                f,
                "exp-golomb code with {leading_zeros} leading zeros (max 31)"
            ),
        }
    }
}

impl std::error::Error for BitReadError {}

impl<'a> RbspBitReader<'a> {
    /// Reader over a full RBSP (bit 0 = MSB of byte 0).
    pub fn new(data: &'a [u8]) -> RbspBitReader<'a> {
        RbspBitReader { data, bit_pos: 0 }
    }

    /// Bits not yet consumed.
    pub fn bits_left(&self) -> usize {
        self.data
            .len()
            .saturating_mul(8)
            .saturating_sub(self.bit_pos)
    }

    /// Bit position of the next read (parse-progress diagnostics).
    pub fn bit_pos(&self) -> usize {
        self.bit_pos
    }

    pub(crate) fn read_bit(&mut self) -> Result<bool, BitReadError> {
        if self.bit_pos >= self.data.len() * 8 {
            return Err(BitReadError::Truncated {
                needed: 1,
                available: 0,
            });
        }
        let byte = self.data[self.bit_pos / 8];
        let bit = 7 - (self.bit_pos % 8) as u32;
        self.bit_pos += 1;
        Ok(byte & (1 << bit) != 0)
    }

    /// Read `n` bits MSB-first (`u(n)`; `n <= 32`).
    ///
    /// # Errors
    /// [`BitReadError::Truncated`] if the RBSP ends first.
    pub fn read_bits(&mut self, n: u32) -> Result<u32, BitReadError> {
        let mut acc: u32 = 0;
        for _ in 0..n {
            let bit = self.read_bit()?;
            acc = (acc << 1) | u32::from(bit);
        }
        Ok(acc)
    }

    /// Read one `ue(v)` Exp-Golomb code (§9.1), value `0..=2^32-1`.
    ///
    /// # Errors
    /// [`BitReadError::Truncated`] / [`BitReadError::GolombOverflow`].
    pub fn read_ue(&mut self) -> Result<u32, BitReadError> {
        let mut leading_zeros: u32 = 0;
        loop {
            if self.read_bit()? {
                break;
            }
            leading_zeros += 1;
            if leading_zeros > 31 {
                return Err(BitReadError::GolombOverflow { leading_zeros });
            }
        }
        // codeNum = 2^leadingZeros − 1 + read_bits(leadingZeros)
        let info = if leading_zeros == 0 {
            0
        } else {
            u64::from(self.read_bits(leading_zeros)?)
        };
        let value = (1u64 << leading_zeros) - 1 + info;
        u32::try_from(value).map_err(|_| BitReadError::GolombOverflow { leading_zeros })
    }

    /// Read one `se(v)` signed Exp-Golomb code (§9.1.1).
    ///
    /// # Errors
    /// [`BitReadError::GolombOverflow`] when the mapped value exceeds
    /// `i32::MIN..=i32::MAX`, plus whatever
    /// [`RbspBitReader::read_ue`] reports.
    pub fn read_se(&mut self) -> Result<i32, BitReadError> {
        let k = u64::from(self.read_ue()?);
        // k odd → (k+1)/2, k even → −k/2 (§9.1.1 Table 9-3).
        let mapped: i64 = if k % 2 == 1 {
            ((k + 1).div_ceil(2).min(i32::MAX as u64)) as i64
        } else {
            -(k as i64 / 2)
        };
        i32::try_from(mapped).map_err(|_| BitReadError::GolombOverflow {
            leading_zeros: k.leading_zeros(),
        })
    }
}

/// Typed slice-header parse failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceHeaderError {
    /// Bit-level truncation or Exp-Golomb range violation.
    Bit(BitReadError),
    /// `slice_type` outside the 0..=9 vocabulary of Table 7-7.
    InvalidSliceType(u32),
    /// `pic_parameter_set_id` outside 0..=255 (§7.4.2.2).
    InvalidPpsId(u32),
    /// `memory_management_control_operation` outside 0..=6.
    InvalidMmco(u32),
    /// `modification_of_pic_nums_idc` outside 0..=5.
    InvalidModOfPicNumsIdc(u32),
    /// `num_ref_idx_lX_active_minus1` outside 0..=31 (§7.4.3.1 —
    /// also the allocation bound for `pred_weight_table`).
    InvalidNumRefIdxActive {
        /// 0 or 1 — which list.
        list: u8,
        /// The offending value.
        value: u32,
    },
    /// `disable_deblocking_filter_idc` outside 0..=2.
    InvalidDeblockIdc(u32),
    /// FMO slice groups: out of the minimal-seam scope (honest
    /// refusal, see module docs).
    FmoUnsupported {
        /// `num_slice_groups_minus1` from the PPS.
        num_slice_groups_minus1: u32,
    },
    /// A malformed-stream loop budget tripped (spec-conformant
    /// streams never do: ref-list modifications per list are ≤ 32,
    /// adaptive-marking operations ≤ 32).
    OpBudgetExceeded {
        /// Which loop (`"ref_pic_list_modification"` or
        /// `"dec_ref_pic_marking"`).
        kind: &'static str,
        /// Operations consumed when the budget tripped.
        ops: usize,
    },
    /// The supplied SPS/PPS context itself is out of spec range.
    InvalidContext(&'static str),
    /// The NAL unit is not a VCL slice (nal_unit_type 1 or 5).
    NotAVclNal {
        /// The offending `nal_unit_type`.
        nal_unit_type: u8,
    },
    /// The NAL header byte has `forbidden_zero_bit` set.
    ForbiddenBitSet,
}

impl fmt::Display for SliceHeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SliceHeaderError::Bit(e) => write!(f, "slice header bit read: {e}"),
            SliceHeaderError::InvalidSliceType(v) => {
                write!(f, "slice_type {v} outside 0..=9 (Table 7-7)")
            }
            SliceHeaderError::InvalidPpsId(v) => {
                write!(f, "pic_parameter_set_id {v} outside 0..=255")
            }
            SliceHeaderError::InvalidMmco(v) => {
                write!(f, "memory_management_control_operation {v} outside 0..=6")
            }
            SliceHeaderError::InvalidModOfPicNumsIdc(v) => {
                write!(f, "modification_of_pic_nums_idc {v} outside 0..=5")
            }
            SliceHeaderError::InvalidNumRefIdxActive { list, value } => write!(
                f,
                "num_ref_idx_l{list}_active_minus1 = {value} outside 0..=31"
            ),
            SliceHeaderError::InvalidDeblockIdc(v) => {
                write!(f, "disable_deblocking_filter_idc {v} outside 0..=2")
            }
            SliceHeaderError::FmoUnsupported {
                num_slice_groups_minus1,
            } => write!(
                f,
                "FMO: num_slice_groups_minus1 = {num_slice_groups_minus1} (out of parse scope)"
            ),
            SliceHeaderError::OpBudgetExceeded { kind, ops } => {
                write!(f, "{kind} op budget exceeded at {ops} operations")
            }
            SliceHeaderError::InvalidContext(what) => {
                write!(f, "out-of-range SPS/PPS context: {what}")
            }
            SliceHeaderError::NotAVclNal { nal_unit_type } => {
                write!(f, "nal_unit_type {nal_unit_type} is not a VCL slice")
            }
            SliceHeaderError::ForbiddenBitSet => {
                write!(f, "NAL header forbidden_zero_bit is set")
            }
        }
    }
}

impl std::error::Error for SliceHeaderError {}

impl From<BitReadError> for SliceHeaderError {
    fn from(e: BitReadError) -> Self {
        SliceHeaderError::Bit(e)
    }
}

/// Typed POC-arithmetic failure (the §8.2 helpers below).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PocError {
    /// `pic_order_cnt_msb` computation overflowed `i64`.
    Type0WrapOverflow,
    /// Type-2 POC (`2·(FrameNumOffset+FrameNum)`) overflowed `i64`.
    Type2Overflow,
}

impl fmt::Display for PocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            PocError::Type0WrapOverflow => {
                write!(f, "pic_order_cnt_msb wrap arithmetic overflowed")
            }
            PocError::Type2Overflow => {
                write!(f, "type-2 pic order cnt arithmetic overflowed")
            }
        }
    }
}

impl std::error::Error for PocError {}

/// Minimal SPS projection the slice header consumes (see module docs:
/// the F1a/F1b unification seam). Field names follow §7.3.2.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceSpsInfo {
    /// `log2_max_frame_num_minus4` (0..=12) → `frame_num` width 4..16.
    pub log2_max_frame_num_minus4: u8,
    /// `pic_order_cnt_type` (0..=2).
    pub pic_order_cnt_type: u8,
    /// `log2_max_pic_order_cnt_lsb_minus4` (0..=12) — only read when
    /// `pic_order_cnt_type == 0`.
    pub log2_max_pic_order_cnt_lsb_minus4: u8,
    /// `delta_pic_order_always_zero_flag` — when set, the type-1
    /// `delta_pic_order_cnt` syntax is absent.
    pub delta_pic_order_always_zero_flag: bool,
    /// `frame_mbs_only_flag` — when clear, the `field_pic_flag`
    /// syntax is present.
    pub frame_mbs_only_flag: bool,
    /// `separate_colour_plane_flag` (only settable when
    /// `chroma_format_idc == 3`).
    pub separate_colour_plane_flag: bool,
    /// `chroma_format_idc` (0..=3) — gates the chroma half of the
    /// optional `pred_weight_table()`.
    pub chroma_format_idc: u8,
}

impl SliceSpsInfo {
    /// `MaxPicOrderCntLsb` = `2^(log2_max_pic_order_cnt_lsb_minus4+4)`.
    pub fn max_pic_order_cnt_lsb(&self) -> u32 {
        1u32 << (u32::from(self.log2_max_pic_order_cnt_lsb_minus4) + 4)
    }

    /// `MaxFrameNum` = `2^(log2_max_frame_num_minus4+4)`.
    pub fn max_frame_num(&self) -> u32 {
        1u32 << (u32::from(self.log2_max_frame_num_minus4) + 4)
    }

    /// `ChromaArrayType` (§7.4.2.1.1): 0 when colour planes are
    /// separate, else the SPS `chroma_format_idc`.
    pub fn chroma_array_type(&self) -> u8 {
        if self.separate_colour_plane_flag {
            0
        } else {
            self.chroma_format_idc
        }
    }

    fn validate(&self) -> Result<(), SliceHeaderError> {
        if self.log2_max_frame_num_minus4 > 12 {
            return Err(SliceHeaderError::InvalidContext(
                "log2_max_frame_num_minus4 > 12",
            ));
        }
        if self.pic_order_cnt_type > 2 {
            return Err(SliceHeaderError::InvalidContext("pic_order_cnt_type > 2"));
        }
        if self.log2_max_pic_order_cnt_lsb_minus4 > 12 {
            return Err(SliceHeaderError::InvalidContext(
                "log2_max_pic_order_cnt_lsb_minus4 > 12",
            ));
        }
        if self.chroma_format_idc > 3 {
            return Err(SliceHeaderError::InvalidContext("chroma_format_idc > 3"));
        }
        Ok(())
    }
}

/// Minimal PPS projection the slice header consumes (F1a/F1b seam;
/// names follow §7.3.2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlicePpsInfo {
    /// `bottom_field_pic_order_in_frame_present_flag` (the "pic order
    /// present" flag of older spec drafts) — gates
    /// `delta_pic_order_cnt_bottom` and the second delta.
    pub bottom_field_pic_order_in_frame_present_flag: bool,
    /// `redundant_pic_cnt_present_flag`.
    pub redundant_pic_cnt_present_flag: bool,
    /// `weighted_pred_flag` — gates `pred_weight_table()` in P/SP.
    pub weighted_pred_flag: bool,
    /// `weighted_bipred_idc` (0..=3) — 1 gates `pred_weight_table()`
    /// in B slices.
    pub weighted_bipred_idc: u8,
    /// `entropy_coding_mode_flag` (CABAC) — gates `cabac_init_idc`.
    pub entropy_coding_mode_flag: bool,
    /// `deblocking_filter_control_present_flag` — gates the deblock
    /// idc / offset syntax.
    pub deblocking_filter_control_present_flag: bool,
    /// `num_slice_groups_minus1`: 0 = no FMO (anything else is
    /// refused typed, see module docs).
    pub num_slice_groups_minus1: u32,
    /// `num_ref_idx_l0_default_active_minus1`.
    pub num_ref_idx_l0_default_active_minus1: u32,
    /// `num_ref_idx_l1_default_active_minus1`.
    pub num_ref_idx_l1_default_active_minus1: u32,
}

impl SlicePpsInfo {
    fn validate(&self) -> Result<(), SliceHeaderError> {
        if self.weighted_bipred_idc > 3 {
            return Err(SliceHeaderError::InvalidContext("weighted_bipred_idc > 3"));
        }
        Ok(())
    }
}

/// NAL-header projection for one VCL slice unit: `nal_ref_idc` and
/// `IdrPicFlag` (= `nal_unit_type == 5`), the two header bits the
/// slice-header syntax reads (§7.3.3 conditions on both).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceNalInfo {
    /// `nal_ref_idc` (0..=3); nonzero gates `dec_ref_pic_marking()`.
    pub nal_ref_idc: u8,
    /// `IdrPicFlag`: true for `nal_unit_type == 5` (IDR).
    pub idr_pic_flag: bool,
}

impl SliceNalInfo {
    /// Project a raw NAL header byte (first byte after the start
    /// code). Refuses non-VCL units and set forbidden bits, typed.
    ///
    /// # Errors
    /// [`SliceHeaderError::NotAVclNal`] when the unit type is not a
    /// slice (1 = non-IDR, 5 = IDR); [`SliceHeaderError::ForbiddenBitSet`]
    /// when bit 7 is set.
    pub fn from_header_byte(byte: u8) -> Result<SliceNalInfo, SliceHeaderError> {
        if byte & 0x80 != 0 {
            // forbidden_zero_bit must be 0 (§7.3.1) — a set bit means
            // the byte is not a spec NAL header at all.
            return Err(SliceHeaderError::ForbiddenBitSet);
        }
        let nal_unit_type = byte & 0x1F;
        let nal_ref_idc = (byte >> 5) & 0x3;
        match nal_unit_type {
            1 => Ok(SliceNalInfo {
                nal_ref_idc,
                idr_pic_flag: false,
            }),
            5 => Ok(SliceNalInfo {
                nal_ref_idc,
                idr_pic_flag: true,
            }),
            other => Err(SliceHeaderError::NotAVclNal {
                nal_unit_type: other,
            }),
        }
    }
}

/// Slice type vocabulary (Table 7-7), aliases 5..=9 ("all slices of
/// this picture are X") folded into a kind + flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceKind {
    /// P slice (inter, one ref list).
    P,
    /// B slice (inter, bi-predictive).
    B,
    /// I slice (intra).
    I,
    /// SP slice (switching P).
    Sp,
    /// SI slice (switching intra).
    Si,
}

/// Decoded `slice_type` with the 5..=9 "all slices in this picture"
/// alias recorded (Table 7-7: e.g. 7 means "I, and every other slice
/// of the picture is I too").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceType {
    /// The kind (P/B/I/SP/SI).
    pub kind: SliceKind,
    /// True when coded as 5..=9 (the "all slices" aliases).
    pub all_slices_of_picture: bool,
}

impl SliceType {
    /// Map a raw `ue(v)` slice_type code (Table 7-7).
    ///
    /// # Errors
    /// [`SliceHeaderError::InvalidSliceType`] for codes > 9.
    pub fn from_code(code: u32) -> Result<SliceType, SliceHeaderError> {
        let (kind, all) = match code {
            0 | 5 => (SliceKind::P, code == 5),
            1 | 6 => (SliceKind::B, code == 6),
            2 | 7 => (SliceKind::I, code == 7),
            3 | 8 => (SliceKind::Sp, code == 8),
            4 | 9 => (SliceKind::Si, code == 9),
            other => return Err(SliceHeaderError::InvalidSliceType(other)),
        };
        Ok(SliceType {
            kind,
            all_slices_of_picture: all,
        })
    }

    fn from_reader(r: &mut RbspBitReader<'_>) -> Result<SliceType, SliceHeaderError> {
        Self::from_code(r.read_ue()?)
    }

    fn is_inter_predictive(&self) -> bool {
        matches!(self.kind, SliceKind::P | SliceKind::Sp)
    }

    fn is_intrinsic(&self) -> bool {
        matches!(self.kind, SliceKind::I | SliceKind::Si)
    }
}

/// POC syntax as parsed, one variant per `pic_order_cnt_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PicOrderSyntax {
    /// Type 0: `pic_order_cnt_lsb` (+ optional
    /// `delta_pic_order_cnt_bottom` when the PPS "pic order present"
    /// flag is set and this is a frame).
    Type0 {
        /// `pic_order_cnt_lsb` (`u(log2_max_pic_order_cnt_lsb)`).
        lsb: u32,
        /// `delta_pic_order_cnt_bottom`, when present.
        delta_bottom: Option<i32>,
    },
    /// Type 1: the two optional deltas (absent when
    /// `delta_pic_order_always_zero_flag` is set — then this variant
    /// still appears with `None`s, keeping the type visible).
    Type1 {
        /// `delta_pic_order_cnt[0]`, when present.
        delta0: Option<i32>,
        /// `delta_pic_order_cnt[1]`, when present (field-ordered
        /// pictures with the PPS flag set).
        delta1: Option<i32>,
    },
    /// Type 2: no POC syntax fields at all; the count derives from
    /// `frame_num` arithmetic (§8.2.1.3, see
    /// [`pic_order_cnt_type2`]).
    Type2,
}

/// `ref_pic_list_modification()` (§7.3.3.1) as parsed — an ordered
/// op list per list (empty when the flag is clear).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RefPicListModification {
    /// Ordered `modification_of_pic_nums_idc` operations for list 0
    /// (P/SP/B slices only).
    pub l0: Vec<RefPicListModOp>,
    /// Operations for list 1 (B slices only).
    pub l1: Vec<RefPicListModOp>,
}

/// One `modification_of_pic_nums_idc` operation (§7.3.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefPicListModOp {
    /// idc 0: `abs_diff_pic_num_minus1` — short-term by pic num.
    ShortTermByPicNum {
        /// `abs_diff_pic_num_minus1`.
        abs_diff_pic_num_minus1: u32,
    },
    /// idc 1: short-term by pic num, re-indexed as long term.
    ShortTermToLongTermByPicNum {
        /// `abs_diff_pic_num_minus1`.
        abs_diff_pic_num_minus1: u32,
    },
    /// idc 2: `long_term_pic_num`.
    LongTermByPicNum {
        /// `long_term_pic_num`.
        long_term_pic_num: u32,
    },
    /// idc 4: `abs_diff_view_idx_minus1` (MVC base-extension).
    ShortTermByViewIdx {
        /// `abs_diff_view_idx_minus1`.
        abs_diff_view_idx_minus1: u32,
    },
    /// idc 5: long-term by view idx (MVC base-extension).
    LongTermByViewIdx {
        /// `abs_diff_view_idx_minus1`.
        abs_diff_view_idx_minus1: u32,
    },
}

/// `pred_weight_table()` (§7.3.3.2) — weights parsed as plain values
/// (parse-level only; their *application* is decode-side).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PredWeightTable {
    /// `luma_log2_weight_denom`.
    pub luma_log2_weight_denom: u32,
    /// `chroma_log2_weight_denom`, when chroma is present.
    pub chroma_log2_weight_denom: Option<u32>,
    /// List-0 entries, index = `num_ref_idx_l0_active_minus1` order.
    pub l0: Vec<PredWeightEntry>,
    /// List-1 entries (B slices).
    pub l1: Vec<PredWeightEntry>,
}

/// Per-reference weights of one list (§7.3.3.2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PredWeightEntry {
    /// `luma_weight_lX_flag` + `luma_weight_lX[i]` / `luma_offset_lX[i]`.
    pub luma: Option<(i32, i32)>,
    /// Chroma pairs when `chroma_weight_lX_flag` is set:
    /// `(weight, offset)` × 2 (Cb, Cr).
    pub chroma: Option<[(i32, i32); 2]>,
}

/// `dec_ref_pic_marking()` (§7.3.3.4) — marking basics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecRefPicMarking {
    /// IDR pictures: the two flags, no operations.
    Idr {
        /// `no_output_of_prior_pics_flag`.
        no_output_of_prior_pics_flag: bool,
        /// `long_term_reference_flag`.
        long_term_reference_flag: bool,
    },
    /// Non-IDR reference pictures with
    /// `adaptive_ref_pic_marking_mode_flag == 1`: the ordered
    /// `memory_management_control_operation` list (MMCO).
    Adaptive(Vec<MmcoOp>),
    /// Non-IDR reference pictures, sliding-window marking (flag 0):
    /// no operations.
    SlidingWindow,
}

/// One MMCO operation (§8.2.5.4 command table; syntax per
/// §7.3.3.4 — operand mapping verified against ffmpeg
/// `trace_headers` on the corpus: op 1 reads only
/// `difference_of_pic_nums_minus1`, op 0 is the terminator).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmcoOp {
    /// idc 0: end of the operation list — the loop terminator.
    End,
    /// idc 1: mark a short-term picture unused for reference.
    MarkShortTermUnused {
        /// `difference_of_pic_nums_minus1`.
        difference_of_pic_nums_minus1: u32,
    },
    /// idc 2: mark a long-term picture unused for reference.
    MarkLongTermUnused {
        /// `long_term_frame_idx`.
        long_term_frame_idx: u32,
    },
    /// idc 3: mark a short-term picture used for long-term
    /// reference.
    ShortTermToLongTerm {
        /// `difference_of_pic_nums_minus1`.
        difference_of_pic_nums_minus1: u32,
        /// `long_term_frame_idx`.
        long_term_frame_idx: u32,
    },
    /// idc 4: set `MaxLongTermFrameIdx`.
    SetMaxLongTermFrameIdx {
        /// `max_long_term_frame_idx_plus1`.
        max_long_term_frame_idx_plus1: u32,
    },
    /// idc 5: mark the current picture used for long-term
    /// reference.
    MarkCurrentLongTerm {
        /// `long_term_frame_idx`.
        long_term_frame_idx: u32,
    },
    /// idc 6: assign a long-term frame index to a short-term
    /// picture (MVC base-extension form of idc 3).
    AssignLongTermFrameIdx {
        /// `difference_of_pic_nums_minus1`.
        difference_of_pic_nums_minus1: u32,
        /// `long_term_frame_idx`.
        long_term_frame_idx: u32,
    },
}

/// Deblocking filter control (§7.3.3, syntax after `slice_qp_delta`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeblockingFilterControl {
    /// `disable_deblocking_filter_idc` (0..=2; 1 = filter off for
    /// this slice, no offsets follow).
    pub disable_deblocking_filter_idc: u32,
    /// `slice_alpha_c0_offset_div2` (idc != 1).
    pub slice_alpha_c0_offset_div2: Option<i32>,
    /// `slice_beta_offset_div2` (idc != 1).
    pub slice_beta_offset_div2: Option<i32>,
}

/// A parsed slice header (§7.3.3), parse-level only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceHeader {
    /// `first_mb_in_slice` (`ue(v)`).
    pub first_mb_in_slice: u32,
    /// `slice_type` (Table 7-7, aliases folded).
    pub slice_type: SliceType,
    /// `pic_parameter_set_id` (`ue(v)`, 0..=255).
    pub pic_parameter_set_id: u8,
    /// `colour_plane_id` (`u(2)`, only when the SPS separates colour
    /// planes).
    pub colour_plane_id: Option<u8>,
    /// `frame_num` (`u(log2_max_frame_num)`).
    pub frame_num: u32,
    /// `field_pic_flag` (only when `!frame_mbs_only_flag`).
    pub field_pic_flag: bool,
    /// `bottom_field_flag` (field pictures only).
    pub bottom_field_flag: Option<bool>,
    /// `idr_pic_id` (`ue(v)`, IDR pictures only).
    pub idr_pic_id: Option<u32>,
    /// POC syntax fields, per SPS `pic_order_cnt_type`.
    pub pic_order: PicOrderSyntax,
    /// `redundant_pic_cnt` (PPS-gated).
    pub redundant_pic_cnt: Option<u32>,
    /// `direct_spatial_mv_pred_flag` (B slices only).
    pub direct_spatial_mv_pred_flag: Option<bool>,
    /// `num_ref_idx_active_override_flag` + the overridden
    /// `num_ref_idx_lX_active_minus1` values (P/SP/B only, when the
    /// flag is set; list 1 exists only in B slices).
    pub num_ref_idx_override: Option<(u32, Option<u32>)>,
    /// `ref_pic_list_modification()` (P/SP/B only).
    pub ref_pic_list_modification: RefPicListModification,
    /// `pred_weight_table()` (when gated in).
    pub pred_weight_table: Option<PredWeightTable>,
    /// `dec_ref_pic_marking()` (reference pictures only:
    /// `nal_ref_idc != 0`).
    pub dec_ref_pic_marking: Option<DecRefPicMarking>,
    /// `cabac_init_idc` (CABAC + non-I/SI only).
    pub cabac_init_idc: Option<u32>,
    /// `slice_qp_delta` (`se(v)`).
    pub slice_qp_delta: i32,
    /// Deblocking filter control (PPS-gated).
    pub deblocking: Option<DeblockingFilterControl>,
    /// Bits of RBSP the header consumed (parse progress; the rest is
    /// entropy-coded macroblock data, untouched by this card).
    pub header_bits: usize,
}

/// Loops bounded against malformed streams (spec-conformant streams
/// never reach a tenth of this; each loop iteration consumes at least
/// one bit, so the input length also bounds them — the budget is the
/// second, allocation-side bound).
const MAX_OPS_PER_LOOP: usize = 256;

/// Parse one slice header from an RBSP (emulation prevention already
/// removed) plus the active contexts (see module docs for the seam).
///
/// # Errors
/// Typed [`SliceHeaderError`]s — truncation, out-of-vocabulary
/// values, FMO refusal, loop budgets. Never panics.
pub fn parse_slice_header(
    rbsp: &[u8],
    nal: SliceNalInfo,
    sps: &SliceSpsInfo,
    pps: &SlicePpsInfo,
) -> Result<SliceHeader, SliceHeaderError> {
    sps.validate()?;
    pps.validate()?;
    if pps.num_slice_groups_minus1 != 0 {
        // slice_group_change_cycle would follow the deblock syntax;
        // refusing here keeps the minimal seam honest (module docs).
        return Err(SliceHeaderError::FmoUnsupported {
            num_slice_groups_minus1: pps.num_slice_groups_minus1,
        });
    }
    let mut r = RbspBitReader::new(rbsp);

    let first_mb_in_slice = r.read_ue()?;
    let slice_type = SliceType::from_reader(&mut r)?;
    let pps_id = r.read_ue()?;
    if pps_id > 255 {
        return Err(SliceHeaderError::InvalidPpsId(pps_id));
    }
    let colour_plane_id = if sps.separate_colour_plane_flag {
        Some(r.read_bits(2)? as u8)
    } else {
        None
    };
    let frame_num_width = u32::from(sps.log2_max_frame_num_minus4) + 4;
    let frame_num = r.read_bits(frame_num_width)?;
    let (field_pic_flag, bottom_field_flag) = if sps.frame_mbs_only_flag {
        (false, None)
    } else {
        let field = r.read_bit()?;
        if field {
            (true, Some(r.read_bit()?))
        } else {
            (false, None)
        }
    };
    let idr_pic_id = if nal.idr_pic_flag {
        Some(r.read_ue()?)
    } else {
        None
    };

    let field_pic = field_pic_flag;
    let pic_order = match sps.pic_order_cnt_type {
        0 => PicOrderSyntax::Type0 {
            lsb: r.read_bits(u32::from(sps.log2_max_pic_order_cnt_lsb_minus4) + 4)?,
            delta_bottom: if pps.bottom_field_pic_order_in_frame_present_flag && !field_pic {
                Some(r.read_se()?)
            } else {
                None
            },
        },
        1 => {
            if sps.delta_pic_order_always_zero_flag {
                PicOrderSyntax::Type1 {
                    delta0: None,
                    delta1: None,
                }
            } else {
                let delta0 = r.read_se()?;
                let delta1 = if pps.bottom_field_pic_order_in_frame_present_flag && !field_pic {
                    Some(r.read_se()?)
                } else {
                    None
                };
                PicOrderSyntax::Type1 {
                    delta0: Some(delta0),
                    delta1,
                }
            }
        }
        _ => PicOrderSyntax::Type2,
    };

    let redundant_pic_cnt = if pps.redundant_pic_cnt_present_flag {
        Some(r.read_ue()?)
    } else {
        None
    };

    let direct_spatial_mv_pred_flag = if slice_type.kind == SliceKind::B {
        Some(r.read_bit()?)
    } else {
        None
    };

    let num_ref_idx_override = if slice_type.is_inter_predictive()
        || slice_type.kind == SliceKind::B
    {
        if r.read_bit()? {
            let l0 = r.read_ue()?;
            if l0 > 31 {
                return Err(SliceHeaderError::InvalidNumRefIdxActive { list: 0, value: l0 });
            }
            let l1 = if slice_type.kind == SliceKind::B {
                let l1 = r.read_ue()?;
                if l1 > 31 {
                    return Err(SliceHeaderError::InvalidNumRefIdxActive { list: 1, value: l1 });
                }
                Some(l1)
            } else {
                None
            };
            Some((l0, l1))
        } else {
            None
        }
    } else {
        None
    };
    // §7.4.3.1 range 0..=31 on both defaults too — also the
    // allocation bound for the weight table below.
    if pps.num_ref_idx_l0_default_active_minus1 > 31 {
        return Err(SliceHeaderError::InvalidNumRefIdxActive {
            list: 0,
            value: pps.num_ref_idx_l0_default_active_minus1,
        });
    }
    if pps.num_ref_idx_l1_default_active_minus1 > 31 {
        return Err(SliceHeaderError::InvalidNumRefIdxActive {
            list: 1,
            value: pps.num_ref_idx_l1_default_active_minus1,
        });
    }
    let num_ref_idx_l0_active_minus1 = num_ref_idx_override
        .map(|(l0, _)| l0)
        .unwrap_or(pps.num_ref_idx_l0_default_active_minus1);
    let num_ref_idx_l1_active_minus1 = match (num_ref_idx_override, slice_type.kind) {
        (Some((_, Some(l1))), _) => l1,
        (_, SliceKind::B) => pps.num_ref_idx_l1_default_active_minus1,
        _ => 0,
    };

    // ref_pic_list_modification() (§7.3.3.1) — P/SP/B only.
    let mut ref_pic_list_modification = RefPicListModification::default();
    if !slice_type.is_intrinsic() && r.read_bit()? {
        ref_pic_list_modification.l0 = read_ref_pic_list_mod_ops(&mut r)?;
    }
    if slice_type.kind == SliceKind::B && r.read_bit()? {
        ref_pic_list_modification.l1 = read_ref_pic_list_mod_ops(&mut r)?;
    }

    // pred_weight_table() (§7.3.3.2).
    let pred_weight_table = if (pps.weighted_pred_flag && slice_type.is_inter_predictive())
        || (pps.weighted_bipred_idc == 1 && slice_type.kind == SliceKind::B)
    {
        Some(read_pred_weight_table(
            &mut r,
            sps.chroma_array_type(),
            num_ref_idx_l0_active_minus1,
            num_ref_idx_l1_active_minus1,
            slice_type.kind == SliceKind::B,
        )?)
    } else {
        None
    };

    // dec_ref_pic_marking() (§7.3.3.4) — reference pictures only.
    let dec_ref_pic_marking = if nal.nal_ref_idc != 0 {
        Some(if nal.idr_pic_flag {
            DecRefPicMarking::Idr {
                no_output_of_prior_pics_flag: r.read_bit()?,
                long_term_reference_flag: r.read_bit()?,
            }
        } else if r.read_bit()? {
            DecRefPicMarking::Adaptive(read_mmco_ops(&mut r)?)
        } else {
            DecRefPicMarking::SlidingWindow
        })
    } else {
        None
    };

    let cabac_init_idc = if pps.entropy_coding_mode_flag && !slice_type.is_intrinsic() {
        Some(r.read_ue()?)
    } else {
        None
    };

    let slice_qp_delta = r.read_se()?;

    let deblocking = if pps.deblocking_filter_control_present_flag {
        let idc = r.read_ue()?;
        if idc > 2 {
            // 3 was removed from the vocabulary by Corigendum 1;
            // treat out-of-range as the typed invalid-value class.
            return Err(SliceHeaderError::InvalidSliceType(idc));
        }
        Some(if idc == 1 {
            DeblockingFilterControl {
                disable_deblocking_filter_idc: idc,
                slice_alpha_c0_offset_div2: None,
                slice_beta_offset_div2: None,
            }
        } else {
            DeblockingFilterControl {
                disable_deblocking_filter_idc: idc,
                slice_alpha_c0_offset_div2: Some(r.read_se()?),
                slice_beta_offset_div2: Some(r.read_se()?),
            }
        })
    } else {
        None
    };

    // num_slice_groups_minus1 == 0 was enforced above, so no
    // slice_group_change_cycle follows: the header is complete.

    Ok(SliceHeader {
        first_mb_in_slice,
        slice_type,
        pic_parameter_set_id: pps_id as u8,
        colour_plane_id,
        frame_num,
        field_pic_flag,
        bottom_field_flag,
        idr_pic_id,
        pic_order,
        redundant_pic_cnt,
        direct_spatial_mv_pred_flag,
        num_ref_idx_override,
        ref_pic_list_modification,
        pred_weight_table,
        dec_ref_pic_marking,
        cabac_init_idc,
        slice_qp_delta,
        deblocking,
        header_bits: r.bit_pos(),
    })
}

/// §7.3.3.1 modification loop for one list (terminator idc 3).
fn read_ref_pic_list_mod_ops(
    r: &mut RbspBitReader<'_>,
) -> Result<Vec<RefPicListModOp>, SliceHeaderError> {
    let mut ops = Vec::new();
    loop {
        let idc = r.read_ue()?;
        let op = match idc {
            0 => RefPicListModOp::ShortTermByPicNum {
                abs_diff_pic_num_minus1: r.read_ue()?,
            },
            1 => RefPicListModOp::ShortTermToLongTermByPicNum {
                abs_diff_pic_num_minus1: r.read_ue()?,
            },
            2 => RefPicListModOp::LongTermByPicNum {
                long_term_pic_num: r.read_ue()?,
            },
            3 => return Ok(ops),
            4 => RefPicListModOp::ShortTermByViewIdx {
                abs_diff_view_idx_minus1: r.read_ue()?,
            },
            5 => RefPicListModOp::LongTermByViewIdx {
                abs_diff_view_idx_minus1: r.read_ue()?,
            },
            other => return Err(SliceHeaderError::InvalidModOfPicNumsIdc(other)),
        };
        ops.push(op);
        if ops.len() > MAX_OPS_PER_LOOP {
            return Err(SliceHeaderError::OpBudgetExceeded {
                kind: "ref_pic_list_modification",
                ops: ops.len(),
            });
        }
    }
}

/// §7.3.3.2 `pred_weight_table()` (values only, no application).
fn read_pred_weight_table(
    r: &mut RbspBitReader<'_>,
    chroma_array_type: u8,
    num_ref_idx_l0_active_minus1: u32,
    num_ref_idx_l1_active_minus1: u32,
    is_b: bool,
) -> Result<PredWeightTable, SliceHeaderError> {
    let luma_log2_weight_denom = r.read_ue()?;
    let chroma_log2_weight_denom = if chroma_array_type != 0 {
        Some(r.read_ue()?)
    } else {
        None
    };
    let mut table = PredWeightTable {
        luma_log2_weight_denom,
        chroma_log2_weight_denom,
        l0: Vec::new(),
        l1: Vec::new(),
    };
    for idx in 0..=num_ref_idx_l0_active_minus1 {
        table
            .l0
            .push(read_pred_weight_entry(r, chroma_array_type).map_err(|_| {
                SliceHeaderError::OpBudgetExceeded {
                    kind: "pred_weight_l0",
                    ops: idx as usize + 1,
                }
            })?);
    }
    if is_b {
        for idx in 0..=num_ref_idx_l1_active_minus1 {
            table
                .l1
                .push(read_pred_weight_entry(r, chroma_array_type).map_err(|_| {
                    SliceHeaderError::OpBudgetExceeded {
                        kind: "pred_weight_l1",
                        ops: idx as usize + 1,
                    }
                })?);
        }
    }
    Ok(table)
}

/// One reference's weight flags + values (§7.3.3.2).
fn read_pred_weight_entry(
    r: &mut RbspBitReader<'_>,
    chroma_array_type: u8,
) -> Result<PredWeightEntry, BitReadError> {
    let mut entry = PredWeightEntry::default();
    if r.read_bit()? {
        entry.luma = Some((r.read_se()?, r.read_se()?));
    }
    if chroma_array_type != 0 && r.read_bit()? {
        let cb = (r.read_se()?, r.read_se()?);
        let cr = (r.read_se()?, r.read_se()?);
        entry.chroma = Some([cb, cr]);
    }
    Ok(entry)
}

/// §7.3.3.4 adaptive marking loop (terminator mmco 0).
fn read_mmco_ops(r: &mut RbspBitReader<'_>) -> Result<Vec<MmcoOp>, SliceHeaderError> {
    let mut ops = Vec::new();
    loop {
        let idc = r.read_ue()?;
        let op = match idc {
            0 => MmcoOp::End,
            1 => MmcoOp::MarkShortTermUnused {
                difference_of_pic_nums_minus1: r.read_ue()?,
            },
            2 => MmcoOp::MarkLongTermUnused {
                long_term_frame_idx: r.read_ue()?,
            },
            3 => MmcoOp::ShortTermToLongTerm {
                difference_of_pic_nums_minus1: r.read_ue()?,
                long_term_frame_idx: r.read_ue()?,
            },
            4 => MmcoOp::SetMaxLongTermFrameIdx {
                max_long_term_frame_idx_plus1: r.read_ue()?,
            },
            5 => MmcoOp::MarkCurrentLongTerm {
                long_term_frame_idx: r.read_ue()?,
            },
            6 => MmcoOp::AssignLongTermFrameIdx {
                difference_of_pic_nums_minus1: r.read_ue()?,
                long_term_frame_idx: r.read_ue()?,
            },
            other => return Err(SliceHeaderError::InvalidMmco(other)),
        };
        let terminate = idc == 0;
        ops.push(op);
        if terminate {
            return Ok(ops);
        }
        if ops.len() > MAX_OPS_PER_LOOP {
            return Err(SliceHeaderError::OpBudgetExceeded {
                kind: "dec_ref_pic_marking",
                ops: ops.len(),
            });
        }
    }
}

/// §8.2.1.1 type-0 MSB computation (the "POC wrap" rule), pure and
/// overflow-checked. Callers hold the previous picture's
/// `(PrevPicOrderCntMsb, PrevPicOrderCntLsb)` — decoder-side state,
/// deliberately not smuggled into the parser.
///
/// # Errors
/// [`PocError::Type0WrapOverflow`] on `i64` overflow.
#[allow(clippy::missing_panics_doc)]
pub fn pic_order_cnt_msb_type0(
    prev_msb: i64,
    prev_lsb: i64,
    lsb: i64,
    max_pic_order_cnt_lsb: i64,
) -> Result<i64, PocError> {
    let msb = if lsb < prev_lsb && prev_lsb - lsb >= max_pic_order_cnt_lsb / 2 {
        prev_msb.checked_add(max_pic_order_cnt_lsb)
    } else if lsb > prev_lsb && lsb - prev_lsb > max_pic_order_cnt_lsb / 2 {
        prev_msb.checked_sub(max_pic_order_cnt_lsb)
    } else {
        Some(prev_msb)
    };
    msb.ok_or(PocError::Type0WrapOverflow)
}

/// §8.2.1 FrameNumOffset one-step transition: 0 at an IDR picture,
/// else `PrevFrameNumOffset + MaxFrameNum` when the count wrapped
/// (`prev_frame_num > frame_num`), else unchanged.
///
/// # Errors
/// [`PocError::Type2Overflow`] on `u32` overflow.
pub fn next_frame_num_offset(
    idr_pic: bool,
    frame_num: u32,
    prev_frame_num: u32,
    prev_offset: u32,
    max_frame_num: u32,
) -> Result<u32, PocError> {
    if idr_pic {
        return Ok(0);
    }
    if prev_frame_num > frame_num {
        prev_offset
            .checked_add(max_frame_num)
            .ok_or(PocError::Type2Overflow)
    } else {
        Ok(prev_offset)
    }
}

/// §8.2.1.3 type-2 POC: `2·(FrameNumOffset + FrameNum)` for
/// reference pictures, one less for non-reference (field pictures
/// apply their own offset, out of parse scope here).
///
/// # Errors
/// [`PocError::Type2Overflow`] on `i64` overflow.
pub fn pic_order_cnt_type2(
    idr_pic: bool,
    nal_ref_idc: u8,
    frame_num: u32,
    frame_num_offset: u32,
) -> Result<i64, PocError> {
    if idr_pic {
        return Ok(0);
    }
    let base = i64::from(frame_num_offset)
        .checked_add(i64::from(frame_num))
        .and_then(|v| v.checked_mul(2))
        .ok_or(PocError::Type2Overflow)?;
    if nal_ref_idc == 0 {
        base.checked_sub(1).ok_or(PocError::Type2Overflow)
    } else {
        Ok(base)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    // ---- test scaffolding: Annex-B split, EP removal, SPS/PPS
    // bootstrap readers (test-only; production equivalents are card
    // 5.F1a's files — see module docs for the seam) ----

    /// Split an Annex-B byte stream into NAL units
    /// `(header_byte, rbsp_after_ep_removal)`.
    pub(crate) fn annexb_nals(data: &[u8]) -> Vec<(u8, Vec<u8>)> {
        let mut nals = Vec::new();
        let mut starts: Vec<(usize, usize)> = Vec::new(); // (payload_start, sc_len)
        let mut i = 0;
        while i + 3 <= data.len() {
            if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
                starts.push((i + 3, 3));
                i += 3;
            } else if i + 4 <= data.len()
                && data[i] == 0
                && data[i + 1] == 0
                && data[i + 2] == 0
                && data[i + 3] == 1
            {
                starts.push((i + 4, 4));
                i += 4;
            } else {
                i += 1;
            }
        }
        for (idx, &(start, _sc_len)) in starts.iter().enumerate() {
            let end = if idx + 1 < starts.len() {
                starts[idx + 1].0 - starts[idx + 1].1
            } else {
                data.len()
            };
            if end <= start {
                continue;
            }
            let header = data[start];
            let rbsp = strip_emulation_prevention(&data[start + 1..end]);
            nals.push((header, rbsp));
        }
        nals
    }

    /// Remove `00 00 03 {00..03}` emulation-prevention bytes.
    pub(crate) fn strip_emulation_prevention(bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(bytes.len());
        let mut zeros = 0usize;
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if zeros >= 2 && b == 3 && i + 1 < bytes.len() && bytes[i + 1] <= 3 {
                zeros = 0; // drop the 0x03
            } else {
                out.push(b);
                zeros = if b == 0 { zeros + 1 } else { 0 };
            }
            i += 1;
        }
        out
    }

    /// Test-only bootstrap SPS reader: exactly the fields the
    /// [`SliceSpsInfo`] seam needs (clean-room, §7.3.2.1 prefix).
    /// Refuses scaling lists / FMO-adjacent features (none in the
    /// corpus; production SPS parse is card 5.F1a's).
    pub(crate) fn bootstrap_sps(rbsp: &[u8]) -> SliceSpsInfo {
        let mut r = RbspBitReader::new(rbsp);
        let profile = r.read_bits(8).unwrap_or_default();
        let _constraints = r.read_bits(8);
        let _level = r.read_bits(8);
        let _sps_id = r.read_ue();
        let (mut chroma_format_idc, mut separate_colour_plane_flag) = (1u8, false);
        if matches!(
            profile,
            100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135
        ) {
            let chroma = r.read_ue().ok();
            chroma_format_idc = chroma.unwrap_or(1) as u8;
            if chroma == Some(3) {
                separate_colour_plane_flag = r.read_bit().unwrap_or_default();
            }
            let _bit_depth_luma = r.read_ue();
            let _bit_depth_chroma = r.read_ue();
            let _qpprime = r.read_bit();
            let scaling = r.read_bit().unwrap_or_default();
            assert!(!scaling, "bootstrap: scaling lists out of scope");
        }
        let log2_max_frame_num_minus4 = r.read_ue().unwrap_or_default() as u8;
        let pic_order_cnt_type = r.read_ue().unwrap_or(2) as u8;
        let mut log2_max_pic_order_cnt_lsb_minus4 = 0u8;
        let mut delta_pic_order_always_zero_flag = false;
        match pic_order_cnt_type {
            0 => log2_max_pic_order_cnt_lsb_minus4 = r.read_ue().unwrap_or_default() as u8,
            1 => {
                delta_pic_order_always_zero_flag = r.read_bit().unwrap_or_default();
                let _off_non_ref = r.read_ue();
                let _off_top_bottom = r.read_ue();
                let cycles = r.read_ue().unwrap_or_default();
                for _ in 0..cycles {
                    let _ = r.read_se();
                }
            }
            _ => {}
        }
        let _max_num_ref = r.read_ue();
        let _gaps = r.read_bit();
        let _width = r.read_ue();
        let _height = r.read_ue();
        let frame_mbs_only_flag = r.read_bit().unwrap_or_default();
        SliceSpsInfo {
            log2_max_frame_num_minus4,
            pic_order_cnt_type,
            log2_max_pic_order_cnt_lsb_minus4,
            delta_pic_order_always_zero_flag,
            frame_mbs_only_flag,
            separate_colour_plane_flag,
            chroma_format_idc,
        }
    }

    /// Test-only bootstrap PPS reader (§7.3.2.2 prefix → the
    /// [`SlicePpsInfo`] seam).
    pub(crate) fn bootstrap_pps(rbsp: &[u8]) -> SlicePpsInfo {
        let mut r = RbspBitReader::new(rbsp);
        let _pps_id = r.read_ue();
        let _sps_id = r.read_ue();
        let entropy_coding_mode_flag = r.read_bit().unwrap_or_default();
        let bottom_field_pic_order_in_frame_present_flag = r.read_bit().unwrap_or_default();
        let num_slice_groups_minus1 = r.read_ue().unwrap_or_default();
        assert_eq!(num_slice_groups_minus1, 0, "bootstrap: FMO out of scope");
        let num_ref_idx_l0_default_active_minus1 = r.read_ue().unwrap_or_default();
        let num_ref_idx_l1_default_active_minus1 = r.read_ue().unwrap_or_default();
        let weighted_pred_flag = r.read_bit().unwrap_or_default();
        let weighted_bipred_idc = r.read_bits(2).unwrap_or_default() as u8;
        let _pic_init_qp = r.read_se();
        let _pic_init_qs = r.read_se();
        let _chroma_qp = r.read_se();
        let deblocking_filter_control_present_flag = r.read_bit().unwrap_or_default();
        let _constrained_intra = r.read_bit();
        let redundant_pic_cnt_present_flag = r.read_bit().unwrap_or_default();
        SlicePpsInfo {
            bottom_field_pic_order_in_frame_present_flag,
            redundant_pic_cnt_present_flag,
            weighted_pred_flag,
            weighted_bipred_idc,
            entropy_coding_mode_flag,
            deblocking_filter_control_present_flag,
            num_slice_groups_minus1,
            num_ref_idx_l0_default_active_minus1,
            num_ref_idx_l1_default_active_minus1,
        }
    }

    /// Test-only bit writer for hand-built fixtures.
    struct BitWriter {
        out: Vec<u8>,
        bit: u32,
    }

    impl BitWriter {
        fn new() -> BitWriter {
            BitWriter {
                out: Vec::new(),
                bit: 0,
            }
        }

        fn put(&mut self, value: u32, n: u32) {
            for i in (0..n).rev() {
                let b = (value >> i) & 1;
                if self.bit == 0 {
                    self.out.push(0);
                }
                if b == 1 {
                    let last = self.out.len() - 1;
                    self.out[last] |= 1 << (7 - self.bit);
                }
                self.bit = (self.bit + 1) % 8;
            }
        }

        fn ue(&mut self, v: u32) {
            let v64 = u64::from(v) + 1;
            let width = 64 - v64.leading_zeros(); // bits of (v+1)
            let total = 2 * width - 1;
            // total = leading (width-1) zeros + (v+1) in `width` bits
            self.put(0, total - width);
            self.put(v + 1, width);
        }

        fn se(&mut self, v: i32) {
            let k: u32 = if v > 0 {
                (2 * i64::from(v) - 1) as u32
            } else {
                (-2 * i64::from(v)) as u32
            };
            self.ue(k);
        }

        fn rbsp_trailing(&mut self) {
            self.put(1, 1);
            while self.bit != 0 {
                self.put(0, 1);
            }
        }
    }

    pub(crate) fn corpus_path(name: &str) -> Option<std::path::PathBuf> {
        let dir = std::env::var("LUMEN_H264_CORPUS") // lumen-allow-literal: cfg(test) corpus-gate env-var name — test harness knob, not product config
            .unwrap_or_else(|_| "/tmp/my-project/corpus-v4/media".to_string()); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        let p = std::path::Path::new(&dir).join(name);
        if p.exists() { Some(p) } else { None }
    }

    /// The raw-Annex-B corpus lane (0019): SPS/PPS seams + every
    /// slice header, using the test-only bootstrap readers.
    fn corpus_0019() -> Option<(SliceSpsInfo, SlicePpsInfo, Vec<SliceHeader>)> {
        let path = corpus_path("0019-raw-video.h264")?;
        let data = std::fs::read(path).ok()?;
        let nals = annexb_nals(&data);
        let mut sps = None;
        let mut pps = None;
        let mut slices = Vec::new();
        for (header, rbsp) in &nals {
            match header & 0x1F {
                7 => sps = Some(bootstrap_sps(rbsp)),
                8 => pps = Some(bootstrap_pps(rbsp)),
                1 | 5 => {
                    let (sps, pps) = (&sps?, &pps?);
                    let nal = SliceNalInfo::from_header_byte(*header).ok()?;
                    slices.push(
                        parse_slice_header(rbsp, nal, sps, pps)
                            .inspect_err(|e| eprintln!("0019 slice parse failed: {e}"))
                            .ok()?,
                    );
                }
                _ => {}
            }
        }
        Some((sps?, pps?, slices))
    }

    /// The mkv h264 lane (0014-muxed-av.mkv), extracted to Annex-B
    /// with the system ffmpeg (live corpus, demux through the real
    /// container — the mkv demuxer itself is card 5.D1, merged).
    /// Shared with the SEI corpus tests (`sei.rs`).
    pub(crate) fn corpus_0014_annexb() -> Option<Vec<u8>> {
        let mkv = corpus_path("0014-muxed-av.mkv")?;
        let out = std::process::Command::new("ffmpeg")
            .arg("-v")
            .arg("error")
            .arg("-i")
            .arg(&mkv)
            .arg("-map")
            .arg("0:v:0")
            .arg("-c")
            .arg("copy")
            .arg("-bsf:v")
            .arg("h264_mp4toannexb")
            .arg("-f")
            .arg("h264")
            .arg("pipe:1")
            .output()
            .ok()?;
        if !out.status.success() || out.stdout.is_empty() {
            return None;
        }
        Some(out.stdout)
    }

    fn ffprobe_frame_count(path: &std::path::Path) -> Option<u32> {
        let out = std::process::Command::new("ffprobe")
            .arg("-v")
            .arg("error")
            .arg("-select_streams")
            .arg("v:0")
            .arg("-count_frames")
            .arg("-show_entries")
            .arg("stream=nb_read_frames")
            .arg("-of")
            .arg("csv=p=0")
            .arg(path)
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&out.stdout);
        s.trim().parse::<u32>().ok()
    }

    #[test]
    fn slice_type_mapping_covers_all_ten_codes_with_aliases() {
        // Table 7-7: 0..=4 single-slice kinds, 5..=9 the "all
        // slices of this picture" aliases of the same kinds.
        let cases = [
            (0u32, SliceKind::P, false),
            (1, SliceKind::B, false),
            (2, SliceKind::I, false),
            (3, SliceKind::Sp, false),
            (4, SliceKind::Si, false),
            (5, SliceKind::P, true),
            (6, SliceKind::B, true),
            (7, SliceKind::I, true),
            (8, SliceKind::Sp, true),
            (9, SliceKind::Si, true),
        ];
        for (code, kind, all) in cases {
            let st =
                SliceType::from_code(code).unwrap_or_else(|e| unreachable!("code {code}: {e}"));
            assert_eq!(st.kind, kind, "code {code}");
            assert_eq!(st.all_slices_of_picture, all, "code {code}");
        }
        for bad in [10u32, 11, 100, u32::MAX] {
            assert_eq!(
                SliceType::from_code(bad),
                Err(SliceHeaderError::InvalidSliceType(bad)),
                "code {bad}"
            );
        }
    }

    #[test]
    fn corpus_0019_slice_headers_match_the_ffmpeg_trace() {
        let Some((sps, pps, slices)) = corpus_0019() else {
            eprintln!("corpus: 0019-raw-video.h264 not present — skipping");
            return;
        };
        // trace_headers ground truth (worklog 5.F1b): log2_max_frame_num
        // 4 bits, poc type 0 with 6-bit lsb, frame_mbs_only.
        assert_eq!(sps.log2_max_frame_num_minus4, 0);
        assert_eq!(sps.pic_order_cnt_type, 0);
        assert_eq!(sps.log2_max_pic_order_cnt_lsb_minus4, 2);
        assert!(sps.frame_mbs_only_flag);
        assert!(pps.deblocking_filter_control_present_flag);

        // 26 slices = 26 frames (1 IDR + 25), single slice per frame.
        assert_eq!(slices.len(), 26);
        let count = |k: SliceKind| slices.iter().filter(|s| s.slice_type.kind == k).count();
        assert_eq!(count(SliceKind::I), 2, "I slices (IDR + one I)");
        assert_eq!(count(SliceKind::P), 8);
        assert_eq!(count(SliceKind::B), 16);
        // x264 codes I as 7, P as 5, B as 6 (the "all slices" aliases).
        assert!(slices.iter().all(|s| s.slice_type.all_slices_of_picture));

        // first_mb_in_slice: single-slice frames → always 0.
        assert!(slices.iter().all(|s| s.first_mb_in_slice == 0));
        // frame_num is 4 bits: every value < 16 (wraps twice over 26
        // frames), IDR frame_num == 0.
        assert!(slices.iter().all(|s| s.frame_num < 16));
        let idr: Vec<&SliceHeader> = slices.iter().filter(|s| s.idr_pic_id.is_some()).collect();
        assert_eq!(idr.len(), 1);
        assert_eq!(idr[0].idr_pic_id, Some(0));
        assert_eq!(idr[0].frame_num, 0);
        assert_eq!(
            idr[0].dec_ref_pic_marking,
            Some(DecRefPicMarking::Idr {
                no_output_of_prior_pics_flag: false,
                long_term_reference_flag: false,
            })
        );
        // POC type 0: 6-bit lsb (values 0..=63, the trace wraps mod
        // 64), no bottom delta (PPS flag clear).
        for s in &slices {
            match s.pic_order {
                PicOrderSyntax::Type0 { lsb, delta_bottom } => {
                    assert!(lsb < 64);
                    assert_eq!(delta_bottom, None);
                }
                ref other => unreachable!("0019 is poc type 0, got {other:?}"),
            }
        }
        // slice_qp_delta trace range −26..=−24; deblock idc == 1 on
        // every slice → no offset pair anywhere.
        assert!(
            slices
                .iter()
                .all(|s| (-26..=-24).contains(&s.slice_qp_delta))
        );
        assert!(slices.iter().all(|s| {
            s.deblocking
                == Some(DeblockingFilterControl {
                    disable_deblocking_filter_idc: 1,
                    slice_alpha_c0_offset_div2: None,
                    slice_beta_offset_div2: None,
                })
        }));
        // Adaptive marking on 7 slices (trace), sliding window on
        // the other reference slices, None for nal_ref_idc == 0.
        let adaptive = slices
            .iter()
            .filter(|s| matches!(s.dec_ref_pic_marking, Some(DecRefPicMarking::Adaptive(_))))
            .count();
        assert_eq!(adaptive, 7);
        assert!(
            slices.iter().any(|s| {
                matches!(s.dec_ref_pic_marking, Some(DecRefPicMarking::SlidingWindow))
            })
        );
        // B slices carry the direct-prediction flag.
        assert!(
            slices
                .iter()
                .filter(|s| s.slice_type.kind == SliceKind::B)
                .all(|s| s.direct_spatial_mv_pred_flag.is_some())
        );

        // vs ffprobe: slice count == decoded frame count. Honest
        // scope note: this 1:1 identity holds because this stream has
        // exactly one slice per picture (every first_mb == 0 above) —
        // multi-slice or field-coded pictures would need the NAL
        // layer's access-unit grouping (card 5.F1a) to compare.
        let path = corpus_path("0019-raw-video.h264")
            .unwrap_or_else(|| unreachable!("checked in corpus_0019"));
        if let Some(frames) = ffprobe_frame_count(&path) {
            assert_eq!(slices.len() as u32, frames);
        } else {
            eprintln!("corpus: ffprobe unavailable — frame-count cross-check skipped");
        }
    }

    #[test]
    fn corpus_0014_mkv_lane_matches_the_ffmpeg_trace() {
        let Some(bytes) = corpus_0014_annexb() else {
            eprintln!("corpus: 0014-muxed-av.mkv or ffmpeg not present — skipping");
            return;
        };
        let nals = annexb_nals(&bytes);
        let mut sps = None;
        let mut pps = None;
        let mut slices = Vec::new();
        for (header, rbsp) in &nals {
            match header & 0x1F {
                7 => sps = Some(bootstrap_sps(rbsp)),
                8 => pps = Some(bootstrap_pps(rbsp)),
                1 | 5 => {
                    let (sps, pps) = match (&sps, &pps) {
                        (Some(s), Some(p)) => (s, p),
                        _ => unreachable!("slice before parameter sets"),
                    };
                    let nal = SliceNalInfo::from_header_byte(*header)
                        .unwrap_or_else(|e| unreachable!("vcl header: {e}"));
                    slices.push(
                        parse_slice_header(rbsp, nal, sps, pps)
                            .unwrap_or_else(|e| unreachable!("0014 slice: {e}")),
                    );
                }
                _ => {}
            }
        }
        // trace truth: 185 slices (4 IDR + 181), types I4/P46/B135,
        // adaptive marking on 42, idr_pic_id values {0,1}.
        assert_eq!(slices.len(), 185);
        let count = |k: SliceKind| slices.iter().filter(|s| s.slice_type.kind == k).count();
        assert_eq!(count(SliceKind::I), 4);
        assert_eq!(count(SliceKind::P), 46);
        assert_eq!(count(SliceKind::B), 135);
        assert!(slices.iter().all(|s| s.first_mb_in_slice == 0));
        let idrs: Vec<&SliceHeader> = slices.iter().filter(|s| s.idr_pic_id.is_some()).collect();
        assert_eq!(idrs.len(), 4);
        let ids: std::collections::BTreeSet<Option<u32>> =
            idrs.iter().map(|s| s.idr_pic_id).collect();
        assert_eq!(ids, [Some(0), Some(1)].into_iter().collect());
        let adaptive = slices
            .iter()
            .filter(|s| matches!(s.dec_ref_pic_marking, Some(DecRefPicMarking::Adaptive(_))))
            .count();
        assert_eq!(adaptive, 42);
        // qp delta trace range −3..=4.
        assert!(
            slices.iter().all(|s| (-3..=4).contains(&s.slice_qp_delta)),
            "qp delta range"
        );
        // mkv demuxed via ffmpeg: 185 packets == 185 slices (the
        // container's frame/packet count matches the VCL count —
        // single slice per picture again).
        if let Some(mkv) = corpus_path("0014-muxed-av.mkv")
            && let Some(frames) = ffprobe_frame_count(&mkv)
        {
            assert_eq!(slices.len() as u32, frames);
        }
    }

    #[test]
    fn hand_built_headers_exercise_the_optional_syntax_gates() {
        // P slice, poc type 1 with deltas, CABAC, deblock idc 0 with
        // offsets, redundant_pic_cnt, ref-list mod, weighted pred,
        // num_ref_idx override, field pic (frame_mbs_only = 0).
        let sps = SliceSpsInfo {
            log2_max_frame_num_minus4: 0,
            pic_order_cnt_type: 1,
            log2_max_pic_order_cnt_lsb_minus4: 0,
            delta_pic_order_always_zero_flag: false,
            frame_mbs_only_flag: false,
            separate_colour_plane_flag: false,
            chroma_format_idc: 1,
        };
        let pps = SlicePpsInfo {
            bottom_field_pic_order_in_frame_present_flag: true,
            redundant_pic_cnt_present_flag: true,
            weighted_pred_flag: true,
            weighted_bipred_idc: 0,
            entropy_coding_mode_flag: true,
            deblocking_filter_control_present_flag: true,
            num_slice_groups_minus1: 0,
            num_ref_idx_l0_default_active_minus1: 0,
            num_ref_idx_l1_default_active_minus1: 0,
        };
        let mut w = BitWriter::new();
        w.ue(0); // first_mb_in_slice
        w.ue(0); // slice_type = P
        w.ue(0); // pps_id
        w.put(0b1011, 4); // frame_num = 11 (4 bits)
        w.put(1, 1); // field_pic_flag
        w.put(1, 1); // bottom_field_flag
        // poc type 1 (field pic → only delta0 despite PPS flag)
        w.se(-4); // delta_pic_order_cnt[0]
        w.ue(2); // redundant_pic_cnt
        w.put(1, 1); // num_ref_idx_active_override_flag (u(1))
        w.ue(3); // l0_active_minus1 = 3
        w.put(1, 1); // ref_pic_list_modification_flag_l0 = 1
        w.ue(0); // idc 0
        w.ue(6); // abs_diff_pic_num_minus1
        w.ue(3); // idc 3: end of list
        // pred_weight_table (weighted_pred_flag + P)
        w.ue(5); // luma_log2_weight_denom
        w.ue(2); // chroma_log2_weight_denom (chroma != 0)
        for _ in 0..4 {
            w.put(1, 1); // luma_weight_l0_flag
            w.se(2); // luma_weight
            w.se(-1); // luma_offset
            w.put(0, 1); // chroma_weight_l0_flag
        }
        w.put(1, 1); // adaptive_ref_pic_marking_mode_flag = 1
        w.ue(4); // mmco 4: set MaxLongTermFrameIdx
        w.ue(1); // max_long_term_frame_idx_plus1 = 1
        w.ue(0); // mmco 0: end of operations
        w.ue(1); // cabac_init_idc
        w.se(-6); // slice_qp_delta
        w.ue(0); // disable_deblocking_filter_idc = 0
        w.se(2); // alpha offset / 2
        w.se(-1); // beta offset / 2
        w.rbsp_trailing();

        let nal = SliceNalInfo {
            nal_ref_idc: 2,
            idr_pic_flag: false,
        };
        let h = parse_slice_header(&w.out, nal, &sps, &pps)
            .unwrap_or_else(|e| unreachable!("hand-built P header: {e}"));
        assert_eq!(h.first_mb_in_slice, 0);
        assert_eq!(h.slice_type.kind, SliceKind::P);
        assert!(!h.slice_type.all_slices_of_picture);
        assert_eq!(h.pic_parameter_set_id, 0);
        assert_eq!(h.frame_num, 11);
        assert!(h.field_pic_flag);
        assert_eq!(h.bottom_field_flag, Some(true));
        assert_eq!(
            h.pic_order,
            PicOrderSyntax::Type1 {
                delta0: Some(-4),
                delta1: None
            }
        );
        assert_eq!(h.redundant_pic_cnt, Some(2));
        assert_eq!(h.num_ref_idx_override, Some((3, None)));
        assert_eq!(
            h.ref_pic_list_modification.l0,
            vec![RefPicListModOp::ShortTermByPicNum {
                abs_diff_pic_num_minus1: 6
            }]
        );
        assert_eq!(
            h.ref_pic_list_modification.l1,
            Vec::<RefPicListModOp>::new()
        );
        let table = h
            .pred_weight_table
            .as_ref()
            .unwrap_or_else(|| unreachable!());
        assert_eq!(table.luma_log2_weight_denom, 5);
        assert_eq!(table.chroma_log2_weight_denom, Some(2));
        assert_eq!(table.l0.len(), 4);
        assert_eq!(table.l0[0].luma, Some((2, -1)));
        assert_eq!(table.l0[0].chroma, None);
        assert_eq!(
            h.dec_ref_pic_marking,
            Some(DecRefPicMarking::Adaptive(vec![
                MmcoOp::SetMaxLongTermFrameIdx {
                    max_long_term_frame_idx_plus1: 1
                },
                MmcoOp::End,
            ]))
        );
        assert_eq!(h.cabac_init_idc, Some(1));
        assert_eq!(h.slice_qp_delta, -6);
        assert_eq!(
            h.deblocking,
            Some(DeblockingFilterControl {
                disable_deblocking_filter_idc: 0,
                slice_alpha_c0_offset_div2: Some(2),
                slice_beta_offset_div2: Some(-1),
            })
        );
    }

    #[test]
    fn idr_header_with_colour_planes_and_poc_type2() {
        let sps = SliceSpsInfo {
            log2_max_frame_num_minus4: 12,
            pic_order_cnt_type: 2,
            log2_max_pic_order_cnt_lsb_minus4: 0,
            delta_pic_order_always_zero_flag: true,
            frame_mbs_only_flag: true,
            separate_colour_plane_flag: true,
            chroma_format_idc: 3,
        };
        let pps = SlicePpsInfo {
            bottom_field_pic_order_in_frame_present_flag: false,
            redundant_pic_cnt_present_flag: false,
            weighted_pred_flag: false,
            weighted_bipred_idc: 0,
            entropy_coding_mode_flag: false,
            deblocking_filter_control_present_flag: false,
            num_slice_groups_minus1: 0,
            num_ref_idx_l0_default_active_minus1: 0,
            num_ref_idx_l1_default_active_minus1: 0,
        };
        let mut w = BitWriter::new();
        w.ue(5); // first_mb
        w.ue(7); // slice_type 7 = I(all)
        w.ue(2); // pps_id
        w.put(0b10, 2); // colour_plane_id
        w.put(0xABCD, 16); // frame_num, 16 bits
        w.ue(9); // idr_pic_id
        w.se(3); // slice_qp_delta
        w.rbsp_trailing();
        let nal = SliceNalInfo {
            nal_ref_idc: 3,
            idr_pic_flag: true,
        };
        let h = parse_slice_header(&w.out, nal, &sps, &pps)
            .unwrap_or_else(|e| unreachable!("idr header: {e}"));
        assert_eq!(h.slice_type.kind, SliceKind::I);
        assert_eq!(h.colour_plane_id, Some(2));
        assert_eq!(h.frame_num, 0xABCD);
        assert_eq!(h.idr_pic_id, Some(9));
        assert_eq!(h.pic_order, PicOrderSyntax::Type2);
        assert_eq!(
            h.dec_ref_pic_marking,
            Some(DecRefPicMarking::Idr {
                no_output_of_prior_pics_flag: false,
                long_term_reference_flag: false,
            })
        );
        assert_eq!(h.deblocking, None);
    }

    #[test]
    fn truncated_headers_are_typed_never_panics() {
        let Some((sps, pps, _)) = corpus_0019() else {
            eprintln!("corpus: 0019 not present — skipping truncation sweep");
            return;
        };
        let data = std::fs::read(
            corpus_path("0019-raw-video.h264").unwrap_or_else(|| unreachable!("present")),
        )
        .unwrap_or_default();
        let nals = annexb_nals(&data);
        let (hdr, rbsp) = nals
            .iter()
            .find(|(h, _)| (h & 0x1F) == 1 || (h & 0x1F) == 5)
            .unwrap_or_else(|| unreachable!("0019 has slices"));
        let nal = SliceNalInfo::from_header_byte(*hdr).unwrap_or_else(|e| unreachable!("vcl: {e}"));
        let mut typed = 0;
        for cut in 0..rbsp.len() {
            match parse_slice_header(&rbsp[..cut], nal, &sps, &pps) {
                Ok(_) => assert!(cut > 2, "a {cut}-byte header cannot be complete"),
                Err(e) => {
                    assert!(
                        matches!(e, SliceHeaderError::Bit(_)),
                        "truncation must be the bit class, got {e:?}"
                    );
                    typed += 1;
                }
            }
        }
        assert!(typed > 0);
    }

    #[test]
    fn invalid_values_are_the_typed_error_paths() {
        let sps = SliceSpsInfo {
            log2_max_frame_num_minus4: 0,
            pic_order_cnt_type: 0,
            log2_max_pic_order_cnt_lsb_minus4: 0,
            delta_pic_order_always_zero_flag: false,
            frame_mbs_only_flag: true,
            separate_colour_plane_flag: false,
            chroma_format_idc: 1,
        };
        let mut pps = SlicePpsInfo {
            bottom_field_pic_order_in_frame_present_flag: false,
            redundant_pic_cnt_present_flag: false,
            weighted_pred_flag: false,
            weighted_bipred_idc: 0,
            entropy_coding_mode_flag: false,
            deblocking_filter_control_present_flag: false,
            num_slice_groups_minus1: 0,
            num_ref_idx_l0_default_active_minus1: 0,
            num_ref_idx_l1_default_active_minus1: 0,
        };
        let nal = SliceNalInfo {
            nal_ref_idc: 1,
            idr_pic_flag: false,
        };

        // pps_id 256 → InvalidPpsId.
        let mut w = BitWriter::new();
        w.ue(0);
        w.ue(2);
        w.ue(256);
        w.ue(0);
        w.put(0, 4);
        w.put(0, 4);
        w.se(0);
        w.rbsp_trailing();
        assert_eq!(
            parse_slice_header(&w.out, nal, &sps, &pps),
            Err(SliceHeaderError::InvalidPpsId(256))
        );

        // slice_type 10 → InvalidSliceType.
        let mut w = BitWriter::new();
        w.ue(0);
        w.ue(10);
        w.rbsp_trailing();
        assert_eq!(
            parse_slice_header(&w.out, nal, &sps, &pps),
            Err(SliceHeaderError::InvalidSliceType(10))
        );

        // FMO: num_slice_groups_minus1 = 2 → typed refusal before
        // any bit is read.
        pps.num_slice_groups_minus1 = 2;
        assert_eq!(
            parse_slice_header(&[], nal, &sps, &pps),
            Err(SliceHeaderError::FmoUnsupported {
                num_slice_groups_minus1: 2
            })
        );
        pps.num_slice_groups_minus1 = 0;

        // Out-of-range context: log2_max_frame_num_minus4 = 13.
        let mut bad = sps;
        bad.log2_max_frame_num_minus4 = 13;
        assert_eq!(
            parse_slice_header(&[], nal, &bad, &pps),
            Err(SliceHeaderError::InvalidContext(
                "log2_max_frame_num_minus4 > 12"
            ))
        );

        // Non-VCL NAL header (SPS unit) → NotAVclNal.
        assert_eq!(
            SliceNalInfo::from_header_byte(0x67),
            Err(SliceHeaderError::NotAVclNal { nal_unit_type: 7 })
        );
        // forbidden_zero_bit set: 0xC1 (bit 7 set, would read as a
        // type-1 non-IDR slice) and 0x85 (would read as type 5) are
        // both typed refusals — the byte is not a spec NAL header.
        assert_eq!(
            SliceNalInfo::from_header_byte(0xC1),
            Err(SliceHeaderError::ForbiddenBitSet)
        );
        assert_eq!(
            SliceNalInfo::from_header_byte(0x85),
            Err(SliceHeaderError::ForbiddenBitSet)
        );

        // mmco 7 → InvalidMmco.
        let mut w = BitWriter::new();
        w.ue(0);
        w.ue(5); // P(all)
        w.ue(0);
        w.put(0, 4); // frame_num
        w.put(0, 4); // poc lsb
        w.put(0, 1); // num_ref_idx_active_override_flag
        w.put(0, 1); // ref_pic_list_modification_flag_l0
        w.put(1, 1); // adaptive marking flag
        w.ue(7); // invalid mmco
        w.rbsp_trailing();
        assert_eq!(
            parse_slice_header(&w.out, nal, &sps, &pps),
            Err(SliceHeaderError::InvalidMmco(7))
        );

        // modification_of_pic_nums_idc 6 → typed.
        let mut w = BitWriter::new();
        w.ue(0);
        w.ue(5);
        w.ue(0);
        w.put(0, 4);
        w.put(0, 4);
        w.put(0, 1); // num_ref_idx_active_override_flag
        w.put(1, 1); // ref_pic_list_modification_flag_l0
        w.ue(6);
        w.rbsp_trailing();
        assert_eq!(
            parse_slice_header(&w.out, nal, &sps, &pps),
            Err(SliceHeaderError::InvalidModOfPicNumsIdc(6))
        );

        // num_ref_idx_l0_active override 32 → InvalidNumRefIdxActive
        // (§7.4.3.1 range 0..=31; also the weight-table bound).
        let mut w = BitWriter::new();
        w.ue(0);
        w.ue(5);
        w.ue(0);
        w.put(0, 4);
        w.put(0, 4);
        w.put(1, 1); // override flag
        w.ue(32); // out of range (error fires immediately)
        w.rbsp_trailing();
        assert_eq!(
            parse_slice_header(&w.out, nal, &sps, &pps),
            Err(SliceHeaderError::InvalidNumRefIdxActive { list: 0, value: 32 })
        );

        // 32 leading zeros with no marker bit → GolombOverflow.
        let zeros = vec![0x00, 0x00, 0x00, 0x00, 0x80];
        match parse_slice_header(&zeros, nal, &sps, &pps) {
            Err(SliceHeaderError::Bit(BitReadError::GolombOverflow { leading_zeros })) => {
                assert!(leading_zeros >= 32);
            }
            other => unreachable!("expected golomb overflow, got {other:?}"),
        }
    }

    #[test]
    fn poc_arithmetic_helpers_follow_the_spec_rules() {
        // Type 0 wrap (§8.2.1.1), MaxPicOrderCntLsb = 64:
        // lsb 2 after prev 62: prev-lsb = 60 >= 32 → msb += 64.
        assert_eq!(pic_order_cnt_msb_type0(0, 62, 2, 64), Ok(64));
        // lsb 60 after prev 2: lsb-prev = 58 > 32 → msb -= 64.
        assert_eq!(pic_order_cnt_msb_type0(64, 2, 60, 64), Ok(0));
        // No wrap: small step keeps the msb.
        assert_eq!(pic_order_cnt_msb_type0(-128, 5, 7, 64), Ok(-128));
        // Overflow: i64::MAX msb + 64.
        assert_eq!(
            pic_order_cnt_msb_type0(i64::MAX, 62, 2, 64),
            Err(PocError::Type0WrapOverflow)
        );

        // Type 2 (§8.2.1.3): IDR → 0; ref pic → 2*(offset+frame_num);
        // non-ref → one less.
        assert_eq!(pic_order_cnt_type2(true, 3, 5, 9), Ok(0));
        assert_eq!(pic_order_cnt_type2(false, 1, 5, 9), Ok(28));
        assert_eq!(pic_order_cnt_type2(false, 0, 5, 9), Ok(27));
        // u32 inputs cannot overflow the i64 accumulator — the
        // checked math still holds at the extremes.
        assert_eq!(
            pic_order_cnt_type2(false, 1, u32::MAX, u32::MAX),
            Ok(17179869180)
        );

        // FrameNumOffset (§8.2.1): reset at IDR, +MaxFrameNum on
        // wrap, else sticky.
        assert_eq!(next_frame_num_offset(true, 3, 15, 15, 16), Ok(0));
        assert_eq!(next_frame_num_offset(false, 1, 15, 7, 16), Ok(23));
        assert_eq!(next_frame_num_offset(false, 15, 1, 7, 16), Ok(7));
        assert_eq!(
            next_frame_num_offset(false, 0, 1, u32::MAX, 16),
            Err(PocError::Type2Overflow)
        );
    }

    /// +FB (card 5.F1b): deterministic mutation property — every
    /// mutant yields Ok or a typed error, never a panic or hang.
    #[test]
    fn mutated_slice_streams_are_typed_or_ok_never_panics() {
        let Some((sps, pps, _)) = corpus_0019() else {
            eprintln!("corpus: 0019 not present — skipping mutation property");
            return;
        };
        let data = std::fs::read(
            corpus_path("0019-raw-video.h264").unwrap_or_else(|| unreachable!("present")),
        )
        .unwrap_or_default();
        let nals = annexb_nals(&data);
        let slices: Vec<&(u8, Vec<u8>)> = nals
            .iter()
            .filter(|(h, _)| (h & 0x1F) == 1 || (h & 0x1F) == 5)
            .collect();
        assert!(!slices.is_empty());
        // Deterministic LCG so failures reproduce bit-for-bit.
        let mut seed: u64 = 0x5F1B_C0DE_5EED;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut oks = 0;
        let mut errs = 0;
        for round in 0..240u32 {
            let pick = next() as usize % slices.len();
            let (header, rbsp) = slices[pick];
            let mut mutant = rbsp.clone();
            let flips = 1 + (next() as usize % 4);
            // Bias flips into the header region (the first bytes) —
            // flips in the entropy payload after `header_bits` are
            // legitimately invisible to a header parser.
            let head = mutant.len().min(16);
            for _ in 0..flips {
                let byte = next() as usize % head;
                let bit = next() as u32 % 8;
                mutant[byte] ^= 1 << bit;
            }
            if round % 3 == 0 {
                // truncation derivative
                let cut = next() as usize % mutant.len();
                mutant.truncate(cut);
            }
            let nal = SliceNalInfo::from_header_byte(*header)
                .unwrap_or_else(|e| unreachable!("vcl header: {e}"));
            match parse_slice_header(&mutant, nal, &sps, &pps) {
                Ok(_) => oks += 1,
                Err(_) => errs += 1,
            }
        }
        assert_eq!(oks + errs, 240);
        assert!(errs > 50, "mutations must actually bite (errs={errs})");
    }
}
