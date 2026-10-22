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
//! exactly its last index entry and still demuxes fully). A lying
//! declared size is clamped to the bytes that actually exist before
//! any cap is applied, and the clamp is damage-counted. Entries
//! pointing outside the `movi` data region are dropped as
//! `BadIndex`-flavored damage — **coalesced to one damage event per
//! reason** (a 2-hour file with a wholly-lying index drops ~524k
//! entries; one event per entry would queue an ~8k-tick event storm,
//! so the count rides the event's byte total instead: 16 bytes per
//! dropped entry). A payload cut *before* the first entry leaves an
//! empty table — seek then falls back to the linear scan.
//!
//! # Entry order (the dirty-world rule, D-9)
//!
//! Real-world writers have emitted `idx1` entries in orders other
//! than file order (partial-sorted tables, appended entries, plain
//! corruption). The entry's `dwChunkOffset` **identifies** its chunk;
//! presentation time is a property of FILE order — the same ladder the
//! `movi` walk advances. The parser therefore collects the raw
//! entries, orders them by `chunk_pos` (FFmpeg's own ordered-insert
//! disambiguation), and only then runs the ladder pass that assigns
//! `pts_ticks`: an out-of-order `idx1` yields the same seek table the
//! file-ordered equivalent would have, so [`search_index`]'s
//! per-stream monotonicity precondition holds by construction. The
//! reorder is never silent — one damage note carries the count of
//! misplaced entries (16 bytes per entry, the established convention).
//! Duplicate `dwOffset`s keep their relative order (the sort is
//! stable); a lying `dwSize` still diverges the replayed ladder from
//! the walk exactly as it did before, bounded and observable via pts.
//!
//! # Offset base (the tribal-knowledge case, D-9)
//!
//! The AVI 1.0 spec says `dwOffset` is relative to the `movi` form
//! FourCC — but real-world writers (and the OpenDML interregnum)
//! emitted absolute file offsets. The parser probes the first packet
//! entry against both bases (FFmpeg's own disambiguation): a base is
//! accepted when the candidate position sits inside the `movi` data
//! region **and** the chunk id actually stored there matches the
//! entry's own fourcc. Relative wins ties (spec default); the
//! absolute interpretation is recorded as a damage note so the
//! variant is observable, never silent.

use media_types::EngineError;

use super::header::{DamageLog, StreamDesc};
use super::riff::{self, MoviChunkKind, SourceReader, classify_movi_chunk};
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

    /// A copy of every stream's ladder position (the re-anchor input
    /// the linear scan captures when it finds a landing).
    pub(crate) fn ticks_snapshot(&self) -> Vec<i64> {
        self.ticks.clone()
    }

    /// Ladders for `streams` restored to a captured position. The
    /// length guard is unreachable within one demuxer instance (the
    /// stream table is fixed after `start`); a mismatch keeps the
    /// fresh ladder rather than guessing.
    pub(crate) fn with_ticks(streams: &[StreamDesc], ticks: Vec<i64>) -> Ladders {
        let mut ladders = Ladders::new(streams);
        if ticks.len() == ladders.ticks.len() {
            ladders.ticks = ticks;
        }
        ladders
    }
}

/// Rebuild the pre-landing ladder state from the index: advance over
/// every entry strictly before `before_pos`, in file order (the same
/// walk `parse_idx1` performed, so index and re-anchor agree on time
/// by construction — exact for healthy indexes; a lying `dwOffset`
/// can shift the reconstruction the same way it already diverges the
/// pure query from the walk, bounded and observable via pts).
pub(crate) fn replay_ladders(
    table: &IndexTable,
    streams: &[StreamDesc],
    before_pos: u64,
) -> Ladders {
    let mut ladders = Ladders::new(streams);
    for entry in &table.entries {
        if entry.chunk_pos >= before_pos {
            break;
        }
        let _ = ladders.advance(entry.stream, u64::from(entry.size));
    }
    ladders
}

/// One raw `idx1` packet entry as stored, before the file-order pass
/// sorts it and the ladders time it (see the module docs' entry-order
/// rule). Positionally validated, not yet timed.
struct RawIdxEntry {
    chunk_pos: u64,
    size: u32,
    stream: u32,
    keyframe: bool,
}

/// Parse the `idx1` chunk at `idx1_pos` (its header) into a table.
///
/// `movi_form_pos` is the absolute position of the `movi` FourCC (the
/// base `dwOffset` is relative to); `movi_data_start`/`movi_data_end`
/// bound entry validation; `bound` is the outer extent
/// (RIFF ∩ stream); `streams` supplies the tick ladders.
///
/// Entries come out **in file order with file-order timing** (the
/// module docs' entry-order rule): raw entries are collected in
/// `idx1` order, ordered by `chunk_pos`, and only then assigned
/// `pts_ticks` by the ladder pass — so out-of-order tables are
/// time-corrected, never silently mis-seeked.
///
/// # Errors
/// [`error::index_too_large`] when the *clamped* entry count (the
/// bytes that actually exist, never the declared size) still exceeds
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
    // Clamp BEFORE the cap check (D-3/D-9): a lying declared size
    // (0xFFFFFFF0 with 40 real bytes in the stream) must meet the
    // bytes that exist, not the bytes it claims — a small salvage with
    // damage, never a fatal over-cap abort for a recoverable lie.
    let available = bound.saturating_sub(idx1_pos + 8);
    let payload = u64::from(declared_size).min(available);
    if u64::from(declared_size) > payload {
        damage.note(
            u64::from(declared_size) - payload,
            "idx1-size-past-available",
        );
    }
    let entry_count = payload.div_ceil(16);
    if entry_count > MAX_IDX1_ENTRIES as u64 {
        return Err(error::index_too_large(MAX_IDX1_ENTRIES as u64, entry_count));
    }

    // Raw entries as stored (idx1 order), positionally validated but
    // not yet timed — the ladder pass runs after the file-order sort
    // (the module docs' entry-order rule).
    let mut raw: Vec<RawIdxEntry> = Vec::new();
    // Coalesced drop counters (one damage event per reason at the
    // end — see the module docs for why per-entry events are a
    // denial-of-service vector, not observability).
    let mut dropped_outside_movi: u64 = 0;
    let mut dropped_unknown_stream: u64 = 0;
    let mut dropped_not_packet: u64 = 0;
    // Entries stored out of file order (each compared against the
    // highest position seen so far — an entry that belongs before an
    // earlier-stored one is misplaced, duplicates are not).
    let mut out_of_order: u64 = 0;
    let mut highest_pos: Option<u64> = None;
    let mut offset: u64 = 0;
    // The dwOffset base: probed against both interpretations on the
    // first packet entry (relative is the spec default; absolute is
    // the legacy variant — see the module docs).
    let mut base: Option<u64> = None;

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
                if base.is_none() {
                    base = Some(probe_offset_base(
                        reader,
                        &fourcc,
                        rel,
                        size,
                        movi_form_pos,
                        movi_data_start,
                        movi_data_end,
                    ));
                    if base == Some(0) {
                        // The legacy absolute-offset variant,
                        // recognized and recorded — never silent.
                        damage.note(0, "idx1-absolute-offsets");
                    }
                }
                let chunk_pos = base.unwrap_or(movi_form_pos).saturating_add(u64::from(rel));
                let data_end = chunk_pos.saturating_add(8 + u64::from(size));
                if chunk_pos < movi_data_start || data_end > movi_data_end {
                    dropped_outside_movi = dropped_outside_movi.saturating_add(1);
                } else {
                    if highest_pos.is_some_and(|highest| chunk_pos < highest) {
                        out_of_order = out_of_order.saturating_add(1);
                    }
                    highest_pos =
                        Some(highest_pos.map_or(chunk_pos, |highest| highest.max(chunk_pos)));
                    raw.push(RawIdxEntry {
                        chunk_pos,
                        size,
                        stream,
                        keyframe: flags & AVIIF_KEYFRAME != 0,
                    });
                }
            }
            _ => {
                dropped_not_packet = dropped_not_packet.saturating_add(1);
            }
        }
        offset += 16;
    }

    if payload % 16 != 0 {
        damage.note(payload % 16, "idx1-partial-entry");
    }
    // The file-order pass: order by the chunk each entry identifies,
    // then time it (a shuffled or partially-sorted idx1 must yield the
    // same seek table its file-ordered equivalent would — see the
    // module docs' entry-order rule). Stable, so duplicate positions
    // keep their stored order.
    raw.sort_by_key(|entry| entry.chunk_pos);
    let mut ladders = Ladders::new(streams);
    // One allocation, sized to the raw table (D-1: the parse is a
    // one-time start cost, never per-frame; the entry cap bounds it).
    let mut entries: Vec<IndexEntry> = Vec::with_capacity(raw.len());
    for entry in raw {
        if let Some(step) = ladders.advance(entry.stream, u64::from(entry.size)) {
            entries.push(IndexEntry {
                chunk_pos: entry.chunk_pos,
                size: entry.size,
                stream: entry.stream,
                keyframe: entry.keyframe,
                pts_ticks: step.pts_ticks,
            });
        } else {
            // A packet entry for a stream outside the table
            // (unsupported type): positionally valid, not
            // timeable — kept out of the seek table.
            dropped_unknown_stream = dropped_unknown_stream.saturating_add(1);
        }
    }
    // The coalesced per-reason damage: one event each, the byte total
    // carrying the entry count (16 bytes per dropped entry).
    if dropped_outside_movi > 0 {
        damage.note(dropped_outside_movi * 16, "idx1-entry-outside-movi");
    }
    if dropped_unknown_stream > 0 {
        damage.note(dropped_unknown_stream * 16, "idx1-entry-unknown-stream");
    }
    if dropped_not_packet > 0 {
        damage.note(dropped_not_packet * 16, "idx1-entry-not-a-packet");
    }
    if out_of_order > 0 {
        damage.note(out_of_order * 16, "idx1-out-of-order-entries");
    }
    if entries.is_empty() {
        Ok(None)
    } else {
        Ok(Some(IndexTable { entries }))
    }
}

/// Probe whether the first packet entry's `dwOffset` is movi-relative
/// (the spec default) or an absolute file position (the legacy
/// variant). A candidate base wins when the position it implies sits
/// inside the `movi` data region **and** the chunk id actually stored
/// at that position matches the entry's fourcc — the same
/// disambiguation FFmpeg performs. Neither candidate matching keeps
/// the spec-relative default (the entries then drop as
/// outside-movi damage, exactly as before).
fn probe_offset_base(
    reader: &mut SourceReader,
    entry_fourcc: &riff::FourCC,
    rel: u32,
    size: u32,
    movi_form_pos: u64,
    movi_data_start: u64,
    movi_data_end: u64,
) -> u64 {
    let mut candidate_matches = |pos: u64| -> bool {
        if pos < movi_data_start {
            return false;
        }
        let end = pos.saturating_add(8 + u64::from(size));
        if end > movi_data_end {
            return false;
        }
        match reader.read_exact_or_end_at(8, pos) {
            Ok(Some(bytes)) => {
                riff::FourCC::new([bytes[0], bytes[1], bytes[2], bytes[3]]) == *entry_fourcc
            }
            _ => false,
        }
    };
    let relative = movi_form_pos.saturating_add(u64::from(rel));
    if candidate_matches(relative) {
        return movi_form_pos;
    }
    let absolute = u64::from(rel);
    if candidate_matches(absolute) {
        return 0;
    }
    movi_form_pos
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
    let state = scan_movi_with(
        reader,
        streams,
        stream,
        movi_data_start,
        movi_data_end,
        target_ticks,
        None,
    )?;
    Ok(state.landing)
}

/// The bounded-scan budget (doc 03 §6 step 4's `SlowLinear` cap).
#[derive(Debug, Clone, Copy)]
pub struct ScanCap {
    /// Maximum packet chunks parsed before the scan gives up.
    pub max_packets: u64,
    /// Maximum container bytes walked before the scan gives up.
    pub max_bytes: u64,
}

/// Doc 03 §6's `[PROPOSAL: cap at 1000 packets or 2 MiB of parse
/// work]` — the engine-side seek flow's budget for the no-index path.
pub const SEEK_SCAN_CAP: ScanCap = ScanCap {
    max_packets: 1_000,
    max_bytes: 2 * 1024 * 1024,
};

/// The scan's full answer: the landing plus the per-stream ladder
/// state a from-start walk would hold just before it (the exact
/// re-anchor input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanLanding {
    /// The last chunk of `stream` with pts ≤ the target, if any.
    pub landing: Option<Landing>,
    /// Per-stream ladder ticks at the landing (empty when no landing).
    pub ladder_ticks: Vec<i64>,
}

/// The scan with a budget and ladder capture (card 3.A1's re-anchor
/// path): like [`scan_movi`], but counts parse work and refuses with
/// a typed too-slow error when `cap` is exceeded (doc 03 §6 step 4 —
/// `SeekFailed{TooSlow}`, the session still at its old position and
/// playable), and captures the pre-landing ladder state so the
/// re-anchored walk continues with exactly the timestamps a
/// from-start walk would produce.
///
/// # Errors
/// As [`scan_movi`], plus the typed [`error::scan_too_slow`] budget
/// refusal. The reader's position is restored on every path.
pub fn scan_movi_with(
    reader: &mut SourceReader,
    streams: &[StreamDesc],
    stream: u32,
    movi_data_start: u64,
    movi_data_end: u64,
    target_ticks: i64,
    cap: Option<ScanCap>,
) -> Result<ScanLanding, EngineError> {
    let saved = reader.position();
    let outcome = scan_movi_inner(
        reader,
        streams,
        stream,
        movi_data_start,
        movi_data_end,
        target_ticks,
        cap,
    );
    let _ = reader.seek_to(saved);
    outcome
}

/// Inner scan (position restored by the wrapper).
#[allow(clippy::too_many_arguments)]
fn scan_movi_inner(
    reader: &mut SourceReader,
    streams: &[StreamDesc],
    stream: u32,
    mut pos: u64,
    end: u64,
    target_ticks: i64,
    cap: Option<ScanCap>,
) -> Result<ScanLanding, EngineError> {
    let mut ladders = Ladders::new(streams);
    let mut best: Option<Landing> = None;
    let mut best_ticks: Vec<i64> = Vec::new();
    let mut ends: Vec<u64> = Vec::new();
    let length = reader.length();
    let mut packets: u64 = 0;
    let mut bytes: u64 = 0;

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
        let next = pos
            .saturating_add(8)
            .saturating_add(u64::from(header.size))
            .saturating_add(riff::pad_for(u64::from(header.size)));
        bytes = bytes.saturating_add(8 + u64::from(header.size));
        let is_packet = matches!(
            classify_movi_chunk(&header.fourcc),
            MoviChunkKind::Packet { .. }
        );
        if is_packet {
            packets = packets.saturating_add(1);
        }
        // The budget is spent only when the scan must walk *past* it
        // (finding the landing exactly at the cap is success).
        if let Some(cap) = cap
            && (packets > cap.max_packets || bytes > cap.max_bytes)
        {
            return Err(error::scan_too_slow(cap.max_bytes, bytes));
        }
        match classify_movi_chunk(&header.fourcc) {
            MoviChunkKind::Packet { stream: s } => {
                if s == stream {
                    // Capture the pre-advance state: the landing chunk
                    // must re-emit its recorded pts after re-anchoring.
                    let snapshot = ladders.ticks_snapshot();
                    if let Some(step) = ladders.advance(s, u64::from(header.size)) {
                        if step.pts_ticks > target_ticks {
                            // Past the target on the seek stream: the
                            // scan's answer is already final.
                            break;
                        }
                        best = Some(Landing {
                            chunk_pos: pos,
                            pts_ticks: step.pts_ticks,
                        });
                        best_ticks = snapshot;
                    }
                } else {
                    let _ = ladders.advance(s, u64::from(header.size));
                }
                pos = next;
            }
            MoviChunkKind::RecList => {
                if ends.len() >= super::MAX_LIST_DEPTH {
                    return Err(error::bad_structure(pos));
                }
                // Bound parity with the walk (D-3): a `rec ` list
                // whose declared end crosses the parent bound is
                // clamped here too — the scan must agree with the walk
                // about where a corrupted rec ends.
                let end = (pos + 8 + u64::from(header.size)).min(bound);
                ends.push(end);
                pos += 8 + 4;
            }
            MoviChunkKind::Other => {
                pos = next;
            }
        }
    }
    Ok(ScanLanding {
        landing: best,
        ladder_ticks: best_ticks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avi::header::StreamFormat;
    use crate::avi::test_support;
    use media_types::ids::MediaKind;

    /// A one-stream video table for the parse-level fixtures (the same
    /// shape the builder tests use: 30 fps, one tick per chunk).
    fn video_stream_desc() -> StreamDesc {
        StreamDesc {
            ordinal: 0,
            kind: MediaKind::Video,
            handler: riff::FourCC::tag(b"MJPG"),
            scale: 1,
            rate: 30,
            start: 0,
            length: 3,
            sample_size: 0,
            format: StreamFormat::MotionJpeg {
                width: 320,
                height: 240,
            },
            title: None,
            strh_offset: 0,
        }
    }

    /// The regression for the out-of-order `idx1` fix (M16 family 1):
    /// entries stored out of file order must come out FILE-ordered with
    /// FILE-order timing (the same table the ordered equivalent would
    /// parse to), and the reorder is damage-counted, never silent.
    /// Pre-fix, the ladder ran in stored order — chunk C recorded pts 0
    /// and in-range seeks mis-landed or refused (`search_index`'s
    /// monotonicity precondition silently violated).
    #[test]
    fn out_of_order_idx1_parses_to_the_file_ordered_table() {
        // Layout: `movi` form at 0 (the dwOffset base), three 1-byte
        // video chunks at 4/14/24, then an idx1 storing them in the
        // order C, A, B.
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(b"movi");
        for label in *b"ABC" {
            bytes.extend_from_slice(b"00dc");
            bytes.extend_from_slice(&1u32.to_le_bytes());
            bytes.push(label);
            bytes.push(0); // WORD pad
        }
        let idx1_pos = bytes.len() as u64;
        let mut idx = Vec::new();
        for rel in [24u32, 4, 14] {
            idx.extend_from_slice(b"00dc");
            idx.extend_from_slice(&0x10u32.to_le_bytes());
            idx.extend_from_slice(&rel.to_le_bytes());
            idx.extend_from_slice(&1u32.to_le_bytes());
        }
        bytes.extend_from_slice(b"idx1");
        bytes.extend_from_slice(&(idx.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&idx);

        let stream = match test_support::mem_stream(bytes.clone()) {
            Ok(stream) => stream,
            Err(err) => unreachable!("mem stream: {err}"),
        };
        let mut reader = match SourceReader::new(stream) {
            Ok(reader) => reader,
            Err(err) => unreachable!("reader: {err}"),
        };
        let streams = [video_stream_desc()];
        let mut damage = DamageLog::default();
        let table = match parse_idx1(
            &mut reader,
            idx1_pos,
            idx.len() as u32,
            0,  // movi_form_pos (the dwOffset base)
            4,  // movi_data_start
            34, // movi_data_end
            bytes.len() as u64,
            &streams,
            &mut damage,
        ) {
            Ok(Some(table)) => table,
            Ok(None) => unreachable!("three valid entries parse"),
            Err(err) => unreachable!("out-of-order entries are salvage, not an error: {err}"),
        };
        // File order, file-order timing: A@4 pts 0, B@14 pts 1, C@24
        // pts 2 — exactly what the ordered table parses to.
        assert_eq!(
            table
                .entries
                .iter()
                .map(|entry| (entry.chunk_pos, entry.pts_ticks))
                .collect::<Vec<_>>(),
            vec![(4, 0), (14, 1), (24, 2)],
            "out-of-order entries are time-corrected by file order"
        );
        // The search still lands correctly on the corrected table.
        let landing = match search_index(&table, 0, 1) {
            Some(landing) => landing,
            None => unreachable!("in-range target lands"),
        };
        assert_eq!(landing.pts_ticks, 1);
        assert_eq!(landing.chunk_pos, 14);
        // And the reorder is observable: both A and B were stored
        // after C though they precede it in the file — 2 misplaced
        // entries, 16 bytes each.
        assert!(
            damage
                .events
                .iter()
                .any(|(skipped, reason)| *reason == "idx1-out-of-order-entries" && *skipped == 32),
            "damage events: {:?}",
            damage.events
        );
    }

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
        // Other streams answer from their own entries only (target 3
        // lands on stream 1's only entry; target 0 is before it).
        assert_eq!(search_index(&table, 1, 3).map(|l| l.pts_ticks), Some(3));
        assert_eq!(search_index(&table, 1, 0).map(|l| l.pts_ticks), None);
        // Before the first keyframe → None.
        assert!(search_index(&table, 0, -1).is_none());
    }
}
