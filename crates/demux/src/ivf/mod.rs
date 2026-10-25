//! The IVF demuxer (card 5.I3b): the AV1/VP9 delivery container —
//! `DKIF` header, fourcc codec tag, length-prefixed frame walk.
//!
//! # What this is
//!
//! [`IvfDemuxer`] implements the landed
//! [`Demuxer`](plugin_api::Demuxer) contract over any seekable
//! [`SourceStream`](plugin_api::SourceStream). IVF is the minimal
//! frame wrapper libvpx coined for VP8/VP9 and that AV1 (AOM) adopted
//! for raw delivery: one 32-byte file header
//! (magic `DKIF`, version, header length, fourcc, width, height, a
//! rational frame rate, a declared frame count), then a flat run of
//! frames — each a 12-byte header (4-byte payload size, 64-bit pts)
//! followed by exactly that many compressed payload bytes. No chunks,
//! no index, no interleaving: one video track, frames in presentation
//! order.
//!
//! # Timing (the honest contract)
//!
//! The header's rate/scale pair (`u32` at offsets 16/20 — libvpx's
//! `rate`/`scale`, which ffmpeg's muxer writes as its stream
//! time_base den/num, verified empirically: ffprobe reads the
//! generated 30 fps fixture back as `time_base = 1/30`) defines the
//! tick: **one tick = scale/rate seconds**, and every frame's 64-bit
//! pts counts those ticks. The track's
//! [`TimeBase`](media_types::time::TimeBase) is exactly that ratio, so
//! packet pts are the container's own numbers, unrescaled and
//! unrenumbered. Each packet carries the header-declared frame
//! duration (one tick) — ffmpeg's own convention (verified: ffprobe
//! reports `duration = 1` on every packet of the generated fixtures)
//! and a *declared* value, not an invention. A VFR file whose pts
//! ladder skips is passed through faithfully; the gaps are the
//! container's data, not this demuxer's to smooth.
//!
//! # Codec honesty (the no-silent-decode rule)
//!
//! The fourcc must be `AV01` (descriptor `av1`) or `VP90`
//! (descriptor `vp9`) — anything else is the typed
//! [`unsupported_fourcc`](error::unsupported_fourcc) refusal at
//! `start`, never a silently dropped or relabeled track. Payloads are
//! **passed through byte-for-byte**: AV1 temporal-unit OBUs and VP9
//! superframes alike — no parse, no reframe, no sequence-header
//! extraction (that is decode domain, cards 5.F2+/5.F3+). The codecs
//! crate has no AV1/VP9 decoder at v0, so any decode of these tracks
//! fails as a *typed* unsupported-codec error at the codec registry —
//! the demuxer's job is to make that refusal honest, which is why
//! the descriptors are exact and the payloads untouched.
//!
//! The container declares **no keyframe bit** (a frame's intra-ness
//! lives in the bitstream), so every packet carries
//! `keyframe = false` — never invented. ffprobe's `K_` flag on the
//! first packet of an ffmpeg-written file is parser-derived
//! (decode-domain), not container bytes; flags are therefore excluded
//! from the differential's compared columns, as the recorded honesty
//! note. Likewise `dts` is [`Stamp::Missing`]: IVF orders frames by
//! presentation time and carries no decode stamps.
//!
//! # Damage policy
//!
//! IVF has no sync words to resynchronize on — the walk is strictly
//! positional — so damage is binomial: a trailing fragment shorter
//! than a frame header (ffmpeg: clean EOF) is a bounded
//! [`Recovered`](plugin_api::DemuxEvent::Recovered) skip and a clean
//! end; a frame whose declared size runs past end-of-stream is the
//! sticky typed truncation (ffmpeg instead emits a short final
//! packet — the recorded divergence, the ADTS precedent); a size
//! above the payload ceiling is the typed [`ResourceLimit`] refusal
//! before any allocation is trusted (MP §12).
//!
//! # Seek
//!
//! No index exists, but frame headers are 12 bytes and positional:
//! [`seek`](Demuxer::seek)/[`reanchor`](Demuxer::reanchor) run a
//! bounded header-only scan (`SlowLinear`, the ADTS precedent — the
//! scan never reads payload bytes). "Keyframe-safe" cannot be
//! container-derived here (no keyframe bit); the landing is the last
//! frame whose pts ≤ the target, and the engine's decoder-side
//! recovery owns the rest.
//!
//! # Scope (v0, this card)
//!
//! Delivery of AV1 (`AV01`) and VP9 (`VP90`) only. ffmpeg's demuxer
//! maps many more fourccs (VP80, H264, AV01 variants like `AV1A`,
//! …); each of those is the typed refusal above, listed in the
//! differential table as deliberate v0 scope, not a parse failure.
//! The declared frame count is advisory metadata (exposed in
//! [`IvfStreamInfo`]); the walk is the truth — a lying count is
//! reported by comparing [`frames_demuxed`](IvfDemuxer::frames_demuxed)
//! against it, never trusted to bound reads.

use std::collections::VecDeque;

use media_types::ids::{CodecDescriptor, GenerationId, StreamId, TrackFlags, TrackId};
use media_types::time::{Duration, Nanoseconds, Stamp, TimeBase, TimeSpan, Timestamp, TrackTiming};
use media_types::{EngineError, MediaKind, Packet, Track};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use crate::avi::header::DamageLog;
use crate::avi::riff::{PayloadSlots, SourceReader};

pub mod error;
pub mod fuzz;
pub mod test_support;

#[cfg(test)]
mod tests;

/// The IVF file header's exact size — and the only header length this
/// demuxer accepts (both ffmpeg's `ivfenc` and libvpx's `vpxenc`
/// write 32; anything else is the typed `bad-header` refusal).
pub const HEADER_BYTES: u64 = 32;

/// One frame header: 4-byte payload size + 64-bit pts.
pub const FRAME_HEADER_BYTES: u64 = 12;

/// Payload sizes up to this are served from the packet pool (the
/// D-1 bounded-memory observable; the MKV precedent's per-slot cap).
pub const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

/// The hard typed ceiling on a declared frame size (a 32-bit size
/// field can claim 4 GiB — MP §12 refuses it before any allocation).
pub const FRAME_CEILING_BYTES: u64 = 16 * 1024 * 1024;

/// Maximum frame headers the seek scan walks before refusing (the
/// `SlowLinear` honesty bound, ADTS's figure).
pub const MAX_SEEK_SCAN_FRAMES: u64 = 2_000_000;

/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["ivf"];

/// The probe: do the leading bytes carry the `DKIF` magic?
#[must_use]
pub fn probe(header: &[u8]) -> bool {
    header.len() >= 4 && &header[0..4] == b"DKIF"
}

/// One parsed IVF file header. Public so later cards (the AV1/VP9
/// decoder seams) can read the declared delivery parameters without
/// re-parsing bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IvfHeader {
    /// The `version` field (advisory; ffmpeg ignores it, so do we —
    /// the fields below are positional and version-independent).
    pub version: u16,
    /// The declared header length (must be exactly 32 — see
    /// [`HEADER_BYTES`]).
    pub header_len: u16,
    /// The codec fourcc (`AV01`/`VP90` for supported files; carried
    /// verbatim for unsupported ones so the refusal can name it).
    pub fourcc: [u8; 4],
    /// Declared coded width.
    pub width: u16,
    /// Declared coded height.
    pub height: u16,
    /// The frame-rate numerator (offset 16): with `scale`, the tick
    /// ratio — one tick = `scale`/`rate` seconds.
    pub rate: u32,
    /// The frame-rate denominator (offset 20).
    pub scale: u32,
    /// The declared frame count (offset 24; advisory — the walk is
    /// the truth, some writers leave it 0).
    pub frame_count: u32,
}

impl IvfHeader {
    /// Parse and validate the 32-byte header. Typed errors for: bad
    /// magic (`not-ivf`), a short buffer (`truncated`), any header
    /// length other than 32, zero dims or a zero rate/scale field
    /// (`bad-header` — ffmpeg's own "Invalid frame rate" refusal
    /// matches). The fourcc is carried, not judged — see
    /// [`codec`](IvfHeader::codec).
    ///
    /// # Errors
    /// Typed `EngineError` (stage `Demux`) per the list above.
    pub fn parse(head: &[u8]) -> Result<IvfHeader, EngineError> {
        if head.len() < usize::try_from(HEADER_BYTES).unwrap_or(usize::MAX) {
            return Err(error::truncated(HEADER_BYTES, head.len() as u64));
        }
        if &head[0..4] != b"DKIF" {
            return Err(error::not_ivf(&head[0..16.min(head.len())]));
        }
        let le16 = |at: usize| u16::from_le_bytes([head[at], head[at + 1]]);
        let le32 =
            |at: usize| u32::from_le_bytes([head[at], head[at + 1], head[at + 2], head[at + 3]]);
        let header = IvfHeader {
            version: le16(4),
            header_len: le16(6),
            fourcc: [head[8], head[9], head[10], head[11]],
            width: le16(12),
            height: le16(14),
            rate: le32(16),
            scale: le32(20),
            frame_count: le32(24),
        };
        // offset 28..32 is the unused field — ignored by every reader.
        if header.header_len != HEADER_BYTES as u16 {
            return Err(error::bad_header(6));
        }
        if header.width == 0 || header.height == 0 || header.rate == 0 || header.scale == 0 {
            return Err(error::bad_header(12));
        }
        Ok(header)
    }

    /// The codec this fourcc names in the v0 delivery scope:
    /// `Some("av1")` for `AV01`, `Some("vp9")` for `VP90`, `None`
    /// for everything else (the typed refusal path).
    #[must_use]
    pub fn codec(&self) -> Option<&'static str> {
        match &self.fourcc {
            b"AV01" => Some("av1"),
            b"VP90" => Some("vp9"),
            _ => None,
        }
    }

    /// The registry codec descriptor for a supported fourcc (the
    /// honest `av1`/`vp9` passthrough names; `None` for the refusal
    /// path).
    #[must_use]
    pub fn descriptor(&self) -> Option<CodecDescriptor> {
        self.codec()
            .and_then(|name| CodecDescriptor::new(name).ok())
    }

    /// The track time base: one tick = `scale`/`rate` seconds (the
    /// container's own units — packet pts are its numbers).
    ///
    /// # Errors
    /// The L0 zero-denominator refusal (unreachable after `parse`
    /// validated `rate > 0`; kept typed for construction discipline).
    pub fn tick_base(&self) -> Result<TimeBase, EngineError> {
        TimeBase::new(self.scale, self.rate)
    }

    /// One tick in nanoseconds (the declared frame duration), `None`
    /// if the ratio overflows u64 (a hostile rate/scale pair).
    #[must_use]
    pub fn tick_nanos(&self) -> Option<u64> {
        u64::try_from(u128::from(self.scale) * 1_000_000_000 / u128::from(self.rate)).ok()
    }
}

/// Demuxer-side stream info (the fields ffprobe reports from its own
/// IVF parser — queryable here for the UI/engine without decoding).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IvfStreamInfo {
    /// The container name (always `"ivf"`).
    pub format: &'static str,
    /// The codec descriptor (`"av1"`/`"vp9"`).
    pub codec: &'static str,
    /// The bare fourcc tag.
    pub fourcc: [u8; 4],
    /// Declared coded width.
    pub width: u16,
    /// Declared coded height.
    pub height: u16,
    /// The tick-rate numerator.
    pub rate: u32,
    /// The tick-rate denominator.
    pub scale: u32,
    /// The declared frame count (advisory; compare
    /// [`frames_demuxed`](IvfDemuxer::frames_demuxed) for the truth).
    pub declared_frames: u32,
}

/// A replayable error shape (the WAV/Ogg/ADTS `StickyShape`
/// precedent): once the walk fails terminally, every later call
/// re-mints the same typed error instead of re-reading damage.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StickyShape {
    stage: media_types::PipelineStage,
    kind: media_types::ErrorKind,
    context: media_types::error::ErrorContext,
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

/// The IVF demuxer.
///
/// Construct with [`IvfDemuxer::new`] over a seekable, length-known
/// source stream; `start` parses the 32-byte header, gates the fourcc
/// and mints the single video track; `next_event` walks frames;
/// `seek`/`reanchor` are a bounded header-only `SlowLinear` scan.
pub struct IvfDemuxer {
    reader: SourceReader,
    started: bool,
    eos: bool,
    sticky: Option<StickyShape>,
    pending: VecDeque<DemuxEvent>,
    track: Option<Track>,
    /// The walk position: absolute byte offset of the next frame
    /// header (the header ends at 32; the walk is strictly
    /// positional — no resync exists in this format).
    walk: u64,
    frames_out: u64,
    /// The highest pts emitted so far (the duration walk's anchor;
    /// frames arrive in presentation order but a hostile file may
    /// regress, so the max, never the last).
    last_pts: i64,
    head: Option<IvfHeader>,
    pool: PayloadSlots,
    damage: DamageLog,
}

impl IvfDemuxer {
    /// Construct over a seekable, length-known source stream (no
    /// container I/O — `start` does the parsing).
    ///
    /// # Errors
    /// Typed `Unsupported` (stage `Open`) for unknown-length,
    /// unseekable or empty sources (the AVI/WAV/Ogg/ADTS precedent).
    pub fn new(stream: Box<dyn plugin_api::SourceStream>) -> Result<IvfDemuxer, EngineError> {
        let reader = SourceReader::new(stream)?;
        Ok(IvfDemuxer {
            reader,
            started: false,
            eos: false,
            sticky: None,
            pending: VecDeque::new(),
            track: None,
            walk: 0,
            frames_out: 0,
            last_pts: 0,
            head: None,
            pool: PayloadSlots::new(2, MAX_FRAME_BYTES),
            damage: DamageLog::default(),
        })
    }

    /// The parsed header. `None` before `start`.
    #[must_use]
    pub fn header(&self) -> Option<&IvfHeader> {
        self.head.as_ref()
    }

    /// Demuxer-side stream info (fourcc/dims/rate — the fields
    /// ffprobe reports; bitstream detail stays in the payloads).
    /// `None` before a successful `start`.
    #[must_use]
    pub fn stream_info(&self) -> Option<IvfStreamInfo> {
        let head = self.head.as_ref()?;
        let codec = head.codec()?;
        Some(IvfStreamInfo {
            format: "ivf",
            codec,
            fourcc: head.fourcc,
            width: head.width,
            height: head.height,
            rate: head.rate,
            scale: head.scale,
            declared_frames: head.frame_count,
        })
    }

    /// Data packets emitted so far (the count differential's truth).
    #[must_use]
    pub fn frames_demuxed(&self) -> u64 {
        self.frames_out
    }

    /// Total bytes skipped by bounded recovery (the `Recovered`
    /// events' sum).
    #[must_use]
    pub fn damage_total(&self) -> u64 {
        self.damage.total
    }

    /// The payload bytes resident in the packet pool (the D-1
    /// observable; bounded by `2 × MAX_FRAME_BYTES`, grown lazily to
    /// demand).
    #[must_use]
    pub fn pool_bytes(&self) -> usize {
        self.pool.pool_bytes()
    }

    /// The track's duration as walked so far: the last emitted frame's
    /// pts + one declared frame duration, in nanoseconds. `None`
    /// before the first packet (or if the tick ratio overflows).
    #[must_use]
    pub fn track_duration(&self) -> Option<Duration> {
        let head = self.head.as_ref()?;
        let tick = head.tick_nanos()?;
        // The duration is derived from the walk only when frames
        // exist; before that it is honestly absent.
        if self.frames_out == 0 {
            return None;
        }
        // last pts + 1 tick: the container's frames are in
        // presentation order, so this is the played-out extent.
        let ns = u128::from(self.last_pts.saturating_add(1).max(0) as u64)
            .saturating_mul(u128::from(tick));
        u64::try_from(ns)
            .ok()
            .map(|n| Duration::Known(Nanoseconds(n)))
    }

    // -- internals ---------------------------------------------------------

    fn fail(&mut self, err: EngineError) -> EngineError {
        self.sticky = Some(StickyShape::of(&err));
        err
    }

    /// Read the 12-byte frame header at `at` and return
    /// `(payload_size, pts)` — typed refusal when the pts does not fit
    /// the signed tick model.
    fn frame_header_at(&mut self, at: u64) -> Result<(u32, i64), EngineError> {
        let raw = self.reader.read_prefix_at(
            usize::try_from(FRAME_HEADER_BYTES).unwrap_or(usize::MAX),
            at,
        )?;
        let mut size = [0u8; 4];
        size.copy_from_slice(&raw[0..4]);
        let mut pts = [0u8; 8];
        pts.copy_from_slice(&raw[4..12]);
        let pts = match i64::try_from(u64::from_le_bytes(pts)) {
            Ok(t) => t,
            Err(_) => return Err(error::pts_overflow(at)),
        };
        Ok((u32::from_le_bytes(size), pts))
    }

    /// The walk step: one frame out of `self.walk`, or `false` at
    /// the container's honest end.
    fn step(&mut self) -> Result<bool, EngineError> {
        let len = self.reader.length();
        if self.walk >= len {
            return Ok(false);
        }
        if len - self.walk < FRAME_HEADER_BYTES {
            // A trailing fragment too short to be a frame header: a
            // bounded skip, then a clean end (ffmpeg's own walk ends
            // cleanly here; the Ogg/ADTS trailing-fragment precedent).
            let left = len - self.walk;
            self.damage.note(left, "ivf: trailing fragment");
            self.pending.push_back(DemuxEvent::Recovered {
                skipped: left,
                reason: "ivf: trailing fragment",
            });
            self.walk = len;
            return Ok(false);
        }
        let (size, pts) = self.frame_header_at(self.walk)?;
        if u64::from(size) > FRAME_CEILING_BYTES {
            // Declared size above every sane delivery frame: the MP
            // §12 ceiling refuses it before an allocation is trusted.
            return Err(error::frame_too_large(FRAME_CEILING_BYTES, u64::from(size)));
        }
        let payload_at = self.walk + FRAME_HEADER_BYTES;
        let payload_end = payload_at + u64::from(size);
        if payload_end > len {
            // Declared frame overruns the stream: typed sticky
            // truncation (ffmpeg emits a short packet instead — the
            // honestly-recorded divergence, the ADTS precedent).
            return Err(error::frame_beyond_file(payload_end, len));
        }
        let data = self.pool.fill(size as usize, |buf| {
            self.reader.read_prefix_into(buf, payload_at)
        })?;
        let track = self
            .track
            .clone()
            .unwrap_or_else(|| unreachable!("start minted the track"));
        let base = track.timing.base;
        let duration = self
            .head
            .as_ref()
            .and_then(IvfHeader::tick_nanos)
            .map(|ns| Duration::Known(Nanoseconds(ns)));
        self.last_pts = self.last_pts.max(pts);
        self.frames_out += 1;
        let packet = Packet {
            track: track.id,
            pts: Stamp::Present(Timestamp::new(pts, base)),
            dts: Stamp::Missing, // IVF carries no decode stamps
            duration,
            data,
            keyframe: false, // the container declares no keyframe bit
            discontinuity: false,
            generation: GenerationId::FIRST,
        };
        self.pending.push_back(DemuxEvent::Packet(packet));
        self.walk = payload_end;
        Ok(true)
    }

    /// The seek core: a bounded, payload-free scan of frame headers
    /// from the data start, landing on the last frame whose pts ≤
    /// `target_ticks` (a target past the last frame lands on it —
    /// ffmpeg's own demuxer semantics; a target at/before the first
    /// frame clamps to it — nothing exists before the stream start).
    /// Returns `(pts, frame_header_offset)`.
    fn scan_landing(&mut self, target_ticks: i64) -> Result<(i64, u64), EngineError> {
        let len = self.reader.length();
        let mut at = HEADER_BYTES;
        let mut landing: Option<(i64, u64)> = None;
        let mut first: Option<(i64, u64)> = None;
        let mut frames = 0u64;
        while at + FRAME_HEADER_BYTES <= len {
            let (size, pts) = self.frame_header_at(at)?;
            frames += 1;
            if frames > MAX_SEEK_SCAN_FRAMES {
                return Err(error::seek_out_of_range());
            }
            if first.is_none() {
                first = Some((pts, at));
            }
            if pts <= target_ticks {
                landing = Some((pts, at));
            }
            at += FRAME_HEADER_BYTES + u64::from(size);
        }
        landing.or(first).ok_or_else(error::seek_out_of_range)
    }

    /// A seek target expressed in the track's own tick base (`None`
    /// for an absent/unknown stamp — the caller treats it as 0).
    fn target_ticks(target: TimeSpan, base: TimeBase) -> Option<i64> {
        let ts = target.start.timestamp().ok()?;
        ts.convert(&base).ok().map(|t| t.ticks)
    }

    /// The landing span for a successful seek (start at the landing
    /// pts, end honestly absent).
    fn landing_span(&self, pts: i64) -> Result<TimeSpan, EngineError> {
        let base = self
            .track
            .as_ref()
            .map(|t| t.timing.base)
            .unwrap_or_else(|| {
                TimeBase::new(1, 1000).unwrap_or_else(|_| unreachable!("constant base"))
            });
        Ok(TimeSpan {
            start: Stamp::Present(Timestamp::new(pts, base)),
            end: Stamp::Missing,
        })
    }
}

impl Demuxer for IvfDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if self.started {
            return Ok(vec![
                self.track
                    .clone()
                    .unwrap_or_else(|| unreachable!("started implies a track")),
            ]);
        }
        if let Some(shape) = &self.sticky {
            return Err(shape.mint());
        }
        let open = (|| -> Result<Track, EngineError> {
            let len = self.reader.length();
            if len < HEADER_BYTES {
                return Err(error::truncated(HEADER_BYTES, len));
            }
            let head_bytes = self
                .reader
                .read_prefix_at(usize::try_from(HEADER_BYTES).unwrap_or(usize::MAX), 0)?;
            let head = IvfHeader::parse(&head_bytes)?;
            let Some(descriptor) = head.descriptor() else {
                return Err(error::unsupported_fourcc(&head.fourcc));
            };
            if len <= HEADER_BYTES {
                // A whole file that is only a header: nothing to
                // demux (ffmpeg opens it and yields an empty stream —
                // the recorded divergence; this layer refuses loudly,
                // the ADIF no-data precedent).
                return Err(error::no_frames());
            }
            let base = head.tick_base()?;
            let track = Track {
                id: TrackId::new(0),
                stream: StreamId::new(0),
                kind: MediaKind::Video,
                language: None,
                codec: descriptor,
                title: None,
                flags: TrackFlags::NONE,
                timing: TrackTiming {
                    span: TimeSpan {
                        start: Stamp::Present(Timestamp::new(0, base)),
                        end: Stamp::Missing, // refined by the walk (track_duration)
                    },
                    base,
                    gaps: Vec::new(),
                },
            };
            self.head = Some(head);
            self.track = Some(track.clone());
            self.walk = HEADER_BYTES;
            Ok(track)
        })();
        match open {
            Ok(track) => {
                self.started = true;
                Ok(vec![track])
            }
            Err(err) => Err(self.fail(err)),
        }
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if let Some(shape) = &self.sticky {
            return Err(shape.mint());
        }
        if !self.started {
            return Err(self.fail(error::not_started()));
        }
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Ok(event);
            }
            if self.eos {
                return Ok(DemuxEvent::EndOfStream);
            }
            match self.step()? {
                true => continue,
                false => {
                    self.eos = true;
                    if let Some(event) = self.pending.pop_front() {
                        return Ok(event);
                    }
                    return Ok(DemuxEvent::EndOfStream);
                }
            }
        }
    }

    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        if let Some(shape) = &self.sticky {
            return Err(shape.mint());
        }
        if !self.started {
            return Err(self.fail(error::not_started()));
        }
        let save = self.reader.position();
        let base = self
            .track
            .as_ref()
            .map(|t| t.timing.base)
            .unwrap_or_else(|| {
                TimeBase::new(1, 1000).unwrap_or_else(|_| unreachable!("constant base"))
            });
        let target_ticks = Self::target_ticks(target, base).unwrap_or(0);
        let result = self.scan_landing(target_ticks);
        self.reader.seek_to(save)?;
        let (pts, _) = result?;
        self.landing_span(pts)
    }

    fn seekability(&self) -> SeekSupport {
        if !self.started || self.sticky.is_some() {
            return SeekSupport::None;
        }
        // No index: landing needs the header-only frame scan.
        SeekSupport::SlowLinear
    }

    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        if let Some(shape) = &self.sticky {
            return Err(shape.mint());
        }
        if !self.started {
            return Err(self.fail(error::not_started()));
        }
        let base = self
            .track
            .as_ref()
            .map(|t| t.timing.base)
            .unwrap_or_else(|| {
                TimeBase::new(1, 1000).unwrap_or_else(|_| unreachable!("constant base"))
            });
        let target_ticks = Self::target_ticks(target, base).unwrap_or(0);
        let (pts, offset) = self.scan_landing(target_ticks)?;
        // Commit (D4: every failure above left us untouched).
        self.walk = offset;
        self.last_pts = pts;
        self.pending.clear();
        self.eos = false;
        self.pending.push_back(DemuxEvent::Discontinuity);
        self.landing_span(pts)
    }
}
