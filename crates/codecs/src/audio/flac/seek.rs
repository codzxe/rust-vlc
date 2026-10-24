//! FLAC seeking (card 5.A1): SEEKTABLE seek-point resolution with a
//! sample-accurate binary-search fallback over frame headers.
//!
//! # What "seek" means at this layer
//!
//! The decoder card owns the *resolution* half of seeking: given the
//! byte stream (native FLAC), the STREAMINFO facts, and an optional
//! parsed [`SeekIndex`], [`SeekPlanner::resolve`] turns a target
//! inter-channel sample number into a [`SeekPlan`] — the byte offset
//! of the frame that **contains** the target, plus that frame's first
//! sample number. The consumer (the future native-FLAC demuxer card)
//! hands the bytes from that offset to the decoder and drops the
//! leading `target - plan.first_sample` samples — sample-accuracy at
//! the consumer, byte-exactness here.
//!
//! # The two resolvers
//!
//! 1. **SEEKTABLE** (spec §8): points are `(sample, offset)` pairs
//!    relative to the first frame. Binary search the points for the
//!    last one at-or-before the target, then *verify* the candidate by
//!    parsing the frame header at the resolved offset — the table is
//!    advisory data and a corrupt/unsorted/mismatched row must degrade
//!    to the fallback, never produce a wrong seek.
//! 2. **Binary-search fallback** (no table, table miss, or
//!    target before the first point): bisect the byte range, at each
//!    probe resync to the next frame header (bounded sync scan) and
//!    parse it — every FLAC frame header is self-describing (sync
//!    code, blocking strategy, block size, UTF-8-coded frame/sample
//!    number) — so the probe learns `first_sample` and the block
//!    duration and can decide `contained / after / before` without
//!    decoding subframes. Fixed and variable blocking are both handled
//!    (containment is judged by the header's own block size).
//!
//! # Header truth (verified against ffmpeg 7.1.5 + claxon 0.4.3)
//!
//! The header's variable-length fields are ordered **number first,
//! then the block-size and sample-rate escape bytes, then CRC-8**
//! (spec §6, claxon `frame.rs` `read_frame_header`): the prior draft
//! of this parser read the escapes before the number — every
//! escape-coded header (the short final block every encoder emits)
//! was misparsed. The fix is pinned by the fixture suite: frame
//! counts and per-frame first samples match ffprobe's own packet
//! pts exactly, including the tail frame. Two further spec rules the
//! resolvers rely on:
//!
//! - the header CRC-8 (poly 0x07, init 0) is **verified**: a scan
//!   candidate that passes structure but fails CRC is not a header
//!   (kills ~255/256 of entropy-data false syncs — measured on the
//!   5 s fixture: one structural false positive, rejected by CRC);
//! - in a **fixed-blocksize** stream (STREAMINFO min == max) the
//!   first sample is `frame number × the stream's block size`, *not*
//!   × the frame's own (possibly shorter, tail) size — ffmpeg's
//!   packet pts and libFLAC agree on this; claxon's `Block::time()`
//!   uses the frame's own size and misplaces the tail, so the
//!   decoder overrides it ([`ParsedFrameHeader::first_sample`]).
//!
//! # Bounded work (MP §12)
//!
//! Each probe parses at most a bounded header window plus a sync scan
//! bounded by the stream's `max_frame_size` (or the [`SCAN_WINDOW`]
//! cap when unknown); the bisection is `O(log file)` probes. No
//! unbounded scans, no panics on any input.

use super::error::{FlacError, corrupt, unsupported};
use super::metadata::{SeekIndex, StreamInfo};
use media_types::EngineError;
use media_types::error::CorruptDetail;

/// Sync-scan window when the stream declares no `max_frame_size`
/// (`[PROPOSAL]`: 64 KiB covers every legal block size × channel
/// count × worst-case subframe ratio observed in practice; a frame
/// larger than this with no declared bound is a pathological
/// encoder).
pub(crate) const SCAN_WINDOW: u64 = 64 * 1024;

/// One parsed frame header — the navigation atom of the fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedFrameHeader {
    /// The raw UTF-8-coded number: the sample number in variable
    /// blocking, the frame ordinal in fixed blocking.
    pub number: u64,
    /// Blocking strategy bit: `true` = variable (sample numbers).
    pub variable: bool,
    /// The frame's block size in inter-channel samples.
    pub block_size: u32,
    /// Total header length in bytes (for consumers that advance past
    /// the header).
    pub header_len: u32,
}

impl ParsedFrameHeader {
    /// The first inter-channel sample of the frame this header
    /// describes, given the stream's facts (spec §6):
    ///
    /// - variable blocking → the header's own sample number;
    /// - fixed blocking → `number × the stream's block size` when
    ///   STREAMINFO declares min == max (the rule ffmpeg and libFLAC
    ///   apply — a shorter *tail* frame does not rescale the ordinal;
    ///   pinned by the fixture suite against ffprobe packet pts),
    ///   else `number × this frame's own block size`.
    #[must_use]
    pub fn first_sample(&self, facts: &StreamInfo) -> u64 {
        if self.variable {
            self.number
        } else if facts.is_fixed_blocksize() {
            self.number.saturating_mul(u64::from(facts.max_block_size))
        } else {
            self.number.saturating_mul(u64::from(self.block_size))
        }
    }

    /// The byte offset one past this frame's potential end bound for
    /// containment tests: first sample + block size.
    #[must_use]
    pub fn end_sample(&self, facts: &StreamInfo) -> u64 {
        self.first_sample(facts)
            .saturating_add(u64::from(self.block_size))
    }
}

/// The resolution result: decode from `byte_offset`, drop
/// `target - first_sample` leading samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeekPlan {
    /// Absolute byte offset of the containing frame's header.
    pub byte_offset: u64,
    /// First inter-channel sample of that frame (≤ target).
    pub first_sample: u64,
}

/// The seek resolver over one in-memory FLAC byte stream.
#[derive(Debug, Clone, Copy)]
pub struct SeekPlanner<'a> {
    data: &'a [u8],
    facts: &'a StreamInfo,
    index: Option<&'a SeekIndex>,
    frames_start: u64,
}

impl<'a> SeekPlanner<'a> {
    /// Assemble a planner. `frames_start` is the metadata walk's
    /// anchor ([`super::metadata::StreamLayout::frames_start`]).
    #[must_use]
    pub fn new(
        data: &'a [u8],
        facts: &'a StreamInfo,
        index: Option<&'a SeekIndex>,
        frames_start: u64,
    ) -> SeekPlanner<'a> {
        SeekPlanner {
            data,
            facts,
            index,
            frames_start,
        }
    }

    /// Resolve `target_sample` to the containing frame.
    ///
    /// # Errors
    /// [`ErrorKind::Corrupt`](media_types::ErrorKind::Corrupt)
    /// (`flac.seek.past_end`) when the target lies beyond the stream's
    /// declared (or observed) sample range; the same kind
    /// (`flac.seek.no_frame`) when bisection cannot find any frame
    /// header — the byte stream is not decodable FLAC frame data.
    pub fn resolve(&self, target_sample: u64) -> Result<SeekPlan, EngineError> {
        self.resolve_inner(target_sample)
            .map_err(FlacError::into_engine_error)
    }

    /// The taxonomy-preserving form of [`Self::resolve`] (module
    /// internal).
    pub(crate) fn resolve_inner(&self, target_sample: u64) -> Result<SeekPlan, FlacError> {
        if let Some(total) = self.facts.total_samples
            && target_sample >= total
        {
            return Err(corrupt(
                "flac.seek.past_end",
                CorruptDetail::BadIndex,
                format!("seek target {target_sample} is at or past the stream's {total} samples"),
            ));
        }
        // 1. SEEKTABLE candidate (advisory — verified before use).
        if let Some(plan) = self.table_candidate(target_sample) {
            return Ok(plan);
        }
        // 2. Bisect the frame stream. The known-EOF bracket: frames
        // live in [frames_start, data.len()).
        self.bisect(self.frames_start, self.data.len() as u64, target_sample)
    }

    /// The SEEKTABLE path: last point ≤ target, absolute offset,
    /// header-verified. `None` = no usable candidate; the caller falls
    /// back to bisection. A *present but wrong* candidate also returns
    /// `None` (fail-safe into the fallback — the table is advisory).
    fn table_candidate(&self, target: u64) -> Option<SeekPlan> {
        let index = self.index?;
        let points = &index.points;
        if points.is_empty() || points[0].sample > target {
            return None;
        }
        // Ascending per spec; partition_point is the last point ≤ target.
        let position = points.partition_point(|p| p.sample <= target);
        let point = points.get(position.checked_sub(1)?)?;
        let absolute = self.frames_start.checked_add(point.offset)?;
        let header = self.header_at(absolute)?;
        if header.first_sample(self.facts) == point.sample
            && header.first_sample(self.facts) <= target
        {
            // Contained iff the frame's own block size covers the
            // target; otherwise the containing frame is *after* this
            // one (possible in variable-blocksize streams) and the
            // bisection refines from here.
            if header.end_sample(self.facts) > target {
                return Some(SeekPlan {
                    byte_offset: absolute,
                    first_sample: header.first_sample(self.facts),
                });
            }
        }
        None
    }

    /// The binary-search fallback: find the **last** frame whose
    /// first sample is ≤ `target` by bisecting byte offsets in
    /// `[lo, hi)` (monotone first-sample assumption — verified per
    /// candidate before any answer is returned).
    ///
    /// Termination (the resume agent's fix for the livelock the first
    /// draft had): every branch strictly shrinks the bracket —
    /// `lo = offset + 1` (the probe's frame is at `offset ≥ mid > lo`),
    /// `hi = offset` (the scan is bounded by `hi`, so `offset < hi`),
    /// or `hi = mid` (`mid < hi` by construction). The first draft's
    /// `hi = offset` let the scan overshoot `hi` and re-find the same
    /// frame forever (state `(lo, hi)` with the first header at
    /// `offset ≥ hi` repeated bit-for-bit; every seek test hung).
    ///
    /// `best` tracks the last frame seen with `first ≤ target`, so a
    /// bracket that closes inside a frame's own bytes still answers
    /// that frame (the containing frame may *start* before every
    /// probe — e.g. target 0 with the first probe landing mid-frame-0:
    /// the scan finds frame 1, whose first sample is past the target,
    /// and the bracket converges on frame 0's start without ever
    /// probing it).
    fn bisect(&self, lo: u64, hi: u64, target: u64) -> Result<SeekPlan, FlacError> {
        let mut lo = lo;
        let mut hi = hi.min(self.data.len() as u64);
        // The best candidate so far: (byte offset, first sample, end).
        let mut best: Option<(u64, u64, u64)> = None;
        while hi > lo {
            let mid = lo + (hi - lo) / 2;
            match self.frame_at_or_after(mid, hi) {
                Some((offset, header)) => {
                    let end = header.end_sample(self.facts);
                    let first = header.first_sample(self.facts);
                    if first <= target {
                        // The answer is at this frame or after it.
                        best = Some((offset, first, end));
                        lo = offset + 1;
                    } else {
                        // First sample already past the target: the
                        // answer, if any, starts before this frame.
                        hi = offset;
                    }
                }
                None => {
                    // No frame header in the scan window from `mid`:
                    // the target frame, if any, starts left of the
                    // probe (frames nearer than the window).
                    hi = mid;
                }
            }
        }
        match best {
            Some((byte_offset, first_sample, end)) if first_sample <= target && end > target => {
                Ok(SeekPlan {
                    byte_offset,
                    first_sample,
                })
            }
            _ => Err(corrupt(
                "flac.seek.no_frame",
                CorruptDetail::BadIndex,
                format!("no frame header brackets sample {target} in bytes {lo}..{hi}"),
            )),
        }
    }

    /// Resync-scan from `start` for the next frame header and parse
    /// it, **never scanning past `bound`** (the bisection's current
    /// `hi`: a probe must only find frames that start inside the
    /// bracket, or `hi = offset` could stop making progress — the
    /// livelock the resume agent fixed). Bounded: at most
    /// [`SCAN_WINDOW`] (or the declared `max_frame_size`, whichever
    /// is smaller) bytes are inspected, and never past the data.
    fn frame_at_or_after(&self, start: u64, bound: u64) -> Option<(u64, ParsedFrameHeader)> {
        let data_len = self.data.len() as u64;
        let start = start.min(data_len);
        let limit = u64::from(self.facts.max_frame_size.unwrap_or(u32::MAX))
            .min(SCAN_WINDOW)
            .min(data_len - start);
        let mut offset = start;
        let end = start.checked_add(limit)?.min(bound);
        while offset < end {
            if let Some(header) = self.header_at(offset) {
                return Some((offset, header));
            }
            offset += 1;
        }
        None
    }

    /// Parse the frame header at exactly `offset` in this planner's
    /// data (the facts-aware wrapper over [`Self::parse_header_at`]).
    fn header_at(&self, offset: u64) -> Option<ParsedFrameHeader> {
        Self::parse_header_at(self.data, offset)
    }

    /// Parse a frame header at exactly `offset` (spec §6).
    ///
    /// The full structural rule set is enforced (sync code, reserved
    /// bits and codes, UTF-8 number coding) **and the header CRC-8 is
    /// verified** (poly 0x07, init 0, over every header byte before
    /// the CRC itself) — a scan candidate that fails CRC is not a
    /// frame header. The frame CRC-16 is left to the decode path; the
    /// subframe bytes are never touched here.
    pub(crate) fn parse_header_at(data: &[u8], offset: u64) -> Option<ParsedFrameHeader> {
        let at = |i: u64| -> Option<u8> { data.get(offset.checked_add(i)? as usize).copied() };
        if at(0)? != 0xFF {
            return None;
        }
        let b1 = at(1)?;
        // 14-bit sync (0b11111111_111110xx), then reserved bit must
        // be 0; blocking strategy bit is free.
        if b1 & 0b1111_1100 != 0b1111_1000 {
            return None;
        }
        let variable = b1 & 0x01 == 1;
        let bs_code = at(2)? >> 4;
        let sr_code = at(2)? & 0x0f;
        if bs_code == 0b0000 {
            return None; // reserved
        }
        if sr_code == 0b1111 {
            return None; // invalid (sync-fooling guard)
        }
        let ch_bps = at(3)?;
        let channel_code = ch_bps >> 4;
        if channel_code > 0b1010 {
            return None; // reserved assignments
        }
        let bps_code = (ch_bps >> 1) & 0x07;
        if bps_code == 0b011 || bps_code == 0b111 {
            return None; // reserved
        }
        if ch_bps & 0x01 != 0 {
            return None; // reserved bit must be 0
        }
        // Spec §6 field order: the UTF-8-coded number comes FIRST,
        // then the block-size escape, then the sample-rate escape,
        // then the CRC-8. (Verified against claxon 0.4.3
        // `read_frame_header` and ffmpeg 7.1.5 output — see the module
        // docs; the prior escape-first draft misparsed every
        // escape-coded header.)
        let mut cursor: u64 = 4;
        let first = at(cursor)?;
        let mut additional = first.leading_ones().min(6) as u64;
        if additional == 1 {
            return None; // 0b10xxxxxx leading byte: invalid start
        }
        additional = additional.saturating_sub(1);
        let mut number: u64 = u64::from(first & (0x7f >> additional)) << (6 * additional);
        // The first byte's payload width must not overflow the coding
        // (overlong forms are invalid per spec — e.g. 0xC0/0xC1).
        if additional >= 1 && first & (0x7f >> additional) == 0 && first & 0x80 != 0 {
            // Overlong leading byte (would encode a value the shorter
            // form covers): treated as not-a-header. The spec's exact
            // rule is that the coding is defined for numbers up to
            // 2^36; claxon applies the same rejection.
            return None;
        }
        for i in 0..additional {
            let byte = at(cursor.checked_add(1 + i)?)?;
            if byte & 0b1100_0000 != 0b1000_0000 {
                return None;
            }
            number |= u64::from(byte & 0x3f) << (6 * (additional - 1 - i));
        }
        cursor += 1 + additional;
        if !variable && number > 0x7fff_ffff {
            return None; // frame number is at most 31 bits (claxon rule)
        }
        // Block size (spec table): 0b0110 → 8-bit end value + 1,
        // 0b0111 → 16-bit end value + 1, else 192 / 576·2^n / 256·2^n.
        let block_size: u32 = match bs_code {
            0b0001 => 192,
            0b0110 => {
                let v = u32::from(at(cursor)?);
                cursor += 1;
                v + 1
            }
            0b0111 => {
                let hi = u32::from(at(cursor)?);
                let lo = u32::from(at(cursor.checked_add(1)?)?);
                cursor += 2;
                // End value is one-based: ((hi<<8)|lo) + 1; 0xffff
                // would exceed the STREAMINFO block-size range.
                let raw = (hi << 8) | lo;
                if raw == 0xffff {
                    return None;
                }
                raw.checked_add(1)?
            }
            code if code < 0b0110 => 576 * (1 << (u32::from(code) - 2)),
            _ => 256 * (1 << (u32::from(bs_code) - 8)),
        };
        // Sample rate escapes: 0b1100 → 8-bit Hz, 0b1101 → 16-bit Hz,
        // 0b1110 → 16-bit tens of Hz (values not needed for
        // navigation; only their byte widths are).
        match sr_code {
            0b1100 => cursor += 1,
            0b1101 | 0b1110 => cursor += 2,
            _ => {}
        }
        // CRC-8 (poly 0x07, init 0) over the header bytes so far.
        let start = offset as usize;
        let covered = cursor as usize;
        let body = data.get(start..start.checked_add(covered)?)?;
        let stored = at(cursor)?;
        if crc8_flac(body) != stored {
            return None;
        }
        Some(ParsedFrameHeader {
            number,
            variable,
            block_size,
            header_len: u32::try_from(cursor + 1).ok()?,
        })
    }
}

/// FLAC header CRC-8 (spec §6: polynomial x⁸+x²+x+1 = 0x07, init 0,
/// no reflection, no final xor) over `data`.
///
/// Bitwise form: headers are ≤ 16 bytes, so the table-free loop is the
/// right cost/complexity trade (a 256-entry table would save ~15
/// shifts per header — noise next to the bisection's probe count).
fn crc8_flac(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Typed constructor guard for the planner: refuses a `frames_start`
/// outside the data (a corrupt layout must not seek into the void).
///
/// # Errors
/// [`FlacError::Unsupported`] (`flac.seek.frames_start`) when the
/// anchor is out of range.
pub(crate) fn planner<'a>(
    data: &'a [u8],
    facts: &'a StreamInfo,
    index: Option<&'a SeekIndex>,
    frames_start: u64,
) -> Result<SeekPlanner<'a>, FlacError> {
    if frames_start > data.len() as u64 {
        return Err(unsupported(
            "flac.seek.frames_start",
            format!(
                "frames start {frames_start} lies outside the {}-byte stream",
                data.len()
            ),
        ));
    }
    Ok(SeekPlanner::new(data, facts, index, frames_start))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 0b0111 block-size escape decode: ((hi<<8)|lo) + 1 — pin the
    /// precedence-sensitive expression so a future refactor cannot
    /// silently change it. Spec field order: number first, *then* the
    /// escape bytes (the prior escape-first draft misparsed this
    /// class of header; the CRC-8 vectors are hand-computed with an
    /// independent implementation and pin the order — swapping the
    /// number and escape bytes fails the CRC).
    #[test]
    fn block_size_escape_is_one_based_and_number_comes_first() {
        // ff f8 71 00 | frame# 01 | escape 0f ff (0x0FFF + 1 = 4096)
        // | crc 93; sr_code 0b0001 (88 200 Hz, no extra bytes),
        // independent 1 channel, bps from streaminfo, fixed blocking.
        let data = [0xFF, 0xF8, 0x71, 0x00, 0x01, 0x0F, 0xFF, 0x93];
        let header = match SeekPlanner::parse_header_at(&data, 0) {
            Some(h) => h,
            None => unreachable!("header must parse"),
        };
        assert_eq!(header.block_size, 4096);
        assert_eq!(header.number, 1);
        assert!(!header.variable);
        assert_eq!(header.header_len, 8);
        // Fixed stream with 4096-sample blocks: first sample = 1 * 4096.
        assert_eq!(header.first_sample(&test_facts()), 4096);
        // The same bytes with number and escape swapped (the old wrong
        // order) fail the CRC — the guard against reintroduction.
        let swapped = [0xFF, 0xF8, 0x71, 0x00, 0x0F, 0xFF, 0x01, 0x93];
        assert!(SeekPlanner::parse_header_at(&swapped, 0).is_none());
    }

    /// The 0b0110 8-bit escape and the variable-blocking sample
    /// number path, with hand-computed CRCs.
    #[test]
    fn eight_bit_escape_and_variable_blocking_parse() {
        // ff f8 61 00 | frame# 05 | escape 0b (11 + 1 = 12) | crc 8c.
        let data = [0xFF, 0xF8, 0x61, 0x00, 0x05, 0x0B, 0x8C];
        let header = match SeekPlanner::parse_header_at(&data, 0) {
            Some(h) => h,
            None => unreachable!("header must parse"),
        };
        assert_eq!(header.block_size, 12);
        assert_eq!(header.number, 5);
        assert_eq!(header.first_sample(&test_facts()), 5 * 4096);
        // ff f9 51 00 | sample# c1 96 (2-byte coding: lead 0xC1,
        // continuation 0x96 → (1 << 6) | 0x16 = 86) | crc 24 —
        // variable blocking: the number IS the sample.
        let var = [0xFF, 0xF9, 0x51, 0x00, 0xC1, 0x96, 0x24];
        let header = match SeekPlanner::parse_header_at(&var, 0) {
            Some(h) => h,
            None => unreachable!("header must parse"),
        };
        assert!(header.variable);
        assert_eq!(header.number, 86);
        assert_eq!(header.first_sample(&test_facts()), 86);
        assert_eq!(header.block_size, 4608);
    }

    /// A plain no-escape header with a correct CRC parses; the same
    /// header with one flipped payload byte fails the CRC (the
    /// false-sync guard the scan relies on).
    #[test]
    fn crc8_guards_false_syncs() {
        let data = [0xFF, 0xF8, 0x51, 0x00, 0x00, 0xC5];
        let header = match SeekPlanner::parse_header_at(&data, 0) {
            Some(h) => h,
            None => unreachable!("header must parse"),
        };
        assert_eq!(header.block_size, 4608);
        let mut broken = data;
        broken[4] = 0x01; // frame# 1 — structure still valid, CRC now wrong
        assert!(SeekPlanner::parse_header_at(&broken, 0).is_none());
        // The CRC helper against independently computed vectors.
        assert_eq!(crc8_flac(&[0xFF, 0xF8, 0x71, 0x00, 0x01, 0x0F, 0xFF]), 0x93);
        assert_eq!(crc8_flac(&[]), 0);
    }

    #[test]
    fn garbage_offsets_do_not_parse_as_headers() {
        // Not starting with 0xFF.
        assert!(SeekPlanner::parse_header_at(&[0x00, 0xF8, 0x70, 0x00, 0xFF, 0xAA], 0).is_none());
        // Sync ok, reserved bit set (0xFB).
        assert!(SeekPlanner::parse_header_at(&[0xFF, 0xFB, 0x70, 0x00, 0xFF, 0xAA], 0).is_none());
        // Reserved block-size code 0b0000.
        assert!(SeekPlanner::parse_header_at(&[0xFF, 0xF8, 0x00, 0x00, 0xFF, 0xAA], 0).is_none());
        // Reserved channel assignment 0b1100.
        assert!(SeekPlanner::parse_header_at(&[0xFF, 0xF8, 0x70, 0xC0, 0xFF, 0xAA], 0).is_none());
        // Reserved bit before the CRC set.
        assert!(SeekPlanner::parse_header_at(&[0xFF, 0xF8, 0x70, 0x01, 0xFF, 0xAA], 0).is_none());
        // Continuation byte without the 0b10 prefix.
        assert!(
            SeekPlanner::parse_header_at(&[0xFF, 0xF9, 0x70, 0x00, 0xC2, 0x0F, 0xAA], 0).is_none()
        );
        // Overlong two-byte form (0xC0 = 0b11000000, payload 0).
        assert!(
            SeekPlanner::parse_header_at(&[0xFF, 0xF9, 0x70, 0x00, 0xC0, 0x00, 0xAA], 0).is_none()
        );
        // Reserved sample-size code 0b111 (the bps field's second
        // reserved value; the structure is otherwise valid).
        assert!(SeekPlanner::parse_header_at(&[0xFF, 0xF8, 0x70, 0xFE, 0x00, 0xAA], 0).is_none());
        // 16-bit escape value 0xffff (block size would exceed 65 535);
        // the CRC is fixed up so only the escape value is rejected.
        let mut oversize = [0xFF, 0xF8, 0x71, 0x00, 0x00, 0xFF, 0xFF, 0x00];
        oversize[7] = crc8_flac(&oversize[..7]);
        assert!(SeekPlanner::parse_header_at(&oversize, 0).is_none());
    }

    /// The tail-frame rule: in a fixed-blocksize stream a shorter
    /// final block keeps the stream's own block size for the sample
    /// conversion (ffmpeg/libFLAC behavior — claxon's
    /// `Block::time()` disagrees; the decoder overrides it).
    #[test]
    fn fixed_stream_tail_frame_uses_the_stream_block_size() {
        let mut facts = test_facts();
        facts.min_block_size = 4608;
        facts.max_block_size = 4608;
        // Frame# 47 with its own (short) block 3924: first sample is
        // 47 × 4608 = 216 576, not 47 × 3924 = 184 428.
        let header = ParsedFrameHeader {
            number: 47,
            variable: false,
            block_size: 3924,
            header_len: 8,
        };
        assert_eq!(header.first_sample(&facts), 216_576);
        assert_eq!(header.end_sample(&facts), 220_500);
        // A stream that is NOT fixed-declared falls back to the
        // frame's own size (claxon-compatible).
        let mut varying = facts;
        varying.min_block_size = 16;
        assert_eq!(header.first_sample(&varying), 47 * 3924);
    }

    #[test]
    fn planner_refuses_out_of_range_frames_start() {
        let facts = test_facts();
        match planner(&[0u8; 8], &facts, None, 100) {
            Err(FlacError::Unsupported { code, .. }) => {
                assert_eq!(code, "flac.seek.frames_start");
            }
            other => unreachable!("{other:?}"),
        }
    }

    fn test_facts() -> StreamInfo {
        StreamInfo {
            min_block_size: 4096,
            max_block_size: 4096,
            min_frame_size: None,
            max_frame_size: None,
            sample_rate: 44_100,
            channels: 2,
            bits_per_sample: 16,
            total_samples: None,
            md5: [0u8; 16],
        }
    }
}
