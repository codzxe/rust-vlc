//! NAL-unit layer (ITU-T H.264 §7.3.1, §7.4.1): byte-stream framing
//! in both entry forms, emulation prevention, NAL classification, the
//! `avcC` record, and the shared Exp-Golomb bit reader.
//!
//! # Entry forms (both handled, card 5.F1 step 1)
//!
//! - **Annex-B** (`.h264` elementary streams, MPEG-TS, some Matroska):
//!   NAL units delimited by `00 00 01` (3-byte) or `00 00 00 01`
//!   (4-byte, leading `zero_byte`) start-code prefixes —
//!   [`AnnexBNals`].
//! - **AVC length-prefixed** (MP4 `stco`/`mdat`, Matroska
//!   `V_MPEG4/ISO/AVC`): each NAL unit preceded by
//!   `lengthSizeMinusOne+1` big-endian length bytes, geometry from
//!   the `avcC` record — [`AvcNals`] + [`AvcConfig`].
//!
//! # Memory discipline
//!
//! Both splitters are iterators of **borrowed slices** (zero copy, no
//! allocation). Emulation-prevention removal necessarily copies
//! (prefix bytes are dropped mid-stream); [`unescape_rbsp_into`] lets
//! a caller reuse one buffer for a whole stream's NALs — the
//! steady-state parse path allocates nothing.

use core::fmt;

/// Typed parse failure for the H.264 parser surface (card 5.F1: no
/// panics, no damage-smoothing — every failure names its rule).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum H264Error {
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
    /// spec itself (§7.4.1: receivers shall discard such units).
    ForbiddenZeroBit,
    /// An emulation-prevention violation: `00 00 03` whose suffix is
    /// outside `{00,01,02,03}`, or `00 00 03` cut off at the end of
    /// the NAL (§7.4.1 B.2: no suffix byte exists to un-escape).
    Emulation {
        /// Stable dotted rule code, e.g. `h264.ep.bad_suffix`.
        code: &'static str,
    },
    /// A syntax value outside its spec-declared range (ue/se fields
    /// carry declared bounds; violations are corruption, not
    /// features).
    OutOfRange {
        /// Field name, e.g. `seq_parameter_set_id`.
        field: &'static str,
        /// The parsed value.
        value: u64,
        /// The inclusive maximum the spec allows.
        max: u64,
    },
    /// Structural violation with a stable rule code (bad NAL type for
    /// the requested parse, missing `rbsp_stop_bit`, bad `avcC`
    /// version, …).
    Invalid {
        /// Stable dotted rule code.
        code: &'static str,
    },
    /// A syntactically parseable feature this parser deliberately
    /// does not model (FMO slice groups, `avcC` version != 1).
    Unsupported {
        /// Stable dotted rule code.
        code: &'static str,
    },
}

impl fmt::Display for H264Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            H264Error::Truncated { needed, have } => {
                write!(f, "truncated: needed {needed}, have {have}")
            }
            H264Error::ForbiddenZeroBit => f.write_str("forbidden_zero_bit set"),
            H264Error::Emulation { code } => write!(f, "emulation prevention: {code}"),
            H264Error::OutOfRange { field, value, max } => {
                write!(f, "{field} {value} out of range (max {max})")
            }
            H264Error::Invalid { code } => write!(f, "invalid: {code}"),
            H264Error::Unsupported { code } => write!(f, "unsupported: {code}"),
        }
    }
}

impl std::error::Error for H264Error {}

/// NAL unit type classification (Table 7-1). Named variants cover the
/// types real streams carry; reserved/unspecified ids are preserved
/// verbatim (never guessed into a semantic bucket).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NalUnitType {
    /// 1 — coded slice of a non-IDR picture.
    Slice,
    /// 2/3/4 — data partitions A/B/C (legacy).
    SliceDataPartitionA,
    SliceDataPartitionB,
    SliceDataPartitionC,
    /// 5 — coded slice of an IDR picture.
    SliceIdr,
    /// 6 — supplemental enhancement information.
    Sei,
    /// 7 — sequence parameter set.
    Sps,
    /// 8 — picture parameter set.
    Pps,
    /// 9 — access unit delimiter.
    Aud,
    /// 10 — end of sequence.
    EndOfSequence,
    /// 11 — end of stream.
    EndOfStream,
    /// 12 — filler data.
    Filler,
    /// 13 — sequence parameter set extension.
    SpsExtension,
    /// 14 — prefix NAL unit (MVC).
    Prefix,
    /// 15 — subset sequence parameter set.
    SubsetSps,
    /// 16..=23 — reserved by the spec (16 was the old DPS).
    Reserved(u8),
    /// 0 and 24..=31 — unspecified.
    Unspecified(u8),
}

impl NalUnitType {
    /// Classify a raw 5-bit `nal_unit_type`.
    #[must_use]
    pub fn from_id(id: u8) -> NalUnitType {
        match id {
            1 => NalUnitType::Slice,
            2 => NalUnitType::SliceDataPartitionA,
            3 => NalUnitType::SliceDataPartitionB,
            4 => NalUnitType::SliceDataPartitionC,
            5 => NalUnitType::SliceIdr,
            6 => NalUnitType::Sei,
            7 => NalUnitType::Sps,
            8 => NalUnitType::Pps,
            9 => NalUnitType::Aud,
            10 => NalUnitType::EndOfSequence,
            11 => NalUnitType::EndOfStream,
            12 => NalUnitType::Filler,
            13 => NalUnitType::SpsExtension,
            14 => NalUnitType::Prefix,
            15 => NalUnitType::SubsetSps,
            16..=23 => NalUnitType::Reserved(id),
            _ => NalUnitType::Unspecified(id),
        }
    }

    /// The raw 5-bit id (inverse of [`NalUnitType::from_id`]).
    #[must_use]
    pub fn id(self) -> u8 {
        match self {
            NalUnitType::Slice => 1,
            NalUnitType::SliceDataPartitionA => 2,
            NalUnitType::SliceDataPartitionB => 3,
            NalUnitType::SliceDataPartitionC => 4,
            NalUnitType::SliceIdr => 5,
            NalUnitType::Sei => 6,
            NalUnitType::Sps => 7,
            NalUnitType::Pps => 8,
            NalUnitType::Aud => 9,
            NalUnitType::EndOfSequence => 10,
            NalUnitType::EndOfStream => 11,
            NalUnitType::Filler => 12,
            NalUnitType::SpsExtension => 13,
            NalUnitType::Prefix => 14,
            NalUnitType::SubsetSps => 15,
            NalUnitType::Reserved(id) | NalUnitType::Unspecified(id) => id,
        }
    }
}

/// The parsed one-byte NAL header (§7.3.1) of a NAL unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NalHeader {
    /// `nal_ref_idc` (0..=3): reference-class importance.
    pub nal_ref_idc: u8,
    /// The classified `nal_unit_type`.
    pub unit_type: NalUnitType,
}

impl NalHeader {
    /// Parse the header byte of a complete NAL unit and return it
    /// together with the remaining payload (everything after the
    /// header byte; still emulation-escaped).
    ///
    /// # Errors
    /// [`H264Error::Truncated`] for an empty slice,
    /// [`H264Error::ForbiddenZeroBit`] when bit 7 is set.
    pub fn split(nal: &[u8]) -> Result<(NalHeader, &[u8]), H264Error> {
        let (&first, payload) = nal
            .split_first()
            .ok_or(H264Error::Truncated { needed: 1, have: 0 })?;
        if first & 0x80 != 0 {
            return Err(H264Error::ForbiddenZeroBit);
        }
        Ok((
            NalHeader {
                nal_ref_idc: (first >> 5) & 0b11,
                unit_type: NalUnitType::from_id(first & 0x1f),
            },
            payload,
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
/// (header byte included) of an H.264 byte stream.
///
/// Leading garbage before the first start code is skipped (Annex-B
/// streams may be prefixed with `leading_zero_8bits`); NAL bodies
/// truncated by end-of-data are yielded as-is (their parsers surface
/// the typed truncation). Zero-length bodies (two adjacent start
/// codes) are yielded and fail header parsing downstream — typed, not
/// silently dropped.
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

/// Remove emulation-prevention bytes (§7.4.1 / Annex B): each
/// `00 00 03 {00,01,02,03}` collapses to `00 00 {suffix}`.
///
/// Writes into `out` (cleared first) so callers can reuse one buffer
/// across a whole stream — the steady-state path allocates nothing.
///
/// # Errors
/// - `Emulation { code: "h264.ep.bad_suffix" }` — `00 00 03` followed
///   by a byte > `0x03` (no legal RBSP produces this);
/// - `Emulation { code: "h264.ep.dangling" }` — `00 00 03` at the
///   very end of the NAL (the escaped suffix byte is missing:
///   truncation).
pub fn unescape_rbsp_into(nal: &[u8], out: &mut Vec<u8>) -> Result<(), H264Error> {
    out.clear();
    out.reserve(nal.len());
    let mut i = 0usize;
    while i < nal.len() {
        if i + 2 < nal.len() && nal[i] == 0 && nal[i + 1] == 0 && nal[i + 2] == 3 {
            match nal.get(i + 3) {
                // `00 00 03` as the final bytes: the escaped suffix
                // byte is missing — truncation.
                None => {
                    return Err(H264Error::Emulation {
                        code: "h264.ep.dangling",
                    });
                }
                Some(&suffix) if suffix <= 0x03 => {
                    out.extend_from_slice(&nal[i..i + 2]);
                    out.push(suffix);
                    i += 4;
                }
                Some(_) => {
                    return Err(H264Error::Emulation {
                        code: "h264.ep.bad_suffix",
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
pub fn unescape_rbsp(nal: &[u8]) -> Result<Vec<u8>, H264Error> {
    let mut out = Vec::with_capacity(nal.len());
    unescape_rbsp_into(nal, &mut out)?;
    Ok(out)
}

/// AVC length-prefixed NAL splitter (MP4 sample data, Matroska
/// `V_MPEG4/ISO/AVC` blocks): each NAL is `length_size` big-endian
/// length bytes + body.
///
/// Trailing padding (fewer than `length_size` bytes left, all zero)
/// ends the iteration, as MP4 samples are zero-padded to sample
/// boundaries; anything else that runs past the data is the typed
/// [`H264Error::Truncated`].
#[derive(Debug, Clone)]
pub struct AvcNals<'a> {
    data: &'a [u8],
    pos: usize,
    length_size: usize,
}

impl<'a> AvcNals<'a> {
    /// Split `data` using `length_size` (1..=4) length prefixes.
    ///
    /// # Errors
    /// [`H264Error::Invalid { code: "h264.avc.length_size" }`] when
    /// `length_size` is outside 1..=4.
    pub fn new(data: &'a [u8], length_size: u8) -> Result<AvcNals<'a>, H264Error> {
        if !(1..=4).contains(&length_size) {
            return Err(H264Error::Invalid {
                code: "h264.avc.length_size",
            });
        }
        Ok(AvcNals {
            data,
            pos: 0,
            length_size: length_size as usize,
        })
    }
}

impl<'a> Iterator for AvcNals<'a> {
    type Item = Result<&'a [u8], H264Error>;

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
                Some(Err(H264Error::Truncated {
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
            return Some(Err(H264Error::Invalid {
                code: "h264.avc.zero_length",
            }));
        }
        let total = len + self.length_size;
        if total > rest.len() {
            self.pos = self.data.len();
            return Some(Err(H264Error::Truncated {
                needed: total as u64,
                have: rest.len() as u64,
            }));
        }
        self.pos += total;
        Some(Ok(&rest[self.length_size..self.length_size + len]))
    }
}

/// The `avcC` AVCDecoderConfigurationRecord (ISO/IEC 14496-15 §5.3.3.1):
/// the parameter-set store MP4/MKV containers carry, plus the NAL
/// length geometry every length-prefixed packet uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvcConfig {
    /// `AVCProfileIndication` (mirrors SPS `profile_idc`).
    pub profile_idc: u8,
    /// `profile_compatibility` (the six constraint flags + reserved).
    pub constraint_flags: u8,
    /// `AVCLevelIndication` (mirrors SPS `level_idc`).
    pub level_idc: u8,
    /// `lengthSizeMinusOne + 1` (1..=4): the NAL length prefix size.
    pub length_size: u8,
    /// The sequence parameter set NAL units (complete, header byte
    /// included), in record order.
    pub sps: Vec<Vec<u8>>,
    /// The picture parameter set NAL units, in record order.
    pub pps: Vec<Vec<u8>>,
}

impl AvcConfig {
    /// Parse a raw `avcC` record (e.g. an MP4 `avcC` box body or a
    /// Matroska `V_MPEG4/ISO/AVC` `CodecPrivate`).
    ///
    /// # Real-world deviation, accepted deliberately
    ///
    /// ISO/IEC 14496-15 §5.3.3.1 declares both parameter-set count
    /// bytes `reserved(3) + count(5)` with reserved = `111`, and
    /// ffmpeg's `ff_isom_write_avcc` — the author of our entire
    /// corpus and the dominant muxer in the wild — writes the **SPS**
    /// count byte that way (`0xE0 | n`) but the **PPS** count byte as
    /// the bare count (`0x01` for one PPS; verified on every corpus
    /// `avcC`). The counts are therefore read as masked 5-bit fields
    /// and the reserved bits are ignored; a *zero* count remains the
    /// typed `Invalid` it always was. Trailing bytes after the last
    /// parameter set (the spec's extension fields, e.g.
    /// `fe f8 f8 00`) are none of this parser's business — the SPS
    /// carries the same facts — and are left unread.
    ///
    /// # Errors
    /// Typed [`H264Error`]s: `Truncated` (short record/NAL),
    /// `Invalid { "h264.avcc.version" }` for a version other than 1,
    /// `Invalid { "h264.avcc.count" }` for a zero parameter-set
    /// count, `Invalid { "h264.avcc.reserved" }` for set reserved
    /// bits in the `lengthSizeMinusOne` byte.
    pub fn parse(bytes: &[u8]) -> Result<AvcConfig, H264Error> {
        if bytes.len() < 7 {
            return Err(H264Error::Truncated {
                needed: 7,
                have: bytes.len() as u64,
            });
        }
        if bytes[0] != 1 {
            return Err(H264Error::Invalid {
                code: "h264.avcc.version",
            });
        }
        if bytes[4] & 0xfc != 0xfc {
            return Err(H264Error::Invalid {
                code: "h264.avcc.reserved",
            });
        }
        let length_size = (bytes[4] & 0x03) + 1;
        let num_sps = bytes[5] & 0x1f;
        if num_sps == 0 {
            return Err(H264Error::Invalid {
                code: "h264.avcc.count",
            });
        }
        let mut pos = 6usize;
        let mut sps = Vec::with_capacity(num_sps as usize);
        for _ in 0..num_sps {
            let nal = read_counted_nal(bytes, &mut pos, 2)?;
            sps.push(nal.to_vec());
        }
        let Some(&count) = bytes.get(pos) else {
            return Err(H264Error::Truncated {
                needed: (pos + 1) as u64,
                have: bytes.len() as u64,
            });
        };
        pos += 1;
        let num_pps = count & 0x1f;
        if num_pps == 0 {
            return Err(H264Error::Invalid {
                code: "h264.avcc.count",
            });
        }
        let mut pps = Vec::with_capacity(num_pps as usize);
        for _ in 0..num_pps {
            let nal = read_counted_nal(bytes, &mut pos, 2)?;
            pps.push(nal.to_vec());
        }
        Ok(AvcConfig {
            profile_idc: bytes[1],
            constraint_flags: bytes[2],
            level_idc: bytes[3],
            length_size,
            sps,
            pps,
        })
    }

    /// Heuristic: do these bytes look like an `avcC` record (vs. e.g.
    /// an Annex-B `CodecPrivate`)? Used by callers whose containers
    /// allow both forms. Matches ffmpeg's real-world writer output
    /// (either count-byte form — see [`AvcConfig::parse`]).
    #[must_use]
    pub fn looks_like_avcc(bytes: &[u8]) -> bool {
        bytes.len() >= 7 && bytes[0] == 1 && bytes[4] & 0xfc == 0xfc && bytes[5] & 0x1f > 0
    }
}

/// One `u16`-length-prefixed NAL inside an `avcC` record.
fn read_counted_nal<'a>(
    bytes: &'a [u8],
    pos: &mut usize,
    len_bytes: usize,
) -> Result<&'a [u8], H264Error> {
    if bytes.len() < *pos + len_bytes {
        return Err(H264Error::Truncated {
            needed: (*pos + len_bytes) as u64,
            have: bytes.len() as u64,
        });
    }
    let mut len = 0usize;
    for &b in &bytes[*pos..*pos + len_bytes] {
        len = (len << 8) | b as usize;
    }
    *pos += len_bytes;
    if len == 0 || bytes.len() < *pos + len {
        return Err(H264Error::Truncated {
            needed: (*pos + len.max(1)) as u64,
            have: bytes.len() as u64,
        });
    }
    let nal = &bytes[*pos..*pos + len];
    *pos += len;
    Ok(nal)
}

/// MSB-first bit reader over an RBSP (emulation prevention already
/// removed) with Exp-Golomb support (§9.1/§9.2) — the shared
/// primitive of every `h264::parser` syntax reader (SPS/PPS here;
/// slice headers/SEI in sibling card 5.F1b).
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
    /// [`H264Error::Truncated`] past the end of the RBSP.
    pub fn bit(&mut self) -> Result<u32, H264Error> {
        if self.bit_pos as u64 >= self.total_bits() {
            return Err(H264Error::Truncated {
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
    /// [`H264Error::Truncated`] when fewer than `n` bits remain.
    pub fn bits(&mut self, n: u32) -> Result<u32, H264Error> {
        if n == 0 || n > 32 {
            return Err(H264Error::Invalid {
                code: "h264.bits.width",
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
    /// [`H264Error::Truncated`] when the code runs past the RBSP;
    /// `Invalid { "h264.ue.overflow" }` for a prefix longer than 31
    /// zero bits (cannot encode a u64-representable field).
    pub fn ue(&mut self) -> Result<u64, H264Error> {
        let mut leading_zeros = 0u32;
        while self.bit()? == 0 {
            leading_zeros += 1;
            if leading_zeros > 31 {
                return Err(H264Error::Invalid {
                    code: "h264.ue.overflow",
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
    pub fn se(&mut self) -> Result<i64, H264Error> {
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
    /// "h264.rbsp.tail" }` for nonzero bits after the stop bit.
    pub fn finish(&mut self) -> Result<(), H264Error> {
        // Scan for the stop bit (the next 1 bit, if any).
        let stop = loop {
            if self.bit_pos as u64 >= self.total_bits() {
                return Err(H264Error::Truncated {
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
                return Err(H264Error::Invalid {
                    code: "h264.rbsp.tail",
                });
            }
        }
        if self.data[byte + 1..].iter().any(|&b| b != 0) {
            return Err(H264Error::Invalid {
                code: "h264.rbsp.tail",
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nal_classification_covers_table_7_1() {
        assert_eq!(NalUnitType::from_id(1), NalUnitType::Slice);
        assert_eq!(NalUnitType::from_id(5), NalUnitType::SliceIdr);
        assert_eq!(NalUnitType::from_id(6), NalUnitType::Sei);
        assert_eq!(NalUnitType::from_id(7), NalUnitType::Sps);
        assert_eq!(NalUnitType::from_id(8), NalUnitType::Pps);
        assert_eq!(NalUnitType::from_id(15), NalUnitType::SubsetSps);
        assert_eq!(NalUnitType::from_id(20), NalUnitType::Reserved(20));
        assert_eq!(NalUnitType::from_id(0), NalUnitType::Unspecified(0));
        assert_eq!(NalUnitType::from_id(31), NalUnitType::Unspecified(31));
        for id in 0u8..=31 {
            assert_eq!(NalUnitType::from_id(id).id(), id);
        }
    }

    #[test]
    fn header_split_checks_forbidden_bit_and_truncation() {
        let (hdr, payload) =
            NalHeader::split(&[0x67, 0xff]).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(hdr.unit_type, NalUnitType::Sps);
        assert_eq!(hdr.nal_ref_idc, 3);
        assert_eq!(payload, &[0xff]);
        assert_eq!(
            NalHeader::split(&[]),
            Err(H264Error::Truncated { needed: 1, have: 0 })
        );
        assert_eq!(NalHeader::split(&[0xf0]), Err(H264Error::ForbiddenZeroBit));
    }

    #[test]
    fn annexb_splits_three_and_four_byte_prefixes() {
        // 4-byte prefix, 3-byte prefix, trailing zeros, garbage head.
        let stream = [
            0xff, 0xff, // leading garbage before first start code
            0x00, 0x00, 0x00, 0x01, 0x67, 0xaa, //
            0x00, 0x00, 0x01, 0x68, 0xbb, 0x00, 0x00, // trailing zeros
        ];
        let nals: Vec<&[u8]> = AnnexBNals::new(&stream).collect();
        assert_eq!(nals, vec![&[0x67u8, 0xaa][..], &[0x68u8, 0xbb][..]]);
        assert!(AnnexBNals::new(&[0x00, 0x00, 0x02, 0x99]).next().is_none());
    }

    #[test]
    fn emulation_removal_and_typed_violations() {
        assert_eq!(
            unescape_rbsp(&[0x00, 0x00, 0x03, 0x02, 0xaa]).unwrap_or_default(),
            vec![0x00, 0x00, 0x02, 0xaa]
        );
        assert_eq!(
            unescape_rbsp(&[0x00, 0x00, 0x03, 0x99]),
            Err(H264Error::Emulation {
                code: "h264.ep.bad_suffix"
            })
        );
        assert_eq!(
            unescape_rbsp(&[0xaa, 0x00, 0x00, 0x03]),
            Err(H264Error::Emulation {
                code: "h264.ep.dangling"
            })
        );
        // Buffer reuse path: clears, no growth churn.
        let mut buf = vec![0xffu8; 128];
        unescape_rbsp_into(&[0x00, 0x00, 0x03, 0x01], &mut buf)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(buf, vec![0x00, 0x00, 0x01]);
    }

    #[test]
    fn avc_length_prefixed_split_with_typed_truncation() {
        let data = [
            0x00, 0x00, 0x00, 0x02, 0x67, 0xaa, 0x00, 0x00, 0x00, 0x01, 0x68,
        ];
        let nals: Vec<_> = AvcNals::new(&data, 4)
            .unwrap_or_else(|e| unreachable!("{e}"))
            .map(|n| n.unwrap_or_else(|e| unreachable!("{e}")))
            .collect();
        assert_eq!(nals, vec![&[0x67u8, 0xaa][..], &[0x68u8][..]]);
        // Length runs past the data: typed truncation.
        let bad = [0x00, 0x00, 0x00, 0x09, 0x67];
        let err = AvcNals::new(&bad, 4)
            .unwrap_or_else(|e| unreachable!("{e}"))
            .find_map(|n| n.err());
        assert_eq!(
            err,
            Some(H264Error::Truncated {
                needed: 13,
                have: 5
            })
        );
        // Zero padding tail is tolerated; nonzero junk is not.
        let padded = [0x00, 0x00, 0x00, 0x01, 0x68, 0x00, 0x00];
        let n: Vec<_> = AvcNals::new(&padded, 4)
            .unwrap_or_else(|e| unreachable!("{e}"))
            .map(|n| n.unwrap_or_else(|e| unreachable!("{e}")))
            .collect();
        assert_eq!(n, vec![&[0x68u8][..]]);
        let junk = [0x00, 0x00, 0x00, 0x01, 0x68, 0x77];
        assert!(
            AvcNals::new(&junk, 4)
                .unwrap_or_else(|e| unreachable!("{e}"))
                .any(|n| n.is_err())
        );
        assert!(matches!(
            AvcNals::new(&[], 5),
            Err(H264Error::Invalid {
                code: "h264.avc.length_size"
            })
        ));
    }

    #[test]
    fn avcc_parse_roundtrip_and_error_paths() {
        // Hand-built minimal avcC: one SPS (2 bytes), one PPS (1).
        let avcc = [
            0x01, // configurationVersion
            0x64, 0x00, 0x0d, // profile, compat, level
            0xff, // reserved + lengthSizeMinusOne = 3
            0xe1, // reserved + 1 SPS
            0x00, 0x02, 0x67, 0xaa, // SPS
            0xe1, // reserved(111) + 1 PPS
            0x00, 0x01, 0x68, // PPS
        ];
        let cfg = AvcConfig::parse(&avcc).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(cfg.profile_idc, 100);
        assert_eq!(cfg.level_idc, 13);
        assert_eq!(cfg.length_size, 4);
        assert_eq!(cfg.sps, vec![vec![0x67, 0xaa]]);
        assert_eq!(cfg.pps, vec![vec![0x68]]);
        assert!(AvcConfig::looks_like_avcc(&avcc));
        // Error paths.
        assert_eq!(
            AvcConfig::parse(&avcc[..6]),
            Err(H264Error::Truncated { needed: 7, have: 6 })
        );
        let mut v0 = avcc;
        v0[0] = 0;
        assert_eq!(
            AvcConfig::parse(&v0),
            Err(H264Error::Invalid {
                code: "h264.avcc.version"
            })
        );
        let mut bad_count = avcc;
        bad_count[5] = 0x00; // zero SPS
        assert_eq!(
            AvcConfig::parse(&bad_count),
            Err(H264Error::Invalid {
                code: "h264.avcc.count"
            })
        );
        let mut cut = avcc.to_vec();
        cut.truncate(9); // SPS length runs past the end
        assert!(matches!(
            AvcConfig::parse(&cut),
            Err(H264Error::Truncated { .. })
        ));
        // Zero PPS count remains a typed invalid.
        let mut no_pps = avcc;
        no_pps[10] = 0x00;
        assert_eq!(
            AvcConfig::parse(&no_pps),
            Err(H264Error::Invalid {
                code: "h264.avcc.count"
            })
        );
    }

    #[test]
    fn avcc_accepts_ffmpeg_writer_reality() {
        // Byte-for-byte the 0003-multi-av.mkv `CodecPrivate` shape:
        // SPS count 0xE1 (reserved set) but PPS count **0x01** —
        // ffmpeg's `ff_isom_write_avcc` writes the bare count — plus
        // four trailing extension bytes the record spec allows and we
        // deliberately leave unread.
        let ffmpeg_avcc = [
            0x01, 0x7a, 0x00, 0x16, // version, profile 122, compat, level 22
            0xff, // reserved + lengthSizeMinusOne = 3
            0xe1, // reserved(111) + 1 SPS
            0x00, 0x19, // SPS length 25
            0x67, 0x7a, 0x00, 0x16, 0xbc, 0xec, 0x0b, 0x43, 0xdb, 0x01, 0x10, 0x00, 0x00, 0x03,
            0x00, 0x10, 0x00, 0x00, 0x03, 0x00, 0xa0, 0xf1, 0x62, 0xd3, 0x80, // SPS NAL
            0x01, // PPS count: bare 1, reserved bits clear (ffmpeg)
            0x00, 0x04, // PPS length 4
            0x68, 0xce, 0x3c, 0x80, // PPS NAL
            0xfe, 0xf8, 0xf8, 0x00, // trailing extension bytes: ignored
        ];
        assert!(AvcConfig::looks_like_avcc(&ffmpeg_avcc));
        let cfg = AvcConfig::parse(&ffmpeg_avcc).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(cfg.profile_idc, 122);
        assert_eq!(cfg.level_idc, 22);
        assert_eq!(cfg.length_size, 4);
        assert_eq!(cfg.sps.len(), 1);
        assert_eq!(cfg.sps[0].len(), 25);
        assert_eq!(cfg.pps, vec![vec![0x68, 0xce, 0x3c, 0x80]]);
        // The SPS itself parses through the normal path (corpus test
        // asserts its fields against ffprobe; here just the round trip).
        assert!(super::super::sps::SeqParameterSet::parse(&cfg.sps[0]).is_ok());
    }

    #[test]
    fn bitreader_exp_golomb_and_trailing_rules() {
        // ue vectors (§9.1): "1"→0, "010"→1, "011"→2. Byte
        // 0b1010_0110 packs exactly 1|010|011|0 — the fourth read
        // runs off the end (typed truncation).
        let mut r = BitReader::new(&[0b1010_0110]);
        assert_eq!(r.ue(), Ok(0));
        assert_eq!(r.ue(), Ok(1));
        assert_eq!(r.ue(), Ok(2));
        assert_eq!(r.ue(), Err(H264Error::Truncated { needed: 9, have: 8 }));
        // se mapping (§9.2): codeNum 1 → +1, codeNum 2 → −1; the
        // byte 0b0100_1100 packs "010"|"011".
        let mut s = BitReader::new(&[0b0100_1100]);
        assert_eq!(s.se(), Ok(1));
        assert_eq!(s.se(), Ok(-1));
        // Trailing bits: a lone stop bit is valid…
        let mut t = BitReader::new(&[0x80]);
        assert!(t.finish().is_ok());
        // …data, then stop, then zeros is valid (ue "010" + stop)…
        let mut t2 = BitReader::new(&[0b0100_0010]);
        assert_eq!(t2.ue(), Ok(1));
        assert!(t2.finish().is_ok());
        // …nonzero bits after the stop are refused…
        let mut t3 = BitReader::new(&[0b0100_0110]);
        assert_eq!(t3.ue(), Ok(1));
        assert_eq!(
            t3.finish(),
            Err(H264Error::Invalid {
                code: "h264.rbsp.tail"
            })
        );
        // …and an RBSP with no stop bit at all is truncation.
        let mut t4 = BitReader::new(&[0b0100_0000]);
        assert_eq!(t4.ue(), Ok(1));
        assert!(matches!(t4.finish(), Err(H264Error::Truncated { .. })));
        // bits(n) reads MSB-first.
        let mut b = BitReader::new(&[0b1011_0010, 0b0101_0000]);
        assert_eq!(b.bits(8), Ok(0b1011_0010));
        assert_eq!(b.bits(4), Ok(0b0101));
        assert_eq!(b.bits(2), Ok(0b00));
    }

    #[test]
    fn property_mutations_never_panic_or_hang() {
        // Deterministic xorshift64* PRNG over mutated NAL streams:
        // every mutation must terminate quickly with Ok or a typed
        // error (the +FB fuzz-lite guarantee, in miniature).
        let seed_stream: Vec<u8> = [
            0x00, 0x00, 0x00, 0x01, 0x67, 0x64, 0x00, 0x0d, 0x00, 0x00, 0x03, 0x00, 0xaa, 0x80,
            0x00, 0x00, 0x00, 0x01, 0x68, 0xeb, 0xec, 0xb2, 0x2c,
        ]
        .to_vec();
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut rbsp_buf = Vec::new();
        for round in 0..3000u32 {
            let mut data = seed_stream.clone();
            let mutations = (next() % 4) + 1;
            for _ in 0..mutations {
                let at = (next() as usize) % data.len();
                data[at] = (next() % 256) as u8;
            }
            if round % 5 == 0 {
                let cut = (next() as usize) % (data.len() + 1);
                data.truncate(cut);
            }
            // Annex-B + AVCC walks must terminate (they are
            // length-bounded by construction) and never panic.
            let nals: Vec<&[u8]> = AnnexBNals::new(&data).collect();
            assert!(nals.len() <= data.len());
            let mut walker = AvcNals::new(&data, ((next() % 4) + 1) as u8);
            if let Ok(ref mut it) = walker {
                let mut guard = 0usize;
                for item in it {
                    // Ok slice or typed Err — both acceptable; only
                    // the walk's termination is being asserted here.
                    let _ = item;
                    guard += 1;
                    assert!(guard <= data.len() + 1, "avc walker spun");
                }
            }
            // Unescape + a raw SPS-style header read: typed or Ok.
            for nal in nals {
                if let Ok((hdr, _)) = NalHeader::split(nal)
                    && matches!(hdr.unit_type, NalUnitType::Sps | NalUnitType::Pps)
                    && unescape_rbsp_into(nal, &mut rbsp_buf).is_ok()
                {
                    let mut br = BitReader::new(&rbsp_buf[1.min(rbsp_buf.len())..]);
                    let _ = br.ue();
                    let _ = br.finish();
                }
            }
        }
    }
}
