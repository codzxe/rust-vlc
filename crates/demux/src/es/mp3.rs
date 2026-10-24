//! Raw MPEG-audio elementary streams (card 5.I3a): `.mp3` (and the
//! `.mp2`/Layer I-II shapes the same header grammar covers) over the
//! landed [`Demuxer`](plugin_api::Demuxer) contract.
//!
//! # What this owns
//!
//! The demux-side thin walk: an ID3v2.2/2.3/2.4 tag skip at the head
//! (and between frames), the 4-byte frame header (ISO/IEC 11172-3
//! §3.1 / 13818-3 §2.1) with the version/layer/bitrate/samplerate
//! tables, and the frame arithmetic that turns a header into an exact
//! packet boundary. The tables are the published ISO tables — the
//! same ones the codecs-side MP3 module pins (card 5.A3, read-only
//! consume: this crate cannot depend on codecs, layer ordering); the
//! Xing/Info/VBRI and gapless surfaces stay with the decoder (they
//! are payload, not framing).
//!
//! # Framing
//!
//! One frame = one packet (the whole frame, header included — the
//! engine's own packet shape, matching the ADTS precedent). Timing is
//! exact sample arithmetic: `pts = Σ samples_per_frame` on a
//! `1/sample_rate` time base, `duration` per packet; a parameter
//! change mid-stream (rate) is surfaced as [`DemuxEvent::Discontinuity`]
//! — the ADTS precedent.
//!
//! # Damage policy
//!
//! The first frame is located by a bounded sync scan and validated
//! strictly; a file with no parseable header inside
//! [`MAX_OPEN_SCAN_BYTES`] is the typed `Unsupported`-after-scan
//! refusal. Mid-stream, a lost header triggers a bounded resync scan
//! (the next parseable header; `Recovered` event), and a frame whose
//! computed size overruns end-of-stream is the typed sticky
//! truncation (the ADTS divergence note applies: ffmpeg's raw mp3
//! demuxer emits a short final packet instead).
//!
//! # The 0145 honesty note
//!
//! An ID3v2 `APIC` (cover art) frame is metadata this demuxer skips
//! with the tag; ffmpeg additionally synthesizes an attached-picture
//! video stream from it (measured: `0145-cover-audio.mp3` reports a
//! second `mjpeg` track + 1 packet). That synthesis is player
//! behavior, not framing — recorded in the worklog table and
//! deliberately not mirrored here.

use std::collections::VecDeque;

use media_types::error::ErrorContext;
use media_types::ids::{CodecDescriptor, GenerationId, MediaKind, StreamId, TrackFlags, TrackId};
use media_types::packet::SharedBytes;
use media_types::time::{Duration, Nanoseconds, Stamp, TimeBase, TimeSpan, Timestamp, TrackTiming};
use media_types::{EngineError, Packet, Track};
use plugin_api::{DemuxEvent, Demuxer};

use crate::avi::header::DamageLog;
use crate::avi::riff::{PayloadSlots, SourceReader};

/// Maximum bytes the open scan walks hunting the first frame header.
pub const MAX_OPEN_SCAN_BYTES: u64 = 64 * 1024;

/// Maximum bytes the resync scan reads hunting the next header.
pub const MAX_RECOVERY_SCAN_BYTES: u64 = 4 * 1024 * 1024;

/// Maximum bytes of ID3v2 tag trusted and skipped (ceiling-checked).
pub const MAX_ID3_BYTES: u64 = 16 * 1024 * 1024;

/// Maximum frame payload trusted into memory (the largest table-
/// derived frame is 2881 bytes; 4 KiB is the pool ceiling).
pub const MAX_FRAME_BYTES: usize = 4096;

/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["mp3", "mp2"];

/// MPEG audio version from the header's version bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpegVersion {
    /// MPEG-2.5 (extension, `00`).
    V2_5,
    /// Reserved (`01`) — never a valid stream.
    Reserved,
    /// MPEG-2 (`10`).
    V2,
    /// MPEG-1 (`11`).
    V1,
}

/// MPEG audio layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// Layer I (`11`): 384 samples/frame, 4-byte slots.
    L1,
    /// Layer II (`10`, MP2): 1152 samples/frame.
    L2,
    /// Layer III (`01`, MP3): 1152 (MPEG-1) / 576 (MPEG-2/2.5).
    L3,
    /// Reserved (`00`) — rejected.
    Reserved,
}

/// One parsed (validated) frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHead {
    /// MPEG version.
    pub version: MpegVersion,
    /// Layer.
    pub layer: Layer,
    /// Bitrate in bits/second (table value; free format is refused).
    pub bitrate_bps: u32,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Padding slot flag.
    pub padding: bool,
    /// True when a CRC-16 word follows the header (protection bit 0).
    pub protected_by_crc: bool,
    /// Channel count (mono 1, everything else 2).
    pub channels: u16,
    /// Frame byte length including the 4 header bytes.
    pub frame_size: u32,
    /// Samples per frame per channel.
    pub samples_per_frame: u32,
}

// The published ISO tables (11172-3/13818-3), indexed exactly as the
// header bits order them (the codecs-side mp3 module's read-only
// twin; layer ordering forbids sharing the crate).
const BITRATES_V1_L1: [u32; 14] = [
    32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
];
const BITRATES_V1_L2: [u32; 14] = [
    32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
];
const BITRATES_V1_L3: [u32; 14] = [
    32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
];
const BITRATES_V2_L1: [u32; 14] = [
    32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
];
const BITRATES_V2_L23: [u32; 14] = [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];

fn table_bitrate(version: MpegVersion, layer: Layer, index: u8) -> Option<u32> {
    let row: &[u32; 14] = match (version, layer) {
        (MpegVersion::V1, Layer::L1) => &BITRATES_V1_L1,
        (MpegVersion::V1, Layer::L2) => &BITRATES_V1_L2,
        (MpegVersion::V1, Layer::L3) => &BITRATES_V1_L3,
        (MpegVersion::V2, Layer::L1) | (MpegVersion::V2_5, Layer::L1) => &BITRATES_V2_L1,
        (MpegVersion::V2, Layer::L2)
        | (MpegVersion::V2, Layer::L3)
        | (MpegVersion::V2_5, Layer::L2)
        | (MpegVersion::V2_5, Layer::L3) => &BITRATES_V2_L23,
        (MpegVersion::Reserved, _) | (_, Layer::Reserved) => return None,
    };
    if index == 0 || index > 14 {
        return None;
    }
    row.get(usize::from(index) - 1).copied()
}

fn table_sample_rate(version: MpegVersion, index: u8) -> Option<u32> {
    let row: [u32; 3] = match version {
        MpegVersion::V2_5 => [11_025, 12_000, 8_000],
        MpegVersion::V2 => [22_050, 24_000, 16_000],
        MpegVersion::V1 => [44_100, 48_000, 32_000],
        MpegVersion::Reserved => return None,
    };
    if index > 2 {
        return None;
    }
    row.get(usize::from(index)).copied()
}

/// The parse-failure discriminant (the caller's resync path only
/// needs "is this a frame"; the typed EngineError taxonomy is minted
/// by the demuxer where the scan context lives).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseFail;

/// Parse a 4-byte frame header at the front of `head`; `Err` when the
/// header is not strictly valid (bad sync, reserved bits, free-format
/// or bad bitrate index, reserved samplerate).
pub fn parse_head(head: &[u8]) -> Result<FrameHead, ParseFail> {
    if head.len() < 4 {
        return Err(ParseFail);
    }
    let [b0, b1, b2, b3] = [head[0], head[1], head[2], head[3]];
    if b0 != 0xFF || b1 & 0xE0 != 0xE0 {
        return Err(ParseFail); // sync
    }
    let version = match b1 >> 3 & 0x3 {
        0 => MpegVersion::V2_5,
        1 => return Err(ParseFail), // reserved version
        2 => MpegVersion::V2,
        _ => MpegVersion::V1,
    };
    let layer = match b1 >> 1 & 0x3 {
        0 => return Err(ParseFail), // reserved layer
        1 => Layer::L3,
        2 => Layer::L2,
        _ => Layer::L1,
    };
    let bitrate_index = b2 >> 4;
    let bitrate_bps = match table_bitrate(version, layer, bitrate_index) {
        Some(b) => b * 1000,
        None => return Err(ParseFail), // free format (0) or bad (15)
    };
    let sample_rate = match table_sample_rate(version, b2 >> 2 & 0x3) {
        Some(r) => r,
        None => return Err(ParseFail), // reserved index 3
    };
    if b3 & 0x3 == 2 {
        return Err(ParseFail); // reserved emphasis
    }
    let spf = match (version, layer) {
        (_, Layer::L1) => 384,
        (_, Layer::L2) => 1152,
        (MpegVersion::V1, Layer::L3) => 1152,
        (MpegVersion::V2, Layer::L3) | (MpegVersion::V2_5, Layer::L3) => 576,
        (MpegVersion::Reserved, _) | (_, Layer::Reserved) => return Err(ParseFail),
    };
    let padding = b2 & 0x2 != 0;
    let coeff: u64 = match layer {
        Layer::L1 => 12 * 4,
        Layer::L2 => 144,
        Layer::L3 if matches!(version, MpegVersion::V1) => 144,
        Layer::L3 => 72,
        Layer::Reserved => return Err(ParseFail),
    };
    let size = coeff * u64::from(bitrate_bps) / u64::from(sample_rate) + u64::from(padding);
    let frame_size = u32::try_from(size).map_err(|_| ParseFail)?;
    if frame_size < 4 || frame_size as usize > MAX_FRAME_BYTES {
        return Err(ParseFail);
    }
    let channels = if b3 >> 6 == 0b11 { 1 } else { 2 };
    Ok(FrameHead {
        version,
        layer,
        bitrate_bps,
        sample_rate,
        padding,
        protected_by_crc: b1 & 0x01 == 0,
        channels,
        frame_size,
        samples_per_frame: spf,
    })
}

/// The registry codec descriptor for a layer (the mkv vocabulary).
#[must_use]
pub fn descriptor(layer: Layer) -> CodecDescriptor {
    let name = match layer {
        Layer::L3 => "mp3",
        Layer::L1 | Layer::L2 => "mp2",
        Layer::Reserved => "mp3",
    };
    CodecDescriptor::new(name).unwrap_or_else(|_| unreachable!("static names are valid"))
}

/// The raw MPEG-audio demuxer.
pub struct RawMp3Demuxer {
    reader: SourceReader,
    started: bool,
    eos: bool,
    sticky: Option<(
        media_types::PipelineStage,
        media_types::ErrorKind,
        ErrorContext,
    )>,
    pending: VecDeque<DemuxEvent>,
    track: Option<Track>,
    walk: u64,
    samples_clock: i64,
    frames_out: u64,
    head: Option<FrameHead>,
    pool: PayloadSlots,
    damage: DamageLog,
    id3_skipped: u64,
}

fn mint(kind: media_types::ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Demux,
        kind,
        ErrorContext::new(code),
    )
}

impl RawMp3Demuxer {
    /// Construct over a seekable, length-known source stream.
    ///
    /// # Errors
    /// Typed `Unsupported` (stage `Open`) for unknown-length,
    /// unseekable or empty sources.
    pub fn new(stream: Box<dyn plugin_api::SourceStream>) -> Result<RawMp3Demuxer, EngineError> {
        Ok(RawMp3Demuxer {
            reader: SourceReader::new(stream)?,
            started: false,
            eos: false,
            sticky: None,
            pending: VecDeque::new(),
            track: None,
            walk: 0,
            samples_clock: 0,
            frames_out: 0,
            head: None,
            pool: PayloadSlots::new(2, MAX_FRAME_BYTES),
            damage: DamageLog::default(),
            id3_skipped: 0,
        })
    }

    /// Frames demuxed so far (the count differential's truth).
    #[must_use]
    pub fn frames_demuxed(&self) -> u64 {
        self.frames_out
    }

    /// The first validated frame header (`None` before `start`).
    #[must_use]
    pub fn head(&self) -> Option<FrameHead> {
        self.head
    }

    /// Bytes skipped as ID3v2 tags (head or inter-frame).
    #[must_use]
    pub fn id3_skipped(&self) -> u64 {
        self.id3_skipped
    }

    /// Total bytes skipped by bounded recovery + tags.
    #[must_use]
    pub fn damage_total(&self) -> u64 {
        self.damage.total
    }

    fn fail(&mut self, err: EngineError) -> EngineError {
        self.sticky = Some((err.stage, err.kind.clone(), err.context.clone()));
        err
    }

    fn sticky_err(&self) -> EngineError {
        let (stage, kind, context) = self
            .sticky
            .as_ref()
            .unwrap_or_else(|| unreachable!("checked by caller"));
        EngineError::new(*stage, kind.clone(), context.clone())
    }

    /// Skip an ID3v2 tag at `pos` (`Ok(None)` = none, `Ok(Some(after))`).
    fn skip_id3(&mut self, pos: u64) -> Result<Option<u64>, EngineError> {
        let len = self.reader.length();
        if len - pos < 10 {
            return Ok(None);
        }
        let tag = self.reader.read_prefix_at(10, pos)?;
        if tag.len() < 10 || &tag[0..3] != b"ID3" {
            return Ok(None);
        }
        let mut size = u64::from(tag[6] & 0x7F) << 21
            | u64::from(tag[7] & 0x7F) << 14
            | u64::from(tag[8] & 0x7F) << 7
            | u64::from(tag[9] & 0x7F);
        if size > MAX_ID3_BYTES {
            return Err(self.fail(super::error::bad_header(pos)));
        }
        if tag[5] & 0x10 != 0 {
            size += 10; // footer
        }
        let next = pos + 10 + size;
        if next > len {
            return Err(self.fail(super::error::truncated(next, len)));
        }
        self.id3_skipped += next - pos;
        self.damage.note(next - pos, "mp3: ID3v2 tag skipped");
        Ok(Some(next))
    }

    /// The bounded open scan: first strictly-valid frame header at or
    /// after the current walk position (0, or just past a head ID3v2
    /// tag).
    fn open_scan(&mut self) -> Result<FrameHead, EngineError> {
        let len = self.reader.length();
        let scan_end = (self.walk + MAX_OPEN_SCAN_BYTES).min(len);
        let mut at = self.walk;
        while at < scan_end {
            let win = self
                .reader
                .read_prefix_at((scan_end - at).min(4096) as usize, at)?;
            if win.len() < 4 {
                break;
            }
            let mut hit = None;
            for i in 0..win.len() - 3 {
                if win[i] == 0xFF
                    && win[i + 1] & 0xE0 == 0xE0
                    && let Ok(h) = parse_head(&win[i..])
                {
                    hit = Some((i, h));
                    break;
                }
            }
            match hit {
                Some((i, h)) => {
                    self.walk = at + i as u64;
                    return Ok(h);
                }
                None => at += win.len() as u64 - 3,
            }
        }
        let sniff = self.reader.read_prefix_at(16, 0)?;
        Err(self.fail(super::error::no_sync("mp3", &sniff)))
    }

    /// Bounded resync: the next parseable header at/after `from`.
    fn resync(&mut self, from: u64) -> Result<Option<u64>, EngineError> {
        let len = self.reader.length();
        let budget_end = (from + MAX_RECOVERY_SCAN_BYTES).min(len);
        let mut at = from;
        while at < len {
            let win_end = (at + 4096).min(budget_end.max(at + 1));
            let win = self.reader.read_prefix_at((win_end - at) as usize, at)?;
            if win.len() < 4 {
                break;
            }
            let mut hit = None;
            for i in 0..win.len() - 3 {
                if win[i] == 0xFF && win[i + 1] & 0xE0 == 0xE0 && parse_head(&win[i..]).is_ok() {
                    hit = Some(at + i as u64);
                    break;
                }
            }
            match hit {
                Some(target) => {
                    let skipped = target - self.walk;
                    self.damage.note(skipped, "mp3: lost sync, resynchronized");
                    self.pending.push_back(DemuxEvent::Recovered {
                        skipped,
                        reason: "mp3: lost sync, resynchronized",
                    });
                    self.walk = target;
                    return Ok(Some(target));
                }
                None => at = win_end,
            }
            if at >= budget_end && at < len {
                return Err(self.fail(mint(
                    media_types::ErrorKind::ResourceLimit {
                        limit: media_types::error::LimitKind::Memory,
                        budget: MAX_RECOVERY_SCAN_BYTES,
                        attempted: at - from,
                    },
                    "demux.es.mp3.recovery-budget",
                )));
            }
        }
        // Stream ended during recovery: final skip, clean end.
        let skipped = len.saturating_sub(self.walk);
        if skipped > 0 {
            self.damage.note(skipped, "mp3: lost sync at end of stream");
            self.pending.push_back(DemuxEvent::Recovered {
                skipped,
                reason: "mp3: lost sync at end of stream",
            });
        }
        self.walk = len;
        Ok(None)
    }

    /// Mint the single audio track from the first frame header.
    fn mint_track(&mut self, h: FrameHead) -> Track {
        let base =
            TimeBase::new(1, h.sample_rate).unwrap_or_else(|_| unreachable!("table rates are > 0"));
        let track = Track {
            id: TrackId::new(0),
            stream: StreamId::new(0),
            kind: MediaKind::Audio,
            language: None,
            codec: descriptor(h.layer),
            title: None,
            flags: TrackFlags::NONE,
            timing: TrackTiming {
                span: TimeSpan {
                    start: Stamp::Present(Timestamp::new(0, base)),
                    end: Stamp::Missing,
                },
                base,
                gaps: Vec::new(),
            },
        };
        self.track = Some(track.clone());
        track
    }

    /// One walk step: skip an inter-frame tag, emit one frame, resync,
    /// or end.
    fn step(&mut self) -> Result<bool, EngineError> {
        let len = self.reader.length();
        if self.walk >= len {
            return Ok(false);
        }
        if let Some(next) = self.skip_id3(self.walk)? {
            self.walk = next;
            return Ok(true);
        }
        if len - self.walk < 4 {
            let left = len - self.walk;
            self.damage.note(left, "mp3: trailing fragment");
            self.pending.push_back(DemuxEvent::Recovered {
                skipped: left,
                reason: "mp3: trailing fragment",
            });
            self.walk = len;
            return Ok(false);
        }
        let head = self.reader.read_prefix_at(4, self.walk)?;
        let h = match parse_head(&head) {
            Ok(h) => h,
            Err(ParseFail) => {
                if self.resync(self.walk + 1)?.is_none() {
                    return Ok(false);
                }
                let head = self.reader.read_prefix_at(4, self.walk)?;
                match parse_head(&head) {
                    Ok(h) => h,
                    Err(ParseFail) => return Ok(false),
                }
            }
        };
        let end = self.walk + u64::from(h.frame_size);
        if end > len {
            // A frame claim that overruns the stream: typed sticky
            // truncation (ffmpeg emits a short packet — divergence).
            return Err(self.fail(super::error::truncated(end, len)));
        }
        // The demux-level Xing/Info/VBRI rule (measured on the corpus:
        // ffmpeg's raw mp3 demuxer parses the first frame's metadata
        // block and does NOT emit it as a packet — `0139`'s VBR header
        // frame at 44 is packet-skipped, packet 1 starts at 1088).
        // Framing-only detection: the side-info arithmetic + the
        // 4-byte tag; the TOC/gapless payloads stay decode-domain.
        if self.frames_out == 0 && self.first_frame_is_metadata(h) {
            self.damage
                .note(h.frame_size as u64, "mp3: Xing/Info/VBRI frame skipped");
            self.walk = end;
            return Ok(true);
        }
        let track = self
            .track
            .clone()
            .unwrap_or_else(|| unreachable!("start minted the track"));
        let base = track.timing.base;
        let data: SharedBytes = self.pool.fill(h.frame_size as usize, |buf| {
            self.reader.read_prefix_into(buf, self.walk)
        })?;
        let pts = Stamp::Present(Timestamp::new(self.samples_clock, base));
        let ns = u128::from(h.samples_per_frame) * 1_000_000_000 / u128::from(h.sample_rate);
        let duration = u64::try_from(ns)
            .ok()
            .map(|n| Duration::Known(Nanoseconds(n)));
        let reference = self
            .head
            .unwrap_or_else(|| unreachable!("start pinned the reference"));
        let param_change =
            h.sample_rate != reference.sample_rate || h.channels != reference.channels;
        if param_change {
            self.damage.note(0, "mp3: mid-stream parameter change");
            self.pending.push_back(DemuxEvent::Discontinuity);
        }
        self.samples_clock = self
            .samples_clock
            .saturating_add(i64::from(h.samples_per_frame));
        self.frames_out += 1;
        self.pending.push_back(DemuxEvent::Packet(Packet {
            track: track.id,
            pts,
            dts: Stamp::Missing,
            duration,
            data,
            keyframe: true, // MPEG-audio frames decode independently
            discontinuity: param_change,
            generation: GenerationId::FIRST,
        }));
        self.walk = end;
        Ok(true)
    }

    /// Whether the frame at `self.walk` carries a Xing/Info tag (at
    /// `4 + crc + side_info`, the ISO side-info sizes per
    /// version/channels) or a VBRI tag (fixed offset 36) — the
    /// metadata-only first frame ffmpeg packet-skips.
    fn first_frame_is_metadata(&mut self, h: FrameHead) -> bool {
        let side = match h.version {
            MpegVersion::V1 => {
                if h.channels == 1 {
                    17
                } else {
                    32
                }
            }
            MpegVersion::V2 | MpegVersion::V2_5 => {
                if h.channels == 1 {
                    9
                } else {
                    17
                }
            }
            MpegVersion::Reserved => return false,
        };
        let xing_at = 4 + usize::from(h.protected_by_crc) * 2 + side;
        let need = xing_at.max(36) + 4;
        if u64::from(h.frame_size) < need as u64 {
            return false;
        }
        let bytes = match self.reader.read_prefix_at(need, self.walk) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        if bytes.len() < need {
            return false;
        }
        if &bytes[xing_at..xing_at + 4] == b"Xing" || &bytes[xing_at..xing_at + 4] == b"Info" {
            return true;
        }
        &bytes[36..40] == b"VBRI"
    }
}

impl Demuxer for RawMp3Demuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if self.started {
            return Ok(vec![
                self.track
                    .clone()
                    .unwrap_or_else(|| unreachable!("start minted the track")),
            ]);
        }
        if self.sticky.is_some() {
            return Err(self.sticky_err());
        }
        // Skip a head tag (if any), then locate the first frame.
        if let Some(next) = self.skip_id3(0)? {
            self.walk = next;
        }
        let h = self.open_scan()?;
        self.started = true;
        self.head = Some(h);
        Ok(vec![self.mint_track(h)])
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if self.sticky.is_some() {
            return Err(self.sticky_err());
        }
        if !self.started {
            return Err(super::error::not_started());
        }
        while self.pending.is_empty() {
            if self.eos || !self.step()? {
                self.eos = true;
                return Ok(DemuxEvent::EndOfStream);
            }
        }
        self.pending
            .pop_front()
            .ok_or_else(|| unreachable!("pending is non-empty"))
    }

    fn seek(
        &mut self,
        target: media_types::time::TimeSpan,
    ) -> Result<media_types::time::TimeSpan, EngineError> {
        let _ = target;
        Err(mint(
            media_types::ErrorKind::Unsupported { format: None },
            "demux.es.mp3.seek.unsupported",
        ))
    }
}
