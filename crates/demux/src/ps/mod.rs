//! The MPEG-PS/VOB demuxer (card 5.I2): pack/system-header walk,
//! DVD-VOB quirks, private-stream-1 routing with AC-3/DTS/LPCM/SPU
//! exposed as honest tracks (decode pending — typed, never silent).
//!
//! # What this is
//!
//! [`PsDemuxer`] implements the landed
//! [`plugin_api::Demuxer`](plugin_api::Demuxer) contract over any
//! seekable [`SourceStream`](plugin_api::SourceStream). The byte
//! layouts are the ones ffmpeg 7.1.5's `mpeg` (MPEG-1 system stream)
//! and `dvd` (MPEG-2 program stream / VOB) muxers emit — the
//! committed fixtures under `fixtures/ps/` are ffmpeg products, and
//! the differential suite pins the demuxed payloads against ffmpeg's
//! own raw extractions bit-for-bit.
//!
//! # Design (recorded for review)
//!
//! - **No central directory: enumeration is a header-only pre-scan.**
//!   MPEG-PS scatters its stream set across the file, so `start`
//!   walks the pack/PES *headers* only (payloads are seek-past), plus
//!   one ≤1-byte substream peek per private-1 PES and one ≤64-byte
//!   elementary peek per new audio/video id. Event delivery re-walks
//!   from position 0 and reads payloads then — two passes over the
//!   structure, one over the bytes `[PROPOSAL: single-pass lazy
//!   track addition needs an engine contract for "tracks added
//!   mid-session"; the pre-scan is the honest v0]`.
//! - **Container truth is the PES packet.** One PES = one demuxed
//!   packet; ffmpeg's demuxer re-packetizes to codec access units
//!   (mpegvideo frame-splitting, AC-3 frame alignment), so packet
//!   *counts* legitimately differ from ffprobe's — the differential
//!   table pins ours and records the divergence instead of matching
//!   it (the 5.I2 prior-session note).
//! - **Stamp conventions: ffmpeg's, not the spec's.** Three deltas
//!   are pinned by exact assertion in the tests: (1) MPEG-1 PES
//!   without a stamp carries a single `0x0F` byte (`'0000'` + the
//!   four low marker bits) where ISO 11172-1 describes optional
//!   `0xFF` stuffing only; (2) ffmpeg's `dvd` muxer writes a 3-byte
//!   filler after the private-1 substream byte before AC-3/DTS
//!   frames (4 bytes total stripped — verified bit-exact against
//!   `ffmpeg -c copy -f ac3`); (3) the trailing marker bits ffmpeg
//!   writes into the MPEG-1 mux-rate field (`01`) and the MPEG-2 PES
//!   flag byte (`0xC0`/`0x80`, marker bit 0) are **not** enforced —
//!   the prior-session bug (`b[11]&1` treated as a marker,
//!   rejecting ffmpeg VOBs at byte 0) is the recorded lesson: those
//!   bits are mux-rate/flag data, and markers are only sanity-sliced
//!   where the arithmetic demands it.
//! - **Private-stream-1 routing (DVD-Video):** payload byte 0 is the
//!   substream id — `0x80..=0x87` AC-3, `0x88..=0x8F` DTS,
//!   `0xA0..=0xA7` LPCM (a 6-byte DVD LPCM header follows the
//!   substream byte), `0x20..=0x3F` sub-picture. These streams are
//!   **exposed with honest descriptors**; no in-house decoder exists
//!   yet, so decode refusal is a typed `Unsupported` at the codec
//!   registry — the demuxer never silently drops them. Unclassified
//!   substreams are skipped positionally with a `Recovered` event.
//! - **VOB nav packs are data, not damage.** `private_stream_2`
//!   (`0xBF`) carries the DVD PCI/DSI navigation payload (ffmpeg's
//!   muxer zero-fills it); it is exposed as a `Data` track
//!   (`dvd_nav_packet`, matching ffprobe's stream shape). Its damage
//!   tolerance is positional skip with a `Recovered` note.
//!
//! # Memory & speed (MP §12)
//!
//! Payloads ride the AVI [`PayloadSlots`] pool (2 arenas × 64 KiB —
//! a PES length is a u16, so 65 535 ≤ the ceiling); the pre-scan and
//! the event walk read fixed stack buffers for headers (`[u8; 4]` /
//! `[u8; 14]` / `[u8; 267]`), so nothing is allocated per packet
//! before the pooled payload view. Walk positions only advance;
//! recovery scans are window-bounded ([`MAX_RECOVERY_SCAN_BYTES`]).
//!
//! # v0 scope (documented limits)
//!
//! - One program (multi-program `program_stream_map` use is
//!   length-skipped, not surfaced).
//! - `PES_packet_length == 0` (legal in MPEG-2 as "unbounded" for
//!   some TS-derived writers) is a damage-noted skip, never an
//!   unbounded read.
//! - `ESCR`/`ES_rate`/trick-mode PES extensions are not surfaced
//!   (stamps only); the DVD LPCM header bytes are skipped, not
//!   re-exposed as channel/rate facts `[PROPOSAL: the 6-byte header
//!   carries the audio facts — owed to the PCM-DVD codec card]`.

use std::collections::VecDeque;

use media_types::ids::{CodecDescriptor, GenerationId, MediaKind, StreamId, TrackFlags, TrackId};
use media_types::time::{Duration, Stamp, TimeBase, TimeSpan, Timestamp, TrackTiming};
use media_types::{EngineError, Packet, Track};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use crate::avi::header::DamageLog;
use crate::avi::riff::{PayloadSlots, SourceReader};

pub mod error;
pub mod fuzz;
pub mod test_support;

#[cfg(test)]
mod tests;

/// Maximum PES payload trusted into memory per packet `[PROPOSAL:
/// 64 KiB — a PES length is u16 (≤ 65 535), so the ceiling never
/// clips a legal packet; the pool bound stays 2 × 64 KiB]`.
pub const MAX_PACKET_BYTES: u64 = 64 * 1024;

/// The re-synchronization window when damaged bytes break the start
/// code cadence `[PROPOSAL: 64 KiB — one DVD sector pair's worth;
/// the AVI W2-07 budget precedent]`.
pub const MAX_RECOVERY_SCAN_BYTES: u64 = 64 * 1024;

/// The MPEG-PS 90 kHz clock (SCR and PTS ticks).
const CLOCK: u32 = 90_000;

/// Maximum PES header bytes read in one prefix (3 flag/header-length
/// bytes + a u8 header length).
const MAX_PES_HEADER: usize = 3 + 255;

/// Maximum MPEG-1 PES stuffing bytes tolerated before the header is
/// declared corrupt.
const MAX_MPEG1_STUFFING: usize = 16;

/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["mpg", "mpeg", "ps", "vob", "m2p"];

/// The probe: does the stream start with a pack start code?
#[must_use]
pub fn probe(header: &[u8]) -> bool {
    header.starts_with(&[0x00, 0x00, 0x01, 0xBA])
}

/// The private-1 payload routing (DVD-Video book): what must be
/// stripped after the substream byte for codec data to start at a
/// frame boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrivateKind {
    /// `0x80..=0x87` — substream byte + 3 ffmpeg filler bytes.
    Ac3,
    /// `0x88..=0x8F` — substream byte + 3 ffmpeg filler bytes.
    Dts,
    /// `0xA0..=0xA7` — substream byte + 6-byte DVD LPCM header.
    Lpcm,
    /// `0x20..=0x3F` — substream byte only (sub-picture unit data).
    Spu,
}

impl PrivateKind {
    /// Classify a private-1 substream byte.
    fn of_substream(byte: u8) -> Option<PrivateKind> {
        match byte {
            0x80..=0x87 => Some(PrivateKind::Ac3),
            0x88..=0x8F => Some(PrivateKind::Dts),
            0xA0..=0xA7 => Some(PrivateKind::Lpcm),
            0x20..=0x3F => Some(PrivateKind::Spu),
            _ => None,
        }
    }

    /// Bytes before the codec data (substream byte included).
    fn strip(self) -> u64 {
        match self {
            PrivateKind::Ac3 | PrivateKind::Dts => 4,
            PrivateKind::Lpcm => 7,
            PrivateKind::Spu => 1,
        }
    }

    /// The registry descriptor (honest: decode pending elsewhere).
    fn descriptor(self) -> &'static str {
        match self {
            PrivateKind::Ac3 => "ac3",
            PrivateKind::Dts => "dts",
            PrivateKind::Lpcm => "pcm_dvd",
            PrivateKind::Spu => "dvd_subtitle",
        }
    }
}

/// One discovered elementary stream (the pre-scan's registry entry).
#[derive(Debug, Clone, PartialEq, Eq)]
struct PsStream {
    /// The routing key: the PES stream id (+ payload substream for
    /// private-1).
    key: StreamKey,
    /// The registry descriptor string.
    codec: &'static str,
    /// The L0 track kind.
    kind: MediaKind,
    /// First/last PTS stamps seen in the pre-scan (90 kHz ticks).
    first_pts: Option<i64>,
    last_pts: Option<i64>,
}

/// A stream's identity inside the container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamKey {
    /// `0xE0..=0xEF` video.
    Video(u8),
    /// `0xC0..=0xDF` MPEG audio.
    Audio(u8),
    /// `0xBD` + the payload substream byte.
    Private1(PrivateKind, u8),
    /// `0xBF` private stream 2 (DVD nav, PCI/DSI).
    Private2,
}

/// One parsed pack header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PackHeader {
    /// The SCR base, 33 bits (90 kHz ticks).
    scr_base: u64,
    /// The program mux rate (50 bytes/s units), 22 bits.
    mux_rate: u32,
    /// MPEG-2 pack stuffing length (bytes after the 14-byte header).
    stuffing: u8,
    /// MPEG-2 (`true`) or MPEG-1 (`false`) flavor.
    mpeg2: bool,
    /// Absolute end of the pack (header + stuffing).
    end: u64,
}

/// One parsed PES header (both flavors).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PesHeader {
    /// The PES stream id.
    stream_id: u8,
    /// `PES_packet_length` (0 = the unbounded damage case).
    declared_len: u16,
    /// PTS/DTS in 90 kHz ticks, when stamped.
    pts: Option<i64>,
    dts: Option<i64>,
    /// Absolute first payload byte.
    data_start: u64,
    /// Payload bytes available (`declared_len` minus the header).
    payload_len: u64,
}

/// A replayable error shape (the WAV `StickyShape` precedent).
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

/// Wrap a stamp's ticks as a [`Stamp`] (a 33-bit tick value cannot
/// fail `Timestamp::from_parts` under the 1/90 000 base; the
/// fallback leg keeps the construction lint-clean).
fn stamp_of(ticks: Option<i64>) -> Stamp {
    match ticks.and_then(|t| Timestamp::from_parts(t, 1, CLOCK).ok()) {
        Some(ts) => Stamp::Present(ts),
        None => Stamp::Missing,
    }
}

/// Parse one 5-byte PTS/DTS stamp (`'0010'`/`'0011'`/`'0001'`
/// prefix; the 3/15/15 split — identical arithmetic for MPEG-1 and
/// MPEG-2 stamps, pinned against ffmpeg's fixtures).
fn parse_stamp(bytes: &[u8], pos: u64) -> Result<i64, EngineError> {
    if bytes.len() < 5 {
        return Err(error::bad_pes_header(pos));
    }
    let value = (u64::from(bytes[0] >> 1 & 0x07) << 30)
        | (u64::from(bytes[1]) << 22)
        | (u64::from(bytes[2] >> 1) << 15)
        | (u64::from(bytes[3]) << 7)
        | u64::from(bytes[4] >> 1);
    if value >> 33 != 0 {
        return Err(error::bad_pes_header(pos));
    }
    i64::try_from(value).map_err(|_| error::bad_pes_header(pos))
}

/// The MPEG-PS/VOB demuxer.
///
/// Construct with [`PsDemuxer::new`] over a seekable source, then
/// drive the [`Demuxer`] trait: `start` pre-scans the structure and
/// enumerates tracks; `next_event` re-walks and yields PES packets
/// (plus `Recovered`/`EndOfStream`); `seek`/`reanchor` map through
/// pack SCRs (`SeekSupport::SlowLinear` — the container has no
/// index).
///
/// # Example
///
/// ```
/// use demux::ps::PsDemuxer;
/// use plugin_api::{Demuxer, DemuxEvent};
///
/// # fn main() -> Result<(), media_types::EngineError> {
/// # let bytes = demux::ps::test_support::demo_stream();
/// let mut stream = demux::ps::test_support::mem_stream(bytes)?;
/// let mut ps = PsDemuxer::new(stream)?;
/// let tracks = ps.start()?;
/// assert_eq!(tracks.len(), 2); // video + audio
/// assert!(matches!(ps.next_event()?, DemuxEvent::Packet(_)));
/// # Ok(())
/// # }
/// ```
pub struct PsDemuxer {
    reader: SourceReader,
    started: bool,
    eos: bool,
    sticky: Option<StickyShape>,
    /// Discovered streams, first-appearance order.
    streams: Vec<PsStream>,
    /// The `start` result, replayed for observability.
    tracks: Option<Vec<Track>>,
    /// Current pack flavor (set by every pack header).
    mpeg2: bool,
    /// Latest pack's SCR base (ticks).
    scr: u64,
    first_scr: Option<u64>,
    last_scr: Option<u64>,
    /// Next byte the event walk examines.
    walk: u64,
    /// Set by `reanchor`: mark the next packet discontinuous.
    next_discontinuity: bool,
    damage: DamageLog,
    pending: VecDeque<DemuxEvent>,
    pool: PayloadSlots,
}

impl PsDemuxer {
    /// Construct over a seekable, length-known source stream.
    ///
    /// No container I/O here — `start` does the parsing. Rejects
    /// sources that cannot carry a walk: unknown length, unseekable,
    /// empty.
    ///
    /// # Errors
    /// Typed `Unsupported` (stage `Open`) for those three cases.
    pub fn new(stream: Box<dyn plugin_api::SourceStream>) -> Result<PsDemuxer, EngineError> {
        Ok(PsDemuxer {
            reader: SourceReader::new(stream)?,
            started: false,
            eos: false,
            sticky: None,
            streams: Vec::new(),
            tracks: None,
            mpeg2: false,
            scr: 0,
            first_scr: None,
            last_scr: None,
            walk: 0,
            next_discontinuity: false,
            damage: DamageLog::default(),
            pending: VecDeque::new(),
            pool: PayloadSlots::new(2, MAX_PACKET_BYTES as usize),
        })
    }

    /// The discovered streams as `(descriptor, kind)` pairs — the
    /// decode-pending exposure observable (AC-3/DTS/LPCM/SPU ride
    /// here; a caller can show "AC-3 (decode pending)" instead of
    /// wondering why audio is silent).
    #[must_use]
    pub fn stream_descriptors(&self) -> Vec<(&'static str, MediaKind)> {
        self.streams.iter().map(|s| (s.codec, s.kind)).collect()
    }

    /// The latest pack's SCR base (90 kHz ticks); 0 before any pack.
    #[must_use]
    pub fn current_scr(&self) -> u64 {
        self.scr
    }

    /// The first/last SCR the walk has observed (`None` each until
    /// the first pack parses).
    #[must_use]
    pub fn scr_extent(&self) -> (Option<u64>, Option<u64>) {
        (self.first_scr, self.last_scr)
    }

    /// Whether the container's packs are MPEG-2 flavor (VOB/DVD) as
    /// opposed to MPEG-1 system stream.
    #[must_use]
    pub fn is_mpeg2(&self) -> bool {
        self.mpeg2
    }

    /// Total bytes skipped by bounded recovery so far.
    #[must_use]
    pub fn damage_total(&self) -> u64 {
        self.damage.total
    }

    /// The payload bytes resident in the packet pool (the D-1
    /// observable; bounded by `2 × MAX_PACKET_BYTES`).
    #[must_use]
    pub fn pool_bytes(&self) -> usize {
        self.pool.pool_bytes()
    }

    /// Record a fatal error: sticky + returned.
    fn fail(&mut self, err: EngineError) -> EngineError {
        self.sticky = Some(StickyShape::of(&err));
        err
    }

    /// Re-emit a sticky error (idempotent after the first failure).
    fn replay(&self) -> EngineError {
        match &self.sticky {
            Some(shape) => shape.mint(),
            None => error::not_started(),
        }
    }

    /// Parse the pack header at `pos` (both flavors; markers are
    /// validated only where the arithmetic demands it — see the
    /// module docs' convention notes).
    fn parse_pack_at(&mut self, pos: u64) -> Result<PackHeader, EngineError> {
        let mut buf = [0u8; 14];
        let got = self.reader.read_prefix_into(&mut buf, pos)?;
        // The window includes the 4-byte start code: pack fields at
        // buf[4..] (the inherited draft indexed from buf[0] and
        // rejected every real file at byte 0 — fixed this session,
        // pinned by the ffmpeg fixtures).
        let mpeg2 = buf[4] & 0xC0 == 0x40;
        if !mpeg2 && buf[4] & 0xF0 != 0x20 {
            // Neither the MPEG-2 '01' nor the MPEG-1 '0010' prefix.
            return Err(error::bad_pack(pos));
        }
        let need = if mpeg2 { 14 } else { 12 };
        if got < need {
            // A recognized pack cut mid-header: truncation, honest.
            return Err(error::truncated(pos + need as u64, pos + got as u64));
        }
        let (scr_base, stuffing) = if mpeg2 {
            // '01' scr[32..30] M scr[29..28] | scr[27..20] |
            // scr[19..15] M scr[14..13] | scr[12..5] | scr[4..0] M
            // ext[8..6] | ext[5..0] M — the 3/15/15 split, pinned
            // against ffmpeg's `44 00 04 00 04 01` zero-SCR packs.
            let scr_base = (u64::from(buf[4] >> 3 & 0x07) << 30)
                | (u64::from(buf[4] & 0x03) << 28)
                | (u64::from(buf[5]) << 20)
                | (u64::from(buf[6] >> 3) << 15)
                | (u64::from(buf[6] & 0x03) << 13)
                | (u64::from(buf[7]) << 5)
                | u64::from(buf[8] >> 3);
            if scr_base >> 33 != 0 {
                return Err(error::bad_pack(pos));
            }
            (scr_base, buf[13] & 0x07)
        } else {
            // '0010' scr[32..30] M | scr[29..15] M | scr[14..0] M.
            let scr_base = (u64::from(buf[4] >> 1 & 0x07) << 30)
                | (u64::from(buf[5]) << 22)
                | (u64::from(buf[6] >> 1) << 15)
                | (u64::from(buf[7]) << 7)
                | u64::from(buf[8] >> 1);
            if scr_base >> 33 != 0 {
                return Err(error::bad_pack(pos));
            }
            (scr_base, 0)
        };
        let (mux_rate, header_len) = if mpeg2 {
            // buffer bytes 10..12: 22-bit rate + 2 trailing bits
            // (observed `11` in the VOB fixtures — parsed, not
            // enforced: those bits are rate data, the recorded
            // prior-session lesson).
            let mux_rate =
                (u32::from(buf[10]) << 14) | (u32::from(buf[11]) << 6) | u32::from(buf[12] >> 2);
            (mux_rate, 14u64)
        } else {
            // buffer bytes 9..11: rate + trailing bits (observed `01`
            // in the ffmpeg MPEG-1 fixtures vs the spec's `11` — the
            // pinned divergence; parsed, never enforced).
            let mux_rate =
                (u32::from(buf[9]) << 14) | (u32::from(buf[10]) << 6) | u32::from(buf[11] >> 2);
            (mux_rate, 12u64)
        };
        let end = pos + header_len + u64::from(stuffing);
        if end > self.reader.length() {
            return Err(error::truncated(end, self.reader.length()));
        }
        Ok(PackHeader {
            scr_base,
            mux_rate,
            stuffing,
            mpeg2,
            end,
        })
    }

    /// Parse the PES at `pos` (a 4-byte start code + stream id).
    fn parse_pes_full(&mut self, pos: u64, mpeg2: bool) -> Result<PesHeader, EngineError> {
        let mut head = [0u8; 6];
        let got = self.reader.read_prefix_into(&mut head, pos)?;
        if got < 6 {
            return Err(error::truncated(pos + 6, pos + got as u64));
        }
        let stream_id = head[3];
        let declared_len = u16::from_be_bytes([head[4], head[5]]);
        if declared_len == 0 {
            // Unbounded PES: damage-noted skip, never an unbounded
            // read (the v0 scope note).
            self.damage.note(0, "pes-length-zero");
            return Ok(PesHeader {
                stream_id,
                declared_len,
                pts: None,
                dts: None,
                data_start: pos + 6,
                payload_len: 0,
            });
        }
        let packet_end = pos + 6 + u64::from(declared_len);
        if packet_end > self.reader.length() {
            return Err(error::truncated(packet_end, self.reader.length()));
        }
        let (header_len, pts, dts) = if mpeg2 {
            let mut buf = [0u8; MAX_PES_HEADER];
            let got3 = self.reader.read_prefix_into(&mut buf[..3], pos + 6)?;
            if got3 < 3 {
                return Err(error::truncated(pos + 9, pos + 6 + got3 as u64));
            }
            if buf[0] >> 6 != 0b10 {
                return Err(error::bad_pes_header(pos));
            }
            let hl = buf[2];
            let want = 3 + usize::from(hl);
            if want > usize::from(declared_len) {
                // The header overruns its own declared packet.
                return Err(error::bad_pes_header(pos));
            }
            let got_h = self.reader.read_prefix_into(&mut buf[..want], pos + 6)?;
            if got_h < want {
                return Err(error::truncated(
                    pos + 6 + want as u64,
                    pos + 6 + got_h as u64,
                ));
            }
            // PTS_DTS flags: 10 = PTS, 11 = PTS+DTS. The trailing
            // marker bit is NOT enforced (ffmpeg writes both 0 and 1
            // — the pinned convention note).
            let mut p = None;
            let mut d = None;
            match buf[1] >> 6 {
                0b10 => p = Some(parse_stamp(&buf[3..8], pos)?),
                0b11 => {
                    p = Some(parse_stamp(&buf[3..8], pos)?);
                    d = Some(parse_stamp(&buf[8..13], pos)?);
                }
                _ => {}
            }
            (want as u64, p, d)
        } else {
            let mut buf = [0u8; MAX_MPEG1_STUFFING + 10];
            let got_h = self.reader.read_prefix_into(&mut buf, pos + 6)?;
            let mut at = 0usize;
            while at < got_h && buf[at] == 0xFF {
                at += 1;
            }
            if at >= got_h {
                return Err(error::truncated(
                    pos + 6 + at as u64,
                    pos + 6 + got_h as u64,
                ));
            }
            if at > MAX_MPEG1_STUFFING {
                return Err(error::bad_pes_header(pos));
            }
            let mut p = None;
            let mut d = None;
            match buf[at] >> 4 {
                0b0001..=0b0011 => {
                    let wide = buf[at] >> 4 == 0b0011;
                    let need = at + if wide { 10 } else { 5 };
                    if got_h < need {
                        return Err(error::truncated(
                            pos + 6 + need as u64,
                            pos + 6 + got_h as u64,
                        ));
                    }
                    let first = parse_stamp(&buf[at..at + 5], pos)?;
                    if buf[at] >> 4 == 0b0001 {
                        d = Some(first);
                    } else {
                        p = Some(first);
                        if wide {
                            d = Some(parse_stamp(&buf[at + 5..at + 10], pos)?);
                        }
                    }
                    at = need;
                }
                _ => {
                    // The ffmpeg no-stamp marker: '0000' + the low
                    // marker nibble (0x0F observed; a bare 0x00 is
                    // tolerated as unstamped too).
                    if buf[at] & 0x0F != 0x0F && buf[at] != 0x00 {
                        return Err(error::bad_pes_header(pos));
                    }
                    at += 1;
                }
            }
            (at as u64, p, d)
        };
        let data_start = pos + 6 + header_len;
        let payload_len = u64::from(declared_len) - header_len;
        Ok(PesHeader {
            stream_id,
            declared_len,
            pts,
            dts,
            data_start,
            payload_len,
        })
    }

    /// The `start` pre-scan: walk the structure, register streams,
    /// collect timing. Payloads are seek-past except the peeks the
    /// routing needs.
    fn scan_streams(&mut self) -> Result<(), EngineError> {
        let mut pos = 0u64;
        let length = self.reader.length();
        while pos + 4 <= length {
            let mut code = [0u8; 4];
            let got = self.reader.read_prefix_into(&mut code, pos)?;
            if got < 4 {
                break;
            }
            if code[0] != 0 || code[1] != 0 || code[2] != 1 {
                // Damaged bytes: re-synchronize to the next start
                // code inside the window (damage-noted).
                match self.resync_from(pos)? {
                    Some(next) => {
                        self.damage.note(next - pos, "start-code-resync");
                        pos = next;
                        continue;
                    }
                    None => {
                        self.damage.note(length - pos, "trailing-garbage");
                        break;
                    }
                }
            }
            let sid = code[3];
            match sid {
                0xBA => {
                    let pack = self.parse_pack_at(pos)?;
                    self.mpeg2 = pack.mpeg2;
                    self.scr = pack.scr_base;
                    if self.first_scr.is_none() {
                        self.first_scr = Some(pack.scr_base);
                    }
                    self.last_scr = Some(pack.scr_base);
                    pos = pack.end;
                }
                0xB9 => break,
                0xBC | 0xBB | 0xBE | 0xBF => {
                    let ext = self.length_of(pos)?;
                    if sid == 0xBF {
                        self.register_key(StreamKey::Private2, "dvd_nav_packet", MediaKind::Data);
                    }
                    pos = pos + 6 + ext;
                }
                _ => {
                    let pes = self.parse_pes_full(pos, self.mpeg2)?;
                    if pes.declared_len == 0 {
                        pos = pes.data_start;
                        continue;
                    }
                    match sid {
                        0xBD => {
                            let mut sub = [0u8; 1];
                            let got = self.reader.read_prefix_into(&mut sub, pes.data_start)?;
                            if got == 0 {
                                self.damage.note(0, "private1-empty");
                            } else {
                                match PrivateKind::of_substream(sub[0]) {
                                    Some(kind) => {
                                        let kind_audio = if matches!(kind, PrivateKind::Spu) {
                                            MediaKind::Subtitle
                                        } else {
                                            MediaKind::Audio
                                        };
                                        self.register_key(
                                            StreamKey::Private1(kind, sub[0]),
                                            kind.descriptor(),
                                            kind_audio,
                                        );
                                    }
                                    None => self.damage.note(0, "private1-substream-unclassified"),
                                }
                            }
                        }
                        0xE0..=0xEF => {
                            let codec = self.probe_video_codec(pes.data_start, pes.payload_len);
                            self.register_key(StreamKey::Video(sid), codec, MediaKind::Video);
                        }
                        0xC0..=0xDF => {
                            let codec = self.probe_audio_codec(pes.data_start, pes.payload_len);
                            self.register_key(StreamKey::Audio(sid), codec, MediaKind::Audio);
                        }
                        _ => self.damage.note(0, "stream-id-skipped"),
                    }
                    self.note_stamps_for(sid, pes.pts, pes.dts);
                    pos = pes.data_start + pes.payload_len;
                }
            }
        }
        Ok(())
    }

    /// Read the 16-bit length field of a system-header/nav/padding
    /// element at `pos`.
    fn length_of(&mut self, pos: u64) -> Result<u64, EngineError> {
        let mut len = [0u8; 2];
        let got = self.reader.read_prefix_into(&mut len, pos + 4)?;
        if got < 2 {
            return Err(error::truncated(pos + 6, pos + 4 + got as u64));
        }
        Ok(u64::from(u16::from_be_bytes(len)))
    }

    /// Update the matching stream's stamp extent (pre-scan only).
    fn note_stamps_for(&mut self, sid: u8, pts: Option<i64>, dts: Option<i64>) {
        let stamp = pts.or(dts);
        let Some(stamp) = stamp else { return };
        for stream in &mut self.streams {
            let matches = match stream.key {
                StreamKey::Video(id) | StreamKey::Audio(id) => id == sid,
                StreamKey::Private1(..) => sid == 0xBD,
                StreamKey::Private2 => false,
            };
            if matches {
                if stream.first_pts.is_none() {
                    stream.first_pts = Some(stamp);
                }
                stream.last_pts = Some(stamp);
                return;
            }
        }
    }

    /// Register a stream (first appearance wins; duplicates are
    /// no-ops — the pre-scan is idempotent across packs).
    fn register_key(&mut self, key: StreamKey, codec: &'static str, kind: MediaKind) {
        if self.streams.iter().any(|s| s.key == key) {
            return;
        }
        self.streams.push(PsStream {
            key,
            codec,
            kind,
            first_pts: None,
            last_pts: None,
        });
    }

    /// Peek the elementary payload to classify MPEG-1 vs MPEG-2
    /// video (a sequence-extension start code `00 00 01 B5` within
    /// the first 64 bytes ⇒ MPEG-2 — matches ffprobe on both fixture
    /// families).
    fn probe_video_codec(&mut self, data_start: u64, payload_len: u64) -> &'static str {
        let want = payload_len.min(64) as usize;
        let mut buf = [0u8; 64];
        let got = match self.reader.read_prefix_into(&mut buf[..want], data_start) {
            Ok(got) => got,
            Err(_) => return "mpeg2video",
        };
        let window = &buf[..got];
        if window.starts_with(&[0x00, 0x00, 0x01, 0xB3]) {
            for i in 0..got.saturating_sub(3) {
                if window[i..].starts_with(&[0x00, 0x00, 0x01, 0xB5]) {
                    return "mpeg2video";
                }
            }
            return "mpeg1video";
        }
        // No sequence header visible: MPEG-2 is the overwhelmingly
        // common flavor; the ES parser re-classifies downstream.
        "mpeg2video"
    }

    /// Peek the MPEG-audio payload's frame header to classify the
    /// layer (sync `0xFF 0Ex` + layer bits ⇒ mp1/mp2/mp3).
    fn probe_audio_codec(&mut self, data_start: u64, payload_len: u64) -> &'static str {
        if payload_len < 4 {
            return "mp2";
        }
        let mut buf = [0u8; 4];
        let got = match self.reader.read_prefix_into(&mut buf, data_start) {
            Ok(got) => got,
            Err(_) => return "mp2",
        };
        if got < 4 || buf[0] != 0xFF || buf[1] & 0xE0 != 0xE0 {
            return "mp2";
        }
        match buf[1] >> 1 & 0x03 {
            0b11 => "mp1",
            0b01 => "mp3",
            _ => "mp2",
        }
    }

    /// Scan forward from `pos` for the next start code inside the
    /// recovery window; `Ok(None)` = no start code before EOF (the
    /// walk treats the rest as trailing garbage).
    fn resync_from(&mut self, pos: u64) -> Result<Option<u64>, EngineError> {
        let window = MAX_RECOVERY_SCAN_BYTES.min(self.reader.length().saturating_sub(pos));
        if window == 0 {
            return Ok(None);
        }
        let want = window as usize;
        let mut buf = vec![0u8; want]; // one bounded scan allocation
        let got = self.reader.read_prefix_into(&mut buf, pos)?;
        let mut i = 0usize;
        while i + 3 < got {
            if buf[i] == 0 && buf[i + 1] == 0 && buf[i + 2] == 1 {
                return Ok(Some(pos + i as u64));
            }
            i += 1;
        }
        if got < want {
            // The stream ends inside the window without a start
            // code: trailing garbage, not a resync failure.
            return Ok(None);
        }
        Err(error::resync_failed(pos))
    }

    /// Find the registry index for `sid` (+ private-1 substream).
    fn stream_index_for(&self, sid: u8, substream: Option<u8>) -> Option<usize> {
        self.streams.iter().position(|s| match s.key {
            StreamKey::Video(id) | StreamKey::Audio(id) => id == sid,
            StreamKey::Private1(kind, sub) => {
                sid == 0xBD
                    && Some(sub) == substream
                    && PrivateKind::of_substream(sub) == Some(kind)
            }
            StreamKey::Private2 => sid == 0xBF,
        })
    }

    /// The tracks for the trait's `start` (rebuilt from the stream
    /// registry + timing).
    fn build_tracks(&self) -> Result<Vec<Track>, EngineError> {
        let base = TimeBase::new(1, CLOCK)?;
        let mut tracks = Vec::new();
        for (index, stream) in self.streams.iter().enumerate() {
            let handle = u32::try_from(index).unwrap_or(u32::MAX);
            let span = match (stream.first_pts, stream.last_pts) {
                (Some(first), Some(last)) if last >= first => TimeSpan {
                    start: stamp_of(Some(first)),
                    end: stamp_of(Some(last)),
                },
                _ => TimeSpan::UNKNOWN,
            };
            tracks.push(Track {
                id: TrackId::new(handle),
                stream: StreamId::new(handle),
                kind: stream.kind,
                language: None,
                codec: CodecDescriptor::new(stream.codec)?,
                title: None,
                flags: TrackFlags::NONE,
                timing: TrackTiming {
                    span,
                    base,
                    gaps: Vec::new(),
                },
            });
        }
        Ok(tracks)
    }

    /// Read one PES payload (post-strip) into the pool and queue it
    /// as a packet.
    fn emit_pes_packet(
        &mut self,
        pes: &PesHeader,
        track_index: usize,
        strip: u64,
        keyframe: bool,
    ) -> Result<(), EngineError> {
        let start = pes.data_start + strip;
        let want = pes.payload_len.saturating_sub(strip);
        if want > MAX_PACKET_BYTES {
            // Unreachable for a u16-declared packet; kept as the
            // ceiling's typed flavor.
            return Err(error::payload_too_large(MAX_PACKET_BYTES, want));
        }
        let at = start;
        let data = self
            .pool
            .fill(want as usize, |buf| self.reader.read_prefix_into(buf, at))?;
        let got = data.len();
        if got == 0 {
            self.damage.note(want, "pes-payload-empty");
            return Ok(());
        }
        if (got as u64) < want {
            self.damage.note(want - got as u64, "pes-payload-short");
        }
        let discontinuity = self.next_discontinuity;
        self.next_discontinuity = false;
        let packet = Packet {
            track: TrackId::new(u32::try_from(track_index).unwrap_or(u32::MAX)),
            pts: stamp_of(pes.pts),
            dts: stamp_of(pes.dts),
            duration: Some(Duration::Unknown),
            data,
            keyframe,
            discontinuity,
            generation: GenerationId::FIRST,
        };
        self.pending.push_back(DemuxEvent::Packet(packet));
        Ok(())
    }
}

impl Demuxer for PsDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if self.sticky.is_some() {
            return Err(self.replay());
        }
        if self.started {
            // Idempotent: replay the enumeration.
            return match &self.tracks {
                Some(tracks) => Ok(tracks.clone()),
                None => Err(self.replay()),
            };
        }
        // The head must be a pack start code (the probe contract);
        // anything else never promised MPEG-PS.
        let mut head = [0u8; 4];
        let got = self.reader.read_prefix_into(&mut head, 0)?;
        if head != [0x00, 0x00, 0x01, 0xBA] {
            let sniffed = &head[..got];
            return Err(self.fail(error::not_ps(sniffed)));
        }
        if let Err(err) = self.scan_streams() {
            return Err(self.fail(err));
        }
        let tracks = match self.build_tracks() {
            Ok(tracks) => tracks,
            Err(err) => return Err(self.fail(err)),
        };
        self.tracks = Some(tracks.clone());
        self.started = true;
        // Surface the pre-scan's damage decisions before packets.
        for (skipped, reason) in std::mem::take(&mut self.damage.events) {
            self.pending
                .push_back(DemuxEvent::Recovered { skipped, reason });
        }
        Ok(tracks)
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if self.sticky.is_some() {
            return Err(self.replay());
        }
        if !self.started {
            return Err(error::not_started());
        }
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Ok(event);
            }
            if self.eos {
                return Ok(DemuxEvent::EndOfStream);
            }
            let length = self.reader.length();
            if self.walk + 4 > length {
                // The stream ends without a program-end code: the
                // walk's own extent is the flush endpoint.
                if self.walk < length {
                    self.damage.note(length - self.walk, "trailing-garbage");
                }
                self.eos = true;
                for (skipped, reason) in std::mem::take(&mut self.damage.events) {
                    self.pending
                        .push_back(DemuxEvent::Recovered { skipped, reason });
                }
                continue;
            }
            let mut code = [0u8; 4];
            let got = self.reader.read_prefix_into(&mut code, self.walk)?;
            if got < 4 {
                // EOF mid-start-code: flush any pending damage notes
                // (the never-silent rule) and end.
                if self.walk + (got as u64) < length {
                    self.damage.note(length - self.walk, "trailing-garbage");
                }
                self.eos = true;
                for (skipped, reason) in std::mem::take(&mut self.damage.events) {
                    self.pending
                        .push_back(DemuxEvent::Recovered { skipped, reason });
                }
                continue;
            }
            if code[0] != 0 || code[1] != 0 || code[2] != 1 {
                match self.resync_from(self.walk)? {
                    Some(next) => {
                        self.damage.note(next - self.walk, "start-code-resync");
                        self.walk = next;
                        for (skipped, reason) in std::mem::take(&mut self.damage.events) {
                            self.pending
                                .push_back(DemuxEvent::Recovered { skipped, reason });
                        }
                        continue;
                    }
                    None => {
                        self.damage.note(length - self.walk, "trailing-garbage");
                        self.eos = true;
                        continue;
                    }
                }
            }
            let sid = code[3];
            match sid {
                0xBA => {
                    let pack = self.parse_pack_at(self.walk)?;
                    self.mpeg2 = pack.mpeg2;
                    self.scr = pack.scr_base;
                    if self.first_scr.is_none() {
                        self.first_scr = Some(pack.scr_base);
                    }
                    self.last_scr = Some(pack.scr_base);
                    self.walk = pack.end;
                }
                0xB9 => {
                    self.eos = true;
                    for (skipped, reason) in std::mem::take(&mut self.damage.events) {
                        self.pending
                            .push_back(DemuxEvent::Recovered { skipped, reason });
                    }
                }
                0xBC | 0xBB | 0xBE => {
                    let ext = self.length_of(self.walk)?;
                    if sid == 0xBE {
                        self.damage.note(ext, "padding-stream");
                    }
                    self.walk += 6 + ext;
                }
                0xBF => {
                    // VOB nav pack payload (PCI/DSI): a Data-track
                    // packet when the stream is registered, else a
                    // positional skip.
                    let declared = self.length_of(self.walk)? as u16;
                    match self.stream_index_for(sid, None) {
                        Some(index) => {
                            let pes = PesHeader {
                                stream_id: sid,
                                declared_len: declared,
                                pts: None,
                                dts: None,
                                data_start: self.walk + 6,
                                payload_len: u64::from(declared),
                            };
                            self.emit_pes_packet(&pes, index, 0, true)?;
                            self.walk = pes.data_start + pes.payload_len;
                        }
                        None => {
                            self.damage.note(u64::from(declared), "nav-unregistered");
                            self.walk += 6 + u64::from(declared);
                        }
                    }
                }
                _ => {
                    let pes = self.parse_pes_full(self.walk, self.mpeg2)?;
                    if pes.declared_len == 0 {
                        self.walk = pes.data_start;
                        continue;
                    }
                    let substream = if sid == 0xBD {
                        let mut sub = [0u8; 1];
                        let got = self.reader.read_prefix_into(&mut sub, pes.data_start)?;
                        if got == 0 { None } else { Some(sub[0]) }
                    } else {
                        None
                    };
                    match self.stream_index_for(sid, substream) {
                        Some(index) => {
                            let key = self.streams[index].key;
                            let (strip, keyframe) = match key {
                                StreamKey::Private1(kind, _) => (kind.strip(), true),
                                StreamKey::Audio(_) | StreamKey::Private2 => (0, true),
                                StreamKey::Video(_) => (0, false),
                            };
                            self.emit_pes_packet(&pes, index, strip, keyframe)?;
                        }
                        None => {
                            let reason: &'static str = if sid == 0xBD {
                                "private1-substream-unregistered"
                            } else {
                                "stream-id-unregistered"
                            };
                            self.damage.note(pes.payload_len, reason);
                        }
                    }
                    self.walk = pes.data_start + pes.payload_len;
                    if !self.pending.is_empty() {
                        for (skipped, reason) in std::mem::take(&mut self.damage.events) {
                            self.pending
                                .push_back(DemuxEvent::Recovered { skipped, reason });
                        }
                    }
                }
            }
        }
    }

    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
        let Stamp::Present(start) = target.start else {
            return Err(error::seek_out_of_range());
        };
        let base = TimeBase::new(1, CLOCK)?;
        let ticks = start
            .convert(&base)
            .map_err(|_| error::seek_out_of_range())?
            .ticks;
        let last = self.last_scr.unwrap_or(0);
        if ticks < 0 {
            return Err(error::seek_out_of_range());
        }
        let clamped = u64::try_from(ticks).unwrap_or(0).min(last);
        Ok(TimeSpan {
            start: stamp_of(Some(clamped as i64)),
            end: Stamp::Missing,
        })
    }

    fn seekability(&self) -> SeekSupport {
        SeekSupport::SlowLinear
    }

    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
        let Stamp::Present(start) = target.start else {
            return Err(error::seek_out_of_range());
        };
        let base = TimeBase::new(1, CLOCK)?;
        let ticks = start
            .convert(&base)
            .map_err(|_| error::seek_out_of_range())?
            .ticks;
        if ticks < 0 {
            return Err(error::seek_out_of_range());
        }
        let want = u64::try_from(ticks).unwrap_or(0);
        // Compute-then-commit (invariant D4): find the pack first,
        // then move the walk.
        let mut pos = 0u64;
        let mut best: Option<(u64, u64)> = None; // (scr, pack end)
        while pos + 4 <= self.reader.length() {
            let mut code = [0u8; 4];
            let got = self.reader.read_prefix_into(&mut code, pos)?;
            if got < 4 {
                break;
            }
            if code[0] != 0 || code[1] != 0 || code[2] != 1 {
                match self.resync_from(pos)? {
                    Some(next) => {
                        pos = next;
                        continue;
                    }
                    None => break,
                }
            }
            match code[3] {
                0xBA => {
                    let pack = self.parse_pack_at(pos)?;
                    if pack.scr_base <= want {
                        best = Some((pack.scr_base, pack.end));
                        pos = pack.end;
                    } else {
                        break;
                    }
                }
                0xB9 => break,
                0xBC | 0xBB | 0xBE | 0xBF => {
                    let ext = match self.length_of(pos) {
                        Ok(ext) => ext,
                        Err(_) => break,
                    };
                    pos += 6 + ext;
                }
                _ => {
                    let pes = match self.parse_pes_full(pos, self.mpeg2) {
                        Ok(pes) => pes,
                        Err(_) => break,
                    };
                    if pes.declared_len == 0 {
                        break;
                    }
                    pos = pes.data_start + pes.payload_len;
                }
            }
        }
        let (scr, end) = best.unwrap_or((0, 0));
        // Commit: re-anchor the walk, mark the next packet.
        self.walk = end;
        self.eos = false;
        self.next_discontinuity = true;
        self.pending.clear();
        Ok(TimeSpan {
            start: stamp_of(Some(scr as i64)),
            end: Stamp::Missing,
        })
    }
}
