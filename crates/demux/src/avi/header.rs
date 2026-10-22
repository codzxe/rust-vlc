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
//! - **A degenerate `strl` skips its stream, never the file** (D-3/D-9
//!   per-stream containment): every unusable `strh`/`strf` parameter
//!   set (zero rate/scale, short chunks, zero dimensions, degenerate
//!   PCM, over-ceiling channel counts) is damage-counted and the
//!   *other* streams survive. Only when no stream survives does the
//!   parse surface the existing typed "no supported streams" refusal —
//!   the honest all-bad verdict. Physical truncation stays a typed
//!   error at the exact boundary (a cut inside the header is not a
//!   per-stream decision).
//! - `biHeight` is read **signed** (D-9): a negative value is the
//!   bottom-up DIB convention, not corruption — the stream is accepted
//!   with the absolute height and a damage note recording the flip
//!   (the render-side row flip itself is a renderer concern
//!   `[PROPOSAL: compositor bottom-up present pass]`).
//! - **The audio `strf` is a WAVEFORMATEX, and ffmpeg's is often the
//!   40-byte WAVEFORMATEXTENSIBLE** (tag `0xFFFE`, SubFormat GUID;
//!   the randomized corpus showed every multi-channel int-PCM file
//!   muxed this way). The PCM family recognized here is tag `1`
//!   (integer PCM: 8/16/24/32-bit), tag `3` (IEEE float: 32/64-bit)
//!   and tag `0xFFFE` resolving to either GUID — the width comes from
//!   `wBitsPerSample` (the container width; `wValidBitsPerSample` and
//!   the channel mask are advisory for the core mapping — mask→layout
//!   is a recorded codec-side gap). Every impossible combination
//!   (tag 1 with 20 bits, tag 3 with 8 bits, an extensible blob whose
//!   GUID is neither PCM nor float, an extensible blob too short to
//!   carry the GUID) skips the stream with a damage reason — never a
//!   panic, never the file.
//! - Advisory `avih` fields (`dwStreams`, `dwTotalFrames`,
//!   `AVIF_HASINDEX`, …) are never trusted for control flow — the
//!   `strl` lists are the stream truth, the parsed `idx1` (or its
//!   absence) is the index truth. A lying advisory field is therefore
//!   *not* damage: nothing was repaired that the demuxer relies on
//!   (ffmpeg's AVI demuxer makes the same choice).

use media_types::ids::{CodecDescriptor, MediaKind, StreamId, TrackFlags, TrackId};
use media_types::time::{Stamp, TimeBase, TimeSpan, TrackTiming};
use media_types::{EngineError, Track};

use super::riff::{self, FourCC, JUNK, SourceReader};
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
        /// Frame height in pixels (`|biHeight|`): the DIB field is
        /// signed — a negative declaration is the bottom-up row order,
        /// accepted with the absolute value plus a damage note (the
        /// demuxer hands the decoder the same JPEG bytes either way;
        /// presenting a bottom-up frame is a renderer concern).
        height: u32,
    },
    /// `auds` + PCM: the WAVEFORMATEX facts.
    Pcm {
        /// Channel count (`nChannels`).
        channels: u16,
        /// Samples per second (`nSamplesPerSec`).
        samples_per_sec: u32,
        /// The sample representation the `strf` tag + `wBitsPerSample`
        /// declare (see [`PcmSample`] — AVI PCM is little-endian by
        /// container definition, so width + signedness is total).
        sample: PcmSample,
        /// Bytes per sample frame (`nBlockAlign`).
        block_align: u16,
    },
}

/// The unpacked-PCM sample representation an AVI audio `strf`
/// declares — the demuxer-side mirror of the codec registry's PCM
/// descriptor vocabulary (doc 04 §3.1: `pcm_u8`, `pcm_s16le`, …,
/// `pcm_f64le`). Kept here rather than imported from a codec crate
/// because the demux layer must not depend on codecs (`boundaries.toml`);
/// the **descriptor string is the seam**, and the codec side parses
/// exactly these names (`codecs::audio::pcm::PcmFormat`).
///
/// AVI/WAVE PCM byte order is little-endian by definition, so width +
/// signedness is the whole vocabulary — no endianness axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcmSample {
    /// Tag `1`, 8-bit container: unsigned (byte-order-free).
    U8,
    /// Tag `1`, 16-bit container: signed little-endian.
    S16Le,
    /// Tag `1`, 24-bit container: signed little-endian (3-byte groups).
    S24Le,
    /// Tag `1`, 32-bit container: signed little-endian.
    S32Le,
    /// Tag `3` (or the float SubFormat GUID), 32-bit: IEEE 754.
    F32Le,
    /// Tag `3` (or the float SubFormat GUID), 64-bit: IEEE 754.
    F64Le,
}

impl PcmSample {
    /// The registry descriptor string (doc 04 §3.1 table names — the
    /// codec-side `PcmFormat` parses exactly this vocabulary).
    #[must_use]
    pub fn descriptor(self) -> &'static str {
        match self {
            PcmSample::U8 => "pcm_u8",
            PcmSample::S16Le => "pcm_s16le",
            PcmSample::S24Le => "pcm_s24le",
            PcmSample::S32Le => "pcm_s32le",
            PcmSample::F32Le => "pcm_f32le",
            PcmSample::F64Le => "pcm_f64le",
        }
    }

    /// Container bits per sample (`wBitsPerSample`).
    #[must_use]
    pub fn bits_per_sample(self) -> u16 {
        match self {
            PcmSample::U8 => 8,
            PcmSample::S16Le => 16,
            PcmSample::S24Le => 24,
            PcmSample::S32Le | PcmSample::F32Le => 32,
            PcmSample::F64Le => 64,
        }
    }

    /// Bytes per sample in the container (bits ÷ 8, exact for every
    /// member of the vocabulary).
    #[must_use]
    pub fn bytes_per_sample(self) -> u16 {
        match self {
            PcmSample::U8 => 1,
            PcmSample::S16Le => 2,
            PcmSample::S24Le => 3,
            PcmSample::S32Le | PcmSample::F32Le => 4,
            PcmSample::F64Le => 8,
        }
    }

    /// Map a WAVE format tag + `wBitsPerSample` to the sample
    /// vocabulary. `None` is the typed refusal for every impossible
    /// combination (tag 1 with 20 bits, tag 3 with 8 bits, unknown
    /// tags …) — the caller decides skip-vs-error per the containment
    /// rules.
    #[must_use]
    pub fn from_wave_tag(tag: u16, bits_per_sample: u16) -> Option<PcmSample> {
        match tag {
            WAVE_FORMAT_PCM => match bits_per_sample {
                8 => Some(PcmSample::U8),
                16 => Some(PcmSample::S16Le),
                24 => Some(PcmSample::S24Le),
                32 => Some(PcmSample::S32Le),
                _ => None,
            },
            WAVE_FORMAT_IEEE_FLOAT => match bits_per_sample {
                32 => Some(PcmSample::F32Le),
                64 => Some(PcmSample::F64Le),
                _ => None,
            },
            _ => None,
        }
    }
}

impl StreamFormat {
    /// The registry vocabulary descriptor for this format (doc 04's
    /// table names: `mjpeg`, `pcm_s16le`, …) — the string the L0
    /// `Track.codec` carries. Every accepted PCM stream maps to a
    /// real decoder vocabulary name (the pre-fix code could emit the
    /// unregistered bare `"pcm"` for 64-bit float — an honest gap
    /// now closed by the closed vocabulary above).
    pub fn codec_descriptor(&self) -> &'static str {
        match *self {
            StreamFormat::MotionJpeg { .. } => "mjpeg",
            StreamFormat::Pcm { sample, .. } => sample.descriptor(),
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
    /// The `strh` chunk's absolute position — error context for the
    /// typed [`error::bad_stream_params`] family (the demuxer's own
    /// field, not an L0 [`Track`] concern).
    pub strh_offset: u64,
}

impl StreamDesc {
    /// The track's time base: one tick = `scale/rate` seconds — exactly
    /// one frame (video) or one sample frame (PCM with `scale = 1`,
    /// `rate = samples/sec`).
    ///
    /// # Errors
    /// [`error::bad_stream_params`] when either component is zero.
    pub fn time_base(&self, strh_offset: u64) -> Result<TimeBase, EngineError> {
        TimeBase::new(self.scale, self.rate).map_err(|_| error::bad_stream_params(strh_offset))
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
        let codec = CodecDescriptor::new(self.format.codec_descriptor())
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
        ticks_for_sample_size(self.sample_size, payload_bytes)
    }
}

/// The [`StreamDesc::ticks_per_chunk`] computation, free-standing so
/// the `movi` walk can apply it to a **clamped** payload without
/// holding a borrow of the stream table (the walk's damage calls need
/// `&mut self`).
pub(crate) fn ticks_for_sample_size(sample_size: u32, payload_bytes: u64) -> (i64, bool) {
    if sample_size == 0 {
        (1, false)
    } else {
        let whole = payload_bytes / u64::from(sample_size);
        let aligned = payload_bytes.is_multiple_of(u64::from(sample_size));
        (whole.min(i64::MAX as u64) as i64, !aligned)
    }
}

/// How many bytes of `strn` payload are read at most
/// `[PROPOSAL: 4 KiB — stream names beyond that are corruption, and
/// the payload is positionally skipped past the cap anyway]`.
pub const MAX_STRN_BYTES: usize = 4096;

/// Maximum accepted PCM channel count `[PROPOSAL: 64 — every real
/// WAVEFORMATEX layout fits (consumers cap at dozens); a `nChannels`
/// like 0xFFFF is corruption, and fabricating a layout for it would
/// violate conceal-don't-fabricate — the stream is skipped with
/// damage instead]`.
pub const MAX_PCM_CHANNELS: u16 = 64;

/// `wFormatTag` for integer PCM (the WAVE family the demuxer
/// recognizes; μ-law/A-law and friends are later codecs, not PCM
/// width variants).
const WAVE_FORMAT_PCM: u16 = 1;
/// `wFormatTag` for IEEE float PCM.
const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;
/// `wFormatTag` for WAVEFORMATEXTENSIBLE — the 40-byte form whose
/// SubFormat GUID names the real format (ffmpeg's AVI muxer emits it
/// for most multi-channel PCM; the randomized corpus is dominated by
/// it).
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;

/// `KSDATAFORMAT_SUBTYPE_PCM` — `{00000001-0000-0010-8000-00AA00389B71}`
/// (the SubFormat GUID for integer PCM inside WAVEFORMATEXTENSIBLE).
const SUBFORMAT_PCM_GUID: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];
/// `KSDATAFORMAT_SUBTYPE_IEEE_FLOAT` —
/// `{00000003-0000-0010-8000-00AA00389B71}`.
const SUBFORMAT_IEEE_FLOAT_GUID: [u8; 16] = [
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];

/// The size of the WAVEFORMATEX prefix every recognized audio `strf`
/// must carry (tag, channels, rate, average, block align, bits).
const WAVEFORMATEX_MIN: u32 = 16;
/// The size of a complete WAVEFORMATEXTENSIBLE as ffmpeg writes it
/// (18-byte WAVEFORMATEX + `cbSize`-worth of extension: valid bits,
/// channel mask, 16-byte SubFormat GUID).
const WAVEFORMATEXTENSIBLE_SIZE: u32 = 40;

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

/// A zeroed advisory header (the value the demuxer starts from when
/// no `avih` exists — the `strl` lists remain the stream truth).
pub fn zeroed_main_header() -> AviMainHeader {
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
/// parser must read (a physical cut, never a per-stream decision);
/// source errors propagate. Degenerate `strh`/`strf` parameters skip
/// the stream with damage (see the module docs) — they never abort
/// the file.
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

    // The walk stopped at the clamped end while the declared extent
    // still had structure, and the clamp is the *physical* stream end
    // (not a lying RIFF size): the file was cut between chunk
    // boundaries — typed truncation at the exact header the walk
    // needed next, never a silent empty header.
    if pos + 8 > end && pos + 8 <= declared_end && bound == reader.length() && bound < declared_end
    {
        return Err(error::truncated(pos + 8, reader.length()));
    }
    if pos < declared_end {
        damage.note(declared_end - pos, "hdrl-declared-size-not-honored");
    }
    Ok(HeaderList {
        main: main.unwrap_or_else(zeroed_main_header),
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
/// The `vprp` chunk id (video properties header — advisory metadata
/// ffmpeg authors into every clean file's video `strl`; skipped, not
/// parsed. W3-16 corpus audit: counting it as damage made all 57 clean
/// supported rows report recovered=1/damage=68 — a silent-skip peer of
/// `strd`/`JUNK`, not an unknown chunk).
const VPRP: FourCC = FourCC::tag(b"vprp");

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
/// (MJPEG video, PCM audio — card scope), carries no `strh`, or
/// declares parameters the timing model cannot honor. Every skip is
/// damage-counted with a reason; **no degenerate stream ever aborts
/// the file** (D-3/D-9 per-stream containment — the caller decides
/// what an empty stream table means).
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
                    // A strh too short to carry the timing fields: the
                    // stream is undeclared, not the file (containment).
                    damage.note(0, "strh-too-short");
                    return Ok(None);
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
                            return Err(error::truncated(pos + 8 + want as u64, reader.length()));
                        }
                    }
                }
                pos = advance(pos, header.size);
            }
            // Legitimate advisory children: strd (codec-private data),
            // JUNK (padding), vprp (video properties) carry nothing the
            // v0 stream model consumes — silently skipped so a clean
            // ffmpeg-authored file walks with zero damage. Everything
            // else really is unknown (D-3: noted, never fatal).
            STRD | JUNK | VPRP => {
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
        // No time base, no stream — but only THIS stream (the sibling
        // tracks keep playing; all-bad files surface the caller's
        // typed no-supported-streams refusal).
        damage.note(0, "zero-rate-or-scale");
        return Ok(None);
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
        format: StreamFormat::MotionJpeg {
            width: 0,
            height: 0,
        },
        title: parts.title,
        strh_offset,
    };

    match fcc_type.as_bytes() {
        b"vids" => {
            if handler.as_bytes() != b"MJPG" {
                damage.note(0, "unsupported-video-handler");
                return Ok(None);
            }
            let Some((strf_offset, strf_size)) = parts.strf else {
                damage.note(0, "strf-missing");
                return Ok(None);
            };
            if strf_size < 40 {
                damage.note(0, "strf-too-short");
                return Ok(None);
            }
            let Some(bytes) = reader.read_exact_or_end_at(40, strf_offset + 8)? else {
                return Err(error::truncated(strf_offset + 8 + 40, reader.length()));
            };
            let width = riff::u32le(&bytes, 4);
            // biHeight is SIGNED (D-9): negative = bottom-up DIB —
            // accept with the absolute height and a damage note; the
            // render-side row flip is a compositor concern.
            let height_raw = riff::i32le(&bytes, 8);
            let bottom_up = height_raw < 0;
            let height = height_raw.unsigned_abs();
            if bottom_up {
                damage.note(0, "negative-biheight-bottom-up");
            }
            if width == 0 || height == 0 || height > i32::MAX as u32 {
                // Zero or absurd (|i32::MIN| and beyond) dimensions:
                // skip the stream, never the file.
                damage.note(0, "video-dimensions-out-of-range");
                return Ok(None);
            }
            Ok(Some(StreamDesc {
                kind: MediaKind::Video,
                format: StreamFormat::MotionJpeg { width, height },
                ..common
            }))
        }
        b"auds" => {
            let Some((strf_offset, strf_size)) = parts.strf else {
                damage.note(0, "strf-missing");
                return Ok(None);
            };
            if strf_size < WAVEFORMATEX_MIN {
                damage.note(0, "strf-too-short");
                return Ok(None);
            }
            let Some(bytes) =
                reader.read_exact_or_end_at(WAVEFORMATEX_MIN as usize, strf_offset + 8)?
            else {
                return Err(error::truncated(
                    strf_offset + 8 + u64::from(WAVEFORMATEX_MIN),
                    reader.length(),
                ));
            };
            let format_tag = riff::u16le(&bytes, 0);
            let channels = riff::u16le(&bytes, 2);
            let samples_per_sec = riff::u32le(&bytes, 4);
            let block_align = riff::u16le(&bytes, 12);
            let bits_per_sample = riff::u16le(&bytes, 14);
            // The channels ceiling (V11): a huge nChannels is
            // corruption; fabricating a layout would lie — skip the
            // stream with damage instead.
            if channels > MAX_PCM_CHANNELS {
                damage.note(0, "audio-channels-over-ceiling");
                return Ok(None);
            }
            // Resolve the effective WAVE format tag: plain tags pass
            // through; WAVEFORMATEXTENSIBLE names its real format in
            // the SubFormat GUID (the 40-byte form — ffmpeg's default
            // for multi-channel PCM). A blob too short to carry the
            // GUID it declares is a per-stream skip (containment), a
            // physical cut inside it is a typed truncation.
            let effective_tag = if format_tag == WAVE_FORMAT_EXTENSIBLE {
                if strf_size < WAVEFORMATEXTENSIBLE_SIZE {
                    damage.note(0, "extensible-strf-truncated");
                    return Ok(None);
                }
                let Some(guid) = reader.read_exact_or_end_at(16, strf_offset + 8 + 24)? else {
                    return Err(error::truncated(
                        strf_offset + 8 + u64::from(WAVEFORMATEXTENSIBLE_SIZE),
                        reader.length(),
                    ));
                };
                if guid == SUBFORMAT_PCM_GUID {
                    WAVE_FORMAT_PCM
                } else if guid == SUBFORMAT_IEEE_FLOAT_GUID {
                    WAVE_FORMAT_IEEE_FLOAT
                } else {
                    // A real extensible stream naming a codec this
                    // demuxer has no PCM mapping for (the GUID
                    // vocabulary is much larger than PCM/float).
                    damage.note(0, "unsupported-audio-format");
                    return Ok(None);
                }
            } else {
                format_tag
            };
            // Tag + width → the sample vocabulary. `None` is the typed
            // refusal for every impossible combination: tag 1 with
            // 20 bits, tag 3 with 8 bits, an unrecognized tag (μ-law,
            // ADPCM, …) — all damage-counted, none fatal to the file.
            let Some(sample) = PcmSample::from_wave_tag(effective_tag, bits_per_sample) else {
                if effective_tag == WAVE_FORMAT_IEEE_FLOAT {
                    damage.note(0, "float-pcm-bit-width-unsupported");
                } else if effective_tag == WAVE_FORMAT_PCM {
                    damage.note(0, "bad-pcm-format");
                } else {
                    damage.note(0, "unsupported-audio-format");
                }
                return Ok(None);
            };
            if channels == 0 || samples_per_sec == 0 || block_align == 0 {
                damage.note(0, "bad-pcm-format");
                return Ok(None);
            }
            if sample_size == 0 {
                // PCM without a fixed sample-frame size has no honest
                // chunk→sample ladder (int and float alike — CBR is
                // the PCM contract).
                damage.note(0, "pcm-sample-size-zero");
                return Ok(None);
            }
            if sample_size != u32::from(block_align) {
                damage.note(0, "sample-size-disagrees-with-block-align");
            }
            if block_align != channels.saturating_mul(sample.bytes_per_sample()) {
                damage.note(0, "block-align-disagrees-with-format");
            }
            Ok(Some(StreamDesc {
                kind: MediaKind::Audio,
                format: StreamFormat::Pcm {
                    channels,
                    samples_per_sec,
                    sample,
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
