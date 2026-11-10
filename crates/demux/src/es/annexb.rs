//! Annex-B raw video elementary streams (card 5.I3a): H.264 `.264`
//! and HEVC `.265` file access over the landed
//! [`Demuxer`](plugin_api::Demuxer) contract.
//!
//! # What this owns
//!
//! The start-code byte stream (ITU-T H.264 Annex B / ITU-T H.265
//! Annex B): `00 00 01` / `00 00 00 01`-delimited NAL units, grouped
//! into **access units** (one demuxed packet per picture) by a
//! NAL-type-aware boundary rule — the rule ffmpeg's own raw-ES demux
//! path exercises, pinned empirically against the corpus
//! differential, not guessed:
//!
//! - a **non-VCL prefix NAL** following a VCL NAL opens a new access
//!   unit (the SPS/PPS/SEI re- declarations between pictures glue to
//!   the *next* picture — measured: ffmpeg's packet 19 in
//!   `0019-raw-video.h264` starts at the mid-stream SPS, not at the
//!   slice after it);
//! - a **VCL NAL that starts a new picture** following a VCL NAL
//!   opens a new access unit: H.264 `first_mb_in_slice == 0`
//!   (readable as the first slice-header bit — `ue(0)` is the single
//!   `1` bit, and a VCL header byte is never `0x00`, so the bit is
//!   escape-free), HEVC `first_slice_segment_in_pic_flag` (bit 15 of
//!   the 2-byte NAL header).
//!
//! The prefix-class sets stay deliberately narrow (H.264 SEI/SPS/PPS/
//! AUD/EOS/EOS-stream/SPS-ext/prefix/subset-SPS; HEVC VPS/SPS/PPS/
//! AUD/EOS/EOB/prefix-SEI): filler data (12 / 38) and HEVC *suffix*
//! SEI (40) trail a picture instead of opening one.
//!
//! # Timing honesty
//!
//! Raw Annex-B streams carry **no timestamps** — the SPS VUI timing
//! info is decode-domain (owned by the landed codecs-side parser,
//! card 5.F1). Packets therefore carry [`Stamp::Missing`] pts/dts
//! (the L0 vocabulary's own "absent for some raw streams" case) and
//! the track's span is missing; the track time base is minted at
//! ffmpeg's documented raw-video fallback (1/25) purely so
//! [`TrackTiming`] has a base, and the module never *derives* any
//! stamp from it.
//!
//! # Memory + bounds
//!
//! The scan is a forward windowed walk ([`WINDOW`] bytes + a
//! [`OVERLAP`] carry): each `next_event` reads only what it needs to
//! reach the next boundary, never the whole file, and only NAL
//! headers are classified (a multi-megabyte NAL body is skipped over
//! without reading it). Packet payloads land in a 2-slot
//! [`PayloadSlots`] pool capped at [`MAX_AU_BYTES`]; a larger access
//! unit is a typed [`ResourceLimit`] refusal, never an unbounded
//! allocation (MP §12). Leading garbage before the first start code
//! rides in the first packet's head (ffmpeg's own packet shape,
//! measured: packet 1 of `0019` is `[0, 1135)` with a stray byte 0).
//!
//! # Truncation
//!
//! Annex-B declares no lengths, so a mid-NAL cut at end-of-file is
//! not demux-detectable: the final partial access unit is emitted
//! (ffmpeg's behavior) and the divergence from the typed-refusal
//! discipline the audio ES formats can afford is documented here.

use std::mem;

use media_types::ids::{CodecDescriptor, GenerationId, MediaKind, StreamId, TrackFlags, TrackId};
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, TimeBase, TimeSpan, TrackTiming};
use media_types::{EngineError, Packet, Track};
use plugin_api::{DemuxEvent, Demuxer};

use crate::avi::header::DamageLog;
use crate::avi::riff::{PayloadSlots, SourceReader};

/// Maximum bytes one access-unit packet may span (bounded memory).
pub const MAX_AU_BYTES: u64 = 16 * 1024 * 1024;

/// The forward scan window (bytes read per scan step, bounded).
const WINDOW: u64 = 256 * 1024;

/// Carry between windows: ≥ a 4-byte start code + a 2-byte NAL header
/// + slack, so no start code or header straddles two windows unseen.
const OVERLAP: u64 = 32;

/// Maximum bytes the `start` probe walks hunting the first NAL.
pub const MAX_OPEN_SCAN_BYTES: u64 = 64 * 1024;

/// The two Annex-B codecs this module demuxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnexbCodec {
    /// H.264 / AVC (`.h264`, `.264`): 1-byte NAL headers, Table 7-1.
    H264,
    /// H.265 / HEVC (`.h265`, `.265`, `.hevc`): 2-byte NAL headers.
    Hevc,
}

impl AnnexbCodec {
    /// NAL header bytes (§7.3.1 H.264; §7.3.1 H.265).
    #[must_use]
    fn header_len(self) -> usize {
        match self {
            AnnexbCodec::H264 => 1,
            AnnexbCodec::Hevc => 2,
        }
    }

    /// The `nal_unit_type` bits of a complete NAL header.
    #[must_use]
    fn nal_type(self, head: &[u8]) -> u8 {
        match self {
            AnnexbCodec::H264 => head[0] & 0x1F,
            AnnexbCodec::Hevc => head[0] >> 1 & 0x3F,
        }
    }

    /// VCL = coded picture data (H.264 1..=5; H.265 0..=31).
    #[must_use]
    fn is_vcl(self, t: u8) -> bool {
        match self {
            AnnexbCodec::H264 => (1..=5).contains(&t),
            AnnexbCodec::Hevc => t <= 31,
        }
    }

    /// Intra/random-access point (H.264 IDR only; H.265 IRAP band).
    #[must_use]
    fn is_irap(self, t: u8) -> bool {
        match self {
            AnnexbCodec::H264 => t == 5,
            AnnexbCodec::Hevc => (16..=23).contains(&t),
        }
    }

    /// Non-VCL types that open a new access unit when they follow VCL.
    #[must_use]
    fn is_prefix_class(self, t: u8) -> bool {
        match self {
            AnnexbCodec::H264 => matches!(t, 6 | 7 | 8 | 9 | 10 | 11 | 13 | 14 | 15),
            AnnexbCodec::Hevc => matches!(t, 32..=37 | 39),
        }
    }

    /// Whether a VCL NAL header opens a new picture: H.264
    /// `first_mb_in_slice == 0` (the first slice-header payload bit —
    /// `ue(0)` is a lone `1`); H.265
    /// `first_slice_segment_in_pic_flag` (bit 15 of the 2-byte NAL
    /// header). Both bits are read from `head[1]` — H.264's classify
    /// window extends one payload byte past the 1-byte header, and a
    /// VCL header byte is never `0x00` in either codec, so emulation
    /// prevention cannot have altered the bit.
    #[must_use]
    fn starts_new_picture(self, head: &[u8]) -> bool {
        head.len() > 1 && head[1] & 0x80 != 0
    }

    /// The classify window: NAL header bytes, plus H.264's first
    /// slice-header payload byte (the `first_mb_in_slice` probe).
    #[must_use]
    fn classify_len(self) -> usize {
        match self {
            AnnexbCodec::H264 => 2,
            AnnexbCodec::Hevc => 2,
        }
    }

    /// The registry codec descriptor (the mkv/mp4 vocabulary).
    fn descriptor(self) -> Result<CodecDescriptor, EngineError> {
        let name = match self {
            AnnexbCodec::H264 => "h264",
            AnnexbCodec::Hevc => "hevc",
        };
        // Static ASCII literal: the grammar cannot refuse it; the
        // typed error is the panic-free residue (8.A1 sweep).
        CodecDescriptor::new(name).map_err(|_| super::error::bad_header(0))
    }

    /// The format name for typed errors.
    #[must_use]
    fn format(self) -> &'static str {
        match self {
            AnnexbCodec::H264 => "h264",
            AnnexbCodec::Hevc => "hevc",
        }
    }
}

/// The honest confidence probe for a header window: an Annex-B start
/// code whose NAL type is plausible for the codec (H.264 VCL/SEI/SPS/
/// PPS/AUD; HEVC VCL 0..=31 or VPS/SPS/PPS 32..=34).
#[must_use]
pub fn probe(codec: AnnexbCodec, header: &[u8]) -> bool {
    let hl = codec.header_len();
    let mut i = 0usize;
    while i + 3 + hl <= header.len() {
        if header[i] == 0 && header[i + 1] == 0 && header[i + 2] == 1 {
            let t = codec.nal_type(&header[i + 3..i + 3 + hl]);
            let plausible = match codec {
                AnnexbCodec::H264 => (1..=5).contains(&t) || matches!(t, 6..=9),
                AnnexbCodec::Hevc => t <= 34,
            };
            if plausible {
                return true;
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    false
}

/// The forward AU-boundary scan state (one per demuxer).
struct AuScan {
    codec: AnnexbCodec,
    /// Where the next scan window starts.
    pos: u64,
    /// Just past the last classified NAL's header: the zero-walk-back
    /// clamp (a start code's leading zeros never eat a NAL header).
    floor: u64,
    /// The last classified NAL's start-code offset (window-overlap
    /// dedup).
    last_code: u64,
    /// The last classified NAL was VCL.
    prev_vcl: bool,
    /// Any NAL classified yet (nothing precedes the first one).
    seen: bool,
    /// The access unit being scanned holds an IRAP NAL.
    au_irap: bool,
}

impl AuScan {
    fn new(codec: AnnexbCodec) -> AuScan {
        AuScan {
            codec,
            pos: 0,
            floor: 0,
            last_code: 0,
            prev_vcl: false,
            seen: false,
            au_irap: false,
        }
    }

    /// Scan forward for the next access-unit boundary.
    ///
    /// `Ok(Some((at, irap)))` — the boundary byte offset (the first
    /// zero of the opening NAL's start-code run) and whether the
    /// access unit being *closed* holds an IRAP. `Ok(None)` — end of
    /// stream; the caller closes the final AU with [`AuScan::au_irap`].
    fn next_boundary(
        &mut self,
        reader: &mut SourceReader,
    ) -> Result<Option<(u64, bool)>, EngineError> {
        let len = reader.length();
        while self.pos < len {
            let win_end = (self.pos + WINDOW).min(len);
            let win = reader.read_prefix_at((win_end - self.pos) as usize, self.pos)?;
            let origin = self.pos;
            let cl = self.codec.classify_len();
            let mut i = 0usize;
            while i + 3 + cl <= win.len() {
                if win[i] == 0 && win[i + 1] == 0 && win[i + 2] == 1 {
                    let code_at = origin + i as u64;
                    if code_at > self.last_code {
                        let head = &win[i + 3..i + 3 + cl];
                        let t = self.codec.nal_type(head);
                        let vcl = self.codec.is_vcl(t);
                        let opens = self.seen
                            && self.prev_vcl
                            && (vcl && self.codec.starts_new_picture(head)
                                || self.codec.is_prefix_class(t));
                        if opens {
                            // Walk back over the start code's leading
                            // zero run (4-byte codes, zero_byte runs);
                            // never past the previous NAL's header.
                            let lo = self.floor.max(origin);
                            let mut at = code_at;
                            while at > lo && win[(at - origin - 1) as usize] == 0 {
                                at -= 1;
                            }
                            let closed = mem::replace(&mut self.au_irap, self.codec.is_irap(t));
                            self.prev_vcl = vcl;
                            self.seen = true;
                            self.floor = code_at + 3 + cl as u64;
                            self.last_code = code_at;
                            self.pos = self.floor;
                            return Ok(Some((at, closed)));
                        }
                        if vcl && self.codec.is_irap(t) {
                            self.au_irap = true;
                        }
                        self.prev_vcl = vcl;
                        self.seen = true;
                        self.floor = code_at + 3 + cl as u64;
                        self.last_code = code_at;
                    }
                    i += 3;
                } else {
                    i += 1;
                }
            }
            // No boundary in this window: carry the overlap forward,
            // never behind the last classified NAL.
            self.pos = if win_end >= len {
                len
            } else {
                (win_end - OVERLAP).max(self.floor).min(win_end)
            };
        }
        Ok(None)
    }
}

/// A replayable error shape (the WAV/Ogg/ADTS `StickyShape` precedent).
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

/// The Annex-B raw video demuxer (H.264 and HEVC; see the module
/// docs for the boundary rule, timing honesty and bounds).
///
/// Construct with [`AnnexbVideoDemuxer::new`] over a seekable,
/// length-known source; `start` probes for the first NAL, mints the
/// single video track and refuses typed when no sync structure is
/// found; `next_event` walks one access unit per call. Seeks are the
/// honest typed refusal (no timing exists to map a target onto).
pub struct AnnexbVideoDemuxer {
    reader: SourceReader,
    scan: AuScan,
    started: bool,
    eos: bool,
    sticky: Option<StickyShape>,
    track: Option<Track>,
    /// First byte of the access unit being emitted (0 for the first).
    au_start: u64,
    aus_out: u64,
    pool: PayloadSlots,
    damage: DamageLog,
    /// The one packet `next_event` is about to hand out.
    pending: Option<DemuxEvent>,
}

impl AnnexbVideoDemuxer {
    /// Construct over a seekable, length-known source stream.
    ///
    /// # Errors
    /// Typed `Unsupported` (stage `Open`) for unknown-length,
    /// unseekable or empty sources (the AVI/WAV/Ogg/ADTS precedent).
    pub fn new(
        codec: AnnexbCodec,
        stream: Box<dyn plugin_api::SourceStream>,
    ) -> Result<AnnexbVideoDemuxer, EngineError> {
        let reader = SourceReader::new(stream)?;
        Ok(AnnexbVideoDemuxer {
            reader,
            scan: AuScan::new(codec),
            started: false,
            eos: false,
            sticky: None,
            track: None,
            au_start: 0,
            aus_out: 0,
            pool: PayloadSlots::new(2, MAX_AU_BYTES.min(u32::MAX as u64) as usize),
            damage: DamageLog::default(),
            pending: None,
        })
    }

    /// Access units demuxed so far (the count differential's truth).
    #[must_use]
    pub fn aus_demuxed(&self) -> u64 {
        self.aus_out
    }

    /// Total bytes logged as damage (always 0 today — Annex-B damage
    /// is not byte-attributable; kept for parity with the siblings).
    #[must_use]
    pub fn damage_total(&self) -> u64 {
        self.damage.total
    }

    fn fail(&mut self, err: EngineError) -> EngineError {
        self.sticky = Some(StickyShape::of(&err));
        err
    }

    /// The honest confidence probe (the shared [`probe`] over the
    /// open-scan window).
    fn probe(&mut self) -> Result<(), EngineError> {
        let len = self.reader.length();
        let win = self
            .reader
            .read_prefix_at(MAX_OPEN_SCAN_BYTES.min(len) as usize, 0)?;
        if probe(self.scan.codec, &win) {
            Ok(())
        } else {
            Err(super::error::no_sync(self.scan.codec.format(), &win))
        }
    }

    /// Mint the single video track (timing honesty: the module docs).
    fn mint_track(&mut self) -> Result<Track, EngineError> {
        // 1/25 fps default (coprime, non-zero): exactly
        // `TimeBase::new(1, 25)`'s Ok value, constructed directly —
        // the DVB-builder precedent (8.A1 sweep).
        let base = TimeBase { num: 1, den: 25 };
        let track = Track {
            id: TrackId::new(0),
            stream: StreamId::new(0),
            kind: MediaKind::Video,
            language: None,
            codec: self.scan.codec.descriptor()?,
            title: None,
            flags: TrackFlags::NONE,
            timing: TrackTiming {
                span: TimeSpan {
                    start: Stamp::Missing,
                    end: Stamp::Missing,
                },
                base,
                gaps: Vec::new(),
            },
        };
        self.track = Some(track.clone());
        Ok(track)
    }

    /// Emit the access unit `[from, to)` as one packet.
    fn emit_au(&mut self, from: u64, to: u64, keyframe: bool) -> Result<(), EngineError> {
        let track = self
            .track
            .clone()
            .ok_or_else(|| self.fail(super::error::not_started()))?;
        let size = to - from;
        if size > MAX_AU_BYTES {
            return Err(self.fail(super::error::au_too_large(size)));
        }
        let data: SharedBytes = self
            .pool
            .fill(size as usize, |buf| self.reader.read_prefix_into(buf, from))?;
        let packet = Packet {
            track: track.id,
            pts: Stamp::Missing,
            dts: Stamp::Missing,
            duration: None,
            data,
            keyframe,
            discontinuity: false,
            generation: GenerationId::FIRST,
        };
        self.aus_out += 1;
        self.au_start = to;
        self.pending = Some(DemuxEvent::Packet(packet));
        Ok(())
    }
}

// One packet per `next_event`; the siblings' event queue is
// unnecessary — this demuxer never queues recovery events.
impl AnnexbVideoDemuxer {
    fn take_pending(&mut self) -> Option<DemuxEvent> {
        self.pending.take()
    }
}

impl Demuxer for AnnexbVideoDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if self.started {
            return Ok(vec![
                self.track
                    .clone()
                    .ok_or_else(|| self.fail(super::error::not_started()))?,
            ]);
        }
        self.probe()?;
        self.started = true;
        Ok(vec![self.mint_track()?])
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if let Some(err) = &self.sticky {
            return Err(err.mint());
        }
        if !self.started {
            return Err(super::error::not_started());
        }
        if self.eos {
            return Ok(DemuxEvent::EndOfStream);
        }
        if let Some(event) = self.take_pending() {
            return Ok(event);
        }
        let from = self.au_start;
        match self.scan.next_boundary(&mut self.reader)? {
            Some((at, closed_irap)) => {
                self.emit_au(from, at, closed_irap)?;
                Ok(self.take_pending().unwrap_or(DemuxEvent::EndOfStream))
            }
            None => {
                let len = self.reader.length();
                if len > from {
                    self.emit_au(from, len, self.scan.au_irap)?;
                    self.eos = true;
                    Ok(self.take_pending().unwrap_or(DemuxEvent::EndOfStream))
                } else {
                    self.eos = true;
                    Ok(DemuxEvent::EndOfStream)
                }
            }
        }
    }

    fn seek(
        &mut self,
        target: media_types::time::TimeSpan,
    ) -> Result<media_types::time::TimeSpan, EngineError> {
        let _ = target;
        Err(EngineError::new(
            media_types::PipelineStage::Demux,
            media_types::ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("demux.es.seek.unsupported"),
        ))
    }
}
