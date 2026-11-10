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
fn read_counted_nal<'a>(bytes: &'a [u8], pos: &mut usize) -> Result<&'a [u8], HevcError> {
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

    /// Is there more RBSP data before the `rbsp_trailing_bits()`?
    /// True iff at least two `1` bits remain unread: the stop bit is
    /// the *last* `1` in the RBSP (§7.2), so a lone remaining `1` is
    /// exactly the trailing bits and not data. (This is stricter than
    /// a naive "any 1 bit" scan, which would count the stop bit as
    /// data — noted for the h264 sibling card's identical helper.)
    #[must_use]
    pub fn more_rbsp_data(&self) -> bool {
        let mut ones = 0u32;
        let mut i = self.bit_pos;
        while i < self.data.len() * 8 {
            if (self.data[i / 8] >> (7 - (i % 8))) & 1 == 1 {
                ones += 1;
                if ones >= 2 {
                    return true;
                }
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// MSB-first bit writer for handcrafting syntax (the inverse of
    /// [`BitReader`]) — shared by every sibling test module.
    pub(crate) struct Bits {
        out: Vec<u8>,
        bit: u8,
        n: u32,
    }

    impl Bits {
        pub(crate) fn new() -> Bits {
            Bits {
                out: Vec::new(),
                bit: 0,
                n: 0,
            }
        }

        pub(crate) fn put(&mut self, n: u32, value: u64) {
            assert!(
                n <= 64 && (n == 64 || value < 1u64 << n),
                "bad put({n}, {value})"
            );
            for i in (0..n).rev() {
                self.bit = (self.bit << 1) | ((value >> i) & 1) as u8;
                self.n += 1;
                if self.n == 8 {
                    self.out.push(self.bit);
                    self.bit = 0;
                    self.n = 0;
                }
            }
        }

        pub(crate) fn flag(&mut self, v: bool) {
            self.put(1, u64::from(v));
        }

        pub(crate) fn ue(&mut self, v: u64) {
            let v1 = v + 1;
            let width = 64 - v1.leading_zeros();
            self.put(width - 1, 0);
            self.put(width, v1);
        }

        pub(crate) fn se(&mut self, v: i64) {
            let k = if v <= 0 { -2 * v } else { 2 * v - 1 };
            self.ue(k as u64);
        }

        /// Finish with `rbsp_trailing_bits` and return the RBSP.
        pub(crate) fn finish(mut self) -> Vec<u8> {
            self.flag(true);
            while self.n != 0 {
                self.bit <<= 1;
                self.n += 1;
                if self.n == 8 {
                    self.out.push(self.bit);
                    self.bit = 0;
                    self.n = 0;
                }
            }
            self.out
        }

        /// Wrap an RBSP in a NAL unit of the given type (layer 0,
        /// temporal id 0 → plus1 = 1), no emulation prevention
        /// (test RBSPs are handcrafted to avoid 00 00 0x runs).
        pub(crate) fn nal(self, unit_type: u8) -> Vec<u8> {
            let rbsp = self.finish();
            let mut nal = vec![(unit_type << 1) & 0x7e, 0x01];
            nal.extend_from_slice(&rbsp);
            nal
        }
    }

    /// The real 0100-mp4-video.mp4 VPS NAL (x265 4.1, from the
    /// `hvcC` arrays) — pins the overlapping-escape constraint run.
    pub(crate) const VPS_0100: [u8; 24] = [
        0x40, 0x01, 0x0c, 0x01, 0xff, 0xff, 0x01, 0x60, 0x00, 0x00, 0x03, 0x00, 0x90, 0x00, 0x00,
        0x03, 0x00, 0x00, 0x03, 0x00, 0x5d, 0xba, 0x02, 0x40,
    ];

    /// The real 0100 SPS NAL (Main / level 93 / 1024x768).
    pub(crate) const SPS_0100: [u8; 41] = [
        0x42, 0x01, 0x01, 0x01, 0x60, 0x00, 0x00, 0x03, 0x00, 0x90, 0x00, 0x00, 0x03, 0x00, 0x00,
        0x03, 0x00, 0x5d, 0xa0, 0x02, 0x00, 0x80, 0x30, 0x16, 0x5b, 0xa4, 0xa4, 0xc2, 0xf0, 0x56,
        0x80, 0x80, 0x00, 0x00, 0x03, 0x00, 0x80, 0x00, 0x00, 0x0c, 0x04,
    ];

    /// The real 0100 PPS NAL.
    pub(crate) const PPS_0100: [u8; 6] = [0x44, 0x01, 0xc0, 0x73, 0xc1, 0x89];

    /// Corpus path lookup (skip-with-msg contract of the wave): the
    /// `LUMEN_HEVC_CORPUS` env var, else the canonical corpus dir.
    pub(crate) fn corpus_path(name: &str) -> Option<std::path::PathBuf> {
        let dir = std::env::var("LUMEN_HEVC_CORPUS") // lumen-allow-literal: cfg(test) corpus-gate env-var name — test harness knob, not product config
            .unwrap_or_else(|_| "/tmp/my-project/corpus-v4/media".to_string()); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        let p = std::path::Path::new(&dir).join(name);
        if p.exists() { Some(p) } else { None }
    }

    /// The corpus's splitmix64 (wave convention) for fuzz-lite.
    pub(crate) fn splitmix64(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    #[test]
    fn nal_classification_roundtrips_table_7_1() {
        assert_eq!(NalUnitType::from_id(0), NalUnitType::TrailN);
        assert_eq!(NalUnitType::from_id(1), NalUnitType::TrailR);
        assert_eq!(NalUnitType::from_id(9), NalUnitType::RaslR);
        assert_eq!(NalUnitType::from_id(19), NalUnitType::IdrWRadl);
        assert_eq!(NalUnitType::from_id(21), NalUnitType::CraNut);
        assert_eq!(NalUnitType::from_id(32), NalUnitType::VpsNut);
        assert_eq!(NalUnitType::from_id(33), NalUnitType::SpsNut);
        assert_eq!(NalUnitType::from_id(34), NalUnitType::PpsNut);
        assert_eq!(NalUnitType::from_id(39), NalUnitType::SeiPrefix);
        assert_eq!(NalUnitType::from_id(13), NalUnitType::Reserved(13));
        assert_eq!(NalUnitType::from_id(22), NalUnitType::Reserved(22));
        assert_eq!(NalUnitType::from_id(45), NalUnitType::Reserved(45));
        assert_eq!(NalUnitType::from_id(48), NalUnitType::Unspecified(48));
        assert_eq!(NalUnitType::from_id(63), NalUnitType::Unspecified(63));
        for id in 0u8..=63 {
            assert_eq!(NalUnitType::from_id(id).id(), id);
        }
        assert!(NalUnitType::from_id(16).is_irap());
        assert!(NalUnitType::from_id(23).is_irap());
        assert!(!NalUnitType::from_id(24).is_irap());
        assert!(NalUnitType::from_id(21).is_vcl());
        assert!(!NalUnitType::from_id(32).is_vcl());
    }

    #[test]
    fn header_split_decodes_two_byte_geometry() {
        let (h, payload) =
            NalHeader::split(&[0x40, 0x01, 0xaa]).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(h.unit_type, NalUnitType::VpsNut);
        assert_eq!(h.layer_id, 0);
        assert_eq!(h.temporal_id_plus1, 1);
        assert_eq!(payload, &[0xaa]);
        // Type 33 (SPS) with layer 33 and tid+1 7: b0 = 0b100_0011,
        // b1 = 0b00001_111 → layer 0b100001, tid 7.
        let (h, _) = NalHeader::split(&[0x43, 0x0f]).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(h.unit_type, NalUnitType::SpsNut);
        assert_eq!(h.layer_id, 33);
        assert_eq!(h.temporal_id_plus1, 7);
        assert_eq!(
            NalHeader::split(&[0x40]),
            Err(HevcError::Truncated { needed: 2, have: 1 })
        );
        assert_eq!(
            NalHeader::split(&[]),
            Err(HevcError::Truncated { needed: 2, have: 0 })
        );
        assert_eq!(
            NalHeader::split(&[0xc0, 0x01]),
            Err(HevcError::ForbiddenZeroBit)
        );
        assert_eq!(
            NalHeader::split(&[0x40, 0x00]),
            Err(HevcError::Invalid {
                code: "hevc.nal.temporal_id"
            })
        );
    }

    #[test]
    fn annexb_splits_three_and_four_byte_prefixes() {
        let stream = [
            0xff, 0xff, // leading garbage before first start code
            0x00, 0x00, 0x00, 0x01, 0x40, 0x01, 0xaa, //
            0x00, 0x00, 0x01, 0x42, 0x01, 0xbb, 0x00, 0x00, // trailing zeros
        ];
        let nals: Vec<&[u8]> = AnnexBNals::new(&stream).collect();
        assert_eq!(
            nals,
            vec![&[0x40u8, 0x01, 0xaa][..], &[0x42u8, 0x01, 0xbb][..]]
        );
        assert!(AnnexBNals::new(&[0x00, 0x00, 0x02, 0x99]).next().is_none());
    }

    #[test]
    fn emulation_removal_handles_overlapping_escapes() {
        // The corpus 0100 VPS constraint run: five RBSP zeros behind
        // a 0x90 escape in TWO chained escapes — the case a
        // consume-the-suffix unescape mis-decodes (see the module
        // doc; the exact bytes are pinned in VPS_0100).
        assert_eq!(
            unescape_rbsp(&[
                0x60, 0x00, 0x00, 0x03, 0x00, 0x90, 0x00, 0x00, 0x03, 0x00, 0x00, 0x03, 0x00, 0x5d
            ])
            .unwrap_or_default(),
            vec![
                0x60, 0x00, 0x00, 0x00, 0x90, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5d
            ]
        );
        // Simple forms.
        assert_eq!(
            unescape_rbsp(&[0x00, 0x00, 0x03, 0x02, 0xaa]).unwrap_or_default(),
            vec![0x00, 0x00, 0x02, 0xaa]
        );
        assert_eq!(
            unescape_rbsp(&[0x00, 0x00, 0x03, 0x03]).unwrap_or_default(),
            vec![0x00, 0x00, 0x03]
        );
        // Typed violations.
        assert_eq!(
            unescape_rbsp(&[0x00, 0x00, 0x03, 0x99]),
            Err(HevcError::Emulation {
                code: "hevc.ep.bad_suffix"
            })
        );
        assert_eq!(
            unescape_rbsp(&[0xaa, 0x00, 0x00, 0x03]),
            Err(HevcError::Emulation {
                code: "hevc.ep.dangling"
            })
        );
        // The whole real VPS payload round-trips (header stripped):
        // 22 escaped bytes → 19 RBSP bytes, three chained escapes
        // collapsed to five constraint-zero bytes, level intact.
        let rbsp = unescape_rbsp(&VPS_0100[2..]).unwrap_or_default();
        assert_eq!(rbsp.len(), 19);
        assert_eq!(rbsp[15], 0x5d); // level_idc survives the unescape
    }

    #[test]
    fn length_prefixed_split_and_typed_failures() {
        // length_size 2: [2, 0x40, 0x01] [3, 0x42, 0x01, 0xaa] pad.
        let data = [0x00, 0x02, 0x40, 0x01, 0x00, 0x03, 0x42, 0x01, 0xaa, 0x00];
        let nals: Vec<_> = LengthPrefixedNals::new(&data, 2)
            .unwrap_or_else(|e| unreachable!("{e}"))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(nals, vec![&[0x40u8, 0x01][..], &[0x42u8, 0x01, 0xaa][..]]);
        // Zero padding ends the walk.
        assert_eq!(
            LengthPrefixedNals::new(&[0x00, 0x02, 0x40, 0x01, 0x00], 2)
                .unwrap_or_else(|e| unreachable!("{e}"))
                .count(),
            1
        );
        // Non-zero short tail: typed truncation.
        assert_eq!(
            LengthPrefixedNals::new(&[0x00, 0x02, 0x40, 0x01, 0x77], 2)
                .unwrap_or_else(|e| unreachable!("{e}"))
                .last(),
            Some(Err(HevcError::Truncated { needed: 2, have: 1 }))
        );
        // Zero length mid-stream: refuse to spin.
        assert_eq!(
            LengthPrefixedNals::new(&[0x00, 0x00, 0x40], 2)
                .unwrap_or_else(|e| unreachable!("{e}"))
                .last(),
            Some(Err(HevcError::Invalid {
                code: "hevc.av.zero_length"
            }))
        );
        // Overrun: typed truncation with byte geometry.
        assert_eq!(
            LengthPrefixedNals::new(&[0x00, 0x09, 0x40], 2)
                .unwrap_or_else(|e| unreachable!("{e}"))
                .last(),
            Some(Err(HevcError::Truncated {
                needed: 11,
                have: 3
            }))
        );
        assert!(matches!(
            LengthPrefixedNals::new(&[], 5),
            Err(HevcError::Invalid {
                code: "hevc.av.length_size"
            })
        ));
    }

    /// Build a minimal-but-real `hvcC` record around the corpus
    /// parameter sets (the 0100 record's arrays — VPS/SPS/PPS, the
    /// record's 4th SEI array dropped and the count fixed to match —
    /// over the byte-exact 0100 header; each array carries its
    /// `numNalus` per ISO/IEC 14496-15 §8.3.3.1).
    fn hvcc_0100() -> Vec<u8> {
        let mut rec = vec![
            0x01, 0x01, 0x60, 0x00, 0x00, 0x00, 0x90, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5d, 0xf0,
            0x00, 0xfc, 0xfd, 0xf8, 0xf8, 0x00, 0x00, 0x0f, 0x03,
        ];
        for (nut, nal) in [
            (32u8, &VPS_0100[..]),
            (33, &SPS_0100[..]),
            (34, &PPS_0100[..]),
        ] {
            rec.push(nut);
            rec.extend_from_slice(&[0x00, 0x01]); // numNalus = 1
            rec.extend_from_slice(
                &u16::try_from(nal.len())
                    .unwrap_or_else(|e| unreachable!("nal len fits u16: {e}"))
                    .to_be_bytes(),
            );
            rec.extend_from_slice(nal);
        }
        rec
    }

    #[test]
    fn hvcc_parse_field_exact_on_corpus_arrays() {
        let rec = hvcc_0100();
        let cfg = HvcCConfig::parse(&rec).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(cfg.profile_space, 0);
        assert!(!cfg.tier);
        assert_eq!(cfg.profile_idc, 1); // Main
        assert_eq!(cfg.profile_compatibility_flags, 0x6000_0000);
        assert_eq!(cfg.constraint_indicator_flags, 0x9000_0000_0000);
        assert_eq!(cfg.level_idc, 93); // 9.3, the ffprobe spelling
        assert_eq!(cfg.min_spatial_segmentation_idc, 0);
        assert_eq!(cfg.parallelism_type, 0);
        assert_eq!(cfg.chroma_format, 1); // 4:2:0
        assert_eq!(cfg.bit_depth_luma_minus8, 0);
        assert_eq!(cfg.bit_depth_chroma_minus8, 0);
        assert_eq!(cfg.avg_frame_rate, 0);
        assert_eq!(cfg.constant_frame_rate, 0);
        assert_eq!(cfg.num_temporal_layers, 1);
        assert!(cfg.temporal_id_nested);
        assert_eq!(cfg.length_size, 4);
        assert_eq!(cfg.arrays.len(), 3);
        assert_eq!(cfg.arrays[0].nal_unit_type, 32);
        assert!(!cfg.arrays[0].completeness);
        assert_eq!(cfg.arrays[0].nals, vec![VPS_0100.to_vec()]);
        assert_eq!(cfg.arrays[1].nal_unit_type, 33);
        assert_eq!(cfg.arrays[1].nals, vec![SPS_0100.to_vec()]);
        assert_eq!(cfg.arrays[2].nal_unit_type, 34);
        assert_eq!(cfg.arrays[2].nals, vec![PPS_0100.to_vec()]);
        assert!(HvcCConfig::looks_like_hvcc(&rec));
    }

    #[test]
    fn hvcc_typed_error_paths() {
        let good = hvcc_0100();
        assert_eq!(
            HvcCConfig::parse(&good[..22]),
            Err(HevcError::Truncated {
                needed: 23,
                have: 22
            })
        );
        let mut bad = good.clone();
        bad[0] = 2;
        assert_eq!(
            HvcCConfig::parse(&bad),
            Err(HevcError::Invalid {
                code: "hevc.hvcc.version"
            })
        );
        let mut bad = good.clone();
        bad[16] = 0x01; // cleared chroma reserved bits
        assert_eq!(
            HvcCConfig::parse(&bad),
            Err(HevcError::Invalid {
                code: "hevc.hvcc.reserved"
            })
        );
        // Array count promises a NAL that isn't there.
        let mut truncated = good.clone();
        truncated[22] = 4; // 4 arrays, only 3 present
        assert!(matches!(
            HvcCConfig::parse(&truncated),
            Err(HevcError::Truncated { .. })
        ));
        // Zero-length array NAL (a 4th SPS array whose single NAL
        // has length 0).
        let mut zero = good.clone();
        zero[22] = 4;
        zero.push(33);
        zero.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        assert_eq!(
            HvcCConfig::parse(&zero),
            Err(HevcError::Invalid {
                code: "hevc.hvcc.zero_length"
            })
        );
        // NAL length overruns the record.
        let mut over = good.clone();
        over[22] = 4;
        over.push(33);
        over.extend_from_slice(&[0x00, 0x01, 0x00, 0x63]); // len 99 > tail
        assert!(matches!(
            HvcCConfig::parse(&over),
            Err(HevcError::Truncated { .. })
        ));
        assert!(!HvcCConfig::looks_like_hvcc(&[0x00; 23]));
    }

    #[test]
    fn bit_reader_roundtrip_and_bounds() {
        let mut b = Bits::new();
        b.put(2, 0b10);
        b.flag(true);
        b.put(5, 0x1f);
        b.ue(0);
        b.ue(5);
        b.ue(255);
        b.se(-3);
        b.se(4);
        let rbsp = b.finish();
        let mut r = BitReader::new(&rbsp);
        assert_eq!(r.bits(2), Ok(0b10));
        assert_eq!(r.bit(), Ok(1));
        assert_eq!(r.bits(5), Ok(0x1f));
        assert_eq!(r.ue(), Ok(0));
        assert_eq!(r.ue(), Ok(5));
        assert_eq!(r.ue(), Ok(255));
        assert_eq!(r.se(), Ok(-3));
        assert_eq!(r.se(), Ok(4));
        assert!(!r.more_rbsp_data());
        assert!(r.finish().is_ok());
        // Reads past the end are typed, not panics.
        let mut r = BitReader::new(&[0x80]);
        assert_eq!(r.bits(9), Err(HevcError::Truncated { needed: 9, have: 8 }));
        assert_eq!(
            r.bits(0),
            Err(HevcError::Invalid {
                code: "hevc.bits.width"
            })
        );
        assert_eq!(
            r.bits(33),
            Err(HevcError::Invalid {
                code: "hevc.bits.width"
            })
        );
        // 32 zero bits then data: the ue prefix cap fires at >31.
        let mut r = BitReader::new(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x80]);
        assert_eq!(
            r.ue(),
            Err(HevcError::Invalid {
                code: "hevc.ue.overflow"
            })
        );
        // Nonzero tail after the stop bit is a violation.
        let mut r = BitReader::new(&[0x03]);
        assert_eq!(
            r.finish(),
            Err(HevcError::Invalid {
                code: "hevc.rbsp.tail"
            })
        );
        let mut r = BitReader::new(&[0x00]);
        assert_eq!(r.finish(), Err(HevcError::Truncated { needed: 9, have: 8 }));
    }

    #[test]
    fn fuzz_lite_no_panics_on_random_and_mutated_bytes() {
        // +FB (card 5.G1): 10k splitmix64 random buffers plus 5k
        // mutations of the real corpus parameter sets, driven
        // through every surface. Structural invariants: the
        // splitters never yield overlapping/descaping slices; the
        // parsers never panic; every failure is a typed HevcError.
        let mut state = 0x5eed_1a5e_u64;
        for i in 0..10_000u32 {
            let len = (splitmix64(&mut state) % 512) as usize;
            let mut buf = Vec::with_capacity(len);
            for _ in 0..len {
                buf.push((splitmix64(&mut state) & 0xff) as u8);
            }
            // Frame it as each NAL type so random bytes reach the
            // deep syntax walks, not just the header check.
            for (head, parse_vps, parse_sps, parse_pps) in [
                ([0x40u8, 0x01], true, false, false),
                ([0x42, 0x01], false, true, false),
                ([0x44, 0x01], false, false, true),
            ] {
                let mut nal = head.to_vec();
                nal.extend_from_slice(&buf);
                let _ = crate::video::hevc::parser::vps::VideoParameterSet::parse(&nal);
                let _ = crate::video::hevc::parser::sps::SeqParameterSet::parse(&nal);
                let _ = crate::video::hevc::parser::pps::PicParameterSet::parse(&nal);
                if parse_vps || parse_sps || parse_pps {
                    // Also exercise the shared unescape on the raw tail.
                    let _ = unescape_rbsp(&nal[2..]);
                }
            }
            if i % 7 == 0 {
                let _ = annexb_nals(&buf);
                let _ = HvcCConfig::parse(&buf);
            }
        }
        // Mutations of the real 0100 parameter sets + the hvcC: bit
        // flips, byte sets, truncations.
        let seeds: Vec<Vec<u8>> = [
            VPS_0100.to_vec(),
            SPS_0100.to_vec(),
            PPS_0100.to_vec(),
            hvcc_0100(),
        ]
        .into_iter()
        .chain((0..16).map(|i| {
            let mut v = SPS_0100.to_vec();
            v.truncate(i);
            v
        }))
        .collect();
        for k in 0..5_000u32 {
            let mut seed = seeds[(k as usize) % seeds.len()].clone();
            if seed.is_empty() {
                seed.push(0); // mutated empties stay parseable inputs
            }
            let rounds = (splitmix64(&mut state) % 4) + 1;
            for _ in 0..rounds {
                match splitmix64(&mut state) % 4 {
                    0 => {
                        let pos = (splitmix64(&mut state) as usize) % seed.len();
                        seed[pos] ^= 1 << (splitmix64(&mut state) % 8);
                    }
                    1 => {
                        let pos = (splitmix64(&mut state) as usize) % seed.len();
                        seed[pos] = (splitmix64(&mut state) & 0xff) as u8;
                    }
                    2 => {
                        let cut = (splitmix64(&mut state) as usize) % seed.len();
                        seed.truncate(cut.max(1));
                    }
                    _ => {
                        seed.push((splitmix64(&mut state) & 0xff) as u8);
                    }
                }
            }
            let _ = crate::video::hevc::parser::vps::VideoParameterSet::parse(&seed);
            let _ = crate::video::hevc::parser::sps::SeqParameterSet::parse(&seed);
            let _ = crate::video::hevc::parser::pps::PicParameterSet::parse(&seed);
            let _ = HvcCConfig::parse(&seed);
            let _ = unescape_rbsp(&seed);
        }
    }
    /// The real corpus 0044 lane (raw Annex-B .265, Rext 360x640):
    /// every parameter set parses, the id lattice is consistent
    /// (VPS←SPS←PPS), the SPS and VPS agree on the general PTL, and
    /// the slice-segment prefix parses for a representative sample
    /// of VCL NALs. The ffprobe oracle comparison lives in
    /// `tests/hevc_parser_corpus.rs` (field table in the worklog).
    #[test]
    fn corpus_0044_parameter_sets_and_slice_prefixes_parse() {
        let Some(path) = corpus_path("0044-raw-video.265") else {
            eprintln!("SKIP hevc corpus: 0044-raw-video.265 absent (corpus re-imaged)");
            return;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("SKIP hevc corpus: 0044 unreadable");
            return;
        };
        let mut vps = None;
        let mut sps = None;
        let mut pps = None;
        for nal in AnnexBNals::new(&bytes) {
            if let Ok((h, _)) = NalHeader::split(nal) {
                match h.unit_type {
                    NalUnitType::VpsNut if vps.is_none() => {
                        vps = crate::video::hevc::parser::vps::VideoParameterSet::parse(nal).ok();
                    }
                    NalUnitType::SpsNut if sps.is_none() => {
                        sps = crate::video::hevc::parser::sps::SeqParameterSet::parse(nal).ok();
                    }
                    NalUnitType::PpsNut if pps.is_none() => {
                        pps = crate::video::hevc::parser::pps::PicParameterSet::parse(nal).ok();
                    }
                    _ => {}
                }
            }
        }
        let (vps, sps, pps) = (vps, sps, pps);
        let (Some(vps), Some(sps), Some(pps)) = (&vps, &sps, &pps) else {
            unreachable!(
                "0044 must yield VPS+SPS+PPS (got {}/{}/{}), parser regressed",
                vps.is_some(),
                sps.is_some(),
                pps.is_some()
            );
        };
        // The id lattice: SPS names the VPS, PPS names the SPS.
        assert_eq!(sps.video_parameter_set_id, vps.video_parameter_set_id);
        assert_eq!(pps.seq_parameter_set_id, sps.seq_parameter_set_id);
        // General PTL agreement between VPS and SPS (base streams).
        assert_eq!(sps.profile_tier_level, vps.profile_tier_level);
        // Slice-segment prefixes over the seam derived from the sets.
        let sps_seam = crate::video::hevc::parser::slice::SliceSegmentSpsInfo::from_sps(sps).ok();
        let pps_seam = crate::video::hevc::parser::slice::SliceSegmentPpsInfo::from_pps(pps);
        let (Some(sps_seam), pps_seam) = (sps_seam, pps_seam) else {
            unreachable!("0044 SPS geometry rejected by the slice seam");
        };
        let poc_mask = (1u64 << sps_seam.log2_max_pic_order_cnt_lsb) - 1;
        let mut vcl = 0usize;
        let mut first_irap = 0usize;
        for nal in AnnexBNals::new(&bytes) {
            let Ok((h, _)) = NalHeader::split(nal) else {
                continue;
            };
            if !h.unit_type.is_vcl() || vcl >= 64 {
                continue;
            }
            vcl += 1;
            let parsed = crate::video::hevc::parser::slice::SliceSegmentHeader::parse(
                nal, &sps_seam, &pps_seam,
            )
            .unwrap_or_else(|e| unreachable!("0044 slice {vcl}: {e}"));
            assert_eq!(parsed.slice_pic_parameter_set_id, pps.pic_parameter_set_id);
            if let Some(poc) = parsed.slice_pic_order_cnt_lsb {
                assert!(
                    u64::from(poc) <= poc_mask,
                    "poc lsb {poc} > mask {poc_mask}"
                );
            }
            if h.unit_type.is_irap() && parsed.first_slice_segment_in_pic {
                first_irap += 1;
            }
        }
        assert!(vcl > 10, "0044 carries a slice train, walked {vcl}");
        assert!(first_irap >= 1, "0044 must open with an IRAP picture");
    }
}
