//! `Info` + `Tracks` parsing → the segment facts and the track table
//! (card 5.D1).
//!
//! # Robustness model (the avi vertical's rules, applied to EBML)
//!
//! - Declared element sizes are hints: every child read is clamped to
//!   its parent's effective end and the stream's physical end; a
//!   truncated child is the typed truncation at its own boundary, a
//!   truncated parent walk stops at the bound.
//! - Unknown children are skipped by declared size (5.D1's
//!   parse-and-skip tolerance for `Tags`/`Attachments`/`Chapters`/
//!   `Editions` — the 5.D2 EXTRAS — and for anything else unknown).
//! - A degenerate `TrackEntry` (missing track number, unknown track
//!   type, over-ceiling `CodecID`) skips *that track* with damage —
//!   the other tracks and the file survive; only an empty surviving
//!   table is the caller's typed "no tracks" refusal.
//! - Fixed-width fields obey the EBML grammar: a uint wider than 8
//!   bytes or a float wider than 8 is corruption, not a big number
//!   (`bad_structure`, never an oversized allocation).
//! - Advisory fields are never trusted for control flow: `Duration`
//!   feeds the seek range check only when it parses to a finite,
//!   non-negative number; `DefaultDuration` feeds packet durations
//!   only as declared nanoseconds.

use media_types::ids::{CodecDescriptor, LanguageTag, MediaKind, TrackFlags};
use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp, TrackTiming};
use media_types::{EngineError, Track};

use crate::mkv::ebml::{
    self, ElementHeader, EbmlReader, ID_AUDIO, ID_BIT_DEPTH, ID_CHANNELS, ID_CODEC_ID,
    ID_CODEC_PRIVATE, ID_CODEC_DELAY, ID_DEFAULT_DURATION, ID_DOCTYPE, ID_DOCTYPE_READ_VERSION,
    ID_SEEK_PREROLL,
    ID_DOCTYPE_VERSION, ID_DURATION, ID_EBML_HEADER, ID_EBML_MAX_ID_LENGTH,
    ID_EBML_MAX_SIZE_LENGTH, ID_EBML_READ_VERSION, ID_EBML_VERSION, ID_FLAG_DEFAULT,
    ID_FLAG_FORCED, ID_FLAG_LACING, ID_LANGUAGE, ID_LANGUAGE_IETF, ID_NAME, ID_PIXEL_HEIGHT,
    ID_PIXEL_WIDTH, ID_SAMPLING_FREQUENCY, ID_TIMESTAMP_SCALE, ID_TRACK_ENTRY, ID_TRACK_NUMBER,
    ID_TRACK_TYPE, ID_VIDEO,
};
use crate::mkv::error;

/// `matroska` — the DocType of general MKV files.
pub const DOCTYPE_MATROSKA: &str = "matroska";
/// `webm` — the DocType of the browser-media subset.
pub const DOCTYPE_WEBM: &str = "webm";

/// Maximum tracks trusted into the table
/// `[PROPOSAL: 64 — real files carry < 20; a TrackEntry past this is
/// skipped with damage, the file survives]`.
pub const MAX_TRACKS: usize = 64;

/// Maximum `CodecPrivate` payload read into memory
/// `[PROPOSAL: 4 MiB — codec init blobs are KBs (avcC ~30 B,
/// OpusHead 19 B, Vorbis headers ~KBs); anything larger is corruption
/// and is skipped with damage, never read]`.
pub const MAX_CODEC_PRIVATE: u64 = 4 * 1024 * 1024;

/// Maximum text-field payload (`DocType`, `CodecID`, `Name`, language
/// strings) read for interpretation
/// `[PROPOSAL: 4 KiB — every real field is < 100 B; longer declared
/// text is corruption and the field is treated as absent]`.
pub const MAX_TEXT_FIELD: u64 = 4 * 1024;

/// The parsed EBML header: the file's identity facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbmlHeader {
    /// The `DocType` string (`matroska` / `webm`).
    pub doctype: String,
    /// `DocTypeVersion` (advisory).
    pub doctype_version: u64,
    /// `DocTypeReadVersion` (advisory).
    pub doctype_read_version: u64,
    /// `EBMLMaxIDLength` (advisory once ≤ 4; the grammar caps IDs at 4
    /// octets and this reader enforces that regardless).
    pub max_id_length: u64,
    /// `EBMLMaxSizeLength` (advisory once ≤ 8).
    pub max_size_length: u64,
}

/// The parsed `Info`: the segment's timing facts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentInfo {
    /// `TimestampScale` — nanoseconds per timestamp tick. The default
    /// (element absent) is 1_000_000.
    pub timestamp_scale: u32,
    /// `Duration` — the segment duration in *seconds* (the spec's own
    /// unit for this float element), when declared and finite.
    pub duration_secs: Option<f64>,
    /// The absolute position of the `Info` element (error context).
    pub offset: u64,
}

impl Default for SegmentInfo {
    fn default() -> SegmentInfo {
        SegmentInfo {
            timestamp_scale: 1_000_000,
            duration_secs: None,
            offset: 0,
        }
    }
}

impl SegmentInfo {
    /// The track time base: one tick = `timestamp_scale` nanoseconds —
    /// the format's native tick (Matroska timestamps are always in
    /// scale units).
    ///
    /// # Errors
    /// [`error::bad_structure`] when the scale is zero (the spec's
    /// default is 1e6; a written zero is corruption).
    pub fn time_base(&self) -> Result<TimeBase, EngineError> {
        TimeBase::new(self.timestamp_scale, 1_000_000_000)
            .map_err(|_| error::bad_structure(self.offset))
    }

    /// The segment duration in scale ticks, when declared (rounded to
    /// the nearest tick — the demux-standard rescale).
    pub fn duration_ticks(&self) -> Option<i64> {
        let secs = self.duration_secs?;
        if !secs.is_finite() || secs < 0.0 {
            return None;
        }
        let scale = f64::from(self.timestamp_scale);
        let ticks = secs * 1_000_000_000.0 / scale;
        if !ticks.is_finite() || ticks > i64::MAX as f64 {
            return None;
        }
        Some(ticks.round() as i64)
    }
}

/// What a Matroska `TrackType` value carries (the L0 vocabulary's
/// kinds; 0x20 logo, 0x21 buttons, 0x22 control and 0x3F metadata all
/// land in `Data` — the honest catch-all). Unknown values are `None`.
pub fn media_kind_from_track_type(track_type: u64) -> Option<MediaKind> {
    match track_type {
        1 => Some(MediaKind::Video),
        2 => Some(MediaKind::Audio),
        0x11 => Some(MediaKind::Subtitle),
        0x20 | 0x21 | 0x22 | 0x3F => Some(MediaKind::Data),
        _ => None,
    }
}

/// The codec/format facts a decoder needs — the decoder-config seam
/// (the L0 `Track` deliberately carries no codec parameters; this is
/// the mkv twin of avi's `StreamFormat`).
#[derive(Debug, Clone, PartialEq)]
pub struct TrackPrivate {
    /// The raw `CodecID` string (`V_MPEG4/ISO/AVC`, `A_OPUS`, …).
    pub codec_id: String,
    /// The `CodecPrivate` bytes (avcC, OpusHead, Vorbis headers, …),
    /// passed through opaquely. Empty when absent.
    pub codec_private: Vec<u8>,
    /// `DefaultDuration` in nanoseconds per frame, when declared.
    pub default_duration_ns: Option<u64>,
    /// `CodecDelay` in nanoseconds (output delay; Opus priming), when
    /// declared.
    pub codec_delay_ns: Option<u64>,
    /// `SeekPreRoll` in nanoseconds, when declared.
    pub seek_preroll_ns: Option<u64>,
    /// `SamplingFrequency` from the `Audio` master, when declared.
    pub sampling_frequency: Option<f64>,
    /// `Channels` from the `Audio` master, when declared.
    pub channels: Option<u64>,
    /// `BitDepth` from the `Audio` master, when declared.
    pub bit_depth: Option<u64>,
    /// `PixelWidth` from the `Video` master, when declared.
    pub pixel_width: Option<u64>,
    /// `PixelHeight` from the `Video` master, when declared.
    pub pixel_height: Option<u64>,
    /// `FlagLacing` — whether the track's blocks may be laced (default
    /// on per spec). Informational: the demuxer honors the *block's*
    /// own lacing bits (the read-side behavior libavformat and the
    /// spec agree on — a laced block on a no-lacing track is a muxer
    /// bug whose block bits still describe the data faithfully).
    pub lacing_allowed: bool,
}

/// One demuxer-side track record (the mkv twin of avi's `StreamDesc`).
#[derive(Debug, Clone, PartialEq)]
pub struct TrackEntry {
    /// The demuxer stream ordinal (position in the table, 0-based) —
    /// the L0 `TrackId`/`StreamId` value. Block track numbers are the
    /// Matroska ids and live in [`TrackEntry::number`].
    pub ordinal: u32,
    /// The Matroska track number blocks carry.
    pub number: u64,
    /// What the track carries.
    pub kind: MediaKind,
    /// The codec facts (raw id + private bytes + cadence).
    pub private: TrackPrivate,
    /// The registry-vocabulary descriptor (`Track.codec`): a known
    /// Matroska CodecID maps to the doc 04 §3.1 vocabulary name; an
    /// unknown one passes through raw — honest exposure either way.
    pub codec_descriptor: String,
    /// BCP 47 language when declared (`LanguageIETF` preferred over the
    /// legacy `Language`; an unparseable value is `None`, never
    /// fabricated).
    pub language: Option<LanguageTag>,
    /// The track name (`Name`), when valid UTF-8.
    pub title: Option<String>,
    /// Container flags (default-on, forced).
    pub flags: TrackFlags,
}

impl TrackEntry {
    /// The L0 [`Track`] for this entry (`base` is the segment scale
    /// base — validated by [`SegmentInfo::time_base`] before any track
    /// is built).
    ///
    /// # Errors
    /// Propagates the registry's descriptor rejection (cannot fire for
    /// the mapped vocabulary — the constructors are still honored,
    /// never bypassed).
    pub fn track(&self, base: TimeBase) -> Result<Track, EngineError> {
        let codec =
            CodecDescriptor::new(&self.codec_descriptor).map_err(|_| error::no_tracks())?;
        Ok(Track {
            id: media_types::ids::TrackId::new(self.ordinal),
            stream: media_types::ids::StreamId::new(self.ordinal),
            kind: self.kind,
            language: self.language.clone(),
            codec,
            title: self.title.clone(),
            flags: self.flags,
            timing: TrackTiming {
                span: TimeSpan {
                    // Matroska declares no per-track length: the span
                    // start is 0, the end is data (`Missing`), the
                    // segment `Duration` is the container-level truth.
                    start: Stamp::Present(Timestamp::new(0, base)),
                    end: Stamp::Missing,
                },
                base,
                gaps: Vec::new(),
            },
        })
    }
}

/// The declared payload length of an element header, as `usize`.
///
/// # Errors
/// [`error::bad_structure`] when the declared length overflows `usize`
/// (an impossible extent — corruption, not a big file).
fn child_len(child: &ElementHeader) -> Result<usize, EngineError> {
    usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))
}

/// The EBML-header parse (`start`'s first step).
///
/// Returns the header facts and the absolute end of the header element.
///
/// # Errors
/// [`error::not_matroska`] when the first element is not the EBML
/// header master; [`error::truncated`] when the header is cut;
/// [`error::bad_structure`] for out-of-grammar children;
/// [`error::unknown_doctype`] is the *caller's* policy (it owns the
/// DocType vocabulary);
/// [`error::unsupported_ebml_limits`] when the declared vint widths
/// exceed the grammar.
pub fn parse_ebml_header(
    reader: &mut EbmlReader,
    bound: u64,
) -> Result<(EbmlHeader, u64), EngineError> {
    let head = reader.read_prefix_at(4, 0, bound)?;
    if head.len() < 4 || u32::from_be_bytes([head[0], head[1], head[2], head[3]]) != ID_EBML_HEADER
    {
        return Err(error::not_matroska(&head));
    }
    let Some(header) = ebml::parse_header_at(reader, 0, bound)? else {
        // The magic matched but the size vint did not complete.
        return Err(error::truncated(6, reader.length().min(bound)));
    };
    let Some(header_end) = header.declared_end() else {
        // An unknown-size EBML header is not a legal use of the
        // unknown-size form — the typed refusal, not a guess.
        return Err(error::bad_structure(header.data_pos));
    };
    if header_end > bound {
        return Err(error::truncated(header_end, reader.length()));
    }
    let mut facts = EbmlHeader {
        doctype: String::new(),
        doctype_version: 0,
        doctype_read_version: 0,
        max_id_length: 4,
        max_size_length: 8,
    };
    let mut saw_doctype = false;
    let mut pos = header.data_pos;
    while pos < header_end {
        let Some(child) = ebml::parse_header_at(reader, pos, header_end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        // Text fields are read under the text ceiling; fixed-width
        // fields are 0..=8 bytes by grammar.
        let len = child_len(&child)?;
        match child.id {
            ID_DOCTYPE => {
                if len <= MAX_TEXT_FIELD as usize {
                    let bytes = reader.read_prefix_at(len, child.data_pos, child_end)?;
                    facts.doctype = ebml::utf8_from(&bytes)
                        .ok_or_else(|| error::bad_structure(child.data_pos))?;
                    saw_doctype = true;
                }
            }
            ID_EBML_MAX_ID_LENGTH => {
                let bytes = reader.read_exact_at(len.min(8), child.data_pos, child_end)?;
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                facts.max_id_length =
                    ebml::uint_from_be(&bytes).ok_or_else(|| error::bad_vint(child.data_pos))?;
                if facts.max_id_length == 0 || facts.max_id_length > 4 {
                    return Err(error::unsupported_ebml_limits());
                }
            }
            ID_EBML_MAX_SIZE_LENGTH => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                facts.max_size_length =
                    ebml::uint_from_be(&bytes).ok_or_else(|| error::bad_vint(child.data_pos))?;
                if facts.max_size_length == 0 || facts.max_size_length > 8 {
                    return Err(error::unsupported_ebml_limits());
                }
            }
            ID_EBML_VERSION | ID_EBML_READ_VERSION | ID_DOCTYPE_VERSION
            | ID_DOCTYPE_READ_VERSION => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                let value = ebml::uint_from_be(&reader.read_exact_at(
                    len,
                    child.data_pos,
                    child_end,
                )?)
                .unwrap_or(0);
                match child.id {
                    ID_DOCTYPE_VERSION => facts.doctype_version = value,
                    ID_DOCTYPE_READ_VERSION => facts.doctype_read_version = value,
                    _ => {
                        // Version fields are advisory here: the
                        // grammar this reader implements is EBML v1,
                        // and a file declaring a different *read*
                        // contract is the DocType policy's business,
                        // not the header parse's.
                    }
                }
            }
            _ => {
                // Unknown header children (the spec names none —
                // future-proof skip by declared size).
            }
        }
        pos = child_end;
    }
    if !saw_doctype {
        // A header without a DocType is a header without an identity.
        return Err(error::bad_structure(header.data_pos));
    }
    Ok((facts, header_end))
}

/// The `Info` parse: timing facts only (muxing app, title, chapter
/// translate and friends are skipped by declared size).
///
/// An unknown-size `Info` clamps to `bound` (the segment end).
///
/// # Errors
/// Typed per the module rules; a zero `TimestampScale` is stored as-is
/// and surfaces as [`error::bad_structure`] through
/// [`SegmentInfo::time_base`] (the parse itself stores the fact).
pub fn parse_info(
    reader: &mut EbmlReader,
    info: &ElementHeader,
    bound: u64,
) -> Result<SegmentInfo, EngineError> {
    let end = info.declared_end().unwrap_or(bound).min(bound);
    let mut parsed = SegmentInfo {
        offset: info.data_pos,
        ..SegmentInfo::default()
    };
    let mut pos = info.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        let len = child_len(&child)?;
        match child.id {
            ID_TIMESTAMP_SCALE => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                match ebml::uint_from_be(&bytes) {
                    Some(scale) if scale <= u64::from(u32::MAX) => {
                        parsed.timestamp_scale = scale as u32;
                    }
                    _ => return Err(error::bad_structure(child.data_pos)),
                }
            }
            ID_DURATION => {
                let bytes = reader.read_prefix_at(len.min(8), child.data_pos, child_end)?;
                match ebml::float_from_be(&bytes) {
                    Some(value) => parsed.duration_secs = Some(value),
                    None => return Err(error::bad_structure(child.data_pos)),
                }
            }
            _ => {}
        }
        pos = child_end;
    }
    Ok(parsed)
}

/// The `Tracks` parse → the demuxer-side track table plus the per-skip
/// damage notes (byte count, reason — the avi `DamageLog` shape).
///
/// Every `TrackEntry` is parsed independently; a degenerate one is
/// skipped with a damage note and the others survive.
///
/// # Errors
/// Typed per the module rules (structural damage inside a track is
/// contained; physical truncation is typed).
pub fn parse_tracks(
    reader: &mut EbmlReader,
    tracks: &ElementHeader,
    bound: u64,
) -> Result<(Vec<TrackEntry>, Vec<(u64, &'static str)>), EngineError> {
    let end = tracks.declared_end().unwrap_or(bound).min(bound);
    let mut entries: Vec<TrackEntry> = Vec::new();
    let mut damage: Vec<(u64, &'static str)> = Vec::new();
    let mut pos = tracks.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        if child.id == ID_TRACK_ENTRY {
            if entries.len() >= MAX_TRACKS {
                damage.push((child_end - pos, "track-over-ceiling"));
            } else {
                match parse_track_entry(reader, &child, child_end)? {
                    Some(mut entry) => {
                        // The table position is the demuxer ordinal
                        // (the L0 TrackId/StreamId value).
                        entry.ordinal = entries.len() as u32;
                        entries.push(entry);
                    }
                    None => damage.push((child_end - pos, "degenerate-track-entry")),
                }
            }
        }
        // Unknown `Tracks` children: skipped by declared size.
        pos = child_end;
    }
    Ok((entries, damage))
}

/// One `TrackEntry` parse. `Ok(None)` = degenerate (skip with damage,
/// the per-stream containment rule); `Ok(Some(_))` = a usable record.
fn parse_track_entry(
    reader: &mut EbmlReader,
    entry: &ElementHeader,
    end: u64,
) -> Result<Option<TrackEntry>, EngineError> {
    let mut number: Option<u64> = None;
    let mut track_type: Option<u64> = None;
    let mut codec_id: Option<String> = None;
    let mut codec_private: Vec<u8> = Vec::new();
    let mut default_duration_ns: Option<u64> = None;
    let mut codec_delay_ns: Option<u64> = None;
    let mut seek_preroll_ns: Option<u64> = None;
    let mut lacing_allowed = true;
    let mut flag_default = true;
    let mut flag_forced = false;
    let mut language_ietf: Option<String> = None;
    let mut language_legacy: Option<String> = None;
    let mut title: Option<String> = None;
    let mut sampling_frequency: Option<f64> = None;
    let mut channels: Option<u64> = None;
    let mut bit_depth: Option<u64> = None;
    let mut pixel_width: Option<u64> = None;
    let mut pixel_height: Option<u64> = None;

    let mut pos = entry.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        let len = child_len(&child)?;
        match child.id {
            ID_TRACK_NUMBER => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                number =
                    ebml::uint_from_be(&reader.read_exact_at(len, child.data_pos, child_end)?);
            }
            ID_TRACK_TYPE => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                track_type =
                    ebml::uint_from_be(&reader.read_exact_at(len, child.data_pos, child_end)?);
            }
            ID_CODEC_ID => {
                if len > MAX_TEXT_FIELD as usize {
                    // Over-ceiling codec id: the track cannot be named
                    // honestly — degenerate, skip with damage.
                    return Ok(None);
                }
                let bytes = reader.read_prefix_at(len, child.data_pos, child_end)?;
                codec_id = Some(
                    ebml::utf8_from(&bytes).ok_or_else(|| error::bad_structure(child.data_pos))?,
                );
            }
            ID_CODEC_PRIVATE => {
                if len <= MAX_CODEC_PRIVATE as usize {
                    codec_private = reader.read_prefix_at(len, child.data_pos, child_end)?;
                }
                // Over-ceiling: skipped (the bytes are never read); the
                // entry survives without its private payload — the
                // decoder seam reports the honest empty.
            }
            ID_CODEC_DELAY => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                codec_delay_ns =
                    ebml::uint_from_be(&reader.read_exact_at(len, child.data_pos, child_end)?);
            }
            ID_SEEK_PREROLL => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                seek_preroll_ns =
                    ebml::uint_from_be(&reader.read_exact_at(len, child.data_pos, child_end)?);
            }
            ID_DEFAULT_DURATION => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                default_duration_ns =
                    ebml::uint_from_be(&reader.read_exact_at(len, child.data_pos, child_end)?);
            }
            ID_FLAG_LACING => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                lacing_allowed =
                    ebml::uint_from_be(&reader.read_exact_at(len, child.data_pos, child_end)?)
                        != Some(0);
            }
            ID_FLAG_DEFAULT => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                flag_default =
                    ebml::uint_from_be(&reader.read_exact_at(len, child.data_pos, child_end)?)
                        != Some(0);
            }
            ID_FLAG_FORCED => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                flag_forced =
                    ebml::uint_from_be(&reader.read_exact_at(len, child.data_pos, child_end)?)
                        != Some(0);
            }
            ID_LANGUAGE => {
                if len <= MAX_TEXT_FIELD as usize {
                    let bytes = reader.read_prefix_at(len, child.data_pos, child_end)?;
                    language_legacy = ebml::utf8_from(&bytes);
                }
            }
            ID_LANGUAGE_IETF => {
                if len <= MAX_TEXT_FIELD as usize {
                    let bytes = reader.read_prefix_at(len, child.data_pos, child_end)?;
                    language_ietf = ebml::utf8_from(&bytes);
                }
            }
            ID_NAME => {
                if len <= MAX_TEXT_FIELD as usize {
                    let bytes = reader.read_prefix_at(len, child.data_pos, child_end)?;
                    title = ebml::utf8_from(&bytes);
                }
            }
            ID_VIDEO => {
                parse_video_master(
                    reader,
                    child.data_pos,
                    child_end,
                    &mut pixel_width,
                    &mut pixel_height,
                )?;
            }
            ID_AUDIO => {
                parse_audio_master(
                    reader,
                    child.data_pos,
                    child_end,
                    &mut sampling_frequency,
                    &mut channels,
                    &mut bit_depth,
                )?;
            }
            _ => {
                // Unknown TrackEntry children (ContentEncodings,
                // TrackOverlay, …) — skipped by declared size.
            }
        }
        pos = child_end;
    }

    // Viability: a track number the blocks could reference and a track
    // type the L0 vocabulary knows are the minimum grammar.
    let (Some(number), Some(kind)) = (number, track_type.and_then(media_kind_from_track_type))
    else {
        return Ok(None);
    };
    let codec_id = codec_id.unwrap_or_default();
    let language = language_ietf
        .or(language_legacy)
        .filter(|tag| !tag.is_empty())
        .and_then(|tag| LanguageTag::parse(&tag).ok());
    Ok(Some(TrackEntry {
        ordinal: 0, // assigned by the caller (table position)
        number,
        kind,
        private: TrackPrivate {
            codec_id: codec_id.clone(),
            codec_private,
            default_duration_ns,
            codec_delay_ns,
            seek_preroll_ns,
            sampling_frequency,
            channels,
            bit_depth,
            pixel_width,
            pixel_height,
            lacing_allowed,
        },
        codec_descriptor: codec_descriptor_for(&codec_id, bit_depth),
        language,
        title,
        flags: TrackFlags {
            default_on: flag_default,
            forced: flag_forced,
            accessibility: false,
            still_image: false,
        },
    }))
}

/// The `Video` master's facts (pixel dimensions — the renderer seam).
fn parse_video_master(
    reader: &mut EbmlReader,
    start: u64,
    end: u64,
    pixel_width: &mut Option<u64>,
    pixel_height: &mut Option<u64>,
) -> Result<(), EngineError> {
    let mut pos = start;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        let len = child_len(&child)?;
        if len <= 8 {
            let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
            match child.id {
                ID_PIXEL_WIDTH => *pixel_width = ebml::uint_from_be(&bytes),
                ID_PIXEL_HEIGHT => *pixel_height = ebml::uint_from_be(&bytes),
                _ => {}
            }
        }
        pos = child_end;
    }
    Ok(())
}

/// The `Audio` master's facts (sampling frequency, channels, bit depth
/// — the audio-config seam).
fn parse_audio_master(
    reader: &mut EbmlReader,
    start: u64,
    end: u64,
    sampling_frequency: &mut Option<f64>,
    channels: &mut Option<u64>,
    bit_depth: &mut Option<u64>,
) -> Result<(), EngineError> {
    let mut pos = start;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        let len = child_len(&child)?;
        match child.id {
            ID_SAMPLING_FREQUENCY => {
                let bytes = reader.read_prefix_at(len.min(8), child.data_pos, child_end)?;
                *sampling_frequency = ebml::float_from_be(&bytes);
            }
            ID_CHANNELS | ID_BIT_DEPTH => {
                if len <= 8 {
                    let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                    match child.id {
                        ID_CHANNELS => *channels = ebml::uint_from_be(&bytes),
                        _ => *bit_depth = ebml::uint_from_be(&bytes),
                    }
                }
            }
            _ => {}
        }
        pos = child_end;
    }
    Ok(())
}

/// Map a Matroska `CodecID` to the doc 04 §3.1 registry-vocabulary
/// descriptor. Unknown ids pass through **raw** — the honest exposure:
/// the codec registry refuses them as typed Unsupported, and the
/// descriptor renders the truth in the UI.
pub fn codec_descriptor_for(codec_id: &str, bit_depth: Option<u64>) -> String {
    if codec_id.starts_with("V_MPEG4/ISO/AVC") {
        return "h264".to_string();
    }
    match codec_id {
        "V_VP9" => return "vp9".to_string(),
        "V_VP8" => return "vp8".to_string(),
        "A_FLAC" => return "flac".to_string(),
        "A_OPUS" => return "opus".to_string(),
        "A_VORBIS" => return "vorbis".to_string(),
        "S_TEXT/UTF8" => return "subrip".to_string(),
        _ => {}
    }
    if codec_id.starts_with("A_AAC") {
        return "aac".to_string();
    }
    if codec_id.starts_with("A_PCM/INT/LIT") {
        return match bit_depth {
            Some(8) => "pcm_u8".to_string(),
            Some(16) => "pcm_s16le".to_string(),
            Some(24) => "pcm_s24le".to_string(),
            Some(32) => "pcm_s32le".to_string(),
            _ => codec_id.to_string(),
        };
    }
    if codec_id.starts_with("A_PCM/FLOAT/IEEE") {
        return match bit_depth {
            Some(32) => "pcm_f32le".to_string(),
            Some(64) => "pcm_f64le".to_string(),
            _ => codec_id.to_string(),
        };
    }
    codec_id.to_string()
}

/// Silence the `MAX_PARSE_DEPTH` import when the fixed-shape parsers
/// above carry the depth bound structurally (the constant documents the
/// bound the walk shape enforces; the cluster walker uses it at
/// runtime).
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_type_maps_to_the_l0_vocabulary() {
        assert_eq!(media_kind_from_track_type(1), Some(MediaKind::Video));
        assert_eq!(media_kind_from_track_type(2), Some(MediaKind::Audio));
        assert_eq!(media_kind_from_track_type(0x11), Some(MediaKind::Subtitle));
        assert_eq!(media_kind_from_track_type(0x20), Some(MediaKind::Data));
        assert_eq!(media_kind_from_track_type(0x3F), Some(MediaKind::Data));
        assert_eq!(media_kind_from_track_type(0), None);
        assert_eq!(media_kind_from_track_type(3), None);
    }

    #[test]
    fn segment_info_defaults_and_duration_ticks() {
        let info = SegmentInfo::default();
        assert_eq!(info.timestamp_scale, 1_000_000);
        assert!(info.duration_ticks().is_none());
        // 2.023 s at the default scale → 2023 ticks of 1 ms.
        let info = SegmentInfo {
            duration_secs: Some(2.023),
            ..SegmentInfo::default()
        };
        assert_eq!(info.duration_ticks(), Some(2023));
        // Negative / NaN durations are absent data, never negative stamps.
        let bad = SegmentInfo {
            duration_secs: Some(-1.0),
            ..SegmentInfo::default()
        };
        assert_eq!(bad.duration_ticks(), None);
        let nan = SegmentInfo {
            duration_secs: Some(f64::NAN),
            ..SegmentInfo::default()
        };
        assert_eq!(nan.duration_ticks(), None);
    }

    #[test]
    fn codec_descriptors_follow_the_registry_vocabulary() {
        assert_eq!(codec_descriptor_for("V_MPEG4/ISO/AVC", None), "h264");
        assert_eq!(codec_descriptor_for("V_MPEG4/ISO/AVC", None), "h264");
        assert_eq!(codec_descriptor_for("V_VP9", None), "vp9");
        assert_eq!(codec_descriptor_for("V_VP8", None), "vp8");
        assert_eq!(codec_descriptor_for("A_FLAC", None), "flac");
        assert_eq!(codec_descriptor_for("A_OPUS", None), "opus");
        assert_eq!(codec_descriptor_for("A_VORBIS", None), "vorbis");
        assert_eq!(codec_descriptor_for("A_AAC/MPEG4/LC", None), "aac");
        assert_eq!(codec_descriptor_for("S_TEXT/UTF8", None), "subrip");
        assert_eq!(codec_descriptor_for("A_PCM/INT/LIT", Some(16)), "pcm_s16le");
        assert_eq!(codec_descriptor_for("A_PCM/INT/LIT", Some(24)), "pcm_s24le");
        assert_eq!(codec_descriptor_for("A_PCM/FLOAT/IEEE", Some(64)), "pcm_f64le");
        // Unknown ids pass through raw (honest exposure).
        assert_eq!(codec_descriptor_for("V_AVS2", None), "V_AVS2");
        assert_eq!(codec_descriptor_for("A_PCM/INT/LIT", None), "A_PCM/INT/LIT");
    }
}
