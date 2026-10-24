//! NAL-unit layer (ITU-T H.265 §7.3.1, §7.4.1): byte-stream framing
//! in both entry forms, emulation prevention, NAL classification, the
//! `hvcC` record, and the shared Exp-Golomb bit reader.
//!
//! # Entry forms (both handled, card 5.G1 step 1)
//!
//! - **Annex-B** (`.265` elementary streams, MPEG-TS, the corpus's
//!   AVI lane): NAL units delimited by `00 00 01` (3-byte) or
//!   `00 00 00 01` (4-byte, leading `zero_byte`) start-code prefixes
//!   — [`AnnexBNals`].
//! - **Length-prefixed** (MP4 samples, Matroska `V_MPEGH/ISO/HEVC`):
//!   each NAL unit preceded by `lengthSizeMinusOne+1` big-endian
//!   length bytes, geometry from the `hvcC` record —
//!   [`LengthPrefixedNals`] + [`HvcCConfig`].
//!
//! # The two-byte NAL header (vs H.264's one byte)
//!
//! HEVC spends 16 bits: `forbidden_zero_bit(1) | nal_unit_type(6) |
//! nuh_layer_id(6) | nuh_temporal_id_plus1(3)` (§7.3.1). The layer id
//! and temporal id are retained verbatim (the multi-layer extension
//! syntax is out of scope — a nonzero layer id parses, its
//! extensions do not).
//!
//! # Memory discipline
//!
//! Both splitters are iterators of **borrowed slices** (zero copy, no
//! allocation). Emulation-prevention removal necessarily copies;
//! [`unescape_rbsp_into`] lets a caller reuse one buffer for a whole
//! stream's NALs — the steady-state parse path allocates nothing.

use core::fmt;

/// Typed parse failure for the HEVC parser surface (card 5.G1: no
/// panics, no damage-smoothing — every failure names its rule).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HevcError {
    /// The NAL/RBSP ended before the syntax element being read
    /// (`needed`/`have` in bits for bit-level reads, bytes for
    /// length-prefix reads).
    Truncated {
        /// Bits/bytes the syntax required.
        needed: u64,
        /// Bits/bytes actually present.
        have: u64,
    },
    /// `forbidden_zero_bit` was 1 — the NAL is marked corrupt by the
    /// spec itself (§7.4.1: such units shall be discarded).
    ForbiddenZeroBit,
    /// An emulation-prevention violation: `00 00 03` whose suffix is
    /// outside `{00,01,02,03}`, or `00 00 03` cut off at the end of
    /// the NAL (§7.4.2.1: no escaped suffix byte exists).
    Emulation {
        /// Stable dotted rule code, e.g. `hevc.ep.bad_suffix`.
        code: &'static str,
    },
    /// A syntax value outside its spec-declared range (ue fields
    /// carry declared bounds; violations are corruption, not
    /// features).
    OutOfRange {
        /// Field name, e.g. `sps_seq_parameter_set_id`.
        field: &'static str,
        /// The parsed value.
        value: u64,
        /// The inclusive maximum the spec allows.
        max: u64,
    },
    /// Structural violation with a stable rule code (bad NAL type for
    /// the requested parse, `nuh_temporal_id_plus1 == 0`, bad `hvcC`
    /// version, conformance-window underflow, …).
    Invalid {
        /// Stable dotted rule code.
        code: &'static str,
    },
    /// A syntactically parseable feature this parser deliberately
    /// does not model (`vps_extension`, `sps_extension`, HRD
    /// structures beyond the walked flags).
    Unsupported {
        /// Stable dotted rule code.
        code: &'static str,
    },
}

impl fmt::Display for HevcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HevcError::Truncated { needed, have } => {
                write!(f, "truncated: needed {needed}, have {have}")
            }
            HevcError::ForbiddenZeroBit => f.write_str("forbidden_zero_bit set"),
            HevcError::Emulation { code } => write!(f, "emulation prevention: {code}"),
            HevcError::OutOfRange { field, value, max } => {
                write!(f, "{field} {value} out of range (max {max})")
            }
            HevcError::Invalid { code } => write!(f, "invalid: {code}"),
            HevcError::Unsupported { code } => write!(f, "unsupported: {code}"),
        }
    }
}

impl std::error::Error for HevcError {}

/// NAL unit type classification (Table 7-1). Named variants cover the
/// types real streams carry; reserved/unspecified ids are preserved
/// verbatim (never guessed into a semantic bucket).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NalUnitType {
    /// 0 — trailing pictures, not used for reference (N suffix).
    TrailN,
    /// 1 — trailing pictures, may be used for reference.
    TrailR,
    /// 2/3 — temporal sub-layer access.
    TsaN,
    TsaR,
    /// 4/5 — step-wise temporal sub-layer access.
    StsaN,
    StsaR,
    /// 6/7 — random-access skipping leading pictures.
    RadlN,
    RadlR,
    /// 8/9 — random-access skipped leading pictures.
    RaslN,
    RaslR,
    /// 16 — BLA with leading pictures.
    BlaWLp,
    /// 17 — BLA using RADL.
    BlaWRadl,
    /// 18 — BLA without leading pictures.
    BlaNLp,
    /// 19 — IDR using RADL.
    IdrWRadl,
    /// 20 — IDR without leading pictures.
    IdrNLp,
    /// 21 — clean random access.
    CraNut,
    /// 32 — video parameter set.
    VpsNut,
    /// 33 — sequence parameter set.
    SpsNut,
    /// 34 — picture parameter set.
    PpsNut,
    /// 35 — access unit delimiter.
    AudNut,
    /// 36 — end of sequence.
    EosNut,
    /// 37 — end of bitstream.
    EobNut,
    /// 38 — filler data.
    FdNut,
    /// 39/40 — SEI prefix/suffix.
    SeiPrefix,
    SeiSuffix,
    /// 10..=15, 22..=31, 41..=47 — reserved by the spec (VCL
    /// reserved 10-15/22-31, non-VCL reserved 41-47).
    Reserved(u8),
    /// 48..=63 — unspecified.
    Unspecified(u8),
}

impl NalUnitType {
    /// Classify a raw 6-bit `nal_unit_type`.
    #[must_use]
    pub fn from_id(id: u8) -> NalUnitType {
        match id {
            0 => NalUnitType::TrailN,
            1 => NalUnitType::TrailR,
            2 => NalUnitType::TsaN,
            3 => NalUnitType::TsaR,
            4 => NalUnitType::StsaN,
            5 => NalUnitType::StsaR,
            6 => NalUnitType::RadlN,
            7 => NalUnitType::RadlR,
            8 => NalUnitType::RaslN,
            9 => NalUnitType::RaslR,
            16 => NalUnitType::BlaWLp,
            17 => NalUnitType::BlaWRadl,
            18 => NalUnitType::BlaNLp,
            19 => NalUnitType::IdrWRadl,
            20 => NalUnitType::IdrNLp,
            21 => NalUnitType::CraNut,
            32 => NalUnitType::VpsNut,
            33 => NalUnitType::SpsNut,
            34 => NalUnitType::PpsNut,
            35 => NalUnitType::AudNut,
            36 => NalUnitType::EosNut,
            37 => NalUnitType::EobNut,
            38 => NalUnitType::FdNut,
            39 => NalUnitType::SeiPrefix,
            40 => NalUnitType::SeiSuffix,
            10..=15 | 22..=31 | 41..=47 => NalUnitType::Reserved(id),
            _ => NalUnitType::Unspecified(id),
        }
    }

    /// The raw 6-bit id (inverse of [`NalUnitType::from_id`]).
    #[must_use]
    pub fn id(self) -> u8 {
        match self {
            NalUnitType::TrailN => 0,
            NalUnitType::TrailR => 1,
            NalUnitType::TsaN => 2,
            NalUnitType::TsaR => 3,
            NalUnitType::StsaN => 4,
            NalUnitType::StsaR => 5,
            NalUnitType::RadlN => 6,
            NalUnitType::RadlR => 7,
            NalUnitType::RaslN => 8,
            NalUnitType::RaslR => 9,
            NalUnitType::BlaWLp => 16,
            NalUnitType::BlaWRadl => 17,
            NalUnitType::BlaNLp => 18,
            NalUnitType::IdrWRadl => 19,
            NalUnitType::IdrNLp => 20,
            NalUnitType::CraNut => 21,
            NalUnitType::VpsNut => 32,
            NalUnitType::SpsNut => 33,
            NalUnitType::PpsNut => 34,
            NalUnitType::AudNut => 35,
            NalUnitType::EosNut => 36,
            NalUnitType::EobNut => 37,
            NalUnitType::FdNut => 38,
            NalUnitType::SeiPrefix => 39,
            NalUnitType::SeiSuffix => 40,
            NalUnitType::Reserved(id) | NalUnitType::Unspecified(id) => id,
        }
    }

    /// IRAP types (16..=23, incl. the reserved 22/23): broken-link /
    /// instantaneous-decoding access points — the slice header reads
    /// `no_output_of_prior_pics_flag` for these.
    #[must_use]
    pub fn is_irap(self) -> bool {
        (16..=23).contains(&self.id())
    }

    /// VCL types (0..=31) — slice-segment payloads; everything ≥ 32
    /// is non-VCL (parameter sets, SEI, delimiters).
    #[must_use]
    pub fn is_vcl(self) -> bool {
        self.id() <= 31
    }
}

/// The parsed two-byte NAL header (§7.3.1) of a NAL unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NalHeader {
    /// The classified `nal_unit_type`.
    pub unit_type: NalUnitType,
    /// `nuh_layer_id` (0..=63): the NAL's layer assignment (base
    /// streams carry 0; the multilayer extension syntax of nonzero
    /// layers is out of scope).
    pub layer_id: u8,
    /// `nuh_temporal_id_plus1` (1..=7; the spec forbids 0).
    pub temporal_id_plus1: u8,
}

impl NalHeader {
    /// Parse the two header bytes of a complete NAL unit and return
    /// them together with the remaining payload (everything after the
    /// header; still emulation-escaped).
    ///
    /// # Errors
    /// [`HevcError::Truncated`] for a NAL shorter than the 2-byte
    /// header, [`HevcError::ForbiddenZeroBit`] when bit 15 is set,
    /// [`HevcError::Invalid { "hevc.nal.temporal_id" }`] when
    /// `nuh_temporal_id_plus1` is 0 (§7.4.1: "shall not be equal to
    /// 0").
    pub fn split(nal: &[u8]) -> Result<(NalHeader, &[u8]), HevcError> {
        if nal.len() < 2 {
            return Err(HevcError::Truncated {
                needed: 2,
                have: nal.len() as u64,
            });
        }
        let (b0, b1) = (nal[0], nal[1]);
        if b0 & 0x80 != 0 {
            return Err(HevcError::ForbiddenZeroBit);
        }
        let temporal_id_plus1 = b1 & 0x07;
        if temporal_id_plus1 == 0 {
            return Err(HevcError::Invalid {
                code: "hevc.nal.temporal_id",
            });
        }
        Ok((
            NalHeader {
                // Type = b0 bits 6..1; layer id = b0 bit 0 ++ b1 bits
                // 7..3; temporal id = b1 bits 2..0.
                unit_type: NalUnitType::from_id((b0 >> 1) & 0x3f),
                layer_id: ((b0 & 0x01) << 5) | (b1 >> 3),
                temporal_id_plus1,
            },
            &nal[2..],
        ))
    }
}

/// Find the first Annex-B start code at/after `from`: the index of the
/// `01` byte of a `00 00 01` run. Returns `None` when no start code
/// follows.
fn next_start_code(data: &[u8], from: usize) -> Option<usize> {
    let end = data.len().saturating_sub(2);
    let mut i = from;
    while i < end {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            // Index just past the `01` byte — the first NAL body byte.
            return Some(i + 3);
        }
        i += 1;
    }
    None
}

/// Trim the `trailing_zero_8bits`/`zero_byte` run off a NAL body's
/// tail (a spec-conforming NAL cannot end in `0x00` — the RBSP stop
/// bit guarantees a nonzero final byte — so all trailing zeros before
/// the next start code belong to that start code's prefix).
fn trim_trailing_zeros(mut body: &[u8]) -> &[u8] {
    while let Some((&last, rest)) = body.split_last() {
        if last == 0 {
            body = rest;
        } else {
            break;
        }
    }
    body
}

/// Annex-B byte-stream NAL splitter: yields the complete NAL units
/// (two-byte headers included) of an HEVC byte stream.
///
/// Leading garbage before the first start code is skipped; NAL bodies
/// truncated by end-of-data are yielded as-is (their parsers surface
/// the typed truncation). Zero-length bodies (two adjacent start
/// codes) are yielded and fail header parsing downstream — typed,
/// not silently dropped.
#[derive(Debug, Clone)]
pub struct AnnexBNals<'a> {
    data: &'a [u8],
    /// Byte index where the next NAL body starts (past its start
    /// code's `01`); `started` gates the initial start-code search.
    pos: usize,
    started: bool,
    done: bool,
}

impl<'a> AnnexBNals<'a> {
    /// Split `data` (a whole Annex-B byte stream) into NAL units.
    #[must_use]
    pub fn new(data: &'a [u8]) -> AnnexBNals<'a> {
        AnnexBNals {
            data,
            pos: 0,
            started: false,
            done: false,
        }
    }
}

impl<'a> Iterator for AnnexBNals<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        if !self.started {
            // Skip to (and past) the first start code; a stream with
            // no start code at all carries no NAL units.
            self.pos = next_start_code(self.data, 0)?;
            self.started = true;
        }
        let start = self.pos;
        // The NAL runs until the next start code's leading zeros;
        // `stop` (when found) is the *next* NAL body's first byte.
        let stop = next_start_code(self.data, start);
        let end_raw = stop.map_or(self.data.len(), |next| next.saturating_sub(3));
        if let Some(next_pos) = stop {
            self.pos = next_pos;
        } else {
            self.done = true;
        }
        let body = &self.data[start..end_raw.max(start).min(self.data.len())];
        Some(trim_trailing_zeros(body))
    }
}

/// Split an Annex-B byte stream into a `Vec` of complete NAL units
/// (convenience form of [`AnnexBNals`] for one-shot callers).
#[must_use]
pub fn annexb_nals(data: &[u8]) -> Vec<&[u8]> {
    AnnexBNals::new(data).collect()
}

/// Remove emulation-prevention bytes (§7.4.2.1): each `00 00 03
/// {00,01,02,03}` collapses to `00 00 {suffix}` — with the **suffix
/// byte re-scanned**, because an escaped `00` suffix can be the first
/// zero of the *next* escape (`00 00 03 00 00 03 00` ↔ `00 00 00 00
/// 00`: the pattern-true corpus VPS/SPS constraint-flag runs — see
/// the test pinned from 0100-mp4-video).
///
/// Writes into `out` (cleared first) so callers can reuse one buffer
/// across a whole stream — the steady-state path allocates nothing.
///
/// # Errors
/// - `Emulation { code: "hevc.ep.bad_suffix" }` — `00 00 03` followed
///   by a byte > `0x03` (no legal escaped RBSP produces this);
/// - `Emulation { code: "hevc.ep.dangling" }` — `00 00 03` at the
///   very end of the NAL (the escaped suffix byte is missing:
///   truncation).
pub fn unescape_rbsp_into(nal: &[u8], out: &mut Vec<u8>) -> Result<(), HevcError> {
    out.clear();
    out.reserve(nal.len());
    let mut i = 0usize;
    while i < nal.len() {
        if i + 2 < nal.len() && nal[i] == 0 && nal[i + 1] == 0 && nal[i + 2] == 3 {
            match nal.get(i + 3) {
                // `00 00 03` as the final bytes: the escaped suffix
                // byte is missing — truncation.
                None => {
                    return Err(HevcError::Emulation {
                        code: "hevc.ep.dangling",
                    });
                }
                Some(&suffix) if suffix <= 0x03 => {
                    // Drop only the 03; the suffix is re-scanned by
                    // the main loop (it may open the next escape).
                    out.extend_from_slice(&nal[i..i + 2]);
                    i += 3;
                }
                Some(_) => {
                    return Err(HevcError::Emulation {
                        code: "hevc.ep.bad_suffix",
                    });
                }
            }
        } else {
            out.push(nal[i]);
            i += 1;
        }
    }
    Ok(())
}

/// [`unescape_rbsp_into`] with a fresh buffer (one-shot callers).
///
/// # Errors
/// Propagates the emulation violations of [`unescape_rbsp_into`].
pub fn unescape_rbsp(nal: &[u8]) -> Result<Vec<u8>, HevcError> {
    let mut out = Vec::with_capacity(nal.len());
    unescape_rbsp_into(nal, &mut out)?;
    Ok(out)
}

/// Length-prefixed NAL splitter (MP4 sample data, Matroska
/// `V_MPEGH/ISO/HEVC` blocks): each NAL is `length_size` big-endian
/// length bytes + body.
///
/// Trailing padding (fewer than `length_size` bytes left, all zero)
/// ends the iteration, as MP4 samples are zero-padded to sample
/// boundaries; anything else that runs past the data is the typed
/// [`HevcError::Truncated`].
#[derive(Debug, Clone)]
pub struct LengthPrefixedNals<'a> {
    data: &'a [u8],
    pos: usize,
    length_size: usize,
}

impl<'a> LengthPrefixedNals<'a> {
    /// Split `data` using `length_size` (1..=4) length prefixes.
    ///
    /// # Errors
    /// [`HevcError::Invalid { code: "hevc.av.length_size" }`] when
    /// `length_size` is outside 1..=4.
    pub fn new(data: &'a [u8], length_size: u8) -> Result<LengthPrefixedNals<'a>, HevcError> {
        if !(1..=4).contains(&length_size) {
            return Err(HevcError::Invalid {
                code: "hevc.av.length_size",
            });
        }
        Ok(LengthPrefixedNals {
            data,
            pos: 0,
            length_size: length_size as usize,
        })
    }
}

impl<'a> Iterator for LengthPrefixedNals<'a> {
    type Item = Result<&'a [u8], HevcError>;

    fn next(&mut self) -> Option<Self::Item> {
        let rest = &self.data[self.pos.min(self.data.len())..];
        if rest.is_empty() {
            return None;
        }
        if rest.len() < self.length_size {
            // Zero padding ends a sample; anything else is damage —
            // either way the walk terminates here.
            self.pos = self.data.len();
            return if rest.iter().all(|&b| b == 0) {
                None
            } else {
                Some(Err(HevcError::Truncated {
                    needed: self.length_size as u64,
                    have: rest.len() as u64,
                }))
            };
        }
        let mut len: usize = 0;
        for &b in &rest[..self.length_size] {
            len = (len << 8) | b as usize;
        }
        if len == 0 {
            // Zero length with more data behind it: refuse to spin.
            self.pos = self.data.len();
            return Some(Err(HevcError::Invalid {
                code: "hevc.av.zero_length",
            }));
        }
        let total = len + self.length_size;
        if total > rest.len() {
            self.pos = self.data.len();
            return Some(Err(HevcError::Truncated {
                needed: total as u64,
                have: rest.len() as u64,
            }));
        }
        self.pos += total;
        Some(Ok(&rest[self.length_size..self.length_size + len]))
    }
}

/// One `hvcC` parameter-set array: a NAL unit type plus the counted
/// NAL units stored under it (ISO/IEC 14496-15 §8.3.3.1 "arrays").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HvcCArray {
    /// `array_completeness`: the array holds *all* parameter sets of
    /// this type that exist in the stream.
    pub completeness: bool,
    /// The array's 6-bit `NAL_unit_type` (e.g. 32 = VPS, 33 = SPS,
    /// 34 = PPS, 39 = prefix SEI).
    pub nal_unit_type: u8,
    /// The stored NAL units (complete, two-byte headers included), in
    /// record order.
    pub nals: Vec<Vec<u8>>,
}

/// The `hvcC` HEVCDecoderConfigurationRecord (ISO/IEC 14496-15
/// §8.3.3.1 / A.4.1): the parameter-set store MP4/MKV containers
/// carry, plus the NAL length geometry every length-prefixed packet
/// uses and the PTL/chroma/depth summary mirrored from the SPS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HvcCConfig {
    /// `general_profile_space` (0..=3).
    pub profile_space: u8,
    /// `general_tier_flag` (Main vs High tier).
    pub tier: bool,
    /// `general_profile_idc` (0..=31; mirrors SPS PTL).
    pub profile_idc: u8,
    /// The 32 `general_profile_compatibility_flag`s, MSB = flag 0.
    pub profile_compatibility_flags: u32,
    /// The 48-bit `general_constraint_indicator_flags` (progressive,
    /// interlaced, non-packed, frame-only, 44 reserved/inbld bits).
    pub constraint_indicator_flags: u64,
    /// `general_level_idc` (30 × level major, e.g. 93 = level 9.3).
    pub level_idc: u8,
    /// `min_spatial_segmentation_idc` (12 bits).
    pub min_spatial_segmentation_idc: u16,
    /// `parallelismType` (0..=3; 0 = unknown).
    pub parallelism_type: u8,
    /// `chromaFormat` from the SPS (0..=3).
    pub chroma_format: u8,
    /// `bitDepthLumaMinus8` (0..=7).
    pub bit_depth_luma_minus8: u8,
    /// `bitDepthChromaMinus8` (0..=7).
    pub bit_depth_chroma_minus8: u8,
    /// `avgFrameRate` (16 bits, in units of 256 fps when
    /// `constant_frame_rate` is set — raw value retained).
    pub avg_frame_rate: u16,
    /// `constantFrameRate` (0..=3).
    pub constant_frame_rate: u8,
    /// `numTemporalLayers` (0..=7).
    pub num_temporal_layers: u8,
    /// `temporalIdNested`.
    pub temporal_id_nested: bool,
    /// `lengthSizeMinusOne + 1` (1..=4): the NAL length prefix size.
    pub length_size: u8,
    /// The parameter-set arrays, in record order.
    pub arrays: Vec<HvcCArray>,
}

impl HvcCConfig {
    /// Parse a raw `hvcC` record (e.g. an MP4 `hvcC` box body or a
    /// Matroska `V_MPEGH/ISO/HEVC` `CodecPrivate`).
    ///
    /// The array-entry reserved bit (`0b01xxxxxx`'s bit 6) is read
    /// and ignored — real-world writers differ on it (ffmpeg zeroes
    /// it); the five fixed reserved patterns the corpus writers all
    /// honour (`f0/fc/fd/f8/f8`) are enforced, matching the
    /// `avcC` posture of card 5.F1. Trailing bytes after the last
    /// array are none of this parser's business and are left unread.
    ///
    /// # Errors
    /// Typed [`HevcError`]s: `Truncated` (short record/NAL),
    /// `Invalid { "hevc.hvcc.version" }` for a version other than 1,
    /// `Invalid { "hevc.hvcc.reserved" }` for a cleared fixed
    /// reserved pattern, `Invalid { "hevc.hvcc.zero_length" }` for a
    /// zero-length array NAL.
    pub fn parse(bytes: &[u8]) -> Result<HvcCConfig, HevcError> {
        const FIXED: usize = 23;
        if bytes.len() < FIXED {
            return Err(HevcError::Truncated {
                needed: FIXED as u64,
                have: bytes.len() as u64,
            });
        }
        if bytes[0] != 1 {
            return Err(HevcError::Invalid {
                code: "hevc.hvcc.version",
            });
        }
        // The five reserved patterns (bit positions per §8.3.3.1).
        if bytes[13] >> 4 != 0x0f
            || bytes[15] >> 2 != 0x3f
            || bytes[16] >> 2 != 0x3f
            || bytes[17] >> 3 != 0x1f
            || bytes[18] >> 3 != 0x1f
        {
            return Err(HevcError::Invalid {
                code: "hevc.hvcc.reserved",
            });
        }
        let mut compat: u32 = 0;
        for &b in &bytes[2..6] {
            compat = (compat << 8) | u32::from(b);
        }
        let mut constraint: u64 = 0;
        for &b in &bytes[6..12] {
            constraint = (constraint << 8) | u64::from(b);
        }
        let num_arrays = bytes[22];
        let mut pos = FIXED;
        let mut arrays = Vec::with_capacity(num_arrays as usize);
        for _ in 0..num_arrays {
            let Some(&hdr) = bytes.get(pos) else {
                return Err(HevcError::Truncated {
                    needed: (pos + 1) as u64,
                    have: bytes.len() as u64,
                });
            };
            pos += 1;
            let num_nalus = read_u16(bytes, &mut pos)?;
            let mut nals = Vec::with_capacity(num_nalus as usize);
            for _ in 0..num_nalus {
                let nal = read_counted_nal(bytes, &mut pos)?;
                nals.push(nal.to_vec());
            }
            arrays.push(HvcCArray {
                completeness: hdr & 0x80 != 0,
                nal_unit_type: hdr & 0x3f,
                nals,
            });
        }
        Ok(HvcCConfig {
            profile_space: bytes[1] >> 6,
            tier: bytes[1] & 0x20 != 0,
            profile_idc: bytes[1] & 0x1f,
            profile_compatibility_flags: compat,
            constraint_indicator_flags: constraint,
            level_idc: bytes[12],
            min_spatial_segmentation_idc: u16::from(bytes[13] & 0x0f) << 8 | u16::from(bytes[14]),
            parallelism_type: bytes[15] & 0x03,
            chroma_format: bytes[16] & 0x03,
            bit_depth_luma_minus8: bytes[17] & 0x07,
            bit_depth_chroma_minus8: bytes[18] & 0x07,
            avg_frame_rate: u16::from(bytes[19]) << 8 | u16::from(bytes[20]),
            constant_frame_rate: bytes[21] >> 6,
            num_temporal_layers: (bytes[21] >> 3) & 0x07,
            temporal_id_nested: bytes[21] & 0x04 != 0,
            length_size: (bytes[21] & 0x03) + 1,
            arrays,
        })
    }

    /// Heuristic: do these bytes look like an `hvcC` record (vs. e.g.
    /// an Annex-B `CodecPrivate`)? Used by callers whose containers
    /// allow both forms. Matches ffmpeg's real-world writer output
    /// (reserved patterns set, version 1).
    #[must_use]
    pub fn looks_like_hvcc(bytes: &[u8]) -> bool {
        bytes.len() >= 23
            && bytes[0] == 1
            && bytes[13] >> 4 == 0x0f
            && bytes[15] >> 2 == 0x3f
            && bytes[16] >> 2 == 0x3f
    }
}

/// One big-endian `u16` at `*pos`, advancing it.
fn read_u16(bytes: &[u8], pos: &mut usize) -> Result<u16, HevcError> {
    if bytes.len() < *pos + 2 {
        return Err(HevcError::Truncated {
            needed: (*pos + 2) as u64,
            have: bytes.len() as u64,
        });
    }
    let v = u16::from(bytes[*pos]) << 8 | u16::from(bytes[*pos + 1]);
    *pos += 2;
    Ok(v)
}

/// One `u16`-length-prefixed NAL inside an `hvcC` record.
fn read_counted_nal<'a>(
    bytes: &'a [u8],
    pos: &mut usize,
) -> Result<&'a [u8], HevcError> {
    let len = usize::from(read_u16(bytes, pos)?);
    if len == 0 || bytes.len() < *pos + len {
        if len == 0 {
            return Err(HevcError::Invalid {
                code: "hevc.hvcc.zero_length",
            });
        }
        return Err(HevcError::Truncated {
            needed: (*pos + len) as u64,
            have: bytes.len() as u64,
        });
    }
    let nal = &bytes[*pos..*pos + len];
    *pos += len;
    Ok(nal)
}

/// MSB-first bit reader over an RBSP (emulation prevention already
/// removed) with Exp-Golomb support (§9.1/§9.2) — the shared
/// primitive of every `hevc::parser` syntax reader (VPS/SPS/PPS and
/// the slice-segment header).
///
/// All reads are bounds-checked and return typed errors; the reader
/// never panics and never reads past the buffer.
#[derive(Debug, Clone)]
pub struct BitReader<'a> {
    data: &'a [u8],
    bit_pos: usize,
}

impl<'a> BitReader<'a> {
    /// Read `bits` (1..=32) from `data` starting at bit 0.
    #[must_use]
    pub fn new(data: &'a [u8]) -> BitReader<'a> {
        BitReader { data, bit_pos: 0 }
    }

    fn total_bits(&self) -> u64 {
        self.data.len() as u64 * 8
    }

    /// Read one bit.
    ///
    /// # Errors
    /// [`HevcError::Truncated`] past the end of the RBSP.
    pub fn bit(&mut self) -> Result<u32, HevcError> {
        if self.bit_pos as u64 >= self.total_bits() {
            return Err(HevcError::Truncated {
                needed: (self.bit_pos + 1) as u64,
                have: self.total_bits(),
            });
        }
        let byte = self.data[self.bit_pos / 8];
        let shift = 7 - (self.bit_pos % 8);
        self.bit_pos += 1;
        Ok(((byte >> shift) & 1) as u32)
    }

    /// Read `n` (1..=32) bits, MSB first.
    ///
    /// # Errors
    /// [`HevcError::Truncated`] when fewer than `n` bits remain;
    /// `Invalid { "hevc.bits.width" }` for `n` outside 1..=32.
    pub fn bits(&mut self, n: u32) -> Result<u32, HevcError> {
        if n == 0 || n > 32 {
            return Err(HevcError::Invalid {
                code: "hevc.bits.width",
            });
        }
        let mut acc: u64 = 0;
        for _ in 0..n {
            acc = (acc << 1) | u64::from(self.bit()?);
        }
        Ok(acc as u32)
    }

    /// Read an unsigned Exp-Golomb code `ue(v)` (§9.1) as u64 (the
    /// caller range-checks against the field's declared bound).
    ///
    /// # Errors
    /// [`HevcError::Truncated`] when the code runs past the RBSP;
    /// `Invalid { "hevc.ue.overflow" }` for a prefix longer than 31
    /// zero bits (cannot encode a u64-representable field).
    pub fn ue(&mut self) -> Result<u64, HevcError> {
        let mut leading_zeros = 0u32;
        while self.bit()? == 0 {
            leading_zeros += 1;
            if leading_zeros > 31 {
                return Err(HevcError::Invalid {
                    code: "hevc.ue.overflow",
                });
            }
        }
        let mut value: u64 = 1;
        for _ in 0..leading_zeros {
            value = (value << 1) | u64::from(self.bit()?);
        }
        Ok(value - 1)
    }

    /// Read a signed Exp-Golomb code `se(v)` (§9.2) as i64.
    ///
    /// # Errors
    /// Propagates [`BitReader::ue`] errors.
    pub fn se(&mut self) -> Result<i64, HevcError> {
        let k = self.ue()?;
        // (−1)^k+1 · ⌈k/2⌉ — mapped without overflow for k ≤ 2^32.
        let magnitude = k.div_ceil(2) as i64;
        if k % 2 == 0 {
            Ok(-magnitude)
        } else {
            Ok(magnitude)
        }
    }

    /// Is there at least one more `1` bit in the unread data (the
    /// `more_rbsp_data()` predicate, §7.2)?
    #[must_use]
    pub fn more_rbsp_data(&self) -> bool {
        let mut i = self.bit_pos;
        while i < self.data.len() * 8 {
            if (self.data[i / 8] >> (7 - (i % 8))) & 1 == 1 {
                return true;
            }
            i += 1;
        }
        false
    }

    /// Consume and verify `rbsp_trailing_bits()` (§7.3.2.11): a `1`
    /// stop bit, then only zero bits to the byte boundary and end of
    /// the RBSP.
    ///
    /// # Errors
    /// `Truncated` when no stop bit exists at all; `Invalid {
    /// "hevc.rbsp.tail" }` for nonzero bits after the stop bit.
    pub fn finish(&mut self) -> Result<(), HevcError> {
        // Scan for the stop bit (the next 1 bit, if any).
        let stop = loop {
            if self.bit_pos as u64 >= self.total_bits() {
                return Err(HevcError::Truncated {
                    needed: (self.bit_pos + 1) as u64,
                    have: self.total_bits(),
                });
            }
            if self.bit()? == 1 {
                break self.bit_pos - 1;
            }
        };
        // Everything after the stop bit must be zero — first the
        // rest of its byte (masked, data bits before it do not
        // count), then any following bytes.
        let (byte, bit_off) = (stop / 8, stop % 8);
        if bit_off + 1 < 8 {
            let mask: u8 = ((1u16 << (7 - bit_off)) - 1) as u8;
            if self.data[byte] & mask != 0 {
                return Err(HevcError::Invalid {
                    code: "hevc.rbsp.tail",
                });
            }
        }
        if self.data[byte + 1..].iter().any(|&b| b != 0) {
            return Err(HevcError::Invalid {
                code: "hevc.rbsp.tail",
            });
        }
        Ok(())
    }
}
