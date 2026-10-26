//! The Ogg demuxer (card 5.B1): physical/logical stream mapping,
//! chained streams, granule discontinuities.
//!
//! # What this is
//!
//! [`OggDemuxer`] implements the landed
//! [`Demuxer`](plugin_api::Demuxer) contract over any seekable
//! [`SourceStream`](plugin_api::SourceStream). The physical stream is
//! a sequence of pages (RFC 3533); each page belongs to a **logical
//! stream** named by its 32-bit serial number. Multiple serials
//! interleaved = a multiplexed (MUX) file; a serial-set change after
//! the current link's streams ended — or a BOS page reusing a serial
//! — is a **chained link** boundary: the demuxer emits
//! [`Discontinuity`](plugin_api::DemuxEvent::Discontinuity), mints
//! fresh track ids for the new link, and never blends granules across
//! the boundary.
//!
//! # Timing (the honest contract)
//!
//! An Ogg page's granule position belongs to the **last packet that
//! completes on that page** (RFC 3533 §6). Apportioning one granule
//! among several packets needs codec-side durations (Vorbis block
//! sizes, Opus frame sizes), which are decode-domain — so this
//! demuxer stamps **only the last-completing packet** of each
//! granule-bearing page; every other packet carries
//! [`Stamp::Missing`] (data, MP §8: a gap is exposed, never
//! invented). Header pages stamp nothing; the first data granule
//! after them re-anchors the clock. A granule that regresses, or a
//! −1 **data** page punching a hole between known granules, emits a
//! `Discontinuity` and is counted in
//! [`granule_discontinuities`](OggDemuxer::granule_discontinuities).
//!
//! Codec identification is magic-prefix only (opaque passthrough —
//! Vorbis/Opus/Theora *decode* is not this card): the BOS packet's
//! magic names the codec, and the identification headers give the
//! granule clock (Vorbis ID rate; Opus is always 48 kHz per RFC 7845
//! §4.2; Theora frame rate + keyframe-granule shift). Unknown magic
//! = a track with an opaque descriptor, `Missing` stamps, no clock.
//!
//! # Damage policy (bounded, honest)
//!
//! Bad CRC, bad version, broken capture: the page is dropped and a
//! **bounded sync-recovery scan** (≤ [`MAX_RECOVERY_SCAN_BYTES`])
//! hunts the next CRC-valid capture — surfaced as
//! [`Recovered`](plugin_api::DemuxEvent::Recovered). A page that runs
//! past the end of the stream is a typed sticky truncation. Sequence
//! discontinuities and continued-flag lies are damage-noted with a
//! `Discontinuity` (timing may be off), never fatal, never hidden.
//!
//! # Zero-copy framing (notes for 5.X2)
//!
//! Page bodies land in a 2-slot [`PayloadSlots`] pool (256 KiB
//! slot cap); a packet completing inside one page is an O(1)
//! [`slice`](media_types::packet::SharedBytes::slice) of the pooled
//! body — zero copy, zero alloc. Only cross-page (continued) packets
//! copy: their runs concatenate into one buffer before the pool
//! adopts it (`from_vec`), and header packets stash one shared view
//! each. Steady-state allocations per page: the CRC scratch
//! (header-sized, reusable) and the segment-table `Vec` — the
//! natural 5.X2 extension is an owned CRC-fold over the pool view
//! and a fixed segment table.

use std::collections::HashMap;
use std::collections::VecDeque;

use media_types::error::ErrorContext;
use media_types::ids::{CodecDescriptor, GenerationId, MediaKind, StreamId, TrackFlags, TrackId};
use media_types::packet::SharedBytes;
use media_types::time::{Duration, Nanoseconds, Stamp, TimeBase, TimeSpan, Timestamp, TrackTiming};
use media_types::{EngineError, Packet, Track};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use crate::avi::header::DamageLog;
use crate::avi::riff::{PayloadSlots, SourceReader};
use page::PageHeader;

pub mod error;
pub mod fuzz;
pub mod page;
pub mod test_support;

#[cfg(test)]
mod tests;

/// Maximum packet payload trusted into memory per packet
/// `[PROPOSAL: 256 KiB — 4× the WAV/AVI budget because a single Ogg
/// page legally carries 65 025 bytes and one continued packet may
/// exceed it; above this a 255-run that never ends is corruption,
/// not media]`.
pub const MAX_PACKET_BYTES: u64 = 256 * 1024;

/// Maximum header packets (codec setup) trusted into memory per
/// logical stream `[PROPOSAL: 256 KiB — the largest sane Vorbis setup
/// header is tens of KiB]`.
pub const MAX_HEADER_BYTES: u64 = 256 * 1024;

/// Maximum bytes the sync-recovery scan reads hunting the next valid
/// capture (MP §12: recovery is bounded, never a whole-file loop).
pub const MAX_RECOVERY_SCAN_BYTES: u64 = 4 * 1024 * 1024;

/// The tail window scanned at `start` for final granules (the cheap
/// duration probe — ffmpeg's own ogg-duration trick).
pub const TAIL_PROBE_BYTES: u64 = 128 * 1024;

/// Maximum pages the seek scan walks before refusing (the
/// `SlowLinear` honesty bound).
pub const MAX_SEEK_SCAN_PAGES: u64 = 200_000;

/// Maximum pages `start` walks hunting the first link's headers (a
/// header run is tens of pages; the bound keeps a pathological file
/// from turning open into a full scan).
pub const MAX_OPEN_PAGES: u32 = 10_000;

/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["ogg", "oga", "ogv", "opus", "spx"];

/// The probe: do the leading bytes look like an Ogg page? (The
/// registry sniffs the capture pattern; page validity is CRC-gated
/// in the walk.)
pub fn probe(header: &[u8]) -> bool {
    header.len() >= 4 && &header[0..4] == page::CAPTURE
}

/// The codec a logical stream's magic bytes named.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OggCodec {
    /// `\x01vorbis` — clock = ID-header sample rate, granule = sample.
    Vorbis,
    /// `OpusHead` — clock always 48 kHz (RFC 7845 §4.2), granule =
    /// 48 kHz samples. Pre-skip is recorded in the header packet
    /// (opaque passthrough); applying it is decode domain.
    Opus,
    /// `\x80theora` — clock = info-header frame rate, granule =
    /// `keyframe << kfg_shift | frames_since_keyframe`.
    Theora,
    /// `\x7fFLAC` — clock = STREAMINFO rate, granule = sample
    /// `[PROPOSAL: pairs with the FLAC decoder card 5.A1]`.
    FlacInOgg,
    /// `Speex   ` — clock = header rate, granule = sample.
    Speex,
    /// `fishead\0` — Ogg Skeleton: physical-stream metadata, no
    /// track, no packet events.
    Skeleton,
    /// No recognized magic: opaque passthrough, no clock.
    Unknown,
}

impl OggCodec {
    /// The registry codec descriptor (opaque for unknowns).
    fn descriptor(self) -> CodecDescriptor {
        let name = match self {
            OggCodec::Vorbis => "vorbis",
            OggCodec::Opus => "opus",
            OggCodec::Theora => "theora",
            OggCodec::FlacInOgg => "flac",
            OggCodec::Speex => "speex",
            OggCodec::Skeleton | OggCodec::Unknown => "ogg-opaque",
        };
        CodecDescriptor::new(name).unwrap_or_else(|_| unreachable!("static names are valid"))
    }

    /// Whether a packet's first bytes still smell like a header
    /// packet for this codec (the header/data split).
    fn is_header_packet(self, data: &[u8]) -> bool {
        match self {
            OggCodec::Vorbis => matches!(data.first(), Some(0x01 | 0x03 | 0x05)),
            OggCodec::Opus => data.starts_with(b"OpusHead") || data.starts_with(b"OpusTags"),
            OggCodec::Theora => matches!(data.first(), Some(0x80..=0x82)),
            OggCodec::FlacInOgg => {
                data.starts_with(&[0x7F, b'F', b'L', b'A', b'C'])
                    || data.starts_with(&[0x00, 0x00, 0x00, 0x22])
            }
            OggCodec::Speex => data.starts_with(b"Speex   "),
            OggCodec::Skeleton | OggCodec::Unknown => false,
        }
    }
}

/// The granule→time clock parsed from a stream's identification
/// header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Clock {
    /// Granule counts samples at `rate` (Vorbis, FLAC, Speex).
    Samples { rate: u32 },
    /// Granule counts 48 kHz samples regardless of the header's
    /// input-rate field (Opus).
    Opus48000,
    /// Theora: frame `n = (g >> shift) + (g & mask)`, time =
    /// `n / (num/den)`.
    Theora { num: u32, den: u32, shift: u32 },
}

impl Clock {
    /// The track's time base (one tick = one granule-domain unit).
    fn base(self) -> TimeBase {
        match self {
            Clock::Samples { rate } => {
                TimeBase::new(1, rate).unwrap_or_else(|_| unreachable!("rate > 0 at parse"))
            }
            Clock::Opus48000 => TimeBase::new(1, 48_000).unwrap_or_else(|_| unreachable!("const")),
            Clock::Theora { num, den, .. } => {
                TimeBase::new(den, num).unwrap_or_else(|_| unreachable!("fps > 0 at parse"))
            }
        }
    }

    /// Granule → nanoseconds, exact (`None` when the granule is not
    /// a position: negative). One tick = `num/den` seconds (the
    /// TimeBase convention), so nanos = ticks × num × 1e9 / den —
    /// u128 intermediate, clamped to u64.
    fn nanos(self, granule: i64) -> Option<u64> {
        if granule < 0 {
            return None;
        }
        let (num, den) = self.base().as_parts();
        let ticks = match self {
            Clock::Samples { .. } | Clock::Opus48000 => granule as u64,
            Clock::Theora { shift, .. } => {
                let mask = (1u64 << shift) - 1;
                ((granule as u64) >> shift) + ((granule as u64) & mask)
            }
        };
        let ns = u128::from(ticks) * u128::from(num) * 1_000_000_000 / u128::from(den);
        u64::try_from(ns).ok()
    }

    /// Nanoseconds → the granule the clock would show (the seek
    /// mapping; approximate for Theora, exact for sample clocks).
    fn granule_of_nanos(self, nanos: u64) -> i64 {
        let (num, den) = self.base().as_parts();
        let ticks = u128::from(nanos) * u128::from(den) / (u128::from(num) * 1_000_000_000);
        ticks.min(u128::from(i64::MAX as u64)) as i64
    }
}

/// Identify a codec from its first (BOS) packet and parse the clock.
fn identify(first_packet: &[u8]) -> (OggCodec, Option<Clock>) {
    if first_packet.starts_with(&[0x01, b'v', b'o', b'r', b'b', b'i', b's'])
        && first_packet.len() >= 15
    {
        // Vorbis ID: version(4) channels(1) rate(LE24) …
        let rate = u32::from(first_packet[12])
            | (u32::from(first_packet[13]) << 8)
            | (u32::from(first_packet[14]) << 16);
        return (
            OggCodec::Vorbis,
            (rate > 0).then_some(Clock::Samples { rate }),
        );
    }
    if first_packet.starts_with(b"OpusHead") && first_packet.len() >= 12 {
        return (OggCodec::Opus, Some(Clock::Opus48000));
    }
    if first_packet.starts_with(&[0x80, b't', b'h', b'e', b'o', b'r', b'a'])
        && first_packet.len() >= 42
    {
        // Theora info: PICW BE24@14, PICH BE24@17, FRN BE32@22, FRD
        // BE32@26, KFGSHIFT packed in the QUAL/KFG/PF tail — offsets
        // pinned empirically against libtheora-encoded fixtures and
        // the corpus .ogv (see the unit tests + the differential).
        let num = be32(first_packet, 22);
        let den = be32(first_packet, 26);
        let shift = (u32::from(first_packet[40] & 0x03) << 3) | u32::from(first_packet[41] >> 5);
        let clock = (num > 0 && den > 0 && shift < 63).then_some(Clock::Theora { num, den, shift });
        return (OggCodec::Theora, clock);
    }
    if first_packet.starts_with(&[0x7F, b'F', b'L', b'A', b'C']) {
        return (OggCodec::FlacInOgg, None); // clock rides the STREAMINFO packet
    }
    if first_packet.starts_with(b"Speex   ") && first_packet.len() >= 40 {
        let rate = u32::from_le_bytes(
            first_packet[36..40]
                .try_into()
                .unwrap_or_else(|_| unreachable!("length checked above")),
        );
        return (
            OggCodec::Speex,
            (rate > 0).then_some(Clock::Samples { rate }),
        );
    }
    if first_packet.starts_with(b"fishead\0") {
        return (OggCodec::Skeleton, None);
    }
    (OggCodec::Unknown, None)
}

fn be32(data: &[u8], at: usize) -> u32 {
    u32::from(data[at]) << 24
        | u32::from(data[at + 1]) << 16
        | u32::from(data[at + 2]) << 8
        | u32::from(data[at + 3])
}

/// The FLAC-in-Ogg STREAMINFO clock (the second header packet):
/// native block header (4 bytes) + min/max blocksize (4) + min/max
/// frame size (6), then the 20-bit sample rate at byte offset 14.
fn flac_streaminfo_rate(packet: &[u8]) -> Option<u32> {
    if packet.len() < 17 {
        return None;
    }
    let rate =
        (u32::from(packet[14]) << 12) | (u32::from(packet[15]) << 4) | u32::from(packet[16] >> 4);
    (rate > 0).then_some(rate)
}

/// One logical stream's demux state.
struct LogicalStream {
    /// The serial number (physical identity).
    serial: u32,
    /// Link index this stream belongs to (0 = the first).
    link: usize,
    /// Track id minted for this stream (fresh per link).
    track: TrackId,
    /// Stream (demuxer) index minted alongside.
    stream: StreamId,
    /// Codec + clock once the BOS packet landed.
    codec: Option<OggCodec>,
    clock: Option<Clock>,
    /// The last page sequence number seen.
    last_seq: Option<u32>,
    /// EOS seen (the link-closing half of chain detection).
    ended: bool,
    /// Header packets collected (opaque passthrough, bounded).
    headers: Vec<SharedBytes>,
    header_bytes: u64,
    /// Cross-page packet assembly: the accumulated tail of a packet
    /// that ran past its page (None = no open packet).
    pending: Option<Vec<u8>>,
    /// The last known (≥ 0) granule.
    last_granule: Option<i64>,
    /// A −1 data page was seen since the last known granule (the
    /// hole flag — leaving it re-anchors with a Discontinuity).
    in_granule_hole: bool,
    /// Data packets emitted (the count differential's truth).
    packets_out: u64,
    /// The first data packet has not been emitted yet.
    first_data_pending: bool,
}

impl LogicalStream {
    fn new(serial: u32, link: usize, track: TrackId, stream: StreamId) -> LogicalStream {
        LogicalStream {
            serial,
            link,
            track,
            stream,
            codec: None,
            clock: None,
            last_seq: None,
            ended: false,
            headers: Vec::new(),
            header_bytes: 0,
            pending: None,
            last_granule: None,
            in_granule_hole: false,
            packets_out: 0,
            first_data_pending: true,
        }
    }
}

/// A replayable error shape (the WAV `StickyShape` precedent).
#[derive(Debug, Clone, PartialEq, Eq)]
struct StickyShape {
    stage: media_types::PipelineStage,
    kind: media_types::ErrorKind,
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

/// The Ogg demuxer.
///
/// Construct with [`OggDemuxer::new`] over a seekable source stream,
/// drive through the [`Demuxer`] trait: `start` walks the first
/// link's header pages and enumerates its tracks (plus a cheap
/// tail-granule probe for the durations); `next_event` yields data
/// packets, `Recovered`/`Discontinuity` container signals and
/// `EndOfStream`; `seek`/`reanchor` are honest `SlowLinear`
/// page-header scans.
///
/// # Example
///
/// ```
/// use demux::ogg::OggDemuxer;
/// use plugin_api::{Demuxer, DemuxEvent};
///
/// # fn main() -> Result<(), media_types::EngineError> {
/// # let bytes = demux::ogg::test_support::demo_stream();
/// let mut stream = demux::ogg::test_support::mem_stream(bytes)?;
/// let mut ogg = OggDemuxer::new(stream)?;
/// let tracks = ogg.start()?;
/// assert_eq!(tracks.len(), 1);
/// # match ogg.next_event()? {
/// #     DemuxEvent::Packet(p) => assert!(!p.data.is_empty()),
/// #     other => unreachable!("a data packet comes first: {other:?}"),
/// # }
/// # Ok(())
/// # }
/// ```
pub struct OggDemuxer {
    reader: SourceReader,
    started: bool,
    eos: bool,
    sticky: Option<StickyShape>,
    /// Live logical streams by serial (across all links).
    streams: HashMap<u32, LogicalStream>,
    /// Serials in arrival order (stable iteration).
    serial_order: Vec<u32>,
    /// Tracks minted so far (link 0's at `start`; chained links' as
    /// their BOS pages arrive — see [`tracks`](OggDemuxer::tracks)).
    tracks: Vec<Track>,
    next_track: u32,
    link: usize,
    pending: VecDeque<DemuxEvent>,
    damage: DamageLog,
    granule_discontinuities: u64,
    /// Next page position (absolute).
    walk: u64,
    /// The reanchor's packet mark: the first DATA packet after a
    /// reanchor carries `discontinuity: true` (the codec-api
    /// `Packet` contract — "demuxer-marked discontinuity (splice,
    /// seek, stream switch)"; the PS demuxer's own `next_discontinuity`
    /// precedent). Header packets never consume it: a landing on a
    /// header page (the small-fixture shape) keeps the mark armed
    /// for the first data packet that follows.
    next_discontinuity: bool,
    /// The tail probe's last granule per serial (durations at start).
    tail_granules: HashMap<u32, i64>,
    pool: PayloadSlots,
}

impl OggDemuxer {
    /// Construct over a seekable, length-known source stream (no
    /// container I/O — `start` does the parsing).
    ///
    /// # Errors
    /// Typed `Unsupported` (stage `Open`) for unknown-length,
    /// unseekable or empty sources (the AVI/WAV precedent).
    pub fn new(stream: Box<dyn plugin_api::SourceStream>) -> Result<OggDemuxer, EngineError> {
        let reader = SourceReader::new(stream)?;
        Ok(OggDemuxer {
            reader,
            started: false,
            eos: false,
            sticky: None,
            streams: HashMap::new(),
            serial_order: Vec::new(),
            tracks: Vec::new(),
            next_track: 0,
            link: 0,
            pending: VecDeque::new(),
            damage: DamageLog::default(),
            granule_discontinuities: 0,
            walk: 0,
            next_discontinuity: false,
            tail_granules: HashMap::new(),
            pool: PayloadSlots::new(2, MAX_PACKET_BYTES as usize),
        })
    }

    /// All tracks minted so far — link 0's after `start`, plus every
    /// chained link's as its BOS pages arrive (`next_event` drives
    /// the discovery). The trait's `start` snapshot is link 0 only
    /// `[PROPOSAL: an engine-side track-registry refresh event is
    /// the R3/5.R1 integration seam]`.
    #[must_use]
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// The number of chained links seen so far (1 = no chain).
    #[must_use]
    pub fn links(&self) -> usize {
        self.link + 1
    }

    /// The live serial numbers in arrival order.
    #[must_use]
    pub fn serial_numbers(&self) -> Vec<u32> {
        self.serial_order.clone()
    }

    /// One stream's collected header packets (opaque passthrough —
    /// the codec cards' `DecoderConfig::extra_data` seam). Empty for
    /// unknown serials.
    #[must_use]
    pub fn header_packets(&self, serial: u32) -> &[SharedBytes] {
        self.streams
            .get(&serial)
            .map(|s| s.headers.as_slice())
            .unwrap_or(&[])
    }

    /// Total granule-discontinuity events (regressions and −1 holes)
    /// across all streams — the honest counter behind the
    /// `Discontinuity` events.
    #[must_use]
    pub fn granule_discontinuities(&self) -> u64 {
        self.granule_discontinuities
    }

    /// Total bytes skipped by bounded recovery (the `Recovered`
    /// events' sum).
    #[must_use]
    pub fn damage_total(&self) -> u64 {
        self.damage.total
    }

    /// The link index a serial belongs to (chain observability).
    #[must_use]
    pub fn serial_link(&self, serial: u32) -> Option<usize> {
        self.streams.get(&serial).map(|s| s.link)
    }

    /// Data packets emitted per serial (the count differential).
    #[must_use]
    pub fn packet_counts(&self) -> Vec<(u32, u64)> {
        self.serial_order
            .iter()
            .filter_map(|s| self.streams.get(s).map(|st| (*s, st.packets_out)))
            .collect()
    }

    /// The payload bytes resident in the packet pool (the D-1
    /// observable; bounded by `2 × MAX_PACKET_BYTES`, grown lazily).
    #[must_use]
    pub fn pool_bytes(&self) -> usize {
        self.pool.pool_bytes()
    }

    /// One track's duration as measured so far (the tail-granule
    /// probe at `start`, refined by the walk): granule → nanoseconds
    /// via the stream's clock. `None` without a clock (opaque
    /// codecs) — never a guess.
    #[must_use]
    pub fn track_duration(&self, track: TrackId) -> Option<Duration> {
        let stream = self.streams.values().find(|s| s.track == track)?;
        let clock = stream.clock?;
        let granule = stream
            .last_granule
            .max(self.tail_granules.get(&stream.serial).copied())?;
        clock
            .nanos(granule)
            .map(|ns| Duration::Known(Nanoseconds(ns)))
    }

    // -- internals ---------------------------------------------------------

    fn fail(&mut self, err: EngineError) -> EngineError {
        self.sticky = Some(StickyShape::of(&err));
        err
    }

    /// Read + verify one page at `self.walk` (looping past damage),
    /// or `Ok(None)` at end-of-stream. Damage (bad capture, bad
    /// version, bad CRC) advances the walk through a bounded
    /// sync-recovery scan and queues `Recovered`.
    fn read_page(&mut self) -> Result<Option<PageHeaderAndBody>, EngineError> {
        let len = self.reader.length();
        while self.walk < len {
            let remaining = len - self.walk;
            if remaining < page::HEADER_BYTES as u64 {
                // A trailing fragment too short to be a page: a
                // bounded skip, then a clean end.
                self.damage.note(remaining, "ogg: trailing fragment");
                self.queue_recovered(remaining, "ogg: trailing fragment");
                self.walk = len;
                return Ok(None);
            }
            let head27 = self.reader.read_prefix_at(page::HEADER_BYTES, self.walk)?;
            if head27.len() < page::HEADER_BYTES {
                self.walk = len;
                return Ok(None);
            }
            if &head27[0..4] != page::CAPTURE {
                if let Some(found) = self.recover_sync("ogg: lost capture")? {
                    return Ok(Some(found));
                }
                return Ok(None);
            }
            let head = self.read_full_head(self.walk, head27[26])?;
            let header = match page::parse_header(&head, self.walk, len) {
                Ok(h) => h,
                Err(err) => {
                    // A truncated page is fatal (typed, sticky) — the
                    // stream promised structure it does not carry.
                    // Other header damage (nonzero version, zero
                    // segments) skips the candidate and rescans.
                    if matches!(
                        err.kind,
                        media_types::ErrorKind::Corrupt {
                            detail: media_types::error::CorruptDetail::Truncated { .. },
                        }
                    ) {
                        return Err(self.fail(err));
                    }
                    if let Some(found) = self.recover_sync("ogg: bad page header")? {
                        return Ok(Some(found));
                    }
                    return Ok(None);
                }
            };
            // CRC over header (field zeroed) + table + body.
            let head_len = page::HEADER_BYTES + header.segments.len();
            let mut whole = Vec::with_capacity(head_len + header.body_len);
            whole.extend_from_slice(&head[0..22]);
            whole.extend_from_slice(&[0u8; 4]);
            whole.extend_from_slice(&head[26..head_len]);
            let body = if header.body_len > 0 {
                let view = self.pool.fill(header.body_len, |buf| {
                    self.reader
                        .read_prefix_into(buf, header.offset + head_len as u64)
                })?;
                whole.extend_from_slice(view.as_slice());
                view
            } else {
                SharedBytes::from_static(b"")
            };
            if page::crc32(&whole) != header.crc {
                let skipped = header.total_len();
                self.damage.note(skipped, "ogg: page CRC mismatch");
                self.queue_recovered(skipped, "ogg: page CRC mismatch");
                self.walk = header.end();
                continue; // the next page may be fine
            }
            self.walk = header.end();
            return Ok(Some(PageHeaderAndBody { header, body }));
        }
        Ok(None)
    }

    /// Bounded sync-recovery: scan forward for the next capture
    /// whose page parses **and** CRC-verifies. Queues one `Recovered`
    /// for the skip. `Ok(None)` = the stream ended first.
    fn recover_sync(
        &mut self,
        reason: &'static str,
    ) -> Result<Option<PageHeaderAndBody>, EngineError> {
        let len = self.reader.length();
        let from = self.walk + 1;
        let budget_end = (from + MAX_RECOVERY_SCAN_BYTES).min(len);
        let mut at = from;
        while at < len {
            let window_end = (at + 64 * 1024).min(budget_end.max(at + 1));
            let window = self.reader.read_prefix_at((window_end - at) as usize, at)?;
            let mut candidate = None;
            let mut i = 0usize;
            while window.len() >= 4 {
                let Some(found) = find_capture(&window[i..]) else {
                    break;
                };
                let pos = at + (i + found) as u64;
                let head27 = self.reader.read_prefix_at(page::HEADER_BYTES, pos)?;
                if head27.len() < page::HEADER_BYTES {
                    break;
                }
                let head = self.read_full_head(pos, head27[26])?;
                if let Ok(hdr) = page::parse_header(&head, pos, len) {
                    let head_len = page::HEADER_BYTES + hdr.segments.len();
                    let mut whole = Vec::with_capacity(head_len + hdr.body_len);
                    whole.extend_from_slice(&head[0..22]);
                    whole.extend_from_slice(&[0u8; 4]);
                    whole.extend_from_slice(&head[26..head_len]);
                    if hdr.body_len > 0 {
                        let body = self
                            .reader
                            .read_prefix_at(hdr.body_len, pos + head_len as u64)?;
                        if body.len() < hdr.body_len {
                            break;
                        }
                        whole.extend_from_slice(&body);
                    }
                    if page::crc32(&whole) == hdr.crc {
                        candidate = Some(pos);
                        break;
                    }
                }
                i = i + found + 1;
                if i + 4 > window.len() {
                    break;
                }
            }
            match candidate {
                Some(pos) => {
                    let skipped = pos - self.walk;
                    self.damage.note(skipped, reason);
                    self.queue_recovered(skipped, reason);
                    self.walk = pos;
                    return self.read_page();
                }
                None => at = window_end,
            }
            if at >= budget_end && at < len {
                return Err(self.fail(error::recovery_budget_exhausted(at - from)));
            }
        }
        // The stream ended during recovery: final skip, clean end.
        let skipped = len.saturating_sub(self.walk);
        if skipped > 0 {
            self.damage.note(skipped, reason);
            self.queue_recovered(skipped, reason);
        }
        self.walk = len;
        Ok(None)
    }

    /// Read the full page header (27 + segment-table bytes) at `pos`.
    fn read_full_head(&mut self, pos: u64, nsegs: u8) -> Result<Vec<u8>, EngineError> {
        let full = page::HEADER_BYTES + nsegs as usize;
        self.reader.read_prefix_at(full, pos)
    }

    fn queue_recovered(&mut self, skipped: u64, reason: &'static str) {
        if skipped > 0 {
            self.pending
                .push_back(DemuxEvent::Recovered { skipped, reason });
        }
    }

    /// Process one page: stream mapping, chain detection, packet
    /// assembly, event queueing. `Ok(false)` = end of stream.
    fn step(&mut self) -> Result<bool, EngineError> {
        let Some(found) = self.read_page()? else {
            return Ok(false);
        };
        let header = found.header;
        let body = found.body;

        // -- physical → logical mapping + chains ------------------------
        let known = self.streams.contains_key(&header.serial);
        let all_ended = self.streams.values().all(|s| s.ended);
        if !known {
            if all_ended && !self.streams.is_empty() {
                // Chained link: everything ended, a new serial arrives
                // (BOS or not — the canonical `cat` boundary).
                self.link += 1;
                self.pending.push_back(DemuxEvent::Discontinuity);
            } else if !self.streams.is_empty() && !header.bos {
                // A mid-link serial with no BOS: the stream is real,
                // its opening page is not — damage-noted, same link.
                self.damage.note(0, "ogg: mid-link stream without BOS");
                self.pending.push_back(DemuxEvent::Discontinuity);
            }
            // else: the normal BOS of the current (first) link.
            self.open_stream(header.serial);
        } else if header.bos {
            // Serial reuse with a fresh BOS: a chained link recycling
            // the serialno — fresh state, fresh track id.
            self.link += 1;
            self.pending.push_back(DemuxEvent::Discontinuity);
            self.open_stream(header.serial);
        }

        // -- sequence continuity -----------------------------------------
        {
            let s = self
                .streams
                .get_mut(&header.serial)
                .unwrap_or_else(|| unreachable!("inserted above"));
            if s.last_seq
                .map(|q| q.wrapping_add(1))
                .is_some_and(|expected| expected != header.seq)
            {
                self.damage.note(0, "ogg: page sequence discontinuity");
                self.granule_discontinuities += 1;
                self.pending.push_back(DemuxEvent::Discontinuity);
            }
            s.last_seq = Some(header.seq);
        }

        // -- granule bookkeeping (holes + regressions) -------------------
        if header.granule >= 0 {
            let s = self
                .streams
                .get(&header.serial)
                .unwrap_or_else(|| unreachable!("inserted above"));
            if let Some(last) = s.last_granule {
                if header.granule < last {
                    // The clock went backwards — honest discontinuity,
                    // never a smoothed jump.
                    self.granule_discontinuities += 1;
                    self.pending.push_back(DemuxEvent::Discontinuity);
                } else if s.in_granule_hole && header.granule > last {
                    // Leaving a −1 hole: the gap is real, expose it.
                    self.granule_discontinuities += 1;
                    self.pending.push_back(DemuxEvent::Discontinuity);
                }
            }
            let s = self
                .streams
                .get_mut(&header.serial)
                .unwrap_or_else(|| unreachable!("inserted above"));
            s.last_granule = Some(header.granule);
            s.in_granule_hole = false;
        } else if self
            .streams
            .get(&header.serial)
            .is_some_and(|s| s.packets_out > 0)
        {
            // A −1 data page between known granules: a timing hole.
            let s = self
                .streams
                .get_mut(&header.serial)
                .unwrap_or_else(|| unreachable!("inserted above"));
            if s.last_granule.is_some() {
                s.in_granule_hole = true;
            }
        }

        // -- continued-flag honesty --------------------------------------
        let had_pending = self
            .streams
            .get(&header.serial)
            .is_some_and(|s| s.pending.is_some());
        let orphan_run = header.continued && !had_pending;
        if orphan_run {
            self.damage
                .note(0, "ogg: continued flag without an open packet");
        } else if !header.continued && had_pending {
            self.damage
                .note(0, "ogg: open packet dropped at a non-continued page");
            self.granule_discontinuities += 1;
            self.pending.push_back(DemuxEvent::Discontinuity);
            if let Some(s) = self.streams.get_mut(&header.serial) {
                s.pending = None;
            }
        }

        // -- packet assembly ---------------------------------------------
        let runs = header.packet_runs();
        // The last terminal run on the page owns the granule (RFC
        // 3533 §6: "the granule position is the position of the last
        // packet completing on the page").
        let last_terminal = runs.iter().rposition(|r| r.2);
        for (run_index, (start, run_len, terminal)) in runs.iter().enumerate() {
            if run_index == 0 && orphan_run {
                continue; // the orphaned continuation tail: dropped
            }
            let run_view = body.slice(*start..*start + *run_len);
            let serial = header.serial;
            // The borrow of the stream must not live across
            // `complete_packet` (which borrows the whole demuxer) —
            // so assembly completes into a local, then hands off.
            let mut completed: Option<SharedBytes> = None;
            let mut overrun: Option<u64> = None;
            {
                let stream = self
                    .streams
                    .get_mut(&serial)
                    .unwrap_or_else(|| unreachable!("inserted above"));
                match stream.pending.as_mut() {
                    Some(acc) => {
                        if acc.len() + run_len > MAX_PACKET_BYTES as usize {
                            overrun = Some((acc.len() + run_len) as u64);
                        } else {
                            acc.extend_from_slice(&run_view);
                            if *terminal {
                                let whole = stream
                                    .pending
                                    .take()
                                    .unwrap_or_else(|| unreachable!("checked"));
                                completed = Some(SharedBytes::from_vec(whole));
                            }
                        }
                    }
                    None if *terminal => {
                        // Fast path: the packet lives in this page —
                        // a zero-copy slice of the pooled body.
                        completed = Some(run_view);
                    }
                    None => {
                        if *run_len > MAX_PACKET_BYTES as usize {
                            overrun = Some(*run_len as u64);
                        } else {
                            stream.pending = Some(run_view.as_slice().to_vec());
                        }
                    }
                }
            }
            if let Some(attempted) = overrun {
                return Err(self.fail(error::packet_overrun(MAX_PACKET_BYTES, attempted)));
            }
            if let Some(data) = completed {
                self.complete_packet(&header, data, Some(run_index) == last_terminal)?;
            }
        }

        // -- EOS ----------------------------------------------------------
        if header.eos
            && let Some(s) = self.streams.get_mut(&header.serial)
        {
            s.ended = true;
        }
        Ok(true)
    }

    /// Insert (or replace) the logical-stream state for `serial`,
    /// minting a fresh track id.
    fn open_stream(&mut self, serial: u32) {
        let track = TrackId::new(self.next_track);
        let stream = StreamId::new(self.next_track);
        self.next_track += 1;
        if !self.streams.contains_key(&serial) {
            self.serial_order.push(serial);
        }
        self.streams
            .insert(serial, LogicalStream::new(serial, self.link, track, stream));
    }

    /// One completed packet: identification, header/data split,
    /// clock stamping, event queueing.
    fn complete_packet(
        &mut self,
        header: &PageHeader,
        data: SharedBytes,
        owns_granule: bool,
    ) -> Result<(), EngineError> {
        let serial = header.serial;
        let mut identify_now = false;
        let mut flac_clock_now = false;
        {
            let stream = self
                .streams
                .get(&serial)
                .unwrap_or_else(|| unreachable!("caller inserted the stream"));
            if stream.codec.is_none() {
                identify_now = true;
            } else if stream.codec == Some(OggCodec::FlacInOgg)
                && stream.clock.is_none()
                && stream.headers.len() == 1
            {
                flac_clock_now = true;
            }
        }
        if identify_now {
            // The first packet of the stream: the BOS identification
            // header by construction — for codecs with a header
            // vocabulary. An unrecognized magic is opaque DATA (the
            // demuxer still tracks and emits it).
            let (codec, clock) = identify(data.as_slice());
            let is_flac_mapping = codec == OggCodec::FlacInOgg;
            if codec != OggCodec::Unknown {
                {
                    let stream = self
                        .streams
                        .get_mut(&serial)
                        .unwrap_or_else(|| unreachable!("checked above"));
                    stream.codec = Some(codec);
                    stream.clock = clock;
                    stash_header(stream, &mut self.damage, data);
                }
                if !is_flac_mapping {
                    self.mint_track(serial);
                }
                return Ok(());
            }
            {
                let stream = self
                    .streams
                    .get_mut(&serial)
                    .unwrap_or_else(|| unreachable!("checked above"));
                stream.codec = Some(codec);
                stream.clock = clock;
            }
            self.mint_track(serial);
            // Fall through: the BOS packet is this stream's first
            // data packet.
        }
        if flac_clock_now {
            if let Some(rate) = flac_streaminfo_rate(data.as_slice()) {
                let stream = self
                    .streams
                    .get_mut(&serial)
                    .unwrap_or_else(|| unreachable!("checked above"));
                stream.clock = Some(Clock::Samples { rate });
                stash_header(stream, &mut self.damage, data);
                self.mint_track(serial);
            } else {
                let stream = self
                    .streams
                    .get_mut(&serial)
                    .unwrap_or_else(|| unreachable!("checked above"));
                stash_header(stream, &mut self.damage, data);
                self.mint_track(serial);
            }
            return Ok(());
        }
        let (codec, clock) = {
            let stream = self
                .streams
                .get(&serial)
                .unwrap_or_else(|| unreachable!("checked above"));
            (stream.codec.unwrap_or(OggCodec::Unknown), stream.clock)
        };
        // Header/data split: recognized codecs keep header packets
        // opaque-passthrough (decoder config, not events). Skeleton
        // streams never surface packets at all.
        if codec == OggCodec::Skeleton {
            let stream = self
                .streams
                .get_mut(&serial)
                .unwrap_or_else(|| unreachable!("checked above"));
            stream.packets_out += 1;
            stream.first_data_pending = false;
            return Ok(());
        }
        if codec.is_header_packet(data.as_slice()) {
            let stream = self
                .streams
                .get_mut(&serial)
                .unwrap_or_else(|| unreachable!("checked above"));
            stash_header(stream, &mut self.damage, data);
            return Ok(());
        }
        // Data packet: only the page's last-completing packet owns
        // the granule (see the module docs — apportioning needs
        // codec durations, decode domain).
        let pts = if owns_granule && header.granule >= 0 {
            match clock {
                Some(k) => Stamp::Present(Timestamp::new(header.granule, k.base())),
                None => Stamp::Missing,
            }
        } else {
            Stamp::Missing
        };
        let theora_key = codec == OggCodec::Theora
            && owns_granule
            && header.granule >= 0
            && matches!(clock, Some(Clock::Theora { shift, .. })
                if (header.granule as u64 & ((1 << shift) - 1)) == 0);
        let track;
        let first_of_stream;
        {
            let stream = self
                .streams
                .get_mut(&serial)
                .unwrap_or_else(|| unreachable!("checked above"));
            track = stream.track;
            first_of_stream = stream.first_data_pending;
            stream.first_data_pending = false;
            stream.packets_out += 1;
        }
        let packet = Packet {
            track,
            pts,
            dts: Stamp::Missing,
            duration: None,
            data,
            keyframe: first_of_stream || theora_key,
            // The reanchor's landing mark, consumed here (data
            // packets only — see the field's doc).
            discontinuity: self.next_discontinuity,
            generation: GenerationId::FIRST,
        };
        self.next_discontinuity = false;
        self.pending.push_back(DemuxEvent::Packet(packet));
        Ok(())
    }

    /// Mint the track for an identified stream (once per stream;
    /// Skeleton streams mint none).
    fn mint_track(&mut self, serial: u32) {
        let Some(stream) = self.streams.get(&serial) else {
            return;
        };
        if stream.codec == Some(OggCodec::Skeleton) {
            return;
        }
        if self.tracks.iter().any(|t| t.id == stream.track) {
            return;
        }
        let codec = stream.codec.unwrap_or(OggCodec::Unknown);
        let kind = if codec == OggCodec::Theora {
            MediaKind::Video
        } else {
            MediaKind::Audio
        };
        let base = stream
            .clock
            .map(|k| k.base())
            .unwrap_or_else(|| TimeBase::new(1, 1000).unwrap_or_else(|_| unreachable!("const")));
        let end = self
            .tail_granules
            .get(&serial)
            .and_then(|g| stream.clock.and_then(|k| k.nanos(*g)))
            .map(|ns| nanos_to_stamp(ns, base))
            .unwrap_or(Stamp::Missing);
        let track = Track {
            id: stream.track,
            stream: stream.stream,
            kind,
            language: None,
            codec: codec.descriptor(),
            title: None,
            flags: TrackFlags::NONE,
            timing: TrackTiming {
                span: TimeSpan {
                    start: Stamp::Present(Timestamp::new(0, base)),
                    end,
                },
                base,
                gaps: Vec::new(),
            },
        };
        self.tracks.push(track);
    }

    /// `start`'s cheap duration probe: scan the file tail for the
    /// last CRC-valid page per serial, remember its granule.
    fn probe_tail(&mut self) -> Result<(), EngineError> {
        let len = self.reader.length();
        let from = len.saturating_sub(TAIL_PROBE_BYTES);
        let window = self.reader.read_prefix_at((len - from) as usize, from)?;
        let mut at = 0usize;
        while at + page::HEADER_BYTES <= window.len() {
            let Some(found) = find_capture(&window[at..]) else {
                break;
            };
            let page_at = at + found;
            if let Ok(hdr) = page::parse_header(&window[page_at..], from + page_at as u64, len) {
                let head_len = page::HEADER_BYTES + hdr.segments.len();
                let end = head_len + hdr.body_len;
                if page_at + end <= window.len() {
                    let mut whole = Vec::with_capacity(end);
                    whole.extend_from_slice(&window[page_at..page_at + 22]);
                    whole.extend_from_slice(&[0u8; 4]);
                    whole.extend_from_slice(&window[page_at + 26..page_at + head_len]);
                    whole.extend_from_slice(&window[page_at + head_len..page_at + end]);
                    if page::crc32(&whole) == hdr.crc && hdr.granule >= 0 {
                        self.tail_granules.insert(hdr.serial, hdr.granule);
                    }
                    at = page_at + end;
                    continue;
                }
            }
            at = page_at + 1;
        }
        Ok(())
    }

    /// Run pages until link 0's tracks are enumerable: every live
    /// stream identified and past its headers (first data packet
    /// seen, or EOS), or the page budget / stream ends.
    fn open(&mut self) -> Result<Vec<Track>, EngineError> {
        self.probe_tail()?;
        let len = self.reader.length();
        let mut budget = MAX_OPEN_PAGES;
        while self.walk < len && budget > 0 {
            self.step()?;
            budget -= 1;
            let ready = !self.streams.is_empty()
                && self
                    .streams
                    .values()
                    .all(|s| (s.codec.is_some() && s.packets_out > 0) || s.ended);
            if ready {
                break;
            }
        }
        if self.tracks.is_empty() {
            // No BOS page reached within the budget / stream: there
            // is no logical stream to demux.
            return Err(error::no_streams());
        }
        self.started = true;
        Ok(self.tracks.clone())
    }

    /// The seek core: map time → granule → the last page of the
    /// reference stream (first clock-bearing track) whose granule is
    /// ≤ the target. Header-only scan (no body reads). Returns the
    /// landing span + the absolute page offset.
    fn scan_landing(&mut self, target: TimeSpan) -> Result<(TimeSpan, u64), EngineError> {
        let Some((serial, clock)) = self.reference_stream() else {
            return Err(error::seek_out_of_range());
        };
        let target_nanos = target
            .start
            .timestamp()
            .ok()
            .and_then(|t| t.as_nanos().ok())
            .map(|n| n.0)
            .unwrap_or(0);
        let target_granule = clock.granule_of_nanos(target_nanos);
        let len = self.reader.length();
        let mut at = 0u64;
        let mut landing: Option<(i64, u64)> = None;
        let mut pages = 0u64;
        while at + page::HEADER_BYTES as u64 <= len {
            let head27 = self.reader.read_prefix_at(page::HEADER_BYTES, at)?;
            if head27.len() < page::HEADER_BYTES {
                break;
            }
            let head = self.read_full_head(at, head27[26])?;
            let Ok(hdr) = page::parse_header(&head, at, len) else {
                at += 1;
                continue;
            };
            pages += 1;
            if pages > MAX_SEEK_SCAN_PAGES {
                return Err(error::seek_out_of_range());
            }
            if hdr.serial == serial && hdr.granule >= 0 && hdr.granule <= target_granule {
                landing = Some((hdr.granule, at));
            }
            at = hdr.end();
        }
        let Some((granule, page_at)) = landing else {
            return Err(error::seek_out_of_range());
        };
        let base = clock.base();
        let span = TimeSpan {
            start: Stamp::Present(Timestamp::new(granule, base)),
            end: Stamp::Missing,
        };
        Ok((span, page_at))
    }

    /// The first clock-bearing stream (the seek reference).
    fn reference_stream(&self) -> Option<(u32, Clock)> {
        self.serial_order.iter().find_map(|serial| {
            self.streams
                .get(serial)
                .and_then(|s| s.clock.map(|c| (*serial, c)))
        })
    }
}

/// One verified page from the walk.
struct PageHeaderAndBody {
    header: PageHeader,
    body: SharedBytes,
}

/// Bounded header stash (opaque passthrough for the codec cards).
fn stash_header(stream: &mut LogicalStream, damage: &mut DamageLog, data: SharedBytes) {
    if stream.header_bytes + data.len() as u64 > MAX_HEADER_BYTES {
        // Refuse to trust the declaration further: damage-noted, the
        // stream still demuxes its data.
        damage.note(0, "ogg: header budget exceeded");
        return;
    }
    stream.header_bytes += data.len() as u64;
    stream.headers.push(data);
}

/// Find `OggS` in `data`, returning the index.
fn find_capture(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|w| w == page::CAPTURE)
}

/// Nanoseconds → a stamp in `base` (one tick = `num/den` seconds,
/// so ticks = ns × den / (num × 1e9), u128 intermediate).
fn nanos_to_stamp(ns: u64, base: TimeBase) -> Stamp {
    let (num, den) = base.as_parts();
    let ticks = u128::from(ns) * u128::from(den) / (u128::from(num) * 1_000_000_000);
    Stamp::Present(Timestamp::new(
        ticks.min(u128::from(i64::MAX as u64)) as i64,
        base,
    ))
}

impl Demuxer for OggDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if !self.tracks.is_empty() {
            return Ok(self.tracks.clone());
        }
        if let Some(shape) = &self.sticky {
            return Err(shape.mint());
        }
        match self.open() {
            Ok(tracks) => Ok(tracks),
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
            match self.step() {
                Ok(true) => continue,
                Ok(false) => {
                    self.eos = true;
                    // Damage queued by the final step surfaces BEFORE
                    // the terminal EndOfStream.
                    if let Some(event) = self.pending.pop_front() {
                        return Ok(event);
                    }
                    return Ok(DemuxEvent::EndOfStream);
                }
                Err(err) => return Err(err),
            }
        }
    }

    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // Pure query: the scan reads via peek-reads and restores the
        // source position — nothing observable moves.
        let save = self.reader.position();
        let result = self.scan_landing(target);
        self.reader.seek_to(save)?;
        result.map(|(span, _)| span)
    }

    fn seekability(&self) -> SeekSupport {
        if !self.started || self.sticky.is_some() {
            return SeekSupport::None;
        }
        // Ogg has no index: landing needs a page-header scan.
        SeekSupport::SlowLinear
    }

    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // Compute (the scan + validation)…
        let landing = self.scan_landing(target)?;
        // …then commit. D4: every failure above left us untouched.
        self.walk = landing.1;
        self.pending.clear();
        for stream in self.streams.values_mut() {
            stream.pending = None;
            stream.first_data_pending = false;
        }
        self.eos = false;
        self.pending.push_back(DemuxEvent::Discontinuity);
        // The landing mark: the first data packet of the new position
        // tells the decoders the timeline jumped (the codec-api
        // `Packet::discontinuity` contract — without it a seek over a
        // granule-domain codec leaves the decoder re-anchoring on a
        // counter that no longer matches the stream).
        self.next_discontinuity = true;
        Ok(landing.0)
    }
}

#[cfg(test)]
mod clock_unit {
    use super::*;

    #[test]
    fn clock_conversions_are_exact() {
        let samples = Clock::Samples { rate: 48_000 };
        assert_eq!(samples.nanos(48_000), Some(1_000_000_000));
        assert_eq!(samples.nanos(-1), None);
        assert_eq!(samples.granule_of_nanos(1_000_000_000), 48_000);
        let opus = Clock::Opus48000;
        assert_eq!(opus.nanos(960), Some(20_000_000));
        assert_eq!(opus.granule_of_nanos(20_000_000), 960);
        let theora = Clock::Theora {
            num: 10,
            den: 1,
            shift: 6,
        };
        // granule 73 = kf 1<<6 | delta 9 → frame 10 → 1.0 s at 10fps
        assert_eq!(theora.nanos(73), Some(1_000_000_000));
        assert_eq!(theora.granule_of_nanos(1_000_000_000), 10);
    }

    #[test]
    fn identify_parses_the_magic_set() {
        let vorbis_id = {
            let mut v = vec![0x01];
            v.extend_from_slice(b"vorbis");
            v.extend_from_slice(&0u32.to_le_bytes()); // version
            v.push(2); // channels
            v.extend_from_slice(&8_000u32.to_le_bytes()[..3]); // rate LE24
            v.extend_from_slice(&[0; 17]);
            v
        };
        let (codec, clock) = identify(&vorbis_id);
        assert_eq!(codec, OggCodec::Vorbis);
        assert_eq!(clock, Some(Clock::Samples { rate: 8_000 }));

        let (codec, clock) = identify(b"OpusHead\x01\x01\x38\x01\x80\x3e\x00\x00");
        assert_eq!(codec, OggCodec::Opus);
        assert_eq!(clock, Some(Clock::Opus48000));

        let (codec, _) = identify(&[0x7F, b'F', b'L', b'A', b'C', 1, 0, 0, 2]);
        assert_eq!(codec, OggCodec::FlacInOgg);

        let (codec, _) = identify(b"fishead\x00\x00\x00\x00");
        assert_eq!(codec, OggCodec::Skeleton);

        let (codec, clock) = identify(&[0xAB; 30]);
        assert_eq!(codec, OggCodec::Unknown);
        assert_eq!(clock, None);
    }

    #[test]
    fn theora_header_offsets_pin_the_corpus_layout() {
        // The corpus .ogv's identification header (ffmpeg 7.1 +
        // libtheora: 360x640 at 1/2 fps, KFGSHIFT 6) — offsets
        // pinned by the differential; tests/ogg_corpus.rs runs the
        // file itself.
        let hdr: [u8; 42] = [
            0x80, b't', b'h', b'e', b'o', b'r', b'a', 0x03, 0x02, 0x01, 0x00, 0x17, 0x00, 0x28,
            0x00, 0x01, 0x68, 0x00, 0x02, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x02, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x03, 0x0d, 0x40, 0x00, 0xc0,
        ];
        let (codec, clock) = identify(&hdr);
        assert_eq!(codec, OggCodec::Theora);
        let Some(Clock::Theora { num, den, shift }) = clock else {
            unreachable!("corpus header must parse");
        };
        assert_eq!((num, den, shift), (1, 2, 6));
        // EOS granule 65 → 2 frames → 4.0 s (matches ffprobe).
        assert_eq!(
            Clock::Theora { num, den, shift }.nanos(65),
            Some(4_000_000_000)
        );
    }

    #[test]
    fn flac_streaminfo_rate_bits() {
        // 20-bit rate 0x0BB80 = 48000: bytes 14,15 = 0x0b,0xb8,
        // byte 16 high nibble 0.
        let mut packet = vec![0x00, 0x00, 0x00, 0x22];
        packet.extend_from_slice(&[0; 10]);
        packet.extend_from_slice(&[0x0b, 0xb8, 0x00, 0x00, 0x00]);
        assert_eq!(flac_streaminfo_rate(&packet), Some(48_000));
    }

    #[test]
    fn find_capture_ignores_short_tails() {
        assert_eq!(find_capture(b"xxOggS"), Some(2));
        assert_eq!(find_capture(b"Ogg"), None);
        assert_eq!(find_capture(b""), None);
    }
}
