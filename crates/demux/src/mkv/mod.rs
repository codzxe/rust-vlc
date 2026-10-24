//! The Matroska / WebM demuxer (card 5.D1 — native self-build, C5:
//! std-only, EBML is pure byte work).
//!
//! Layered like the crate's other natives:
//!
//! - [`ebml`] — vints, element headers, the bounded positional
//!   reader, typed value reads (uint/int/float/utf8).
//! - [`header`] — EBML header / `Info` / `Tracks` parsing.
//! - [`cluster`] — `Cluster`/`SimpleBlock`/`BlockGroup` + the three
//!   lacing codings (Xiph, EBML, fixed).
//! - [`cues`] — the `Cues` seek index.
//! - [`error`] — typed constructors (`demux.mkv.*`).
//!
//! # WebM payload set (recorded honestly, card AC)
//!
//! WebM is Matroska with `DocType webm` and a restricted payload set.
//! This demuxer passes every track through with its raw CodecID
//! exposed; **decode** availability is the codec registry's truth: at
//! the time of this card the registry decodes **Vorbis, Opus (audio)
//! and MJPEG/H.264-in-avcC (video)** — **VP8/VP9 and AV1 WebM video
//! are demuxed (packets + timestamps + seek) but NOT decodable**
//! (typed `Unsupported` at the decode seam, never a silent skip at
//! demux). VP9 decode is out of this card's scope by assignment.
//!
//! # Memory model (5.X2 notes)
//!
//! Cluster payloads are walked in bounded windows
//! ([`MAX_CLUSTER_WINDOW`]); packet bytes land in a 2-slot
//! [`PayloadSlots`] pool ([`MAX_PACKET_BYTES`] ceiling per slot,
//! larger payloads spill unpooled but stay block-bounded by
//! [`MAX_BLOCK_BYTES`]). Per-packet steady-state allocations are
//! zero for the common case.

pub mod attachments;
pub mod chapters;
pub mod cluster;
pub mod cues;
pub mod ebml;
pub mod editions;
pub mod error;
pub mod fuzz;
pub mod header;
pub mod tags;

use std::collections::HashMap;
use std::collections::VecDeque;

use media_types::ids::{GenerationId, TrackId};
use media_types::time::{Duration, Nanoseconds, Stamp, TimeBase, TimeSpan, Timestamp};
use media_types::{EngineError, Packet, Track};
use plugin_api::demux::{DemuxEvent, Demuxer, SeekSupport};

use crate::avi::riff::PayloadSlots;
use crate::mkv::cluster::ParsedBlock;
use crate::mkv::cues::CueIndex;
use crate::mkv::ebml::{EbmlReader, ElementHeader};
use crate::mkv::header::{SegmentInfo, TrackEntry};

/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["mkv", "mka", "mk3d", "webm"];

/// The cluster-walk window (one bounded read per window; huge or
/// unknown-size clusters walk in consecutive windows).
pub const MAX_CLUSTER_WINDOW: u64 = 4 * 1024 * 1024;

/// Per-packet pool ceiling; larger frames spill unpooled (block-
/// bounded by [`MAX_BLOCK_BYTES`]).
pub const MAX_PACKET_BYTES: usize = 4 * 1024 * 1024;

/// The single-block ceiling (a declared block payload larger than
/// this is the typed structural refusal — bounded memory, MP §12).
pub const MAX_BLOCK_BYTES: u64 = 32 * 1024 * 1024;

/// The no-cues linear seek scan budget (bytes of cluster headers
/// walked before the honest `TooSlow` refusal).
pub const MAX_SEEK_SCAN_BYTES: u64 = 512 * 1024 * 1024;

/// The probe: the leading bytes are the EBML-header element id
/// `0x1A45DFA3` (the registry sniffs the id; DocType policy is
/// `start`'s typed verdict).
#[must_use]
pub fn probe(header: &[u8]) -> bool {
    header.len() >= 4
        && u32::from_be_bytes([header[0], header[1], header[2], header[3]]) == ebml::ID_EBML_HEADER
}

/// One loaded cluster window.
struct ClusterWindow {
    /// The window bytes.
    data: Vec<u8>,
    /// The absolute position of `data[0]`.
    base: u64,
    /// The parse cursor (absolute).
    cursor: u64,
    /// The cluster's effective end (absolute).
    end: u64,
    /// The cluster's absolute base timecode (scale ticks).
    timecode: i64,
}

/// The Matroska/WebM demuxer.
pub struct MkvDemuxer {
    reader: EbmlReader,
    started: bool,
    eos: bool,
    /// The accepted DocType (`matroska` or `webm`).
    doctype: String,
    info: SegmentInfo,
    base: TimeBase,
    tracks: Vec<Track>,
    entries: Vec<TrackEntry>,
    /// Matroska track number → stream ordinal.
    by_number: HashMap<u64, u32>,
    cues: Option<CueIndex>,
    /// Absolute position of the `Segment` payload (cue offsets are
    /// relative to it).
    segment_data_start: u64,
    segment_end: u64,
    /// The current top-level walk position (next element to load).
    walk: u64,
    window: Option<ClusterWindow>,
    pending: VecDeque<DemuxEvent>,
    /// Damage bytes skipped in the current cluster (one `Recovered`
    /// per cluster with damage).
    cluster_damage: u64,
    damage_total: u64,
    pool: PayloadSlots,
}

impl MkvDemuxer {
    /// Construct over a seekable, length-known source (no container
    /// I/O — `start` does the parsing).
    ///
    /// # Errors
    /// Typed `Unsupported` (stage `Open`) for unknown-length,
    /// unseekable or empty sources.
    pub fn new(stream: Box<dyn plugin_api::SourceStream>) -> Result<MkvDemuxer, EngineError> {
        let reader = EbmlReader::new(stream)?;
        Ok(MkvDemuxer {
            reader,
            started: false,
            eos: false,
            doctype: String::new(),
            info: SegmentInfo::default(),
            base: TimeBase::new(1, 1)?,
            tracks: Vec::new(),
            entries: Vec::new(),
            by_number: HashMap::new(),
            cues: None,
            segment_data_start: 0,
            segment_end: 0,
            walk: 0,
            window: None,
            pending: VecDeque::new(),
            cluster_damage: 0,
            damage_total: 0,
            pool: PayloadSlots::new(2, MAX_PACKET_BYTES),
        })
    }

    /// The accepted DocType (`matroska` or `webm`).
    #[must_use]
    pub fn doctype(&self) -> &str {
        &self.doctype
    }

    /// The segment facts (scale, duration).
    #[must_use]
    pub fn segment_info(&self) -> &SegmentInfo {
        &self.info
    }

    /// The parsed track table (diagnostics/tests + the R3 registry's
    /// CodecID exposure seam).
    #[must_use]
    pub fn track_entries(&self) -> &[TrackEntry] {
        &self.entries
    }

    /// The cue index when the segment carries one (diagnostics/tests).
    #[must_use]
    pub fn cue_index(&self) -> Option<&CueIndex> {
        self.cues.as_ref()
    }

    /// Total damage bytes skipped by recovery (honesty counter).
    #[must_use]
    pub fn damage_bytes(&self) -> u64 {
        self.damage_total
    }

    /// The live pool footprint (the 5.X2 observable).
    #[must_use]
    pub fn pool_bytes(&self) -> usize {
        self.pool.pool_bytes()
    }

    /// `start`'s top-level walk: parse the EBML header, find the
    /// `Segment`, walk its children (headers only — payload reads
    /// happen in the targeted parsers), collect Info/Tracks/Cues and
    /// the first cluster position.
    fn open(&mut self) -> Result<(), EngineError> {
        let length = self.reader.length();
        let (ebml_header, header_end) = header::parse_ebml_header(&mut self.reader, length)?;
        match ebml_header.doctype.as_str() {
            "matroska" | "webm" => {}
            other => return Err(error::unknown_doctype(other)),
        }
        self.doctype = ebml_header.doctype.clone();
        // Find the Segment (the first top-level element; anything
        // else between is skipped — SeekHead policy).
        let mut pos = header_end;
        let mut segment: Option<ElementHeader> = None;
        while pos < length {
            let Some(el) = ebml::parse_header_at(&mut self.reader, pos, length)? else {
                break;
            };
            if el.id == ebml::ID_SEGMENT {
                segment = Some(el);
                break;
            }
            let end = el
                .declared_end()
                .ok_or_else(|| error::bad_structure(el.data_pos))?;
            if end > length {
                return Err(error::truncated(end, length));
            }
            pos = end;
        }
        let Some(segment) = segment else {
            return Err(error::not_matroska(&[]));
        };
        self.segment_data_start = segment.data_pos;
        // Unknown-size Segment: extends to the stream end (the
        // streaming-EBML case).
        self.segment_end = segment.declared_end().unwrap_or(length).min(length);
        let mut first_cluster: Option<u64> = None;
        let mut child_pos = segment.data_pos;
        while child_pos < self.segment_end {
            let Some(child) = ebml::parse_header_at(&mut self.reader, child_pos, self.segment_end)?
            else {
                break;
            };
            let child_end = match child.declared_end() {
                Some(end) => end.min(self.segment_end),
                None => {
                    // Unknown-size child inside the Segment: the
                    // streaming form — only a Cluster may legally
                    // carry it in this scope; treat it as extending
                    // to the segment end.
                    if child.id == ebml::ID_CLUSTER && first_cluster.is_none() {
                        first_cluster = Some(child_pos);
                    }
                    break;
                }
            };
            match child.id {
                ebml::ID_INFO => {
                    self.info = header::parse_info(&mut self.reader, &child, self.segment_end)?;
                }
                ebml::ID_TRACKS => {
                    let (entries, damage) =
                        header::parse_tracks(&mut self.reader, &child, self.segment_end)?;
                    self.entries = entries;
                    self.damage_total += damage.iter().map(|(n, _)| *n).sum::<u64>();
                }
                ebml::ID_CUES => {
                    self.cues = Some(cues::parse_cues(
                        &mut self.reader,
                        &child,
                        self.segment_end,
                    )?);
                }
                ebml::ID_CLUSTER => {
                    if first_cluster.is_none() {
                        first_cluster = Some(child_pos);
                    }
                    // Optimization: keep walking top-level headers
                    // only until Cues is found (it usually trails the
                    // clusters); once both it and a cluster are known
                    // the walk can stop.
                    if self.cues.is_some() {
                        break;
                    }
                }
                _ => {} // SeekHead/Tags/Attachments/Chapters: skip.
            }
            child_pos = child_end;
        }
        let Some(first_cluster) = first_cluster else {
            return Err(error::no_clusters());
        };
        if self.entries.is_empty() {
            return Err(error::no_tracks());
        }
        self.base = self.info.time_base()?;
        self.by_number = self
            .entries
            .iter()
            .map(|e| (e.number, e.ordinal))
            .collect::<HashMap<_, _>>();
        self.tracks = self
            .entries
            .iter()
            .map(|e| e.track(self.base))
            .collect::<Result<Vec<_>, _>>()?;
        self.walk = first_cluster;
        Ok(())
    }

    /// Load the next cluster (or top-level element) at `self.walk`
    /// into a window. `Ok(false)` = the segment is exhausted.
    fn load_next(&mut self) -> Result<bool, EngineError> {
        loop {
            if self.walk >= self.segment_end {
                return Ok(false);
            }
            let Some(el) = ebml::parse_header_at(&mut self.reader, self.walk, self.segment_end)?
            else {
                return Ok(false);
            };
            match el.id {
                ebml::ID_CLUSTER => {
                    let effective_end = el
                        .declared_end()
                        .unwrap_or(self.segment_end)
                        .min(self.segment_end);
                    let timecode =
                        cluster::parse_timecode(&mut self.reader, &el, self.segment_end)?;
                    let window_end =
                        effective_end.min(self.walk.saturating_add(MAX_CLUSTER_WINDOW));
                    let data = self.reader.read_prefix_at(
                        (window_end - el.data_pos) as usize,
                        el.data_pos,
                        window_end,
                    )?;
                    // Past the Timecode element: find where the
                    // timecode child ended so blocks start there.
                    let block_start = self.block_start_after_timecode(&el, &data)?;
                    self.window = Some(ClusterWindow {
                        data,
                        base: el.data_pos,
                        cursor: block_start,
                        end: effective_end,
                        timecode: timecode.ticks,
                    });
                    return Ok(true);
                }
                _ => {
                    // A non-cluster top-level element (Cues trailing
                    // clusters, Tags, Attachments…): skip it.
                    let end = el
                        .declared_end()
                        .ok_or_else(|| error::bad_structure(el.data_pos))?
                        .min(self.segment_end);
                    self.walk = end;
                }
            }
        }
    }

    /// Where blocks start inside a just-loaded window: the byte
    /// offset just past the `Timecode` element (blocks may legally
    /// precede it in malformed files; the spec mandates Timecode
    /// first — files that violate that lose those leading blocks to
    /// the skip, damage-counted).
    fn block_start_after_timecode(
        &mut self,
        el: &ElementHeader,
        data: &[u8],
    ) -> Result<u64, EngineError> {
        let mut at = 0usize;
        while at < data.len() {
            let Some((id, size, hdr_len, unknown)) = parse_header_mem(&data[at..]) else {
                break;
            };
            if unknown {
                return Err(error::bad_structure(el.data_pos + at as u64));
            }
            if id == ebml::ID_TIMECODE {
                return Ok(el.data_pos + at as u64 + hdr_len as u64 + size);
            }
            at += hdr_len + size as usize;
        }
        // No Timecode found in the window: the walk starts at the
        // cluster payload head (the typed timecode parse already
        // refused a missing one).
        Ok(el.data_pos)
    }

    /// Parse one in-memory element header. `None` = not enough bytes
    /// or bad grammar (the caller decides which).
    fn next_block_packets(&mut self) -> Result<Option<Vec<Packet>>, EngineError> {
        // Take the window (owned) so pool fills and reader seeks below
        // do not fight the borrow; every exit path restores it.
        let Some(mut window) = self.window.take() else {
            return Ok(None);
        };
        let outcome = loop {
            // Slide the window when the cursor reaches its end but
            // not the cluster's.
            if window.cursor >= window.base + window.data.len() as u64 && window.cursor < window.end
            {
                let from = window.cursor;
                let to = window.end.min(from + MAX_CLUSTER_WINDOW);
                let data = self.reader.read_prefix_at((to - from) as usize, from, to)?;
                window = ClusterWindow {
                    data,
                    base: from,
                    cursor: from,
                    end: window.end,
                    timecode: window.timecode,
                };
                continue;
            }
            if window.cursor >= window.end || window.data.is_empty() {
                break Ok(None);
            }
            let offset = (window.cursor - window.base) as usize;
            if offset >= window.data.len() {
                break Ok(None);
            }
            let Some((id, size, hdr_len, unknown)) = parse_header_mem(&window.data[offset..])
            else {
                // Trailing partial header: the cluster's declared end
                // crosses the stream — typed truncation.
                break Err(error::truncated(
                    window.cursor + 12,
                    window.base + window.data.len() as u64,
                ));
            };
            if unknown {
                // Unknown-size inside a cluster: not legal (the
                // streaming form is a Segment/Cluster property).
                break Err(error::bad_structure(window.cursor));
            }
            let data_start = window.cursor + hdr_len as u64;
            let data_end = data_start + size;
            if data_end > window.end {
                // The block claims bytes past the cluster: contained
                // lie → typed truncation at the cluster end.
                break Err(error::truncated(data_end, window.end));
            }
            if size > MAX_BLOCK_BYTES {
                break Err(error::bad_structure(data_start));
            }
            if data_end > window.base + window.data.len() as u64 {
                // Spans past the window but inside the cluster: slide
                // so the block's head is at the window start.
                let from = window.cursor;
                let to = window.end.min(from + MAX_CLUSTER_WINDOW);
                let data = self.reader.read_prefix_at((to - from) as usize, from, to)?;
                window = ClusterWindow {
                    data,
                    base: from,
                    cursor: from,
                    end: window.end,
                    timecode: window.timecode,
                };
                continue;
            }
            let rel = offset + hdr_len;
            let payload_end = rel + size as usize;
            let timecode = window.timecode;
            let payload = window.data[rel..payload_end].to_vec();
            let packets = self.packets_from_element(id, &payload, data_start, timecode)?;
            window.cursor = data_end;
            if let Some(packets) = packets {
                if packets.is_empty() {
                    continue;
                }
                break Ok(Some(packets));
            }
        };
        // Every exit restores the window (the recovery path needs the
        // failure position to skip the damaged remainder).
        self.window = Some(window);
        outcome
    }

    /// Turn one cluster child element (SimpleBlock or BlockGroup)
    /// into packets.
    fn packets_from_element(
        &mut self,
        id: u32,
        payload: &[u8],
        payload_offset: u64,
        cluster_timecode: i64,
    ) -> Result<Option<Vec<Packet>>, EngineError> {
        match id {
            ebml::ID_SIMPLE_BLOCK => {
                let block = cluster::parse_block(payload, payload_offset, true)?;
                Ok(Some(self.emit_block(&block, payload, cluster_timecode)?))
            }
            ebml::ID_BLOCK_GROUP => {
                // Walk BlockGroup children: Block (+ flags from
                // ReferenceBlock presence), BlockDuration.
                let mut packets = Vec::new();
                let mut at = 0usize;
                while at < payload.len() {
                    let Some((cid, csize, chdr_len, cunknown)) = parse_header_mem(&payload[at..])
                    else {
                        break;
                    };
                    if cunknown {
                        return Err(error::bad_structure(payload_offset + at as u64));
                    }
                    let cstart = at + chdr_len;
                    let cend = cstart + csize as usize;
                    if cend > payload.len() {
                        return Err(error::truncated(
                            payload_offset + cend as u64,
                            payload_offset + payload.len() as u64,
                        ));
                    }
                    if cid == ebml::ID_BLOCK {
                        let block = cluster::parse_block(
                            &payload[cstart..cend],
                            payload_offset + cstart as u64,
                            false,
                        )?;
                        packets =
                            self.emit_block(&block, &payload[cstart..cend], cluster_timecode)?;
                    }
                    // BlockDuration/ReferenceBlock: the keyframe
                    // derivation is ReferenceBlock-absence; duration
                    // rides DefaultDuration.
                    at = cend;
                }
                Ok(Some(packets))
            }
            _ => Ok(None), // Timecode (parsed already) / unknown: skip.
        }
    }

    /// Emit one block's frames as packets.
    fn emit_block(
        &mut self,
        block: &ParsedBlock,
        payload: &[u8],
        cluster_timecode: i64,
    ) -> Result<Vec<Packet>, EngineError> {
        let Some(&ordinal) = self.by_number.get(&block.track) else {
            // Unknown track number: damage-skip the whole block
            // (never a fabricated track).
            self.cluster_damage += block.payload_len as u64;
            return Ok(Vec::new());
        };
        let entry = &self.entries[ordinal as usize];
        let absolute =
            cluster::absolute_ticks(cluster_timecode, block.relative, block.payload_offset)?;
        // Laced frames advance by the track's DefaultDuration when
        // declared (the spec's cadence rule); without it every frame
        // carries the block stamp (recorded honest absence).
        let step_ticks: i64 = entry
            .private
            .default_duration_ns
            .and_then(|ns| {
                // A zero scale is the typed honest absence (checked
                // division refuses it rather than panicking).
                let scale = u64::from(self.info.timestamp_scale);
                let ticks = ns.checked_div(scale)?;
                i64::try_from(ticks).ok()
            })
            .unwrap_or(0);
        let duration = entry
            .private
            .default_duration_ns
            .map(|ns| Duration::Known(Nanoseconds(ns)));
        let mut packets = Vec::with_capacity(block.frames.len());
        for (i, frame) in block.frames.iter().enumerate() {
            if frame.len > MAX_PACKET_BYTES * 4 {
                // Beyond every sane ceiling: contained refusal.
                self.cluster_damage += frame.len as u64;
                continue;
            }
            let at = block.payload_offset + (block.body_offset + frame.start) as u64;
            let data = self.pool.fill(frame.len, |buf| {
                let got = self
                    .reader
                    .read_prefix_at(buf.len(), at, at + buf.len() as u64)?;
                buf.copy_from_slice(&got);
                Ok(got.len())
            })?;
            // BlockGroup keyframes: no ReferenceBlock (derive at the
            // group level; SimpleBlock carries the flag bit).
            let pts = absolute
                .checked_add(step_ticks.saturating_mul(i as i64))
                .ok_or_else(|| error::timecode_overflow(block.payload_offset))?;
            packets.push(Packet {
                track: TrackId::new(ordinal),
                pts: Stamp::Present(Timestamp::new(pts, self.base)),
                dts: Stamp::Missing,
                duration,
                data,
                keyframe: block.keyframe,
                discontinuity: false,
                generation: GenerationId::FIRST,
            });
        }
        // The pool read moved the source position; the window walk
        // re-seeks per read, so no repair is needed (every window /
        // header read is positional).
        let _ = payload;
        Ok(packets)
    }

    /// The keyframe-safe landing for `target` (scale ticks): the cue
    /// point at-or-before, or the bounded cluster scan without cues.
    /// Returns `(landing_ticks, absolute_position)`.
    fn landing(&mut self, target_ticks: i64) -> Result<(i64, u64), EngineError> {
        if let Some(index) = self.cues.as_ref() {
            if let Some(point) = index.landing_before(target_ticks) {
                let pos = self
                    .segment_data_start
                    .checked_add(point.cluster_pos)
                    .filter(|p| *p < self.segment_end)
                    .ok_or_else(error::seek_out_of_range)?;
                return Ok((point.time_ticks, pos));
            }
            return Err(error::seek_out_of_range());
        }
        // No cues: the bounded linear cluster scan (header reads
        // only, budget-bounded).
        let save = self.walk;
        let mut pos = self.segment_data_start;
        let mut best: Option<(i64, u64)> = None;
        let mut scanned = 0u64;
        let result = loop {
            if pos >= self.segment_end {
                break Ok(best);
            }
            let Some(el) = ebml::parse_header_at(&mut self.reader, pos, self.segment_end)? else {
                break Ok(best);
            };
            scanned += 12;
            if scanned > MAX_SEEK_SCAN_BYTES {
                break Err(error::scan_too_slow(MAX_SEEK_SCAN_BYTES, scanned));
            }
            let end = match el.declared_end() {
                Some(end) => end.min(self.segment_end),
                None => self.segment_end,
            };
            if el.id == ebml::ID_CLUSTER {
                match cluster::parse_timecode(&mut self.reader, &el, self.segment_end) {
                    Ok(tc) if tc.ticks <= target_ticks => best = Some((tc.ticks, pos)),
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
            pos = end;
        };
        self.walk = save;
        result.and_then(|best| best.ok_or_else(error::seek_out_of_range))
    }

    /// A target span's start converted into this segment's scale
    /// ticks.
    fn target_ticks(&self, target: &TimeSpan) -> Result<i64, EngineError> {
        match &target.start {
            Stamp::Present(ts) => ts
                .convert(&self.base)
                .map(|t| t.ticks)
                .map_err(|_| error::seek_out_of_range()),
            Stamp::Missing => Ok(0),
        }
    }
}

/// Parse an element header from the head of `data` (the in-memory
/// twin of [`ebml::parse_header_at`]). `None` = grammar failure or
/// not enough bytes.
fn parse_header_mem(data: &[u8]) -> Option<(u32, u64, usize, bool)> {
    let id = ebml::parse_vint_id(data).ok()?;
    let size = ebml::parse_vint_size(&data[id.len..]).ok()?;
    Some((id.id, size.value, id.len + size.len, size.unknown))
}

impl Demuxer for MkvDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if !self.started {
            self.open()?;
            self.started = true;
        }
        Ok(self.tracks.clone())
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
        if self.eos {
            return Ok(DemuxEvent::EndOfStream);
        }
        if let Some(event) = self.pending.pop_front() {
            return Ok(event);
        }
        loop {
            if self.window.is_none() && !self.load_next()? {
                self.eos = true;
                if self.cluster_damage > 0 {
                    self.damage_total += self.cluster_damage;
                    self.cluster_damage = 0;
                    return Ok(DemuxEvent::Recovered {
                        skipped: self.damage_total,
                        reason: "unknown-track-or-oversize-blocks",
                    });
                }
                return Ok(DemuxEvent::EndOfStream);
            }
            match self.next_block_packets() {
                Ok(Some(packets)) => {
                    if packets.is_empty() {
                        continue;
                    }
                    self.pending
                        .extend(packets.into_iter().map(DemuxEvent::Packet));
                    return Ok(self.pending.pop_front().unwrap_or(DemuxEvent::EndOfStream));
                }
                Ok(None) => {
                    // Cluster exhausted: advance the top-level walk
                    // past it (the window's end IS the cluster's
                    // effective end), flush its damage note, load
                    // the next element.
                    if let Some(window) = self.window.take() {
                        self.walk = window.end;
                    }
                    if self.cluster_damage > 0 {
                        self.damage_total += self.cluster_damage;
                        self.cluster_damage = 0;
                        return Ok(DemuxEvent::Recovered {
                            skipped: self.damage_total,
                            reason: "unknown-track-or-oversize-blocks",
                        });
                    }
                    self.window = None;
                }
                Err(e) => {
                    // A structurally failed block inside a declared
                    // cluster: skip the rest of the cluster (the
                    // contained-lie recovery) and continue at the
                    // next top-level element — typed error only when
                    // the *stream* itself is truncated.
                    if matches!(
                        e.kind,
                        media_types::ErrorKind::Corrupt {
                            detail: media_types::error::CorruptDetail::Truncated { .. }
                        }
                    ) && self.reader.length() <= self.segment_end
                    {
                        return Err(e);
                    }
                    let mut skipped = 0u64;
                    if let Some(window) = self.window.take() {
                        skipped = window.end.saturating_sub(window.cursor);
                        self.walk = window.end;
                    }
                    self.damage_total += skipped;
                    return Ok(DemuxEvent::Recovered {
                        skipped: self.damage_total,
                        reason: "cluster-parse-recovery",
                    });
                }
            }
        }
    }

    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
        let ticks = self.target_ticks(&target)?;
        let (landing, _pos) = self.landing(ticks)?;
        Ok(TimeSpan {
            start: Stamp::Present(Timestamp::new(landing, self.base)),
            end: target.end,
        })
    }

    fn seekability(&self) -> SeekSupport {
        if !self.started {
            return SeekSupport::None;
        }
        if self.cues.is_some() {
            SeekSupport::Fast { indexed: true }
        } else {
            SeekSupport::SlowLinear
        }
    }

    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
        let ticks = self.target_ticks(&target)?;
        // Compute…
        let (landing, pos) = self.landing(ticks)?;
        // …then commit (D4: nothing above moved the walk).
        self.walk = pos;
        self.window = None;
        self.pending.clear();
        self.cluster_damage = 0;
        self.eos = false;
        self.pending.push_back(DemuxEvent::Discontinuity);
        Ok(TimeSpan {
            start: Stamp::Present(Timestamp::new(landing, self.base)),
            end: target.end,
        })
    }
}
