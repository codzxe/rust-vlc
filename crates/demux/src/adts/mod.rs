//! The ADTS/ADIF raw-AAC demuxer (card 5.C2b): syncword frame walk,
//! resync, ID3v2 skip, ADIF CBR block arithmetic.
//!
//! # What this is
//!
//! [`AdtsDemuxer`] implements the landed
//! [`Demuxer`](plugin_api::Demuxer) contract over any seekable
//! [`SourceStream`](plugin_api::SourceStream) for the two headered raw
//! AAC stream formats of ISO/IEC 13818-7:
//!
//! - **ADTS** (§6.5): a sequence of self-delimited frames. Each frame
//!   header (`adts_fixed_header` + `adts_variable_header`, 56 bits,
//!   plus a 16-bit `crc_check` when `protection_absent == 0`) carries
//!   profile, sampling index, channel configuration, frame length and
//!   the raw-data-block count. The demuxer walks frame to frame;
//!   packets carry the **full ADTS frame including its header**
//!   (ffmpeg's own packet shape, verified by the corpus differential).
//! - **ADIF** (§6.5/A.2): one `adif_header` (copyright string,
//!   original/copy + home bits, bitstream-type map with a 23-bit
//!   bitrate, one program_config_element) followed by an un delimited
//!   run of raw_data_blocks. Only the CBR flavor has derivable frame
//!   boundaries: `block_bytes = bitrate × 128 / sample_rate` (one
//!   1024-sample block at `bitrate` bits/s). VBR ADIF and multi-program
//!   ADIF are typed refusals — their boundaries live in
//!   syntax-element lengths, which are decode-domain (ffmpeg has no
//!   ADIF demuxer at all; the honest note is in the worklog table).
//!
//! # Timing (the honest contract)
//!
//! A raw AAC frame is `1024 × (number_of_raw_data_blocks_in_frame)`
//! samples (the 960-sample MPEG-4 frame mode is signaled only inside
//! an AudioSpecificConfig — decode domain — so 1024 is the demux
//! constant; likewise HE-AAC's SBR doubling). ADTS timestamps are pure
//! frame arithmetic: packet *n* is stamped at the cumulative sample
//! count in the track's `1/rate` base, `duration = samples/rate` —
//! exactly the clock ffprobe reports (`pts_time = n·1024/rate`,
//! verified frame-by-frame on the corpus). Damage is never smoothed:
//! a resync gap or a mid-stream parameter change emits
//! [`Discontinuity`](plugin_api::DemuxEvent::Discontinuity) and marks
//! the packet, but the sample arithmetic stays explicit (a skipped
//! frame leaves a real hole in the timeline, MP §8).
//!
//! # Damage policy (bounded, honest)
//!
//! The first frame is validated strictly and refuses typed:
//! sampling index 13–15, channel configuration 0, a CRC that fails
//! its header check, a frame shorter than its own header, or a frame
//! running past end-of-stream (`truncated`, sticky — the Ogg
//! page-truncation precedent; ffmpeg instead emits a short final
//! packet, an honestly-recorded divergence). Mid-stream, a lost
//! syncword triggers a **bounded resync scan**
//! (≤ [`MAX_RECOVERY_SCAN_BYTES`]) that accepts the next
//! syncword-delimited frame that fits the stream — the acceptance
//! predicate ffmpeg's raw ADTS demuxer was measured using on the
//! corpus's dirty lane (it does not re-check layer or sampling index
//! mid-scan; both divergences are measured and recorded in the
//! worklog). Recovered skips surface as `Recovered` events; an ID3v2
//! tag at the head (or between frames) is parsed and skipped, bounded
//! by [`MAX_ID3_BYTES`].
//!
//! # Zero-copy framing (notes for 5.X2)
//!
//! Frame payloads land in a 2-slot [`PayloadSlots`] pool (slot cap
//! [`MAX_FRAME_BYTES`], the 13-bit ADTS length ceiling + header, so
//! the pool can never grow beyond ~16 KiB × 2): one frame = one
//! [`fill`](PayloadSlots::fill) = one O(1) shared view handed to the
//! decoder — zero per-packet allocation in steady state
//! ([`pool_bytes`](AdtsDemuxer::pool_bytes) is the observable). The
//! CRC scratch is a 9-byte stack array; the only per-frame heap
//! traffic is the event queue node.
//!
//! # Example
//!
//! ```
//! use demux::adts::AdtsDemuxer;
//! use plugin_api::{DemuxEvent, Demuxer};
//!
//! # fn main() -> Result<(), media_types::EngineError> {
//! # let bytes = demux::adts::test_support::demo_stream();
//! let mut stream = demux::adts::test_support::mem_stream(bytes)?;
//! let mut adts = AdtsDemuxer::new(stream)?;
//! let tracks = adts.start()?;
//! assert_eq!(tracks.len(), 1);
//! # match adts.next_event()? {
//! #     DemuxEvent::Packet(p) => assert_eq!(p.data.len(), 7 + 8),
//! #     other => unreachable!("a data packet comes first: {other:?}"),
//! # }
//! # Ok(())
//! # }
//! ```

use std::collections::VecDeque;

use media_types::error::ErrorContext;
use media_types::ids::{CodecDescriptor, GenerationId, MediaKind, StreamId, TrackFlags, TrackId};
use media_types::packet::SharedBytes;
use media_types::time::{Duration, Nanoseconds, Stamp, TimeBase, TimeSpan, Timestamp, TrackTiming};
use media_types::{EngineError, Packet, Track};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use crate::avi::header::DamageLog;
use crate::avi::riff::{PayloadSlots, SourceReader};

pub mod error;
pub mod fuzz;
pub mod test_support;

#[cfg(test)]
mod tests;

/// Samples in one raw AAC data block (ISO 13818-7; the 960-sample
/// MPEG-4 mode and HE-AAC SBR doubling are ASC/decode-domain).
pub const SAMPLES_PER_BLOCK: u64 = 1024;

/// Maximum frame payload trusted into memory: the ADTS 13-bit
/// `frame_length` ceiling (8191) plus the 9-byte protected header.
pub const MAX_FRAME_BYTES: u64 = 8191 + 9;

/// Maximum bytes the resync scan reads hunting the next syncword
/// (MP §12: recovery is bounded, never a whole-file loop).
pub const MAX_RECOVERY_SCAN_BYTES: u64 = 4 * 1024 * 1024;

/// Maximum bytes the `start` scan walks hunting the first frame.
pub const MAX_OPEN_SCAN_BYTES: u64 = 64 * 1024;

/// Maximum ID3v2 tag size trusted at the head or between frames
/// (the declared size is ceiling-checked before it is skipped).
pub const MAX_ID3_BYTES: u64 = 16 * 1024 * 1024;

/// Maximum frames the seek scan walks before refusing (the
/// `SlowLinear` honesty bound).
pub const MAX_SEEK_SCAN_FRAMES: u64 = 2_000_000;

/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["aac", "adts", "adif"];

/// The MPEG-4 sampling-frequency table (indices 0–12; 13–15 are
/// reserved/escape and refused typed at the first frame).
pub const SAMPLE_RATES: [u32; 13] = [
    96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025, 8_000,
    7_350,
];

/// The probe: do the leading bytes look like ADTS or ADIF? (ID3-prefixed
/// streams are legal too — the registry sniffs past the tag at open.)
#[must_use]
pub fn probe(header: &[u8]) -> bool {
    if header.len() >= 4 && &header[0..4] == b"ADIF" {
        return true;
    }
    header.len() >= 2 && header[0] == 0xFF && header[1] & 0xF0 == 0xF0
}

/// One parsed ADTS frame header (ISO 13818-7 §6.5.1). Public so the
/// AAC decoder card (5.C2a) can build its AudioSpecificConfig from the
/// demuxed stream without re-parsing bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdtsHeader {
    /// The `ID` bit (true = MPEG-2 style, false = MPEG-4).
    pub mpeg2: bool,
    /// `protection_absent` (true = no `crc_check` in this frame).
    pub protection_absent: bool,
    /// `profile` bits (object type − 1: 0 Main, 1 LC, 2 SSR, 3 LTP).
    pub profile: u8,
    /// Raw `sampling_frequency_index` (0–15; 13–15 are invalid).
    pub sampling_index: u8,
    /// The `private_bit`.
    pub private_bit: bool,
    /// Raw `channel_configuration` (0–7; 0 means PCE-in-band).
    pub channel_config: u8,
    /// `original_copy`.
    pub original_copy: bool,
    /// `home`.
    pub home: bool,
    /// `copyright_id_bit`.
    pub copyright_id_bit: bool,
    /// `copyright_id_start`.
    pub copyright_id_start: bool,
    /// `frame_length` — the whole frame, header included.
    pub frame_length: u16,
    /// `adts_buffer_fullness` (0x7FF = VBR).
    pub buffer_fullness: u16,
    /// `number_of_raw_data_blocks_in_frame − 1`.
    pub blocks_minus_one: u8,
}

impl AdtsHeader {
    /// Parse the 7 fixed header bytes (the caller reads 9 when
    /// protection is present). `None` when the syncword is absent —
    /// field validation is the caller's policy (strict at the first
    /// frame, lenient in resync).
    #[must_use]
    pub fn parse(head: &[u8]) -> Option<AdtsHeader> {
        if head.len() < 7 || head[0] != 0xFF || head[1] & 0xF0 != 0xF0 {
            return None;
        }
        Some(AdtsHeader {
            mpeg2: head[1] & 0x08 != 0,
            protection_absent: head[1] & 0x01 != 0,
            profile: head[2] >> 6,
            sampling_index: head[2] >> 2 & 0x0F,
            private_bit: head[2] & 0x02 != 0,
            channel_config: (head[2] & 0x01) << 2 | head[3] >> 6,
            original_copy: head[3] & 0x20 != 0,
            home: head[3] & 0x10 != 0,
            copyright_id_bit: head[3] & 0x08 != 0,
            copyright_id_start: head[3] & 0x04 != 0,
            frame_length: u16::from(head[3] & 0x03) << 11
                | u16::from(head[4]) << 3
                | u16::from(head[5] >> 5),
            buffer_fullness: u16::from(head[5] & 0x1F) << 6 | u16::from(head[6] >> 2),
            blocks_minus_one: head[6] & 0x03,
        })
    }

    /// The header's byte size (9 with protection, 7 without).
    #[must_use]
    pub fn header_len(&self) -> u64 {
        if self.protection_absent { 7 } else { 9 }
    }

    /// Samples this frame carries (`1024 × raw_data_blocks`).
    #[must_use]
    pub fn samples(&self) -> u64 {
        SAMPLES_PER_BLOCK * u64::from(self.blocks_minus_one + 1)
    }

    /// The rate table entry for the sampling index (`None` for 13–15).
    #[must_use]
    pub fn sample_rate(&self) -> Option<u32> {
        if self.sampling_index < SAMPLE_RATES.len() as u8 {
            Some(SAMPLE_RATES[usize::from(self.sampling_index)])
        } else {
            None
        }
    }

    /// The registry codec descriptor for the profile bits. ADTS cannot
    /// prove HE-AAC (SBR rides in-band, decode domain), so the honest
    /// vocabulary is Main/LC/SSR/LTP.
    pub fn descriptor(&self) -> Result<CodecDescriptor, EngineError> {
        let name = match self.profile {
            0 => "aac-main",
            2 => "aac-ssr",
            3 => "aac-ltp",
            _ => "aac-lc",
        };
        // Static ASCII literal: the grammar cannot refuse it; the
        // typed error is the panic-free residue (8.A1 sweep).
        CodecDescriptor::new(name).map_err(|_| error::bad_descriptor())
    }
}

/// One parsed ADIF header + the CBR walk parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdifInfo {
    /// `profile` bits from the program_config_element (object type − 1).
    pub profile: u8,
    /// The PCE's `sampling_frequency_index` (validated 0–12).
    pub sampling_index: u8,
    /// Total channels derived from the channel elements (SCE 1, CPE 2,
    /// LFE 1 each).
    pub channels: u16,
    /// `original_copy`.
    pub original_copy: bool,
    /// `home`.
    pub home: bool,
    /// The 23-bit `bitrate` (CBR only; bits per second).
    pub bitrate: u32,
    /// Bytes per 1024-sample block (`bitrate × 128 / rate`).
    pub block_bytes: u64,
    /// Absolute offset of the first raw_data_block (header is
    /// byte-aligned by spec).
    pub data_start: u64,
    /// Blocks derivable from the stream length (`None` until `start`
    /// measured it).
    pub block_count: Option<u64>,
}

impl AdifInfo {
    /// The registry codec descriptor (profile-honest, as ADTS).
    pub fn descriptor(&self) -> Result<CodecDescriptor, EngineError> {
        let name = match self.profile {
            0 => "aac-main",
            2 => "aac-ssr",
            3 => "aac-ltp",
            _ => "aac-lc",
        };
        // Static ASCII literal: the grammar cannot refuse it; the
        // typed error is the panic-free residue (8.A1 sweep).
        CodecDescriptor::new(name).map_err(|_| error::bad_descriptor())
    }

    /// The rate table entry.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        SAMPLE_RATES[usize::from(self.sampling_index)]
    }
}

/// Which raw format the stream declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawAacFormat {
    /// ADTS frames (self-delimited).
    Adts(AdtsHeader),
    /// An ADIF header + CBR block walk.
    Adif(AdifInfo),
}

/// Demuxer-side stream info (the fields ffprobe reports from its own
/// ADTS parser — queryable here for the UI/engine without decoding).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdtsStreamInfo {
    /// The raw format in use.
    pub format: &'static str,
    /// Sample rate in Hz (the track's time base denominator).
    pub rate: u32,
    /// Channels (ADTS header / ADIF PCE count).
    pub channels: u16,
    /// Profile bits (object type − 1).
    pub profile: u8,
}

/// A replayable error shape (the WAV/Ogg `StickyShape` precedent).
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

/// The ADTS/ADIF demuxer.
///
/// Construct with [`AdtsDemuxer::new`] over a seekable, length-known
/// source stream; `start` parses the first frame (or the ADIF header)
/// and mints the single audio track; `next_event` walks frames;
/// `seek`/`reanchor` are a `SlowLinear` frame scan for ADTS and
/// format-inherent arithmetic for ADIF.
pub struct AdtsDemuxer {
    reader: SourceReader,
    started: bool,
    eos: bool,
    sticky: Option<StickyShape>,
    pending: VecDeque<DemuxEvent>,
    track: Option<Track>,
    /// The walk position (an absolute byte offset for ADTS, a block
    /// index for ADIF — the format decides).
    walk: u64,
    /// Cumulative samples at the next packet's pts (the ADTS clock).
    samples_clock: i64,
    frames_out: u64,
    head: Option<RawAacFormat>,
    pool: PayloadSlots,
    damage: DamageLog,
    id3_skipped: u64,
}

impl AdtsDemuxer {
    /// Construct over a seekable, length-known source stream (no
    /// container I/O — `start` does the parsing).
    ///
    /// # Errors
    /// Typed `Unsupported` (stage `Open`) for unknown-length,
    /// unseekable or empty sources (the AVI/WAV/Ogg precedent).
    pub fn new(stream: Box<dyn plugin_api::SourceStream>) -> Result<AdtsDemuxer, EngineError> {
        let reader = SourceReader::new(stream)?;
        Ok(AdtsDemuxer {
            reader,
            started: false,
            eos: false,
            sticky: None,
            pending: VecDeque::new(),
            track: None,
            walk: 0,
            samples_clock: 0,
            frames_out: 0,
            head: None,
            pool: PayloadSlots::new(2, MAX_FRAME_BYTES as usize),
            damage: DamageLog::default(),
            id3_skipped: 0,
        })
    }

    /// The parsed format (header fields of the first frame / the ADIF
    /// info). `None` before `start`.
    #[must_use]
    pub fn format(&self) -> Option<&RawAacFormat> {
        self.head.as_ref()
    }

    /// Demuxer-side stream info (rate/channels/profile — the fields
    /// ffprobe reports; decode-domain detail stays in the bytes).
    #[must_use]
    pub fn stream_info(&self) -> Option<AdtsStreamInfo> {
        Some(match self.head.as_ref()? {
            RawAacFormat::Adts(h) => AdtsStreamInfo {
                format: "adts",
                rate: h.sample_rate()?,
                channels: u16::from(h.channel_config),
                profile: h.profile,
            },
            RawAacFormat::Adif(info) => AdtsStreamInfo {
                format: "adif",
                rate: info.sample_rate(),
                channels: info.channels,
                profile: info.profile,
            },
        })
    }

    /// Data packets emitted so far (the count differential's truth).
    #[must_use]
    pub fn frames_demuxed(&self) -> u64 {
        self.frames_out
    }

    /// Total bytes skipped by bounded recovery + ID3 tags (the
    /// `Recovered` events' sum).
    #[must_use]
    pub fn damage_total(&self) -> u64 {
        self.damage.total
    }

    /// Bytes skipped as ID3v2 metadata (head or inter-frame).
    #[must_use]
    pub fn id3_skipped(&self) -> u64 {
        self.id3_skipped
    }

    /// The payload bytes resident in the packet pool (the D-1
    /// observable; bounded by `2 × MAX_FRAME_BYTES`, grown lazily).
    #[must_use]
    pub fn pool_bytes(&self) -> usize {
        self.pool.pool_bytes()
    }

    /// The track's duration as walked so far (`samples_clock` →
    /// nanoseconds). `None` before `start`.
    #[must_use]
    pub fn track_duration(&self) -> Option<Duration> {
        let rate = match self.head.as_ref()? {
            RawAacFormat::Adts(h) => h.sample_rate()?,
            RawAacFormat::Adif(i) => i.sample_rate(),
        };
        let ns = u128::from(self.samples_clock.unsigned_abs()) * 1_000_000_000 / u128::from(rate);
        u64::try_from(ns)
            .ok()
            .map(|n| Duration::Known(Nanoseconds(n)))
    }

    // -- internals ---------------------------------------------------------

    fn fail(&mut self, err: EngineError) -> EngineError {
        self.sticky = Some(StickyShape::of(&err));
        err
    }

    /// Skip an ID3v2 tag at `pos` if one is there; `Ok(None)` = no
    /// tag, `Ok(Some(next))` = the position after the tag.
    fn skip_id3(&mut self, pos: u64) -> Result<Option<u64>, EngineError> {
        let len = self.reader.length();
        if len - pos < 10 {
            return Ok(None);
        }
        let tag = self.reader.read_prefix_at(10, pos)?;
        if tag.len() < 10 || &tag[0..3] != b"ID3" {
            return Ok(None);
        }
        // Syncsafe 28-bit size (4 × 7 bits), optional 10-byte footer.
        let mut size = u64::from(tag[6] & 0x7F) << 21
            | u64::from(tag[7] & 0x7F) << 14
            | u64::from(tag[8] & 0x7F) << 7
            | u64::from(tag[9] & 0x7F);
        if size > MAX_ID3_BYTES {
            return Err(error::bad_header(pos));
        }
        if tag[5] & 0x10 != 0 {
            size += 10; // footer
        }
        let next = pos + 10 + size;
        if next > len {
            return Err(error::truncated(next, len));
        }
        self.id3_skipped += next - pos;
        self.damage.note(next - pos, "adts: ID3v2 tag skipped");
        Ok(Some(next))
    }

    /// `start` for ADTS: bounded scan for the first strict-valid frame,
    /// then the typed first-frame validation.
    fn open_adts(&mut self, from: u64) -> Result<AdtsHeader, EngineError> {
        let len = self.reader.length();
        let scan_end = (from + MAX_OPEN_SCAN_BYTES).min(len);
        let mut at = from;
        while at < len {
            if at >= scan_end {
                break;
            }
            let head = self.reader.read_prefix_at(7, at)?;
            let Some(raw) = AdtsHeader::parse(&head) else {
                at += 1;
                continue;
            };
            if head[1] & 0x06 != 0 {
                at += 1; // layer != 0: not an ADTS claim
                continue;
            }
            // The first ADTS claim: strict, typed validation.
            if raw.sampling_index >= SAMPLE_RATES.len() as u8 {
                return Err(error::bad_sampling_index());
            }
            if raw.channel_config == 0 {
                return Err(error::bad_channel_config(at));
            }
            let hlen = raw.header_len();
            if u64::from(raw.frame_length) < hlen {
                return Err(error::bad_header(at));
            }
            let full = self.reader.read_prefix_at(hlen as usize, at)?;
            if full.len() < hlen as usize {
                return Err(error::truncated(at + hlen, len));
            }
            if !raw.protection_absent && crc16_header(&full[..7]) != be16(&full[7..9]) {
                return Err(error::crc_mismatch(at));
            }
            if at + u64::from(raw.frame_length) > len {
                return Err(error::truncated(at + u64::from(raw.frame_length), len));
            }
            self.walk = at;
            return Ok(raw);
        }
        Err(error::not_adts(&self.reader.read_prefix_at(16, from)?))
    }

    /// `start` for ADIF: parse the header, validate the single-program
    /// CBR shape, derive the block walk.
    fn open_adif(&mut self, at: u64) -> Result<AdifInfo, EngineError> {
        let len = self.reader.length();
        let head = self.reader.read_prefix_at(4096, at)?;
        if head.len() < 8 {
            return Err(error::not_adts(&head));
        }
        let mut bits = BitReader::new(&head);
        bits.skip(32)?; // 'ADIF'
        let copyright_present = bits.read_bit()?;
        if copyright_present {
            // Byte-terminated null string (the GPAC parser's
            // convention; bounded so a lying length cannot loop).
            let mut seen = 0u32;
            while bits.read_byte()? != 0 {
                seen += 1;
                if seen > 64 {
                    return Err(error::adif_bad_params());
                }
            }
        }
        let original_copy = bits.read_bit()?;
        let home = bits.read_bit()?;
        let bitstream_type = bits.read_bit()?;
        let bitrate = bits.read_bits(23)? as u32;
        let num_pce = bits.read_bits(4)? as u8;
        if !bitstream_type && num_pce > 0 {
            return Err(error::adif_multi_program());
        }
        if bitstream_type {
            return Err(error::adif_vbr());
        }
        if bitrate == 0 {
            return Err(error::adif_bad_params());
        }
        // One program_config_element (single program — validated above).
        let pce = ProgramConfigElement::parse(&mut bits)?;
        if pce.sampling_index >= SAMPLE_RATES.len() as u8 {
            return Err(error::adif_bad_params());
        }
        if pce.channels == 0 {
            return Err(error::adif_bad_params());
        }
        let rate = SAMPLE_RATES[usize::from(pce.sampling_index)];
        let block_bytes = u64::from(bitrate) * 128 / u64::from(rate);
        if block_bytes == 0 {
            return Err(error::adif_bad_params());
        }
        let data_start = at + bits.byte_pos() as u64;
        if data_start >= len {
            return Err(error::adif_no_data());
        }
        let data_len = len - data_start;
        let info = AdifInfo {
            profile: pce.profile,
            sampling_index: pce.sampling_index,
            channels: pce.channels,
            original_copy,
            home,
            bitrate,
            block_bytes,
            data_start,
            block_count: Some(
                data_len / block_bytes + u64::from(!data_len.is_multiple_of(block_bytes)),
            ),
        };
        self.walk = 0; // block index
        Ok(info)
    }

    /// Mint the single audio track from the parsed head.
    fn mint_track(&mut self, rate: u32, descriptor: CodecDescriptor) -> Track {
        // 1/rate with rate = a SAMPLE_RATES table entry (> 0, index
        // validated 0–12 at parse): gcd(1, r) = 1, so this literal is
        // exactly `TimeBase::new(1, rate)`'s Ok value — the DVB-builder
        // precedent (8.A1 sweep: no unreachable error path).
        let base = TimeBase { num: 1, den: rate };
        let track = Track {
            id: TrackId::new(0),
            stream: StreamId::new(0),
            kind: MediaKind::Audio,
            language: None,
            codec: descriptor,
            title: None,
            flags: TrackFlags::NONE,
            timing: TrackTiming {
                span: TimeSpan {
                    start: Stamp::Present(Timestamp::new(0, base)),
                    end: Stamp::Missing, // refined by the walk; ffprobe's own
                                         // raw-ADTS duration is a bitrate estimate — ours is exact
                                         // arithmetic once walked (track_duration).
                },
                base,
                gaps: Vec::new(),
            },
        };
        self.track = Some(track.clone());
        track
    }

    /// The strict walk step at `self.walk`: one verified frame, or a
    /// typed/skip outcome. Returns the header on success.
    fn read_frame_head(&mut self) -> Result<Option<AdtsHeader>, EngineError> {
        let len = self.reader.length();
        let head = self.reader.read_prefix_at(7, self.walk)?;
        let Some(raw) = AdtsHeader::parse(&head) else {
            return Ok(None); // lost sync
        };
        let hlen = raw.header_len();
        if u64::from(raw.frame_length) < hlen {
            return Ok(None); // impossible structure: resync skips it
        }
        if !raw.protection_absent {
            let full = self.reader.read_prefix_at(9, self.walk)?;
            if full.len() < 9 || crc16_header(&full[..7]) != be16(&full[7..9]) {
                self.damage.note(0, "adts: frame CRC mismatch (skipped)");
                return Ok(None); // damaged: resync past it
            }
        }
        if self.walk + u64::from(raw.frame_length) > len {
            // Declared frame overruns the stream: typed sticky
            // truncation (ffmpeg emits a short packet instead — the
            // honestly-recorded divergence).
            return Err(error::truncated(
                self.walk + u64::from(raw.frame_length),
                len,
            ));
        }
        Ok(Some(raw))
    }

    /// Bounded resync: scan forward for the next syncword-delimited
    /// frame that fits the stream (the measured ffmpeg acceptance:
    /// syncword + frame covers its header + frame fits; layer and
    /// sampling index are NOT rechecked mid-scan). A structurally
    /// valid candidate that overruns end-of-stream raises the same
    /// typed sticky truncation as the strict walk (a frame claim is a
    /// claim wherever the scan finds it). Queues `Recovered`.
    fn resync(&mut self) -> Result<Option<AdtsHeader>, EngineError> {
        let len = self.reader.length();
        let from = self.walk + 1;
        let budget_end = (from + MAX_RECOVERY_SCAN_BYTES).min(len);
        let mut at = from;
        while at < len {
            let window_end = (at + 64 * 1024).min(budget_end.max(at + 1));
            let window = self.reader.read_prefix_at((window_end - at) as usize, at)?;
            let mut found = None;
            for (i, pair) in window.windows(2).enumerate() {
                if pair[0] == 0xFF
                    && pair[1] & 0xF0 == 0xF0
                    && i + 7 <= window.len()
                    && let Some(raw) = AdtsHeader::parse(&window[i..i + 7])
                    && u64::from(raw.frame_length) >= raw.header_len()
                {
                    let end = at + i as u64 + u64::from(raw.frame_length);
                    if end <= len {
                        found = Some((i, raw));
                    } else {
                        // A structurally valid frame claim that overruns
                        // the stream: the same typed sticky truncation
                        // the strict walk raises — a claim is a claim
                        // wherever the scan finds it (ffmpeg instead
                        // emits the overrun as a short final packet,
                        // the honestly-recorded divergence).
                        return Err(error::truncated(end, len));
                    }
                    break;
                }
            }
            match found {
                Some((i, raw)) => {
                    let target = at + i as u64;
                    let skipped = target - self.walk;
                    self.damage.note(skipped, "adts: lost sync, resynchronized");
                    self.pending.push_back(DemuxEvent::Recovered {
                        skipped,
                        reason: "adts: lost sync, resynchronized",
                    });
                    self.walk = target;
                    return Ok(Some(raw));
                }
                None => at = window_end,
            }
            if at >= budget_end && at < len {
                return Err(error::recovery_budget_exhausted(at - from));
            }
        }
        // Stream ended during recovery: final skip, clean end.
        let skipped = len.saturating_sub(self.walk);
        if skipped > 0 {
            self.damage
                .note(skipped, "adts: lost sync at end of stream");
            self.pending.push_back(DemuxEvent::Recovered {
                skipped,
                reason: "adts: lost sync at end of stream",
            });
        }
        self.walk = len;
        Ok(None)
    }

    /// Read one frame's full bytes into the pool (zero-copy view out).
    fn fill_frame(&mut self, frame_length: u16) -> Result<SharedBytes, EngineError> {
        let len = frame_length as usize;
        let at = self.walk;
        self.pool
            .fill(len, |buf| self.reader.read_prefix_into(buf, at))
    }

    /// One ADTS packet out of a verified frame header.
    fn emit_adts_frame(&mut self, raw: AdtsHeader, damaged: bool) -> Result<(), EngineError> {
        let track = self
            .track
            .clone()
            .ok_or_else(|| self.fail(error::not_started()))?;
        let base = track.timing.base;
        let data = self.fill_frame(raw.frame_length)?;
        let pts = Stamp::Present(Timestamp::new(self.samples_clock, base));
        let samples = raw.samples();
        let rate = match self.head.as_ref() {
            Some(RawAacFormat::Adts(h)) => h.sample_rate(),
            _ => None,
        };
        let duration = rate.and_then(|r| {
            let ns = u128::from(samples) * 1_000_000_000 / u128::from(r);
            u64::try_from(ns)
                .ok()
                .map(|n| Duration::Known(Nanoseconds(n)))
        });
        self.samples_clock = self
            .samples_clock
            .saturating_add(i64::try_from(samples).unwrap_or(i64::MAX));
        self.frames_out += 1;
        let reference = match self.head.as_ref() {
            Some(RawAacFormat::Adts(h)) => *h,
            _ => raw,
        };
        let param_change = raw.sampling_index != reference.sampling_index
            || raw.channel_config != reference.channel_config
            || raw.profile != reference.profile;
        if param_change {
            self.damage.note(0, "adts: mid-stream parameter change");
            self.pending.push_back(DemuxEvent::Discontinuity);
        }
        let packet = Packet {
            track: track.id,
            pts,
            dts: Stamp::Missing,
            duration,
            data,
            keyframe: true, // raw AAC frames are independently decodable
            discontinuity: damaged || param_change,
            generation: GenerationId::FIRST,
        };
        self.pending.push_back(DemuxEvent::Packet(packet));
        Ok(())
    }

    /// The ADTS walk step (one frame, or recovery, or end).
    fn step_adts(&mut self) -> Result<bool, EngineError> {
        let len = self.reader.length();
        if self.walk >= len {
            return Ok(false);
        }
        // Inter-frame ID3 tags are skipped (bounded).
        if let Some(next) = self.skip_id3(self.walk)? {
            self.walk = next;
            return Ok(true);
        }
        if len - self.walk < 7 {
            // A trailing fragment too short to be a frame header: a
            // bounded skip, then a clean end (the Ogg precedent).
            let left = len - self.walk;
            self.damage.note(left, "adts: trailing fragment");
            self.pending.push_back(DemuxEvent::Recovered {
                skipped: left,
                reason: "adts: trailing fragment",
            });
            self.walk = len;
            return Ok(false);
        }
        let mut damaged = false;
        let raw = match self.read_frame_head()? {
            Some(raw) => raw,
            None => match self.resync()? {
                Some(raw) => {
                    damaged = true;
                    raw
                }
                None => return Ok(false),
            },
        };
        self.emit_adts_frame(raw, damaged)?;
        self.walk += u64::from(raw.frame_length);
        Ok(true)
    }

    /// The ADIF walk step (one CBR block).
    fn step_adif(&mut self) -> Result<bool, EngineError> {
        let Some(RawAacFormat::Adif(info)) = self.head.clone() else {
            return Ok(false);
        };
        let len = self.reader.length();
        let Some(blocks) = info.block_count else {
            return Ok(false);
        };
        if self.walk >= blocks {
            return Ok(false);
        }
        let at = info.data_start + self.walk * info.block_bytes;
        let left = len.saturating_sub(at);
        if left == 0 {
            return Ok(false);
        }
        let take = info.block_bytes.min(left);
        let data = self
            .pool
            .fill(take as usize, |buf| self.reader.read_prefix_into(buf, at))?;
        let track = self
            .track
            .clone()
            .ok_or_else(|| self.fail(error::not_started()))?;
        let base = track.timing.base;
        let pts = Stamp::Present(Timestamp::new(self.samples_clock, base));
        let samples = SAMPLES_PER_BLOCK;
        let rate = info.sample_rate();
        let ns = u128::from(samples) * 1_000_000_000 / u128::from(rate);
        let duration = u64::try_from(ns)
            .ok()
            .map(|n| Duration::Known(Nanoseconds(n)));
        let partial = take < info.block_bytes;
        self.samples_clock = self
            .samples_clock
            .saturating_add(i64::try_from(samples).unwrap_or(i64::MAX));
        self.frames_out += 1;
        let packet = Packet {
            track: track.id,
            pts,
            dts: Stamp::Missing,
            duration,
            data,
            keyframe: true,
            discontinuity: partial,
            generation: GenerationId::FIRST,
        };
        if partial {
            self.damage.note(0, "adif: final partial block");
            self.pending.push_back(DemuxEvent::Discontinuity);
        }
        self.pending.push_back(DemuxEvent::Packet(packet));
        self.walk += 1;
        Ok(true)
    }

    /// The seek core for ADTS: a bounded frame-header scan from 0 to
    /// the last frame whose sample position is ≤ the target. Returns
    /// the landing (samples, byte offset).
    fn scan_landing_adts(&mut self, target_nanos: u64) -> Result<(i64, u64), EngineError> {
        let Some(RawAacFormat::Adts(reference)) = self.head.clone() else {
            return Err(error::seek_out_of_range());
        };
        let Some(rate) = reference.sample_rate() else {
            return Err(error::seek_out_of_range());
        };
        let target_samples = (u128::from(target_nanos) * u128::from(rate) / 1_000_000_000u128)
            .min(u128::from(i64::MAX as u64)) as i64;
        let len = self.reader.length();
        let mut at = 0u64;
        let mut samples = 0i64;
        let mut landing: Option<(i64, u64)> = None;
        let mut frames = 0u64;
        while at + 7 <= len {
            let head = self.reader.read_prefix_at(7, at)?;
            let Some(raw) = AdtsHeader::parse(&head) else {
                at += 1;
                continue;
            };
            let hlen = raw.header_len();
            if u64::from(raw.frame_length) < hlen || at + u64::from(raw.frame_length) > len {
                at += 1;
                continue;
            }
            frames += 1;
            if frames > MAX_SEEK_SCAN_FRAMES {
                return Err(error::seek_out_of_range());
            }
            if samples <= target_samples {
                landing = Some((samples, at));
            }
            samples = samples.saturating_add(i64::try_from(raw.samples()).unwrap_or(i64::MAX));
            at += u64::from(raw.frame_length);
        }
        landing.ok_or_else(error::seek_out_of_range)
    }

    /// The seek core for ADIF: pure block arithmetic (O(1)).
    fn landing_adif(&self, target_nanos: u64) -> Result<(i64, u64), EngineError> {
        let Some(RawAacFormat::Adif(info)) = self.head.as_ref() else {
            return Err(error::seek_out_of_range());
        };
        let blocks = info.block_count.unwrap_or(0);
        let rate = info.sample_rate();
        let target_block = u128::from(target_nanos) * u128::from(rate)
            / (1_000_000_000u128 * u128::from(SAMPLES_PER_BLOCK));
        let block = u64::try_from(target_block).unwrap_or(0);
        if block >= blocks {
            return Err(error::seek_out_of_range());
        }
        Ok((
            i64::try_from(block * SAMPLES_PER_BLOCK).unwrap_or(i64::MAX),
            block,
        ))
    }

    fn target_nanos(target: TimeSpan) -> u64 {
        target
            .start
            .timestamp()
            .ok()
            .and_then(|t| t.as_nanos().ok())
            .map(|n| n.0)
            .unwrap_or(0)
    }
}

/// CRC-16/MPEG-2 (poly 0x1021, init 0xFFFF, MSB-first, no xorout) —
/// the ADTS `crc_check` polynomial — over the 56 fixed+variable
/// header bits (the decode-domain raw-data protection half is not
/// computable without syntax-element parsing; documented scope).
fn crc16_header(head7: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for byte in head7 {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn be16(bytes: &[u8]) -> u16 {
    u16::from(bytes[0]) << 8 | u16::from(bytes[1])
}

/// One parsed ADIF `program_config_element` (ISO 13818-7 / 14496-3
/// §4.4.1 shape).
struct ProgramConfigElement {
    profile: u8,
    sampling_index: u8,
    channels: u16,
}

impl ProgramConfigElement {
    fn parse(bits: &mut BitReader) -> Result<ProgramConfigElement, EngineError> {
        bits.skip(4)?; // element_instance_tag
        let profile = bits.read_bits(2)? as u8;
        let sampling_index = bits.read_bits(4)? as u8;
        let front = bits.read_bits(4)? as u16;
        let side = bits.read_bits(4)? as u16;
        let back = bits.read_bits(4)? as u16;
        let lfe = bits.read_bits(2)? as u16;
        let assoc = bits.read_bits(3)? as u16;
        let valid_cc = bits.read_bits(4)? as u16;
        if bits.read_bit()? {
            bits.skip(4)?; // mono_mixdown_element_number
        }
        if bits.read_bit()? {
            bits.skip(4)?; // stereo_mixdown_element_number
        }
        if bits.read_bit()? {
            bits.skip(3)?; // matrix_mixdown_idx + pseudo_surround
        }
        let mut channels = 0u16;
        for _ in 0..front {
            let cpe = bits.read_bit()?;
            bits.skip(4)?; // element_tag_select
            channels += if cpe { 2 } else { 1 };
        }
        for _ in 0..side {
            let cpe = bits.read_bit()?;
            bits.skip(4)?;
            channels += if cpe { 2 } else { 1 };
        }
        for _ in 0..back {
            let cpe = bits.read_bit()?;
            bits.skip(4)?;
            channels += if cpe { 2 } else { 1 };
        }
        for _ in 0..lfe {
            bits.skip(4)?; // lfe_element_tag_select
            channels += 1;
        }
        for _ in 0..assoc {
            bits.skip(4)?; // assoc_data_element_tag_select
        }
        for _ in 0..valid_cc {
            bits.skip(1)?; // cc_element_is_ind_sw
            bits.skip(4)?; // valid_cc_element_tag_select
        }
        bits.align_byte()?;
        let comment_len = bits.read_bits(8)? as usize;
        if bits.bits_left() < comment_len * 8 {
            return Err(error::adif_bad_params());
        }
        bits.skip(comment_len * 8)?;
        Ok(ProgramConfigElement {
            profile,
            sampling_index,
            channels,
        })
    }
}

/// A bounded MSB-first bit reader over an in-memory head window
/// (ADIF parsing only — every read is length-checked, `None`-free
/// via typed errors).
struct BitReader<'a> {
    data: &'a [u8],
    bit_pos: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> BitReader<'a> {
        BitReader { data, bit_pos: 0 }
    }

    fn byte_pos(&self) -> usize {
        self.bit_pos / 8
    }

    fn bits_left(&self) -> usize {
        self.data.len() * 8 - self.bit_pos
    }

    fn read_bit(&mut self) -> Result<bool, EngineError> {
        self.read_bits(1).map(|v| v & 1 == 1)
    }

    fn read_byte(&mut self) -> Result<u8, EngineError> {
        self.read_bits(8).map(|v| v as u8)
    }

    fn read_bits(&mut self, n: usize) -> Result<u64, EngineError> {
        if n > 32 || self.bits_left() < n {
            return Err(error::adif_bad_params());
        }
        let mut value = 0u64;
        for _ in 0..n {
            let byte = self.data[self.bit_pos / 8];
            let bit = byte >> (7 - self.bit_pos % 8) & 1;
            value = value << 1 | u64::from(bit);
            self.bit_pos += 1;
        }
        Ok(value)
    }

    fn skip(&mut self, n: usize) -> Result<(), EngineError> {
        if self.bits_left() < n {
            return Err(error::adif_bad_params());
        }
        self.bit_pos += n;
        Ok(())
    }

    fn align_byte(&mut self) -> Result<(), EngineError> {
        let rem = self.bit_pos % 8;
        if rem != 0 {
            self.skip(8 - rem)?;
        }
        Ok(())
    }
}

impl Demuxer for AdtsDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if self.started {
            return Ok(vec![
                self.track
                    .clone()
                    .ok_or_else(|| self.fail(error::not_started()))?,
            ]);
        }
        if let Some(shape) = &self.sticky {
            return Err(shape.mint());
        }
        let open = (|| -> Result<Track, EngineError> {
            let len = self.reader.length();
            // ID3v2 at the head (bounded, possibly chained).
            let mut pos = 0u64;
            while let Some(next) = self.skip_id3(pos)? {
                pos = next;
            }
            if len - pos >= 4 && self.reader.read_prefix_at(4, pos)?[..] == *b"ADIF" {
                let info = self.open_adif(pos)?;
                self.head = Some(RawAacFormat::Adif(info.clone()));
                let rate = info.sample_rate();
                return Ok(self.mint_track(rate, info.descriptor()?));
            }
            let raw = self.open_adts(pos)?;
            let rate = raw.sample_rate().ok_or_else(error::bad_sampling_index)?;
            self.head = Some(RawAacFormat::Adts(raw));
            Ok(self.mint_track(rate, raw.descriptor()?))
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
            let step = match self.head.as_ref() {
                Some(RawAacFormat::Adif(_)) => self.step_adif(),
                _ => self.step_adts(),
            };
            match step {
                Ok(true) => continue,
                Ok(false) => {
                    self.eos = true;
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
        let save = self.reader.position();
        let target_nanos = Self::target_nanos(target);
        let result = match self.head.as_ref() {
            Some(RawAacFormat::Adif(_)) => self.landing_adif(target_nanos),
            _ => self.scan_landing_adts(target_nanos),
        };
        self.reader.seek_to(save)?;
        let (samples, _) = result?;
        // 1/1000 (coprime, non-zero): exactly `TimeBase::new`'s Ok
        // value, constructed directly — the DVB-builder precedent. The
        // fallback only exists for the dead track-absent arm (seek is
        // `not_started`-guarded; 8.A1 sweep).
        let base = self
            .track
            .as_ref()
            .map(|t| t.timing.base)
            .unwrap_or(TimeBase { num: 1, den: 1_000 });
        Ok(TimeSpan {
            start: Stamp::Present(Timestamp::new(samples, base)),
            end: Stamp::Missing,
        })
    }

    fn seekability(&self) -> SeekSupport {
        if !self.started || self.sticky.is_some() {
            return SeekSupport::None;
        }
        match self.head.as_ref() {
            // ADIF: format-inherent boundaries map time → position
            // directly (no index behind them).
            Some(RawAacFormat::Adif(_)) => SeekSupport::Fast { indexed: false },
            // ADTS: no index — landing needs a frame-header scan.
            _ => SeekSupport::SlowLinear,
        }
    }

    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        if !self.started {
            return Err(self.fail(error::not_started()));
        }
        let target_nanos = Self::target_nanos(target);
        let landing = match self.head.as_ref() {
            Some(RawAacFormat::Adif(_)) => self.landing_adif(target_nanos),
            _ => self.scan_landing_adts(target_nanos),
        };
        let (samples, position) = landing?;
        // Commit (D4: every failure above left us untouched).
        match self.head.as_ref() {
            Some(RawAacFormat::Adif(_)) => self.walk = position, // block index
            _ => self.walk = position,                           // byte offset
        }
        self.samples_clock = samples;
        self.pending.clear();
        self.eos = false;
        self.pending.push_back(DemuxEvent::Discontinuity);
        // 1/1000 (coprime, non-zero): exactly `TimeBase::new`'s Ok
        // value, constructed directly — the DVB-builder precedent. The
        // fallback only exists for the dead track-absent arm (seek is
        // `not_started`-guarded; 8.A1 sweep).
        let base = self
            .track
            .as_ref()
            .map(|t| t.timing.base)
            .unwrap_or(TimeBase { num: 1, den: 1_000 });
        Ok(TimeSpan {
            start: Stamp::Present(Timestamp::new(samples, base)),
            end: Stamp::Missing,
        })
    }
}
