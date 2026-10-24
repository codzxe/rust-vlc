//! `Cluster` parsing: `Timecode`, `SimpleBlock`, `BlockGroup`/`Block`,
//! and the three lacing codings (card 5.D1).
//!
//! # Robustness model
//!
//! - A block whose header/payload crosses the cluster end or the
//!   stream end is the typed truncation at that block's offset —
//!   never a partial frame handed downstream.
//! - A block for an unknown track number is damage (skipped, counted)
//!   — the track table is the only truth; no track is fabricated.
//! - Lacing mismatches (frame-count vs data, a Xiph size overrun, a
//!   final frame shorter than declared) are typed
//!   [`crate::mkv::error::bad_structure`] at the block offset — the
//!   card's "lacing count vs data mismatch" error path.
//! - Relative timecodes are `i16` on disk; the absolute stamp is
//!   `cluster_timecode as i64 + relative as i64` in scale ticks,
//!   checked against the timeline (negative or overflowing = the
//!   typed [`crate::mkv::error::timecode_overflow`]).

use media_types::EngineError;

use crate::mkv::ebml::{self, EbmlReader, ElementHeader};
use crate::mkv::error;

/// `SimpleBlock`/`Block` flag bit: keyframe (SimpleBlock only).
pub const FLAG_KEYFRAME: u8 = 0x80;
/// Lacing selector: bits 0..=1 of the flags byte.
pub const FLAG_LACING_MASK: u8 = 0x06;

/// One parsed frame of a block (a laced block yields many).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockFrame {
    /// The frame bytes (a slice of the block payload, offsets into it).
    pub start: usize,
    pub len: usize,
}

/// A parsed block header + its lacing plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedBlock {
    /// The Matroska track number (blocks address tracks by number).
    pub track: u64,
    /// The block's relative timecode (i16 on disk, scale ticks).
    pub relative: i16,
    /// The keyframe flag (SimpleBlock bit 0x80; BlockGroup: derived
    /// from `ReferenceBlock` absence by the caller).
    pub keyframe: bool,
    /// Lacing coding of the payload.
    pub lacing: Lacing,
    /// The frames (offsets relative to the *body* — see
    /// [`ParsedBlock::body_offset`]).
    pub frames: Vec<BlockFrame>,
    /// Where the laced body starts inside the block payload (past
    /// the track vint + i16 timecode + flags byte). Frame starts are
    /// body-relative; absolute payload offsets are
    /// `payload_offset + body_offset + frame.start`.
    pub body_offset: usize,
    /// Absolute file offset of the block payload start (error context).
    pub payload_offset: u64,
    /// The block payload's total declared length in bytes.
    pub payload_len: usize,
}

/// The lacing codings (spec §6.3.6 table, bits 1..=0 → value).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lacing {
    /// `00` — one frame, the whole payload.
    None,
    /// `01` — Xiph-style little-endian sizes with 255 continuation.
    Xiph,
    /// `11` — EBML vint-coded sizes (first frame size − 1 encoded).
    Ebml,
    /// `10` — equal-sized frames (count byte only).
    Fixed,
}

/// The cluster's parsed `Timecode` (absolute, scale ticks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClusterTimecode {
    /// The absolute base timestamp of the cluster.
    pub ticks: i64,
    /// Where the `Timecode` element sat (error context).
    pub offset: u64,
}

/// Read the cluster's `Timecode` child (must precede the blocks for
/// the timestamps to be meaningful; absence is the typed structural
/// refusal — every block would carry an unknown base).
///
/// `bound` is the cluster's effective end (declared or segment end).
///
/// # Errors
/// [`error::bad_structure`] when the cluster carries no `Timecode` or
/// a non-uint one; [`error::truncated`] when cut by the stream.
pub fn parse_timecode(
    reader: &mut EbmlReader,
    cluster: &ElementHeader,
    bound: u64,
) -> Result<ClusterTimecode, EngineError> {
    let end = cluster.declared_end().unwrap_or(bound).min(bound);
    let mut pos = cluster.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            // Unknown-size children inside a cluster are not legal —
            // the typed refusal (the streaming form is a Segment- or
            // Cluster-level property, not a Timecode one).
            return Err(error::bad_structure(child.data_pos));
        };
        if child.id == ebml::ID_TIMECODE {
            let len =
                usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
            if len > 8 {
                return Err(error::bad_structure(child.data_pos));
            }
            let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
            let value =
                ebml::uint_from_be(&bytes).ok_or_else(|| error::bad_structure(child.data_pos))?;
            return Ok(ClusterTimecode {
                ticks: value as i64,
                offset: child.data_pos,
            });
        }
        // Skip non-Timecode children (the blocks are walked separately).
        pos = child_end;
    }
    Err(error::bad_structure(cluster.data_pos))
}

/// Parse one block (SimpleBlock or Block payload) from `data`.
///
/// Grammar (spec §6.3.6 / WebM §5.5): track vint, i16 BE relative
/// timecode, flags byte, then the laced frame payload.
///
/// # Errors
/// [`error::bad_structure`] for a block too short for its header, an
/// unknown lacing value is impossible (2 bits saturate), or a lacing
/// count/size contradiction; the offset is the payload's absolute
/// position plus the in-block offset for precision.
pub fn parse_block(
    data: &[u8],
    payload_offset: u64,
    simple: bool,
) -> Result<ParsedBlock, EngineError> {
    let bad = |at: usize| error::bad_structure(payload_offset + at as u64);
    if data.len() < 4 {
        return Err(bad(data.len()));
    }
    // Track numbers are vint *values* (marker stripped), unlike
    // element ids; a size-shaped parse of the same head yields the
    // number. The width comes from the id-shaped parse (same marker
    // grammar) so both agree on `len`.
    let tid = ebml::parse_vint_id(data).map_err(|_| bad(0))?;
    let (track, after_track) = {
        let mut len = 1usize;
        let mut mask = 0x80u8;
        while data[0] & mask == 0 {
            mask >>= 1;
            len += 1;
        }
        let mut value = u64::from(data[0] & (mask - 1));
        for &b in &data[1..len] {
            value = (value << 8) | u64::from(b);
        }
        (value, tid.len)
    };
    if data.len() < after_track + 3 {
        return Err(bad(data.len()));
    }
    let relative = i16::from_be_bytes([data[after_track], data[after_track + 1]]);
    let flags = data[after_track + 2];
    let lacing_bits = (flags & FLAG_LACING_MASK) >> 1;
    let lacing = match lacing_bits {
        0 => Lacing::None,
        1 => Lacing::Xiph,
        2 => Lacing::Fixed,
        3 => Lacing::Ebml,
        _ => unreachable!(),
    };
    let body = &data[after_track + 3..];
    let frames = lace_frames(body, lacing).map_err(|at| bad(after_track + 3 + at))?;
    Ok(ParsedBlock {
        track,
        relative,
        keyframe: simple && flags & FLAG_KEYFRAME != 0,
        lacing,
        frames,
        body_offset: after_track + 3,
        payload_offset,
        payload_len: data.len(),
    })
}

/// Split `body` into frames per the lacing coding. `Err(at)` is the
/// in-body offset of the contradiction.
fn lace_frames(body: &[u8], lacing: Lacing) -> Result<Vec<BlockFrame>, usize> {
    match lacing {
        Lacing::None => Ok(vec![BlockFrame {
            start: 0,
            len: body.len(),
        }]),
        Lacing::Xiph => {
            let (frame_count, sizes, consumed) = xiph_sizes(body)?;
            if frame_count == 0 {
                return Err(consumed);
            }
            build_frames(body, consumed, &sizes, frame_count)
        }
        Lacing::Ebml => {
            let (frame_count, sizes, consumed) = ebml_sizes(body)?;
            if frame_count == 0 {
                return Err(consumed);
            }
            build_frames(body, consumed, &sizes, frame_count)
        }
        Lacing::Fixed => {
            if body.is_empty() {
                return Err(0);
            }
            let frame_count = body[0] as usize;
            let consumed = 1usize;
            if frame_count == 0 {
                return Err(0);
            }
            let rest = body.len() - consumed;
            if !rest.is_multiple_of(frame_count) || rest == 0 {
                // Fixed lacing divides the payload exactly.
                return Err(consumed);
            }
            let each = rest / frame_count;
            Ok((0..frame_count)
                .map(|i| BlockFrame {
                    start: consumed + i * each,
                    len: each,
                })
                .collect())
        }
    }
}

/// Xiph lacing sizes: count byte, then `count-1` little-endian sizes
/// in 255-continuation form. The last frame is the remainder.
fn xiph_sizes(body: &[u8]) -> Result<(usize, Vec<usize>, usize), usize> {
    let Some(&count) = body.first() else {
        return Err(0);
    };
    let frame_count = count as usize;
    let mut sizes = Vec::with_capacity(frame_count.saturating_sub(1));
    let mut at = 1usize;
    for _ in 0..frame_count.saturating_sub(1) {
        let mut size = 0usize;
        loop {
            let Some(&b) = body.get(at) else {
                return Err(at);
            };
            at += 1;
            size += usize::from(b);
            if b != 0xFF {
                break;
            }
            // 255-continuation; guard the accumulation against a
            // size-only body (a missing terminator).
            if size > body.len() {
                return Err(at);
            }
        }
        sizes.push(size);
    }
    Ok((frame_count, sizes, at))
}

/// EBML lacing sizes: count byte, then `count-1` sizes where the
/// first is a vint of (size − 1) and the rest are vint *deltas*
/// (signed accumulate).
fn ebml_sizes(body: &[u8]) -> Result<(usize, Vec<usize>, usize), usize> {
    let Some(&count) = body.first() else {
        return Err(0);
    };
    let frame_count = count as usize;
    if frame_count == 0 {
        return Err(0);
    }
    let mut sizes: Vec<usize> = Vec::with_capacity(frame_count);
    let mut at = 1usize;
    // First size: vint value plus 1 (the spec's −1 offset keeps the
    // 1-byte vint usable for small frames). A single-frame lace
    // (count 1) carries just it.
    let v = parse_vint_value(body, at)?;
    at = v.1;
    let mut current = usize::try_from(v.0.checked_add(1).ok_or(at)?).map_err(|_| at)?;
    sizes.push(current);
    // The first size vint plus (count − 2) deltas: count − 1 vints
    // total, the last frame is the payload remainder (Xiph's shape).
    for _ in 2..frame_count {
        let prev = at;
        let (value, end) = parse_vint_value(body, at)?;
        at = end;
        // Deltas center at 2^(7·n − 1) − 1 where n is the *vint's
        // byte length* (its value bits are 7·n — the bias
        // libmatroska/ffmpeg share).
        let vlen = (end - prev) as u32;
        let center = (1u64 << (vlen * 7 - 1)).saturating_sub(1);
        let delta = value as i64 - center as i64;
        let next = (current as i64).checked_add(delta).ok_or(at)?;
        if next < 0 {
            return Err(at);
        }
        current = usize::try_from(next).map_err(|_| at)?;
        sizes.push(current);
        if current > body.len() {
            return Err(at);
        }
    }
    Ok((frame_count, sizes, at))
}

/// Parse a size-shaped vint at `at`; `Err(at)` on grammar failure.
fn parse_vint_value(data: &[u8], at: usize) -> Result<(u64, usize), usize> {
    let Some(&first) = data.get(at) else {
        return Err(at);
    };
    if first == 0 {
        return Err(at);
    }
    let mut len = 1usize;
    let mut mask = 0x80u8;
    while first & mask == 0 {
        mask >>= 1;
        len += 1;
        if len > 8 {
            return Err(at);
        }
    }
    if at + len > data.len() {
        return Err(at);
    }
    let mut value = u64::from(first & (mask - 1));
    for &b in &data[at + 1..at + len] {
        value = (value << 8) | u64::from(b);
    }
    Ok((value, at + len))
}

/// Build the frame table from per-frame sizes + the trailing "last
/// frame is the remainder" rule (Xiph and EBML share it).
fn build_frames(
    body: &[u8],
    consumed: usize,
    sizes: &[usize],
    frame_count: usize,
) -> Result<Vec<BlockFrame>, usize> {
    let declared: usize = sizes.iter().sum();
    let rest = body.len().saturating_sub(consumed);

    if declared > rest {
        // The declared head frames overrun the payload: the card's
        // lacing-count-vs-data mismatch.
        return Err(consumed);
    }
    let last = rest - declared;
    let mut frames = Vec::with_capacity(frame_count);
    let mut at = consumed;
    for &size in sizes {
        frames.push(BlockFrame {
            start: at,
            len: size,
        });
        at += size;
    }
    frames.push(BlockFrame {
        start: at,
        len: last,
    });
    Ok(frames)
}

/// The absolute presentation stamp of a block (scale ticks), with the
/// timeline check.
///
/// # Errors
/// [`error::timecode_overflow`] when the sum leaves `i64` or goes
/// negative (the card's block-timecode-overflow path).
pub fn absolute_ticks(cluster: i64, relative: i16, offset: u64) -> Result<i64, EngineError> {
    let sum = cluster
        .checked_add(i64::from(relative))
        .ok_or_else(|| error::timecode_overflow(offset))?;
    if sum < 0 {
        return Err(error::timecode_overflow(offset));
    }
    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    fn block(track: u8, rel: i16, flags: u8, body: &[u8]) -> Vec<u8> {
        // Track numbers ride as vints: the marker bit for ≤ 126.
        assert!(track <= 126);
        let mut v = vec![0x80 | track];
        v.extend_from_slice(&rel.to_be_bytes());
        v.push(flags);
        v.extend_from_slice(body);
        v
    }

    #[test]
    fn no_lacing_single_frame() {
        let data = block(3, -5, 0x80, b"frame-bytes");
        let b = parse_block(&data, 0, true).must();
        assert_eq!(b.track, 3);
        assert_eq!(b.relative, -5);
        assert!(b.keyframe);
        assert_eq!(b.lacing, Lacing::None);
        assert_eq!(b.frames.len(), 1);
        assert_eq!(
            &data[4 + b.frames[0].start..4 + b.frames[0].start + b.frames[0].len],
            b"frame-bytes"
        );
    }

    #[test]
    fn xiph_lacing_splits_and_remainder_last() {
        // 3 frames: sizes 5, 300 (0xFF,0x2D continuation), rest.
        let mut body = vec![3, 5, 0xFF, 0x2D];
        body.extend_from_slice(&[0xA0; 5]);
        body.extend_from_slice(&[0xB0; 300]);
        body.extend_from_slice(&[0xC0; 160]);
        let data = block(1, 0, 0x02, &body); // 0b10 >> 1 = Xiph
        let b = parse_block(&data, 0, true).must();
        assert_eq!(b.lacing, Lacing::Xiph);
        assert_eq!(b.frames.len(), 3);
        assert_eq!(b.frames[0].len, 5);
        assert_eq!(b.frames[1].len, 300);
        assert_eq!(b.frames[2].len, 160);
        assert_eq!(b.frames[0].start, 1 + 3);
    }

    #[test]
    fn fixed_lacing_divides_exactly() {
        let body = [4u8, 1, 2, 3, 4, 5, 6, 7, 8];
        let data = block(2, 10, 0x04, &body); // 0b100 >> 1 = Fixed
        let b = parse_block(&data, 0, true).must();
        assert_eq!(b.lacing, Lacing::Fixed);
        assert_eq!(b.frames.len(), 4);
        assert!(b.frames.iter().all(|f| f.len == 2));
    }

    #[test]
    fn ebml_lacing_sizes_decode() {
        // 3 frames; first size vint 0x82 (value 2 → size 3); the
        // equal-size delta 0 encodes as value 63 = bias 2^6−1 → 0xBF.
        let mut body = vec![3, 0x82, 0xBF];
        body.extend_from_slice(&[0xAA; 3]);
        body.extend_from_slice(&[0xBB; 3]);
        body.extend_from_slice(&[0xCC; 2]);
        let data = block(1, 0, 0x06, &body); // 0b110 >> 1 = EBML
        let b = parse_block(&data, 0, true).must();
        assert_eq!(b.lacing, Lacing::Ebml);
        assert_eq!(b.frames.len(), 3);
        assert_eq!(b.frames[0].len, 3);
        assert_eq!(b.frames[1].len, 3);
        assert_eq!(b.frames[2].len, 2); // remainder
    }

    #[test]
    fn lacing_overrun_is_typed_mismatch() {
        // Declared head sizes exceed the body.
        let body = [2, 10, 10, 0x01];
        let data = block(1, 0, 0x02, &body);
        let err = parse_block(&data, 100, true).err().must();
        assert_eq!(err.context.code, "demux.mkv.bad-structure");
    }

    #[test]
    fn fixed_lacing_remainder_is_refused() {
        let body = [3u8, 1, 2, 3, 4, 5];
        let data = block(1, 0, 0x04, &body);
        let err = parse_block(&data, 0, true).err().must();
        assert_eq!(err.context.code, "demux.mkv.bad-structure");
    }

    #[test]
    fn short_block_header_is_typed() {
        let err = parse_block(&[0x81, 0x00], 0, true).err().must();
        assert_eq!(err.context.code, "demux.mkv.bad-structure");
    }

    #[test]
    fn absolute_ticks_checks_the_timeline() {
        assert_eq!(absolute_ticks(1000, -40, 0).must(), 960);
        assert!(absolute_ticks(10, -40, 7).is_err());
        assert!(absolute_ticks(i64::MAX, 2, 7).is_err());
    }
}
