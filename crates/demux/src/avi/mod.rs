//! The AVI 1.0 demuxer (card 2.B2; doc 03 §4.3 + §7 first vertical,
//! D-018 variant A). RIFF chunk walk, `hdrl`/`strl` track discovery,
//! `idx1` seek table, MJPEG + PCM packet extraction, seek-to-time.
//!
//! # What this is
//!
//! [`AviDemuxer`] implements the landed
//! [`plugin_api::Demuxer`](plugin_api::Demuxer) contract over any
//! seekable [`SourceStream`](plugin_api::SourceStream) (the `input`
//! crate's file access is the first vertical's byte source). The
//! byte layout it parses is exactly the one the in-repo fixture
//! writer emits (`tools/xtask/src/cmd/fixtures.rs`) — the generated
//! corpus and its expectation ledgers are the conformance truth.
//!
//! # Design (recorded for review)
//!
//! - **The `movi` walk is the packet-extraction truth.** Packets are
//!   extracted by walking `LIST movi` chunk by chunk — index or no
//!   index. The `idx1` is the *seek table* (keyframe flags + the
//!   binary-searched time→position mapping); when it is absent
//!   (the `noidx0` class-B fixture), demux is unaffected and seek
//!   falls back to a position-restoring linear scan.
//! - **Declared sizes are hints, recognition is authority.** Every
//!   declared extent is clamped to its parent's and the stream's, and
//!   the parse stops recognizing a list at the first chunk that does
//!   not belong there, handing the position back to the parent walk
//!   (counted as damage — doc 03 §4.3(d) bounded recovery). This is
//!   what survives a corrupted `hdrl` size without losing the `movi`.
//! - **Truncation is a typed error at the boundary where data runs
//!   out, never a silent EOF.** A cut inside a payload errors with the
//!   chunk's needed end; a `movi` clamped before its declared end
//!   errors with the declared end when the walk reaches the clamp;
//!   the *index* is the one place truncation is non-fatal (salvage
//!   complete entries, count the partial one — the walk is intact).
//! - **Seek is a pure query** (the landed trait's wording: "does not
//!   move the read position — the engine decides whether to act").
//!   `seek(target)` maps target time → keyframe-safe position ≤
//!   target on the video stream (first stream for audio-only files)
//!   and reports where the stream would resume. The re-anchor hook
//!   that moves the cursor is card 3.A1's seek flow.
//! - **Timing is the AVI ladder, not per-chunk stamps.** AVI chunks
//!   carry no timestamps; presentation time is computed from each
//!   stream's `strh` scale/rate and per-chunk sample arithmetic —
//!   format-canonical, not invented (invariant D1: stamps are present
//!   by construction).
//! - **Keyframe flags are format-truth** (invariant D2): `idx1`'s
//!   `AVIIF_KEYFRAME` when the entry exists; codec grammar otherwise
//!   (v0's filter keeps only MJPEG and PCM, both all-keyframe).
//!
//! # v0 scope (documented limits)
//!
//! - AVI 1.0 only: no OpenDML (`indx`/`odml`, >2 GiB) — `idx1` only.
//! - Codec filter: `vids`+`MJPG` and `auds`+PCM (integer + IEEE
//!   float). Other stream types are skipped, damage-counted.
//! - No stream switching, no `rec `-specific semantics beyond
//!   transparent descent.

use std::collections::VecDeque;

use media_types::ids::{GenerationId, TrackId};
use media_types::packet::SharedBytes;
use media_types::time::{Duration, Nanoseconds, Stamp, TimeSpan, Timestamp};
use media_types::{EngineError, ErrorContext, ErrorKind, Packet, PipelineStage, Track};
use plugin_api::{DemuxEvent, Demuxer};

use super::error;
use crate::avi::header::{
    parse_hdrl, AviMainHeader, DamageLog, StreamDesc, StreamFormat,
};
use crate::avi::index::{self, IndexTable, Ladders};
use crate::avi::riff::{
    self, classify_movi_chunk, peek_chunk_header, ChunkHeader, FourCC, MoviChunkKind,
    SourceReader, FORM_AVI, FORM_HDRL, FORM_MOVI, LIST,
};

/// Maximum `movi` chunk payload trusted into memory
/// `[PROPOSAL: 32 MiB — MJPEG frames are ~KBs; anything larger is
/// corruption and is positionally skipped with damage, not read]`.
pub const MAX_CHUNK_PAYLOAD: u64 = 32 * 1024 * 1024;

/// Maximum `movi` list nesting (`rec ` lists inside `rec ` lists …)
/// `[PROPOSAL: 8 — the AVI 1.0 spec nests one level (`rec `); deeper
/// is corruption]`.
pub const MAX_LIST_DEPTH: usize = 8;

/// The `movi` region, absolute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MoviRegion {
    /// Position of the `movi` form FourCC (the `idx1` `dwOffset` base).
    form_pos: u64,
    /// First chunk position (right after the form FourCC).
    data_start: u64,
    /// The **declared** data end (the list extent, unclamped).
    declared_end: u64,
    /// The effective walk end (declared ∩ RIFF extent ∩ stream).
    effective_end: u64,
}

/// A replayable error shape: stage + kind + context (all the cloneable
/// parts of an [`EngineError`]). The demuxer's own errors carry no
/// source chain; a propagated source error keeps its chain on the
/// first return and replays by shape afterwards (documented in
/// [`AviDemuxer::next_event`]).
#[derive(Debug, Clone, PartialEq, Eq)]
struct StickyShape {
    stage: PipelineStage,
    kind: ErrorKind,
    context: ErrorContext,
}

impl StickyShape {
    fn of(err: &EngineError) -> StickyShape {
        StickyShape {
            stage: err.stage,
            kind: err.kind.clone(),
            context: err.context.clone(),
        }
    }

    fn mint(&self) -> EngineError {
        EngineError::new(self.stage, self.kind.clone(), self.context.clone())
    }
}

/// The `movi` walk cursor.
struct WalkCursor {
    pos: u64,
    /// Open `rec ` list ends (innermost last).
    ends: Vec<u64>,
    /// Per-stream presentation ladders.
    ladders: Ladders,
    /// Position into the index's file-ordered entries (keyframe
    /// lookup without re-scanning).
    index_cursor: usize,
}

/// The AVI 1.0 demuxer.
///
/// Construct with [`AviDemuxer::new`] over a seekable source stream,
/// then drive it through the [`Demuxer`] trait the engine owns:
/// `start` parses the header and enumerates tracks; `next_event`
/// yields one packet per `movi` chunk (plus `Recovered`/`EndOfStream`
/// signals); `seek` maps a target time onto a keyframe-safe position.
///
/// # Example
///
/// ```
/// use demux::avi::AviDemuxer;
/// use plugin_api::{Demuxer, DemuxEvent};
/// use media_types::time::{Stamp, TimeSpan, Timestamp};
///
/// # fn main() -> Result<(), media_types::EngineError> {
/// // Any seekable SourceStream; the fixture writer's output parsed
/// # let bytes = avi_demo_bytes();
/// let mut stream = demux::avi::test_support::mem_stream(bytes);
/// let mut avi = AviDemuxer::new(stream)?;
/// let tracks = avi.start()?;
/// assert_eq!(tracks.len(), 2);
/// match avi.next_event()? {
///     DemuxEvent::Packet(p) => assert!(p.keyframe),
///     _ => unreachable!("first movi chunk is a packet"),
/// }
/// # Ok(())
/// # }
/// # fn avi_demo_bytes() -> std::sync::Arc<[u8]> {
/// #   // A minimal two-stream AVI built the same way the fixture
/// #   // writer builds it (see tests for the real corpus).
/// #   let mut b = Vec::new();
/// #   b.extend_from_slice(b"RIFF");
/// #   b.extend_from_slice(&0u32.to_le_bytes());
/// #   b.extend_from_slice(b"AVI ");
/// #   // hdrl: avih + strl(video MJPG) + strl(audio PCM)
/// #   let hdrl_start = b.len();
/// #   b.extend_from_slice(b"LIST");
/// #   b.extend_from_slice(&0u32.to_le_bytes());
/// #   b.extend_from_slice(b"hdrl");
/// #   let mut avih = Vec::new();
/// #   avih.extend_from_slice(&33_333u32.to_le_bytes());
/// #   avih.extend_from_slice(&0u32.to_le_bytes());
/// #   avih.extend_from_slice(&0u32.to_le_bytes());
/// #   avih.extend_from_slice(&0x110u32.to_le_bytes());
/// #   avih.extend_from_slice(&1u32.to_le_bytes());
/// #   avih.extend_from_slice(&0u32.to_le_bytes());
/// #   avih.extend_from_slice(&2u32.to_le_bytes());
/// #   avih.extend_from_slice(&4u32.to_le_bytes());
/// #   avih.extend_from_slice(&320u32.to_le_bytes());
/// #   avih.extend_from_slice(&240u32.to_le_bytes());
/// #   avih.extend(&[0u8; 16]);
/// #   b.extend_from_slice(b"avih");
/// #   b.extend_from_slice(&(avih.len() as u32).to_le_bytes());
/// #   b.extend_from_slice(&avih);
/// #   for (fcc, handler, scale, rate, length, ssize, strf) in [
/// #     ([b'v',b'i',b'd',b's'], [b'M',b'J',b'P',b'G'], 1u32, 30u32, 1u32, 0u32, {
/// #        let mut v = Vec::new();
/// #        v.extend_from_slice(&40u32.to_le_bytes());
/// #        v.extend_from_slice(&320u32.to_le_bytes());
/// #        v.extend_from_slice(&240u32.to_le_bytes());
/// #        v.extend_from_slice(&1u16.to_le_bytes());
/// #        v.extend_from_slice(&24u16.to_le_bytes());
/// #        v.extend_from_slice(b"MJPG");
/// #        v.extend(&vec![0u8; 16]);
/// #        v
/// #     }),
/// #     ([b'a',b'u',b'd',b's'], [0u8; 4], 1u32, 44_100u32, 44_100u32, 4u32, {
/// #        let mut a = Vec::new();
/// #        a.extend_from_slice(&1u16.to_le_bytes());
/// #        a.extend_from_slice(&2u16.to_le_bytes());
/// #        a.extend_from_slice(&44_100u32.to_le_bytes());
/// #        a.extend_from_slice(&176_400u32.to_le_bytes());
/// #        a.extend_from_slice(&4u16.to_le_bytes());
/// #        a.extend_from_slice(&16u16.to_le_bytes());
/// #        a
/// #     }),
/// #   ] {
/// #     b.extend_from_slice(b"LIST");
/// #     let strl_size_pos = b.len();
/// #     b.extend_from_slice(&0u32.to_le_bytes());
/// #     b.extend_from_slice(b"strl");
/// #     let mut strh = Vec::new();
/// #     strh.extend_from_slice(&fcc);
/// #     strh.extend_from_slice(&handler);
/// #     strh.extend(&[0u8; 12]);
/// #     strh.extend_from_slice(&scale.to_le_bytes());
/// #     strh.extend_from_slice(&rate.to_le_bytes());
/// #     strh.extend_from_slice(&0u32.to_le_bytes());
/// #     strh.extend_from_slice(&length.to_le_bytes());
/// #     strh.extend_from_slice(&4u32.to_le_bytes());
/// #     strh.extend_from_slice(&u32::MAX.to_le_bytes());
/// #     strh.extend_from_slice(&ssize.to_le_bytes());
/// #     strh.extend(&[0u8; 8]);
/// #     b.extend_from_slice(b"strh");
/// #     b.extend_from_slice(&(strh.len() as u32).to_le_bytes());
/// #     b.extend_from_slice(&strh);
/// #     b.extend_from_slice(b"strf");
/// #     b.extend_from_slice(&(strf.len() as u32).to_le_bytes());
/// #     b.extend_from_slice(&strf);
/// #     let size = b.len() - strl_size_pos - 4;
/// #     b[strl_size_pos..strl_size_pos + 4].copy_from_slice(&(size as u32).to_le_bytes());
/// #   }
/// #   let hdrl_size = b.len() - hdrl_start - 4;
/// #   b[hdrl_start + 4..hdrl_start + 8].copy_from_slice(&(hdrl_size as u32).to_le_bytes());
/// #   // movi: one video chunk + one audio chunk
/// #   let movi_start = b.len();
/// #   b.extend_from_slice(b"LIST");
/// #   let movi_size_pos = b.len();
/// #   b.extend_from_slice(&0u32.to_le_bytes());
/// #   b.extend_from_slice(b"movi");
/// #   for (tag, data) in [(&b"00dc"[..], &b"jpeg-bytes"[..]), (&b"01wb"[..], &b"pcm!"[..])] {
/// #     b.extend_from_slice(tag);
/// #     b.extend_from_slice(&(data.len() as u32).to_le_bytes());
/// #     b.extend_from_slice(data);
/// #     if data.len() % 2 == 1 { b.push(0); }
/// #   }
/// #   let movi_size = b.len() - movi_size_pos - 4;
/// #   b[movi_size_pos..movi_size_pos + 4].copy_from_slice(&(movi_size as u32).to_le_bytes());
/// #   let riff_size = b.len() - 8;
/// #   b[4..8].copy_from_slice(&(riff_size as u32).to_le_bytes());
/// #   std::sync::Arc::from(b)
/// # }
/// ```
pub struct AviDemuxer {
    reader: SourceReader,
    started: bool,
    eos: bool,
    sticky: Option<StickyShape>,
    main: AviMainHeader,
    streams: Vec<StreamDesc>,
    movi: Option<MoviRegion>,
    index: Option<IndexTable>,
    cursor: Option<WalkCursor>,
    pending: VecDeque<DemuxEvent>,
    damage: DamageLog,
    tracks: Option<Vec<Track>>,
    /// The seek-primary stream ordinal (video if present, else first).
    seek_stream: Option<u32>,
}

impl AviDemuxer {
    /// Construct over a seekable, length-known source stream.
    ///
    /// No container I/O happens here (the trait's `start` does the
    /// parsing) — this only rejects sources that cannot carry an AVI
    /// 1.0 file: unknown length, unseekable, empty.
    ///
    /// # Errors
    /// Typed `Unsupported` (stage `Open`) for those three cases.
    pub fn new(stream: Box<dyn plugin_api::SourceStream>) -> Result<AviDemuxer, EngineError> {
        let reader = SourceReader::new(stream)?;
        Ok(AviDemuxer {
            reader,
            started: false,
            eos: false,
            sticky: None,
            main: header::zeroed_main_header(),
            streams: Vec::new(),
            movi: None,
            index: None,
            cursor: None,
            pending: VecDeque::new(),
            damage: DamageLog::default(),
            tracks: None,
            seek_stream: None,
        })
    }

    /// The parsed `avih` (advisory fields; `start` must have run).
    pub fn main_header(&self) -> AviMainHeader {
        self.main
    }

    /// The `strf` format facts for a track — the decoder-config seam
    /// for cards 2.B4/2.B5 (the L0 `Track` deliberately carries no
    /// codec parameters).
    pub fn stream_format(&self, track: TrackId) -> Option<StreamFormat> {
        self.streams
            .iter()
            .find(|s| s.ordinal == track.handle())
            .map(|s| s.format)
    }

    /// Total bytes/blocks skipped by bounded recovery so far (the
    /// `Recovered` events' sum; observable without driving the trait).
    pub fn damage_total(&self) -> u64 {
        self.damage.total
    }

    /// Record a fatal error: sticky + returned.
    fn fail(&mut self, err: EngineError) -> EngineError {
        self.sticky = Some(StickyShape::of(&err));
        err
    }

    /// Queue the damage log's events for `next_event` to surface.
    fn surface_damage(&mut self) {
        for (skipped, reason) in std::mem::take(&mut self.damage.events) {
            self.pending.push_back(DemuxEvent::Recovered { skipped, reason });
        }
    }

    /// One `movi` walk step (the packet-extraction core).
    ///
    /// Returns the next event, or `None` when the walk reached its
    /// end and the caller should report `EndOfStream`.
    fn step_movi(&mut self) -> Result<Option<DemuxEvent>, EngineError> {
        let Some(region) = self.movi else {
            // `start` guarantees a movi; defensive shape for the
            // impossible case.
            return Err(self.fail(error::no_movi()));
        };
        let cursor = self
            .cursor
            .as_mut()
            .ok_or_else(|| error::no_movi())
            .map_err(|e| self.fail(e))?;

        loop {
            let bound = cursor.ends.last().copied().unwrap_or(region.effective_end);
            if cursor.pos + 8 > bound {
                if let Some(end) = cursor.ends.pop() {
                    cursor.pos = cursor.pos.max(end);
                    continue;
                }
                if region.effective_end < region.declared_end {
                    // The movi was clamped (RIFF extent ∩ stream) but
                    // declared more: the container is truncated, and
                    // the walk says so instead of a silent empty EOF.
                    return Err(self.fail(error::truncated(
                        region.declared_end,
                        region.effective_end,
                    )));
                }
                self.eos = true;
                return Ok(None);
            }

            let pos = cursor.pos;
            let header = match peek_chunk_header(&mut self.reader, pos) {
                Ok(header) => header,
                Err(err) => return Err(self.fail(err)),
            };
            let next = pos
                .saturating_add(8)
                .saturating_add(u64::from(header.size))
                .saturating_add(riff::pad_for(u64::from(header.size)));

            match classify_movi_chunk(&header.fourcc) {
                MoviChunkKind::Packet { stream } => {
                    let Some(desc) = self.streams.iter().find(|s| s.ordinal == stream) else {
                        // A packet chunk of an unsupported/unknown
                        // stream: skipped silently here — the parse-time
                        // skip already counted it.
                        cursor.pos = next;
                        continue;
                    };
                    let size = u64::from(header.size);
                    if size > MAX_CHUNK_PAYLOAD {
                        self.damage
                            .note(size, "chunk-payload-over-ceiling");
                        self.surface_damage();
                        cursor.pos = next;
                        continue;
                    }
                    let payload =
                        match self.reader.read_exact_or_end_at(size as usize, pos + 8) {
                            Ok(Some(bytes)) => bytes,
                            Ok(None) => {
                                return Err(self.fail(error::truncated(
                                    pos + 8 + size,
                                    self.reader.length(),
                                )))
                            }
                            Err(err) => return Err(self.fail(err)),
                        };
                    let Some(step) = cursor.ladders.advance(stream, size) else {
                        cursor.pos = next;
                        continue;
                    };
                    if step.unaligned {
                        let stray = size % u64::from(desc.sample_size.max(1));
                        self.damage.note(stray, "pcm-chunk-not-sample-aligned");
                        self.surface_damage();
                    }
                    let base = match desc.time_base(0) {
                        Ok(base) => base,
                        Err(err) => return Err(self.fail(err)),
                    };
                    let keyframe = self.keyframe_for(pos, stream);
                    let pts = Timestamp::new(step.pts_ticks, base);
                    let (delta, _) = desc.ticks_per_chunk(size);
                    let duration = match Timestamp::new(delta, base).as_nanos() {
                        Ok(Nanoseconds(nanos)) => Some(Duration::Known(Nanoseconds(nanos))),
                        Err(_) => {
                            self.damage.note(0, "packet-duration-unrepresentable");
                            self.surface_damage();
                            None
                        }
                    };
                    let packet = Packet {
                        track: TrackId::new(stream),
                        pts: Stamp::Present(pts),
                        // AVI 1.0 has no decode-order separation for
                        // MJPEG (no B-frames) — DTS mirrors PTS.
                        dts: Stamp::Present(pts),
                        duration,
                        data: SharedBytes::from_vec(payload),
                        keyframe,
                        discontinuity: false,
                        // The engine's demux driver re-stamps the
                        // live generation at push (doc 03 §5 checking
                        // point 3); the demuxer's stamp is
                        // authoritative for nothing.
                        generation: GenerationId::FIRST,
                    };
                    cursor.pos = next;
                    return Ok(Some(DemuxEvent::Packet(packet)));
                }
                MoviChunkKind::RecList => {
                    let end = pos + 8 + u64::from(header.size);
                    if end > bound || cursor.ends.len() >= MAX_LIST_DEPTH {
                        return Err(self.fail(error::bad_structure(pos)));
                    }
                    cursor.ends.push(end);
                    cursor.pos = pos + 8 + 4;
                }
                MoviChunkKind::Other => {
                    self.damage.note(u64::from(header.size), "unknown-chunk-in-movi");
                    self.surface_damage();
                    cursor.pos = next;
                }
            }
        }
    }

    /// The keyframe truth for a chunk: `idx1`'s flag when an entry
    /// exists for the position, codec grammar otherwise (MJPEG and
    /// PCM are all-keyframe — invariant D2's grammar clause).
    fn keyframe_for(&mut self, chunk_pos: u64, stream: u32) -> bool {
        let Some(table) = &self.index else {
            return true;
        };
        let cursor = match self.cursor.as_mut() {
            Some(cursor) => cursor,
            None => return true,
        };
        while cursor.index_cursor < table.entries.len()
            && table.entries[cursor.index_cursor].chunk_pos < chunk_pos
        {
            cursor.index_cursor += 1;
        }
        match table.entries.get(cursor.index_cursor) {
            Some(entry) if entry.chunk_pos == chunk_pos && entry.stream == stream => entry.keyframe,
            _ => true,
        }
    }
}

impl Demuxer for AviDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if let Some(tracks) = &self.tracks {
            return Ok(tracks.clone());
        }
        if let Some(shape) = &self.sticky {
            return Err(shape.mint());
        }

        match self.parse_container() {
            Ok(tracks) => {
                self.tracks = Some(tracks.clone());
                self.surface_damage();
                Ok(tracks)
            }
            Err(err) => {
                self.sticky = Some(StickyShape::of(&err));
                Err(err)
            }
        }
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if let Some(shape) = &self.sticky {
            // Fatal errors are sticky: the demuxer stays in its failed
            // state (the walk cursor frozen at the damage site). The
            // first return carried the full source chain; replays
            // carry stage/kind/context.
            return Err(shape.mint());
        }
        if !self.started {
            return Err(self.fail(error::not_started()));
        }
        if let Some(event) = self.pending.pop_front() {
            return Ok(event);
        }
        if self.eos {
            return Ok(DemuxEvent::EndOfStream);
        }
        match self.step_movi() {
            Ok(Some(event)) => Ok(event),
            Ok(None) => Ok(DemuxEvent::EndOfStream),
            Err(err) => Err(err),
        }
    }

    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
        if let Some(shape) = &self.sticky {
            return Err(shape.mint());
        }
        let Some(ordinal) = self.seek_stream else {
            return Err(error::seek_out_of_range());
        };
        let Some(desc) = self.streams.iter().find(|s| s.ordinal == ordinal) else {
            return Err(error::seek_out_of_range());
        };
        let base = desc.time_base(0)?;
        let target_ts = target
            .start
            .timestamp()
            .map_err(|_| error::seek_out_of_range())?;
        if target_ts.ticks < 0 {
            return Err(error::seek_out_of_range());
        }
        let converted = target_ts
            .convert(&base)
            .map_err(|_| error::seek_out_of_range())?;
        if desc.length > 0 {
            let end_ticks = i64::from(desc.start) + i64::from(desc.length);
            if converted.ticks > end_ticks {
                return Err(error::seek_out_of_range());
            }
        }

        // Pure query: the index search never touches the source; the
        // no-index linear scan restores the read position before
        // returning (the landed trait's "does not move the read
        // position").
        let landing = match &self.index {
            Some(table) => index::search_index(table, ordinal, converted.ticks),
            None => {
                let Some(region) = self.movi else {
                    return Err(error::seek_out_of_range());
                };
                index::scan_movi(
                    &mut self.reader,
                    &self.streams,
                    ordinal,
                    region.data_start,
                    region.effective_end,
                    converted.ticks,
                )?
            }
        };
        let Some(landing) = landing else {
            return Err(error::seek_out_of_range());
        };
        let end = if desc.length == 0 {
            Stamp::Missing
        } else {
            Stamp::Present(Timestamp::new(
                i64::from(desc.start) + i64::from(desc.length),
                base,
            ))
        };
        Ok(TimeSpan {
            start: Stamp::Present(Timestamp::new(landing.pts_ticks, base)),
            end,
        })
    }
}

impl AviDemuxer {
    /// The full header parse (the body of the trait's `start`).
    fn parse_container(&mut self) -> Result<Vec<Track>, EngineError> {
        // Top-of-file: `RIFF <size> AVI `.
        let head = self
            .reader
            .read_exact_or_end_at(12, 0)?
            .ok_or_else(|| error::not_avi(&[]))?;
        if &head[0..4] != RIFF.as_bytes() || &head[8..12] != FORM_AVI.as_bytes() {
            return Err(error::not_avi(&head));
        }
        let riff_size = riff::u32le(&head, 4);
        let riff_declared_end = 8u64.saturating_add(u64::from(riff_size));
        let bound = riff_declared_end.min(self.reader.length());
        if riff_declared_end > bound {
            self.damage
                .note(riff_declared_end - bound, "riff-extends-past-stream");
        }

        // Top-level walk: header peeks only (payloads skipped
        // positionally) — hdrl, movi, idx1 wherever they sit.
        let mut pos = 12u64;
        let mut saw_hdrl = false;
        while pos + 8 <= bound {
            let header = peek_chunk_header(&mut self.reader, pos)?;
            let next = pos
                .saturating_add(8)
                .saturating_add(u64::from(header.size))
                .saturating_add(riff::pad_for(u64::from(header.size)));
            if header.fourcc == LIST {
                let Some(form) = self.reader.read_exact_or_end_at(4, pos + 8)? else {
                    return Err(error::truncated(pos + 12, self.reader.length()));
                };
                let form = FourCC::new([form[0], form[1], form[2], form[3]]);
                match form {
                    FORM_HDRL => {
                        let declared_end = pos + 8 + u64::from(header.size);
                        let hdr = parse_hdrl(
                            &mut self.reader,
                            pos,
                            declared_end,
                            bound,
                            &mut self.damage,
                        )?;
                        self.main = hdr.main;
                        self.streams = hdr.streams;
                        saw_hdrl = true;
                        // Recognition, not the declared size, decides
                        // where the walk resumes (a lying hdrl size
                        // must not swallow the movi).
                        pos = hdr.resume_at;
                    }
                    FORM_MOVI => {
                        if self.movi.is_some() {
                            self.damage.note(u64::from(header.size), "duplicate-movi");
                            pos = next;
                        } else {
                            let declared_end = pos + 8 + u64::from(header.size);
                            let effective_end = declared_end.min(bound);
                            if declared_end > effective_end {
                                self.damage
                                    .note(declared_end - effective_end, "movi-extends-past-riff");
                            }
                            self.movi = Some(MoviRegion {
                                form_pos: pos + 8,
                                data_start: pos + 8 + 4,
                                declared_end,
                                effective_end,
                            });
                            pos = effective_end;
                        }
                    }
                    _ => {
                        pos = next;
                    }
                }
            } else if header.fourcc.as_bytes() == b"idx1" {
                if self.index.is_some() {
                    self.damage.note(u64::from(header.size), "duplicate-idx1");
                } else if let Some(region) = self.movi {
                    let parsed = index::parse_idx1(
                        &mut self.reader,
                        pos,
                        header.size,
                        region.form_pos,
                        region.data_start,
                        region.declared_end,
                        bound,
                        &self.streams,
                        &mut self.damage,
                    )?;
                    if parsed.is_some() {
                        let mut table = parsed.unwrap_or_else(|| unreachable!("checked"));
                        table.entries.sort_by_key(|entry| entry.chunk_pos);
                        self.index = Some(table);
                    }
                } else {
                    // An idx1 before any movi: keep it only if a movi
                    // shows up later — simplest honest shape is to
                    // skip it (damage) and let a later idx1 win.
                    self.damage.note(u64::from(header.size), "idx1-before-movi");
                }
                pos = next;
            } else {
                // JUNK and unknown top-level chunks: skipped.
                pos = next;
            }
        }

        if !saw_hdrl {
            if riff_declared_end > self.reader.length() {
                // The file promised an AVI and delivered a prefix —
                // truncation is the honest flavor, not "not an AVI".
                return Err(error::truncated(riff_declared_end, self.reader.length()));
            }
            return Err(error::no_header());
        }
        let Some(region) = self.movi else {
            return Err(error::no_movi());
        };
        if self.streams.is_empty() {
            return Err(error::no_supported_streams());
        }

        self.seek_stream = self
            .streams
            .iter()
            .find(|s| s.kind == media_types::ids::MediaKind::Video)
            .or_else(|| self.streams.first())
            .map(|s| s.ordinal);
        let tracks: Result<Vec<Track>, EngineError> =
            self.streams.iter().map(|s| s.track(s.strh_offset)).collect();
        let tracks = tracks?;
        self.cursor = Some(WalkCursor {
            pos: region.data_start,
            ends: Vec::new(),
            ladders: Ladders::new(&self.streams),
            index_cursor: 0,
        });
        self.started = true;
        Ok(tracks)
    }
}

/// The `idx1` top-level chunk id as bytes (pattern-match shape).
mod chunk_ids {
    /// `idx1`.
    pub const IDX1: &[u8; 4] = b"idx1";
}

// Re-exported for the module's own match arms and for tests.
use chunk_ids::IDX1;

/// Unused-import guards for items referenced only in docs/tests.
#[allow(unused_imports)]
use header as header_mod;

#[cfg(test)]
mod tests;
