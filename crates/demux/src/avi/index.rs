//! `idx1` index parsing + the seek search (card 2.B2; doc 03 §4.3).
//!
//! The AVI 1.0 index: one 16-byte entry per `movi` chunk — `fourcc`,
//! `dwFlags` (`AVIIF_KEYFRAME` = `0x10`), `dwOffset` (relative to the
//! position of the `movi` form FourCC), `dwSize`.
//!
//! # Role in this demuxer
//!
//! The `movi` **walk is the packet-extraction truth** (it works with or
//! without an index — the `noidx0` class-B fixture demuxes fully
//! through it). The index is the *seek table*: keyframe flags (with
//! codec-grammar fallback when an entry is absent) and the
//! searched time→position mapping.
//!
//! # Salvage rules (bounded recovery, doc 03 §4.3(d))
//!
//! A truncated trailing entry is dropped, never fatal — the walk
//! itself is intact whenever the `movi` is (the `trunc4` fixture loses
//! exactly its last index entry and still demuxes fully). Entries
//! pointing outside the `movi` data region are dropped as
//! `BadIndex`-flavored damage. A payload cut *before* the first entry
//! leaves an empty table — seek then falls back to the linear scan.

use media_types::EngineError;

use super::header::{DamageLog, StreamDesc};
use super::riff::{self, classify_movi_chunk, MoviChunkKind, SourceReader};
use crate::error;

/// `AVIIF_KEYFRAME` (the only flag bit v0 consumes).
pub const AVIIF_KEYFRAME: u32 = 0x0000_0010;

/// Maximum parsed index entries `[PROPOSAL: 524_288 = 8 MiB of raw
/// idx1 payload; a 2-hour 30 fps AVI with per-frame audio interleaving
/// needs ~432k — above the cap is corruption, not media]`.
pub const MAX_IDX1_ENTRIES: usize = 524_288;

/// One validated index entry (absolute positions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexEntry {
    /// The `movi` chunk's absolute file position.
    pub chunk_pos: u64,
    /// Declared payload size (pad excluded).
    pub size: u32,
    /// The stream ordinal from the entry's fourcc.
    pub stream: u32,
    /// `AVIIF_KEYFRAME` was set.
    pub keyframe: bool,
    /// The entry's presentation ticks on its stream's base (computed
    /// by the same per-stream ladder the walk uses, so index and walk
    /// agree on time).
    pub pts_ticks: i64,
}

/// The parsed `idx1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexTable {
    /// Validated entries in file order.
    pub entries: Vec<IndexEntry>,
}

/// A seek landing: where the stream would resume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Landing {
    /// The landing chunk's absolute position.
    pub chunk_pos: u64,
    /// Presentation ticks of the landing chunk (stream base).
    pub pts_ticks: i64,
}

/// One step of a stream's presentation ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LadderStep {
    /// The chunk's presentation ticks (the ladder position *before*
    /// the advance).
    pub pts_ticks: i64,
    /// The chunk's payload was not a whole number of sample frames
    /// (PCM misalignment) — the caller counts damage.
    pub unaligned: bool,
}

/// The per-stream presentation ladders: how far each stream has
/// advanced, in its own base's ticks. Shared by the `movi` walk, the
/// index parse and the linear-scan fallback so all three agree on
/// time by construction.
pub(crate) struct Ladders {
    ticks: Vec<i64>,
    descs: Vec<StreamDesc>,
}

impl Ladders {
    /// Ladders for the given stream table (indexed by ordinal).
    pub(crate) fn new(streams: &[StreamDesc]) -> Ladders {
        let max_ordinal = streams.iter().map(|s| s.ordinal).max().unwrap_or(0);
        let mut ticks = vec![0i64; max_ordinal as usize + 1];
        for stream in streams {
            let idx = stream.ordinal as usize;
            if idx < ticks.len() {
                ticks[idx] = i64::from(stream.start);
            }
        }
        Ladders {
            ticks,
            descs: streams.to_vec(),
        }
    }

    /// Advance the stream's ladder over a chunk of `payload_bytes`;
    /// returns the chunk's presentation ticks, or `None` for an
    /// ordinal outside the table.
    pub(crate) fn advance(&mut self, stream: u32, payload_bytes: u64) -> Option<LadderStep> {
        let desc = self.descs.iter().find(|d| d.ordinal == stream)?;
        let idx = stream as usize;
        if idx >= self.ticks.len() {
            return None;
        }
        let (delta, unaligned) = desc.ticks_per_chunk(payload_bytes);
        let before = self.ticks[idx];
        self.ticks[idx] = self.ticks[idx].saturating_add(delta);
        Some(LadderStep {
            pts_ticks: before,
            unaligned,
        })
    }
}

/// Parse the `idx1` chunk at `idx1_pos` (its header) into a table.
///
/// `movi_form_pos` is the absolute position of the `movi` FourCC (the
/// base `dwOffset` is relative to); `movi_data_start`/`movi_data_end`
/// bound entry validation; `bound` is the outer extent
/// (RIFF ∩ stream); `streams` supplies the tick ladders.
///
/// # Errors
/// [`error::index_too_large`] when the declared entry count exceeds
/// the cap; source errors propagate. Truncation is *never* fatal here
/// (see the module docs).
#[allow(clippy::too_many_arguments)]
pub fn parse_idx1(
    reader: &mut SourceReader,
    idx1_pos: u64,
    declared_size: u32,
    movi_form_pos: u64,
    movi_data_start: u64,
    movi_data_end: u64,
    bound: u64,
    streams: &[StreamDesc],
    damage: &mut DamageLog,
) -> Result<Option<IndexTable>, EngineError> {
    if declared_size == 0 {
        // An empty idx1 is an index that says nothing.
        return Ok(None);
    }
    let entry_count = (u64::from(declared_size) + 15) / 16;
    if entry_count > MAX_IDX1_ENTRIES as u64 {
        return Err(error::index_too_large(
            MAX_IDX1_ENTRIES as u64,
            entry_count,
        ));
    }

    let available = bound.saturating_sub(idx1_pos + 8);
    let payload = u64::from(declared_size).min(available);
    let mut entries: Vec<IndexEntry> = Vec::new();
    let mut ladders = Ladders::new(streams);
    let mut offset: u64 = 0;

    while offset + 16 <= payload {
        let Some(block) = reader.read_exact_or_end_at(16, idx1_pos + 8 + offset)? else {
            break;
        };
        let fourcc = riff::FourCC::new([block[0], block[1], block[2], block[3]]);
        let flags = riff::u32le(&block, 4);
        let rel = riff::u32le(&block, 8);
        let size = riff::u32le(&block, 12);
        match classify_movi_chunk(&fourcc) {
            MoviChunkKind::Packet { stream } => {
                let chunk_pos = movi_form_pos.saturating_add(u64::from(rel));
                let data_end = chunk_pos.saturating_add(8 + u64::from(size));
                if chunk_pos < movi_data_start || data_end > movi_data_end {
                    damage.note(16, "idx1-entry-outside-movi");
                } else if let Some(step) = ladders.advance(stream, u64::from(size)) {
                    entries.push(IndexEntry {
                        chunk_pos,
                        size,
                        stream,
                        keyframe: flags & AVIIF_KEYFRAME != 0,
                        pts_ticks: step.pts_ticks,
                    });
                } else {
                    // A packet entry for a stream outside the table
                    // (unsupported type): positionally valid, not
                    // timeable — kept out of the seek table.
                    damage.note(16, "idx1-entry-unknown-stream");
                }
            }
            _ => damage.note(16, "idx1-entry-not-a-packet"),
        }
        offset += 16;
    }

    if payload % 16 != 0 {
        damage.note(payload % 16, "idx1-partial-entry");
    }
    if entries.is_empty() {
        Ok(None)
    } else {
        Ok(Some(IndexTable { entries }))
    }
}

/// Search the index for the seek landing: the **last keyframe entry of
/// `stream` whose pts ≤ `target_ticks`** (doc 03 §4.3(f):
/// keyframe-safe position at-or-below the target).
///
/// `None` when no such entry exists (target before the first
/// keyframe — the caller decides whether that is out of range).
pub fn search_index(table: &IndexTable, stream: u32, target_ticks: i64) -> Option<Landing> {
    let mut best: Option<Landing> = None;
    // Entries are in file order; per-stream pts is monotone in file
    // order, so the scan can stop early once past the target on the
    // seek stream.
    for entry in &table.entries {
        if entry.stream != stream {
            continue;
        }
        if entry.pts_ticks > target_ticks {
            break;
        }
        if entry.keyframe {
            best = Some(Landing {
                chunk_pos: entry.chunk_pos,
                pts_ticks: entry.pts_ticks,
            });
        }
    }
    best
}

/// Linear-scan fallback when no `idx1` exists (the `noidx0`
/// contract): peek the `movi` chunk headers positionally and find the
/// last chunk of `stream` with pts ≤ `target_ticks`.
///
/// The reader's read position is **restored** before returning — the
/// landed `Demuxer::seek` contract forbids moving it, and this is the
/// slow path that must honor it most carefully.
///
/// Keyframeness without an index is codec-grammar truth: v0's filter
/// keeps only MJPEG video and PCM audio, both all-keyframe by
/// construction (every chunk decodes from its own bytes — doc 03 §4.3
/// invariant D2's "codec grammar guarantees" clause).
///
/// # Errors
/// Source errors propagate; list nesting deeper than
/// [`super::MAX_LIST_DEPTH`] aborts with a typed structure error (the
/// walk would hit the same damage when it reaches it).
pub fn scan_movi(
    reader: &mut SourceReader,
    streams: &[StreamDesc],
    stream: u32,
    movi_data_start: u64,
    movi_data_end: u64,
    target_ticks: i64,
) -> Result<Option<Landing>, EngineError> {
    let saved = reader.position();
    let outcome = scan_movi_inner(
        reader,
        streams,
        stream,
        movi_data_start,
        movi_data_end,
        target_ticks,
    );
    let _ = reader.seek_to(saved);
    outcome
}

/// Inner scan (position left wherever; the wrapper restores).
fn scan_movi_inner(
    reader: &mut SourceReader,
    streams: &[StreamDesc],
    stream: u32,
    mut pos: u64,
    end: u64,
    target_ticks: i64,
) -> Result<Option<Landing>, EngineError> {
    let mut ladders = Ladders::new(streams);
    let mut best: Option<Landing> = None;
    let mut ends: Vec<u64> = Vec::new();
    let length = reader.length();

    loop {
        let bound = ends.last().copied().unwrap_or(end);
        if pos + 8 > bound {
            match ends.pop() {
                Some(_) => continue,
                None => break,
            }
        }
        if pos + 8 > length {
            // The scan hit the physical end mid-list: no landing at or
            // below the target was found before the damage.
            break;
        }
        let header = riff::peek_chunk_header(reader, pos)?;
        let next = pos + 8 + u64::from(header.size) + riff::pad_for(u64::from(header.size));
        match classify_movi_chunk(&header.fourcc) {
            MoviChunkKind::Packet { stream: s } => {
                if let Some(step) = ladders.advance(s, u64::from(header.size)) {
                    if s == stream {
                        if step.pts_ticks > target_ticks {
                            // Past the target on the seek stream: the
                            // scan's answer is already final.
                            break;
                        }
                        best = Some(Landing {
                            chunk_pos: pos,
                            pts_ticks: step.pts_ticks,
                        });
                    }
                }
                pos = next;
            }
            MoviChunkKind::RecList => {
                if ends.len() >= super::MAX_LIST_DEPTH {
                    return Err(error::bad_structure(pos));
                }
                ends.push(pos + 8 + u64::from(header.size));
                pos += 8 + 4;
            }
            MoviChunkKind::Other => {
                pos = next;
            }
        }
    }
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_index_finds_last_keyframe_at_or_below() {
        let table = IndexTable {
            entries: vec![
                IndexEntry {
                    chunk_pos: 100,
                    size: 10,
                    stream: 0,
                    keyframe: true,
                    pts_ticks: 0,
                },
                IndexEntry {
                    chunk_pos: 200,
                    size: 10,
                    stream: 1,
                    keyframe: true,
                    pts_ticks: 3,
                },
                IndexEntry {
                    chunk_pos: 300,
                    size: 10,
                    stream: 0,
                    keyframe: false,
                    pts_ticks: 5,
                },
                IndexEntry {
                    chunk_pos: 400,
                    size: 10,
                    stream: 0,
                    keyframe: true,
                    pts_ticks: 10,
                },
                IndexEntry {
                    chunk_pos: 500,
                    size: 10,
                    stream: 0,
                    keyframe: true,
                    pts_ticks: 20,
                },
            ],
        };
        // Target 10 → the keyframe at ticks 10 (not the non-keyframe 5).
        let landing = search_index(&table, 0, 10).unwrap_or_else(|| unreachable!("must land"));
        assert_eq!(landing.pts_ticks, 10);
        assert_eq!(landing.chunk_pos, 400);
        // Target 15 → still 10 (20 is past).
        let landing = search_index(&table, 0, 15).unwrap_or_else(|| unreachable!("must land"));
        assert_eq!(landing.pts_ticks, 10);
        // Target 5 → the 0 keyframe (5 is not a keyframe).
        let landing = search_index(&table, 0, 5).unwrap_or_else(|| unreachable!("must land"));
        assert_eq!(landing.pts_ticks, 0);
        // Other streams answer from their own entries only.
        assert_eq!(search_index(&table, 1, 0).map(|l| l.pts_ticks), Some(3));
        // Before the first keyframe → None.
        assert!(search_index(&table, 0, -1).is_none());
    }
}
