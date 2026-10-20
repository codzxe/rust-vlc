//! `hdrl`/`strl` parsing → the track table (card 2.B2; doc 03 §4.3).
//!
//! Parses the AVI 1.0 header list the fixture writer emits
//! (`tools/xtask/src/cmd/fixtures.rs`, the byte-level ground truth):
//! `LIST hdrl` → one `avih` + one `LIST strl` per stream, each `strl`
//! carrying `strh` (stream header) + `strf` (format) + optional
//! `strn` (name) / `strd` (codec-private data, skipped).
//!
//! # Robustness model (pinned by the class-B fixture contract)
//!
//! Declared list/chunk sizes are **hints, not authority**. The parser
//! consumes what it structurally recognizes, in order:
//!
//! - `avih` first, then `strl` lists. The first chunk inside `hdrl`
//!   that is neither `avih`, a `LIST strl`, nor `JUNK` **ends the
//!   header parse** and hands the position back to the top-level walk
//!   (counted as damage). This is what recovers a corrupted `hdrl`
//!   size that would otherwise swallow the `movi` list: recognition
//!   stops at the `LIST movi` header and the top-level walk takes over
//!   from exactly there.
//! - Unknown chunks *inside* a `strl` are skipped (damage), because a
//!   stream list can legally carry private chunks; the surrounding
//!   structure stays trustworthy.
//! - Advisory `avih` fields (`dwStreams`, `dwTotalFrames`,
//!   `AVIF_HASINDEX`, …) are never trusted for control flow — the
//!   `strl` lists are the stream truth, the parsed `idx1` (or its
//!   absence) is the index truth. A lying advisory field is therefore
//!   *not* damage: nothing was repaired that the demuxer relies on
//!   (ffmpeg's AVI demuxer makes the same choice).

use media_types::ids::{CodecDescriptor, MediaKind, StreamId, TrackFlags, TrackId};
use media_types::time::{Stamp, TimeBase, TimeSpan, TrackTiming};
use media_types::{EngineError, Track};

use super::riff::{self, FourCC, JUNK, LIST, SourceReader};
use crate::error;

/// `avih` — the parsed fields the demuxer actually uses. Everything
/// else in the 56-byte main header is advisory (see the module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AviMainHeader {
    /// `dwMicroSecPerFrame` — advisory (timing comes from the per-stream
    /// `strh` scale/rate); exposed for observability.
    pub micros_per_frame: u32,
    /// `dwFlags` (`AVIF_*`) — advisory, never control flow.
    pub flags: u32,
    /// `dwTotalFrames` — advisory (the `movi` walk counts truth).
    pub total_frames: u32,
    /// `dwStreams` — advisory (the `strl` lists count truth).
    pub streams: u32,
    /// `dwWidth` — advisory (the video `strf` is truth).
    pub width: u32,
    /// `dwHeight` — advisory (the video `strf` is truth).
    pub height: u32,
}

/// The codec/format facts a decoder needs, parsed from `strf`.
///
/// This is the v0 seam for the decoder-config handoff (cards 2.B4 /
/// 2.B5): `AviDemuxer::stream_format` answers with this, keyed by
/// [`TrackId`], because the L0 [`Track`] vocabulary deliberately
/// carries no codec parameters (doc 01 §3.1 — the registry descriptor
/// names the codec; parameters ride the format-specific demuxer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamFormat {
    /// `vids` + `MJPG`: a BITMAPINFOHEADER-declared frame size.
    MotionJpeg {
        /// Frame width in pixels (`biWidth`).
        width: u32,
        /// Frame height in pixels (`biHeight`; a positive value — a
        /// top-down DIB is not a meaningful MJPEG declaration).
        height: u32,
    },
    /// `auds` + PCM: the WAVEFORMATEX facts.
    Pcm {
        /// Channel count (`nChannels`).
        channels: u16,
        /// Samples per second (`nSamplesPerSec`).
        samples_per_sec: u32,
        /// Bits per sample (`wBitsPerSample`).
        bits_per_sample: u16,
        /// Bytes per sample frame (`nBlockAlign`).
        block_align: u16,
    },
}

impl StreamFormat {
    /// The registry vocabulary descriptor for this format (doc 04's
    /// table names: `mjpeg`, `pcm_s16le`, …) — the string the L0
    /// `Track.codec` carries.
    pub fn codec_descriptor(&self) -> &'static str {
        match *self {
            StreamFormat::MotionJpeg { .. } => "mjpeg",
            StreamFormat::Pcm {
                bits_per_sample, ..
            } => match bits_per_sample {
                8 => "pcm_u8",
                16 => "pcm_s16le",
                24 => "pcm_s24le",
                32 => "pcm_s32le",
                _ => "pcm",
            },
        }
    }
}

/// One parsed `strl` stream: the demuxer-side track record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamDesc {
    /// The AVI stream ordinal (`strl` order; the `NN` prefix of
    /// `NNdc`/`NNwb` movi chunk ids).
    pub ordinal: u32,
    /// `strh` fccType as a media kind (video/audio; anything else is
    /// filtered before a `StreamDesc` is kept).
    pub kind: MediaKind,
    /// `strh` fccHandler (`MJPG` for the v0 video path; PCM audio has
    /// no handler).
    pub handler: FourCC,
    /// `strh` `dwScale` — with `rate`, one tick of the track's base.
    pub scale: u32,
    /// `strh` `dwRate`.
    pub rate: u32,
    /// `strh` `dwStart` (ticks; 0 in practice — the fixture writer and
    /// ffmpeg both write 0).
    pub start: u32,
    /// `strh` `dwLength` (frames for video, sample frames for PCM; 0 =
    /// not declared → a `Missing` span end, never guessed).
    pub length: u32,
    /// `strh` `dwSampleSize`: > 0 = fixed bytes-per-sample-frame (PCM);
    /// 0 = one tick per chunk (video, VBR audio).
    pub sample_size: u32,
    /// The parsed `strf` format.
    pub format: StreamFormat,
    /// The `strn` stream name, when present and valid UTF-8.
    pub title: Option<String>,
}

impl StreamDesc {
    /// The track's time base: one tick = `scale/rate` seconds — exactly
    /// one frame (video) or one sample frame (PCM with `scale = 1`,
    /// `rate = samples/sec`).
    ///
    /// # Errors
    /// [`error::bad_stream_params`] when either component is zero.
    pub fn time_base(&self, strh_offset: u64) -> Result<TimeBase, EngineError> {
        TimeBase::new(self.scale, self.rate)
            .map_err(|_| error::bad_stream_params(strh_offset))
    }

    /// The L0 [`Track`] for this stream.
    ///
    /// # Errors
    /// [`error::bad_stream_params`] if the validated base or the
    /// registry descriptor string is rejected (cannot happen for the
    /// literal vocabulary above — the constructors are still honored,
    /// never bypassed).
    pub fn track(&self, strh_offset: u64) -> Result<Track, EngineError> {
        let base = self.time_base(strh_offset)?;
        let start = i64::from(self.start);
        let end = if self.length == 0 {
            Stamp::Missing
        } else {
            Stamp::Present(media_types::Timestamp::new(
                start.saturating_add(i64::from(self.length)),
                base,
            ))
        };
        let codec =
            CodecDescriptor::new(self.format.codec_descriptor())
                .map_err(|_| error::bad_stream_params(strh_offset))?;
        Ok(Track {
            id: TrackId::new(self.ordinal),
            stream: StreamId::new(self.ordinal),
            kind: self.kind,
            // AVI 1.0's `strh` `wLanguage` is a Windows LANGID, not a
            // BCP 47 tag — mapping it is a separate, data-owning task,
            // so v0 reports no language rather than a fabricated one.
            language: None,
            codec,
            title: self.title.clone(),
            flags: TrackFlags::NONE,
            timing: TrackTiming {
                span: TimeSpan {
                    start: Stamp::Present(media_types::Timestamp::new(start, base)),
                    end,
                },
                base,
                gaps: Vec::new(),
            },
        })
    }

    /// How many ticks one chunk of `payload_bytes` advances this
    /// stream's position (the PTS ladder of the `movi` walk).
    ///
    /// Video (`dwSampleSize == 0`): one frame per chunk. PCM: bytes ÷
    /// `dwSampleSize`; a partial trailing sample floors and the caller
    /// counts damage (a PCM chunk not sample-aligned is corrupt data,
    /// not a rounding choice).
    pub fn ticks_per_chunk(&self, payload_bytes: u64) -> (i64, bool) {
        if self.sample_size == 0 {
            (1, false)
        } else {
            let whole = payload_bytes / u64::from(self.sample_size);
            let aligned = payload_bytes % u64::from(self.sample_size) == 0;
            (whole.min(i64::MAX as u64) as i64, !aligned)
        }
    }
}

/// How many bytes of `strn` payload are read at most
/// `[PROPOSAL: 4 KiB — stream names beyond that are corruption, and
/// the payload is positionally skipped past the cap anyway]`.
pub const MAX_STRN_BYTES: usize = 4096;

/// Damage observed while parsing (bounded-recovery bookkeeping — doc 03
/// §4.3(d): repairs are counted and surfaced, never silent).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DamageLog {
    /// One `(skipped bytes, reason)` pair per recovery decision; `0`
    /// bytes means "a decision, not a byte range" (e.g. a skipped
    /// unsupported stream).
    pub events: Vec<(u64, &'static str)>,
    /// Total skipped bytes (the sum of `events`).
    pub total: u64,
}

impl DamageLog {
    /// Record one recovery: `skipped` bytes (or blocks — the reason
    /// string says which) were ignored to make progress.
    pub fn note(&mut self, skipped: u64, reason: &'static str) {
        self.total = self.total.saturating_add(skipped);
        self.events.push((skipped, reason));
    }
}

/// The `hdrl` parse result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderList {
    /// The `avih` fields (zeroed when no `avih` exists — the `strl`
    /// lists remain the stream truth).
    pub main: AviMainHeader,
    /// Every kept stream, in `strl` order (unsupported stream types are
    /// filtered; each skip is recorded in `damage`).
    pub streams: Vec<StreamDesc>,
    /// Where the header parse stopped handing structure to its parent —
    /// the declared `hdrl` end when fully recognized, or the position
    /// of the first unrecognized chunk (early stop).
    pub resume_at: u64,
}

/// A zeroed advisory header.
fn zero_avih() -> AviMainHeader {
    AviMainHeader {
        micros_per_frame: 0,
        flags: 0,
        total_frames: 0,
        streams: 0,
        width: 0,
        height: 0,
    }
}

/// Parse `LIST hdrl` whose header sits at `hdrl_pos` (the `LIST`
/// fourcc), with the list's declared end and the outer bound (RIFF
/// extent ∩ stream length) the caller computed.
///
/// # Errors
/// [`error::truncated`] when the stream ends inside a structure the
/// parser must read; [`error::bad_stream_params`] for unusable `strh`
/// parameters (zero rate/scale, degenerate PCM); source errors
/// propagate.
pub fn parse_hdrl(
    reader: &mut SourceReader,
    hdrl_pos: u64,
    declared_end: u64,
    bound: u64,
    damage: &mut DamageLog,
) -> Result<HeaderList, EngineError> {
    let mut pos = hdrl_pos + 8 + 4;
    let end = declared_end.min(bound);
    if declared_end > bound {
        damage.note(declared_end - bound, "hdrl-extends-past-riff");
    }
    let mut main: Option<AviMainHeader> = None;
    let mut streams: Vec<StreamDesc> = Vec::new();
    let mut ordinal: u32 = 0;

    while pos + 8 <= end {
        let header = riff::peek_chunk_header(reader, pos)?;
        match header.fourcc.as_bytes() {
            b"avih" => {
                if main.is_some() {
                    // A second avih inside one hdrl: keep the first,
                    // skip the impostor.
                    damage.note(u64::from(header.size), "duplicate-avih");
                } else {
                    main = Some(parse_avih(reader, pos, header.size)?);
                }
                pos = advance(pos, header.size);
            }
            b"LIST" => {
                if peek_form(reader, pos)? == Some(FourCC::tag(b"strl")) {
                    if let Some(desc) = parse_strl(reader, pos, end, ordinal, damage)? {
                        streams.push(desc);
                    }
                    ordinal = ordinal.saturating_add(1);
                    pos = advance(pos, header.size);
                } else {
                    // A non-strl LIST inside hdrl — most notably the
                    // `LIST movi` a lying hdrl size tried to swallow.
                    // Stop recognizing hdrl here; the top-level walk
                    // resumes at exactly this position.
                    break;
                }
            }
            b"JUNK" => {
                pos = advance(pos, header.size);
            }
            _ => {
                // Unrecognized chunk inside hdrl: same early-stop rule
                // as a non-strl LIST (see above).
                break;
            }
        }
    }

    if pos < declared_end {
        damage.note(declared_end - pos, "hdrl-declared-size-not-honored");
    }
    Ok(HeaderList {
        main: main.unwrap_or_else(zero_avih),
        streams,
        resume_at: pos,
    })
}

/// The `strh` chunk id.
const STRH: FourCC = FourCC::tag(b"strh");
/// The `strf` chunk id.
const STRF: FourCC = FourCC::tag(b"strf");
/// The `strn` chunk id (stream name).
const STRN: FourCC = FourCC::tag(b"strn");
/// The `strd` chunk id (codec-private data — skipped).
const STRD: FourCC = FourCC::tag(b"strd");

/// Advance past a chunk payload + WORD pad (RIFF alignment).
fn advance(pos: u64, size: u32) -> u64 {
    pos + 8 + u64::from(size) + riff::pad_for(u64::from(size))
}

/// Read the 4-byte list form at `pos + 8`, non-destructively.
fn peek_form(reader: &mut SourceReader, pos: u64) -> Result<Option<FourCC>, EngineError> {
    match reader.read_exact_or_end_at(4, pos + 8)? {
        Some(bytes) => Ok(Some(FourCC::new([bytes[0], bytes[1], bytes[2], bytes[3]]))),
        None => Ok(None),
    }
}

/// Parse the 56-byte `avih` payload at `chunk_pos`.
fn parse_avih(
    reader: &mut SourceReader,
    chunk_pos: u64,
    declared_size: u32,
) -> Result<AviMainHeader, EngineError> {
    if declared_size < 56 {
        return Err(error::bad_stream_params(chunk_pos));
    }
    let Some(bytes) = reader.read_exact_or_end_at(56, chunk_pos + 8)? else {
        return Err(error::truncated(chunk_pos + 8 + 56, reader.length()));
    };
    Ok(AviMainHeader {
        micros_per_frame: riff::u32le(&bytes, 0),
        flags: riff::u32le(&bytes, 12),
        total_frames: riff::u32le(&bytes, 16),
        streams: riff::u32le(&bytes, 24),
        width: riff::u32le(&bytes, 32),
        height: riff::u32le(&bytes, 36),
    })
}

/// One `strl`'s collected raw pieces (the format is read after the
/// loop, once `strh` has revealed the stream type).
struct StrlParts {
    strh: Option<(u64, [u8; 56])>,
    strf: Option<(u64, u32)>,
    title: Option<String>,
}

/// Parse one `LIST strl` at `strl_pos` into a [`StreamDesc`] — or
/// `None` when the stream is real but outside v0's codec filter
/// (MJPEG video, PCM audio — card scope), or carries no `strh`. Every
/// skip is damage-counted with a reason.
fn parse_strl(
    reader: &mut SourceReader,
    strl_pos: u64,
    parent_end: u64,
    ordinal: u32,
    damage: &mut DamageLog,
) -> Result<Option<StreamDesc>, EngineError> {
    let declared_end = strl_pos + 8 + u64::from(peek_size(reader, strl_pos)?);
    let end = declared_end.min(parent_end).min(reader.length());
    if declared_end > end {
        damage.note(declared_end - end, "strl-extends-past-hdrl");
    }
    let mut pos = strl_pos + 8 + 4;
    let mut parts = StrlParts {
        strh: None,
        strf: None,
        title: None,
    };

    while pos + 8 <= end {
        let header = riff::peek_chunk_header(reader, pos)?;
        match header.fourcc {
            STRH => {
                if header.size < 56 {
                    return Err(error::bad_stream_params(pos));
                }
                match reader.read_exact_or_end_at(56, pos + 8)? {
                    Some(bytes) => {
                        let mut raw = [0u8; 56];
                        raw.copy_from_slice(&bytes);
                        parts.strh = Some((pos, raw));
                    }
                    None => return Err(error::truncated(pos + 8 + 56, reader.length())),
                }
                pos = advance(pos, header.size);
            }
            STRF => {
                parts.strf = Some((pos, header.size));
                pos = advance(pos, header.size);
            }
            STRN => {
                let want = (u64::from(header.size).min(MAX_STRN_BYTES as u64)) as usize;
                if want > 0 {
                    match reader.read_exact_or_end_at(want, pos + 8)? {
                        Some(bytes) => {
                            // NUL-terminated; valid UTF-8 or nothing
                            // (a title is data, never worth a failure).
                            let stop = bytes.iter().position(|b| *b == 0).unwrap_or(want);
                            parts.title =
                                std::str::from_utf8(&bytes[..stop]).ok().map(str::to_string);
                        }
                        None => {
                            return Err(error::truncated(
                                pos + 8 + want as u64,
                                reader.length(),
                            ))
                        }
                    }
                }
                pos = advance(pos, header.size);
            }
            STRD | JUNK => {
                pos = advance(pos, header.size);
            }
            _ => {
                damage.note(u64::from(header.size), "unknown-chunk-in-strl");
                pos = advance(pos, header.size);
            }
        }
    }

    let Some((strh_offset, raw)) = parts.strh else {
        // A strl without a strh is not a stream.
        damage.note(0, "strl-without-strh");
        return Ok(None);
    };
    let fcc_type = FourCC::new([raw[0], raw[1], raw[2], raw[3]]);
    let handler = FourCC::new([raw[4], raw[5], raw[6], raw[7]]);
    let scale = riff::u32le(&raw, 20);
    let rate = riff::u32le(&raw, 24);
    let start = riff::u32le(&raw, 28);
    let length = riff::u32le(&raw, 32);
    let sample_size = riff::u32le(&raw, 44);

    if scale == 0 || rate == 0 {
        return Err(error::bad_stream_params(strh_offset));
    }
    let common = StreamDesc {
        ordinal,
        kind: MediaKind::Video,
        handler,
        scale,
        rate,
        start,
        length,
        sample_size,
        format: StreamFormat::MotionJpeg { width: 0, height: 0 },
        title: parts.title,
    };

    match fcc_type.as_bytes() {
        b"vids" => {
            if handler.as_bytes() != b"MJPG" {
                damage.note(0, "unsupported-video-handler");
                return Ok(None);
            }
            let Some((strf_offset, strf_size)) = parts.strf else {
                return Err(error::bad_stream_params(strh_offset));
            };
            if strf_size < 40 {
                return Err(error::bad_stream_params(strf_offset));
            }
            let Some(bytes) = reader.read_exact_or_end_at(40, strf_offset + 8)? else {
                return Err(error::truncated(strf_offset + 8 + 40, reader.length()));
            };
            let width = riff::u32le(&bytes, 4);
            let height = riff::u32le(&bytes, 8);
            if width == 0 || height == 0 || height > i32::MAX as u32 {
                return Err(error::bad_stream_params(strf_offset));
            }
            Ok(Some(StreamDesc {
                kind: MediaKind::Video,
                format: StreamFormat::MotionJpeg { width, height },
                ..common
            }))
        }
        b"auds" => {
            let Some((strf_offset, strf_size)) = parts.strf else {
                return Err(error::bad_stream_params(strh_offset));
            };
            if strf_size < 16 {
                return Err(error::bad_stream_params(strf_offset));
            }
            let Some(bytes) = reader.read_exact_or_end_at(16, strf_offset + 8)? else {
                return Err(error::truncated(strf_offset + 8 + 16, reader.length()));
            };
            let format_tag = riff::u16le(&bytes, 0);
            let channels = riff::u16le(&bytes, 2);
            let samples_per_sec = riff::u32le(&bytes, 4);
            let block_align = riff::u16le(&bytes, 12);
            let bits_per_sample = riff::u16le(&bytes, 14);
            if format_tag == 3 && !matches!(bits_per_sample, 32 | 64) {
                // IEEE float PCM comes in f32/f64 only.
                return Err(error::bad_stream_params(strf_offset));
            }
            if format_tag == 1 {
                if channels == 0
                    || samples_per_sec == 0
                    || block_align == 0
                    || !matches!(bits_per_sample, 8 | 16 | 24 | 32)
                {
                    return Err(error::bad_stream_params(strf_offset));
                }
                if sample_size == 0 {
                    // PCM without a fixed sample-frame size has no
                    // honest chunk→sample ladder.
                    return Err(error::bad_stream_params(strh_offset));
                }
                if sample_size != u32::from(block_align) {
                    damage.note(0, "sample-size-disagrees-with-block-align");
                }
            } else if format_tag != 3 {
                damage.note(0, "unsupported-audio-format");
                return Ok(None);
            }
            if channels == 0 || samples_per_sec == 0 || block_align == 0 {
                return Err(error::bad_stream_params(strf_offset));
            }
            Ok(Some(StreamDesc {
                kind: MediaKind::Audio,
                format: StreamFormat::Pcm {
                    channels,
                    samples_per_sec,
                    bits_per_sample,
                    block_align,
                },
                ..common
            }))
        }
        _ => {
            // Text/MIDI/timecode streams: real AVI content outside the
            // v0 codec filter — skipped, observable through the reason.
            damage.note(0, "unsupported-stream-type");
            Ok(None)
        }
    }
}

/// The declared size of the chunk at `pos` (for extent math).
fn peek_size(reader: &mut SourceReader, pos: u64) -> Result<u32, EngineError> {
    riff::peek_chunk_header(reader, pos).map(|header| header.size)
}
