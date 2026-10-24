//! EBML byte primitives (card 5.D1 — self-build, C5 std-only).
//!
//! The Extensible Binary Meta Language layer every Matroska/WebM file
//! is built from: variable-length integers (IDs and sizes), the
//! element grammar (id + size + typed payload), master/binary/uint/
//! float/string/utf8 value reads, and the tolerance rules a dirty-world
//! parser needs:
//!
//! - **Unknown elements are skipped by their declared size** — never
//!   fatal, never descended into (no recursion; the parse depth is the
//!   fixed structural shape of the known masters, and the one
//!   data-driven descent the parsers make — [`master::walk`]-style
//!   child loops — carries an explicit depth counter refused past
//!   [`MAX_PARSE_DEPTH`]).
//! - **Unknown sizes** (the all-ones vint) are legal **only for master
//!   elements** and mean "extends to the parent's end"; a non-master
//!   with an unknown size is a typed [`crate::mkv::error::bad_vint`].
//!   An unknown-size `Segment`/`Cluster` therefore clamps to the file /
//!   segment end (the streaming-EBML case, doc 04 §2.2's live gap).
//! - **Declared sizes are hints**: every size is checked against the
//!   enclosing bound before it is trusted into a read or a skip; the
//!   reader clamps at the stream's physical end and reports the real
//!   available prefix (the avi vertical's contained-lie discipline,
//!   applied to EBML).
//! - `CRC-32` (0xBF) and `Void` (0xEC) elements are skipped wherever
//!   they appear (MKV files carry no element CRC by default, but
//!   muxers write `CRC-32` inside masters — the tolerance is
//!   parse-and-skip, never verification).

use media_types::EngineError;
use plugin_api::SeekOrigin;

use crate::mkv::error;

/// Maximum bytes requested per `SourceStream::read` call (the avi
/// vertical's [`crate::avi::riff::READ_STEP_MAX`] value — one shared
/// per-source-call budget shape).
pub const READ_STEP_MAX: usize = 64 * 1024;

/// The parse-depth guard (MP §12, card 5.D1): the known parse shape
/// nests at most Segment → {Info,Tracks,Cues,Cluster} →
/// {TrackEntry,BlockGroup} → {Video,Audio,children} — 5 levels of
/// interest. Every data-driven master descent counts against this
/// bound; deeper declared nesting is corruption
/// `[PROPOSAL: 16 — 3× the known shape, far under any real file]`.
pub const MAX_PARSE_DEPTH: usize = 16;

// ---- Element IDs (the value WITH its length-marker bits, the form
// ---- EBML writes on disk and every spec table names) ----------------

/// `EBML` — the header master (must be the file's first element).
pub const ID_EBML_HEADER: u32 = 0x1A45_DFA3;
/// `EBMLVersion` / `EBMLReadVersion` / `EBMLMaxIDLength` /
/// `EBMLMaxSizeLength` / `DocType` / `DocTypeVersion` /
/// `DocTypeReadVersion` (parsed as a group).
pub const ID_EBML_VERSION: u32 = 0x4286;
pub const ID_EBML_READ_VERSION: u32 = 0x42F7;
pub const ID_EBML_MAX_ID_LENGTH: u32 = 0x42F2;
pub const ID_EBML_MAX_SIZE_LENGTH: u32 = 0x42F3;
pub const ID_DOCTYPE: u32 = 0x4282;
pub const ID_DOCTYPE_VERSION: u32 = 0x4287;
pub const ID_DOCTYPE_READ_VERSION: u32 = 0x4285;
/// `Void` — padding, skipped wherever it appears.
pub const ID_VOID: u32 = 0xEC;
/// `CRC-32` — skipped (no verification; the card's CRC-less tolerance).
pub const ID_CRC32: u32 = 0xBF;
/// `Segment` — the top-level master.
pub const ID_SEGMENT: u32 = 0x1853_8067;
/// `SeekHead` — parsed-and-skipped (the start() walk finds `Cues`
/// directly; the head is advisory for this scope).
pub const ID_SEEKHEAD: u32 = 0x114D_9B74;
/// `Info` — segment information.
pub const ID_INFO: u32 = 0x1549_A966;
/// `TimestampScale` — nanoseconds per timestamp tick (default 1_000_000).
pub const ID_TIMESTAMP_SCALE: u32 = 0x2AD7_B1;
/// `Duration` — segment duration in scale units... in *seconds* (a
/// float; the spec's own unit quirk — see `header::SegmentInfo`).
pub const ID_DURATION: u32 = 0x4489;
/// `Tracks` — the track table master.
pub const ID_TRACKS: u32 = 0x1654_AE6B;
/// `TrackEntry`.
pub const ID_TRACK_ENTRY: u32 = 0xAE;
/// `TrackNumber` — the id blocks reference.
pub const ID_TRACK_NUMBER: u32 = 0xD7;
/// `TrackType` — 1 video, 2 audio, 0x11 subtitle, 0x20+ special.
pub const ID_TRACK_TYPE: u32 = 0x83;
/// `FlagLacing` — whether the track's blocks may be laced (default on).
pub const ID_FLAG_LACING: u32 = 0x9C;
/// `FlagDefault` (default-on for its kind).
pub const ID_FLAG_DEFAULT: u32 = 0x88;
/// `FlagForced`.
pub const ID_FLAG_FORCED: u32 = 0x55AA;
/// `DefaultDuration` — nanoseconds per frame (video/audio cadence).
pub const ID_DEFAULT_DURATION: u32 = 0x23E3_83;
/// `CodecID` — the registry string (`V_MPEG4/ISO/AVC`, `A_OPUS`, …).
pub const ID_CODEC_ID: u32 = 0x86;
/// `CodecPrivate` — codec initialization bytes (avcC, OpusHead, …);
/// passed through opaquely (the decoder-config seam).
pub const ID_CODEC_PRIVATE: u32 = 0x63A2;
/// `CodecDelay` — nanoseconds the codec delays output (Opus priming).
pub const ID_CODEC_DELAY: u32 = 0x56AA;
/// `SeekPreRoll` — nanoseconds to decode before the seek target (Opus).
pub const ID_SEEK_PREROLL: u32 = 0x56BB;
/// `Language` — the legacy Matroska language element (default `eng`).
pub const ID_LANGUAGE: u32 = 0x22B5_9C;
/// `LanguageIETF` — the BCP 47 language element (preferred over the
/// legacy form when both exist).
pub const ID_LANGUAGE_IETF: u32 = 0x24B7_D;
/// `Name` — the human-readable track name.
pub const ID_NAME: u32 = 0x536E;
/// `Video` master + `PixelWidth`/`PixelHeight`.
pub const ID_VIDEO: u32 = 0xE0;
pub const ID_PIXEL_WIDTH: u32 = 0xB0;
pub const ID_PIXEL_HEIGHT: u32 = 0xBA;
/// `Audio` master + `SamplingFrequency`/`Channels`/`BitDepth`.
pub const ID_AUDIO: u32 = 0xE1;
pub const ID_SAMPLING_FREQUENCY: u32 = 0xB5;
pub const ID_CHANNELS: u32 = 0x9F;
pub const ID_BIT_DEPTH: u32 = 0x55EE;
/// `Cluster` — the packet-data master.
pub const ID_CLUSTER: u32 = 0x1F43_B675;
/// `Timecode` — the cluster's absolute base timestamp (scale ticks).
pub const ID_TIMECODE: u32 = 0xE7;
/// `SimpleBlock` — the laced, keyframe-flagged block form.
pub const ID_SIMPLE_BLOCK: u32 = 0xA3;
/// `BlockGroup` — the block master (`Block` + `BlockDuration` +
/// `ReferenceBlock`).
pub const ID_BLOCK_GROUP: u32 = 0xA0;
pub const ID_BLOCK: u32 = 0xA1;
pub const ID_BLOCK_DURATION: u32 = 0x9B;
/// `ReferenceBlock` — its presence marks a non-keyframe in a
/// `BlockGroup`.
pub const ID_REFERENCE_BLOCK: u32 = 0xFB;
/// `Cues` — the seek index master.
pub const ID_CUES: u32 = 0x1C53_BB6B;
pub const ID_CUE_POINT: u32 = 0xBB;
pub const ID_CUE_TIME: u32 = 0xB3;
pub const ID_CUE_TRACK_POSITIONS: u32 = 0xB7;
pub const ID_CUE_TRACK: u32 = 0xF7;
pub const ID_CUE_CLUSTER_POSITION: u32 = 0xF1;
pub const ID_CUE_RELATIVE_POSITION: u32 = 0xF0;

// ---- Vint primitives -------------------------------------------------

/// The outcome of parsing an EBML **element ID** vint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VintId {
    /// The id value with its length-marker bits intact (the on-disk
    /// form the spec's tables name).
    pub id: u32,
    /// The vint's total length in bytes (1..=4).
    pub len: usize,
}

/// The outcome of parsing an EBML **size** vint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VintSize {
    /// The size value with the length marker stripped. Meaningless when
    /// [`VintSize::unknown`] is set (the all-ones value).
    pub value: u64,
    /// The all-ones encoding: legal only for master elements, meaning
    /// "extends to the parent's end".
    pub unknown: bool,
    /// The vint's total length in bytes (1..=8).
    pub len: usize,
}

/// Parse an element-ID vint from the head of `data`.
///
/// `Err(None)`-style is avoided: the caller knows the offset, so the
/// `Option` here is only "not enough bytes" vs the typed verdict.
pub fn parse_vint_id(data: &[u8]) -> Result<VintId, VintError> {
    let Some(&first) = data.first() else {
        return Err(VintError::NeedMore);
    };
    if first == 0 {
        // Leading zero: the length prefix must carry at least one 1
        // bit (the marker); a 0 byte is not a legal vint head.
        return Err(VintError::Invalid);
    }
    let mut len = 1usize;
    let mut mask = 0x80u8;
    while first & mask == 0 {
        mask >>= 1;
        len += 1;
        if len > 4 {
            // Element IDs are 1..=4 octets (all length-marker patterns
            // longer than four octets are reserved/invalid).
            return Err(VintError::Invalid);
        }
    }
    if data.len() < len {
        return Err(VintError::NeedMore);
    }
    let mut id = u32::from(first);
    for &b in &data[1..len] {
        id = (id << 8) | u32::from(b);
    }
    Ok(VintId { id, len })
}

/// Parse a size vint from the head of `data`.
pub fn parse_vint_size(data: &[u8]) -> Result<VintSize, VintError> {
    let Some(&first) = data.first() else {
        return Err(VintError::NeedMore);
    };
    if first == 0 {
        return Err(VintError::Invalid);
    }
    let mut len = 1usize;
    let mut mask = 0x80u8;
    while first & mask == 0 {
        mask >>= 1;
        len += 1;
        if len > 8 {
            // Sizes are 1..=8 octets; the 9th pattern does not exist.
            return Err(VintError::Invalid);
        }
    }
    if data.len() < len {
        return Err(VintError::NeedMore);
    }
    let value_bits = u64::from(first & (mask - 1));
    let mut value = value_bits;
    for &b in &data[1..len] {
        value = (value << 8) | u64::from(b);
    }
    // The unknown-size marker: every value bit set. Note the first
    // byte's marker bit is not part of `value` — the all-ones check is
    // `value_bits == mask - 1` and all data bytes 0xFF.
    let all_ones = value_bits == u64::from(mask - 1)
        && data[1..len].iter().all(|&b| b == 0xFF);
    Ok(VintSize {
        value,
        unknown: all_ones,
        len,
    })
}

/// Why a vint parse failed (the caller mints the typed container error —
/// it owns the offset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VintError {
    /// The buffer did not carry the full vint.
    NeedMore,
    /// The vint is structurally invalid (zero length byte, or a length
    /// pattern the grammar reserves).
    Invalid,
}

// ---- The bounded positional reader -----------------------------------

/// The bounded positional reader over the demuxer's byte source (the
/// mkv-zone twin of the avi vertical's `SourceReader`: same
/// per-source-call budget, same truncation-as-data classification).
pub struct EbmlReader {
    stream: Box<dyn plugin_api::SourceStream>,
    length: u64,
}

impl EbmlReader {
    /// Wrap a source stream.
    ///
    /// # Errors
    /// A source with no known length or no positional seek cannot carry
    /// a Matroska file in this demuxer's scope (cues-based seeking is
    /// positional) — typed `Unsupported` at stage `Open`.
    pub fn new(stream: Box<dyn plugin_api::SourceStream>) -> Result<EbmlReader, EngineError> {
        let unsupported = |code: &'static str| {
            EngineError::new(
                media_types::PipelineStage::Open,
                media_types::ErrorKind::Unsupported { format: None },
                media_types::error::ErrorContext::new(code),
            )
        };
        let Some(length) = stream.size() else {
            return Err(unsupported("demux.source.unknown-length"));
        };
        if !stream.supports_seek() {
            return Err(unsupported("demux.source.unseekable"));
        }
        if length == 0 {
            return Err(unsupported("demux.source.empty"));
        }
        Ok(EbmlReader { stream, length })
    }

    /// The stream's total length (known by construction).
    pub fn length(&self) -> u64 {
        self.length
    }

    /// Reposition absolutely. Errors are the source's own (stage `Open`).
    pub fn seek_to(&mut self, to: u64) -> Result<(), EngineError> {
        self.stream.seek(SeekOrigin::Start(to)).map(|_| ())
    }

    /// Read **at most** `max` bytes at absolute position `at`, bounded
    /// additionally by `until` (the enclosing element's end): whatever
    /// bytes the stream actually carries in `[at, min(at+max, until))`.
    ///
    /// Never errors on a short read — the caller classifies (a short
    /// read inside a declared extent is the truncation data of
    /// [`crate::mkv::error::truncated`]).
    ///
    /// # Errors
    /// Source errors propagate (stage `Open`, the access layer's own).
    pub fn read_prefix_at(
        &mut self,
        max: usize,
        at: u64,
        until: u64,
    ) -> Result<Vec<u8>, EngineError> {
        let bound = until.min(self.length).min(at.saturating_add(max as u64));
        let want = bound.saturating_sub(at).min(max as u64) as usize;
        self.seek_to(at)?;
        let mut out = vec![0u8; want];
        let mut filled = 0usize;
        while filled < want {
            let step = (want - filled).min(READ_STEP_MAX);
            let got = self.stream.read_into(&mut out[filled..filled + step])?;
            if got == 0 {
                out.truncate(filled);
                break;
            }
            filled += got;
        }
        out.truncate(filled);
        Ok(out)
    }

    /// Read exactly `want` bytes inside `[at, until)` — the fixed-width
    /// value read (uint/float/timecode fields).
    ///
    /// # Errors
    /// [`crate::mkv::error::truncated`] when the stream (or the
    /// enclosing bound) ends first — the caller's offset context makes
    /// the expected/found numbers precise.
    pub fn read_exact_at(
        &mut self,
        want: usize,
        at: u64,
        until: u64,
    ) -> Result<Vec<u8>, EngineError> {
        let bound = until.min(self.length);
        if at.saturating_add(want as u64) > bound {
            return Err(error::truncated(at + want as u64, bound));
        }
        let bytes = self.read_prefix_at(want, at, until)?;
        if bytes.len() < want {
            return Err(error::truncated(at + want as u64, at + bytes.len() as u64));
        }
        Ok(bytes)
    }
}

// ---- Element header + typed value reads ------------------------------

/// One parsed element header: id + size + where the payload starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementHeader {
    /// The element id (length-marker bits intact).
    pub id: u32,
    /// The declared payload size — **only meaningful when**
    /// [`ElementHeader::unknown`] is `false`.
    pub size: u64,
    /// The all-ones unknown-size encoding (master elements only).
    pub unknown: bool,
    /// Absolute position of the payload (past the id+size vints).
    pub data_pos: u64,
    /// Absolute position just past the header vints.
    pub end_of_header: u64,
}

impl ElementHeader {
    /// The element's declared end (payload included) when its size is
    /// known; `None` for the unknown-size form.
    pub fn declared_end(&self) -> Option<u64> {
        if self.unknown {
            None
        } else {
            self.data_pos.checked_add(self.size)
        }
    }
}

/// Parse the element header at `pos`, bounded by the enclosing element's
/// effective end `bound` (and the stream's physical end).
///
/// `Ok(None)` = the bound (or stream) is exactly reached — a clean walk
/// end, not truncation. A header that starts but does not complete
/// inside the *stream* is the typed truncation; one cut by the parent's
/// bound is the parent's clean end (the parent decides — mirroring the
/// avi walk's bound authority).
///
/// # Errors
/// [`crate::mkv::error::bad_vint`] for invalid vint grammar,
/// [`crate::mkv::error::truncated`] for a header cut by the stream end.
pub fn parse_header_at(
    reader: &mut EbmlReader,
    pos: u64,
    bound: u64,
) -> Result<Option<ElementHeader>, EngineError> {
    if pos >= bound {
        return Ok(None);
    }
    // A full header is at most 4 (id) + 8 (size) bytes; the window is
    // additionally cut by the parent bound so a header crossing the
    // parent's end reads only the bytes the parent declares.
    let window = (pos + 12).min(bound);
    let bytes = reader.read_prefix_at(12, pos, window)?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let vint = parse_vint_id(&bytes);
    let id = match vint {
        Ok(parsed) => parsed,
        Err(VintError::NeedMore) => {
            // The parent bound cut the window before the id completed:
            // the parent's end decides (clean).
            if window < pos + 12 {
                return Ok(None);
            }
            // The stream itself ended mid-id: truncation.
            return Err(error::truncated(pos + 1, pos + bytes.len() as u64));
        }
        Err(VintError::Invalid) => return Err(error::bad_vint(pos)),
    };
    let after_id = pos + id.len as u64;
    let size = parse_vint_size(&bytes[id.len..]);
    let size = match size {
        Ok(parsed) => parsed,
        Err(VintError::NeedMore) => {
            if window < pos + 12 {
                return Ok(None);
            }
            return Err(error::truncated(after_id + 1, pos + bytes.len() as u64));
        }
        Err(VintError::Invalid) => return Err(error::bad_vint(after_id)),
    };
    Ok(Some(ElementHeader {
        id: id.id,
        size: size.value,
        unknown: size.unknown,
        data_pos: after_id + size.len as u64,
        end_of_header: after_id + size.len as u64,
    }))
}

/// Big-endian unsigned value of `bytes` (0..=8 bytes; empty = 0 — the
/// spec's "default value written as zero-length" quirk).
pub fn uint_from_be(bytes: &[u8]) -> Option<u64> {
    if bytes.len() > 8 {
        return None;
    }
    let mut value = 0u64;
    for &b in bytes {
        value = (value << 8) | u64::from(b);
    }
    Some(value)
}

/// Big-endian signed value of `bytes` (0..=8 bytes, two's complement —
/// `ReferenceBlock` and friends).
pub fn int_from_be(bytes: &[u8]) -> Option<i64> {
    if bytes.len() > 8 {
        return None;
    }
    if bytes.is_empty() {
        return Some(0);
    }
    let negative = bytes[0] & 0x80 != 0;
    let mut value = if negative { -1i64 } else { 0i64 };
    for &b in bytes {
        value = (value << 8) | i64::from(b);
    }
    Some(value)
}

/// Float value of `bytes` per the EBML grammar: 0 bytes = 0.0, 4 bytes
/// = f32, 8 bytes = f64, anything else is invalid (`None` — the caller
/// mints [`crate::mkv::error::bad_structure`]).
pub fn float_from_be(bytes: &[u8]) -> Option<f64> {
    match bytes.len() {
        0 => Some(0.0),
        4 => Some(f64::from(f32::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]))),
        8 => Some(f64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])),
        _ => None,
    }
}

/// Interpret a payload as UTF-8 text (`Name`, `DocType`, `CodecID`).
/// `None` on invalid UTF-8 — the caller skips with damage rather than
/// fabricating a string.
pub fn utf8_from(bytes: &[u8]) -> Option<String> {
    std::str::from_utf8(bytes).ok().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_vints_roundtrip_all_four_lengths() {
        // 1-byte ids carry the 0x80 marker (Void, SimpleBlock, Timecode…).
        assert_eq!(
            parse_vint_id(&[0xEC, 0x00]),
            Ok(VintId { id: 0xEC, len: 1 })
        );
        // 2-byte ids carry the 0x40 marker (DocType, CodecPrivate, Name…).
        assert_eq!(
            parse_vint_id(&[0x42, 0x86]),
            Ok(VintId { id: 0x4286, len: 2 })
        );
        // 3-byte: 0x2AD7B1 (TimestampScale).
        assert_eq!(
            parse_vint_id(&[0x2A, 0xD7, 0xB1]),
            Ok(VintId {
                id: 0x2A_D7B1,
                len: 3
            })
        );
        // 4-byte: 0x1A45DFA3 (the EBML header).
        assert_eq!(
            parse_vint_id(&[0x1A, 0x45, 0xDF, 0xA3]),
            Ok(VintId {
                id: 0x1A45_DFA3,
                len: 4
            })
        );
    }

    #[test]
    fn id_vints_reject_invalid_grammar() {
        // Zero length byte.
        assert_eq!(parse_vint_id(&[0x00]), Err(VintError::Invalid));
        // 0x08 = 0000_1000 → first 1 at bit 3 → five-octet pattern:
        // invalid for ids (the 4-octet cap).
        assert_eq!(
            parse_vint_id(&[0x08, 0x00, 0x00, 0x00, 0x01]),
            Err(VintError::Invalid)
        );
        // 5-octet id pattern: 0x04 = 0000_0100 → len 5 → invalid for ids.
        assert_eq!(
            parse_vint_id(&[0x04, 0x00, 0x00, 0x00, 0x01]),
            Err(VintError::Invalid)
        );
        // Truncated.
        assert_eq!(parse_vint_id(&[]), Err(VintError::NeedMore));
        assert_eq!(parse_vint_id(&[0x42]), Err(VintError::NeedMore));
    }

    #[test]
    fn size_vints_roundtrip_and_flag_unknown() {
        assert_eq!(
            parse_vint_size(&[0xA3]),
            Ok(VintSize {
                value: 0x23,
                unknown: false,
                len: 1
            })
        );
        // A 2-byte size head (0x40 marker) with a zero data byte.
        assert_eq!(
            parse_vint_size(&[0x40, 0x00]),
            Ok(VintSize {
                value: 0,
                unknown: false,
                len: 2
            })
        );
        // The unknown-size marker at every width.
        assert_eq!(
            parse_vint_size(&[0xFF]),
            Ok(VintSize {
                value: 0x7F,
                unknown: true,
                len: 1
            })
        );
        assert_eq!(
            parse_vint_size(&[0x3F, 0xFF, 0xFF]),
            Ok(VintSize {
                value: 0x1F_FF_FF,
                unknown: true,
                len: 3
            })
        );
        // 0x7F is a *two*-octet head (one leading zero) — value bits
        // all ones + 0xFF data byte still spell unknown size.
        assert_eq!(
            parse_vint_size(&[0x7F, 0xFF]),
            Ok(VintSize {
                value: 0x3F_FF,
                unknown: true,
                len: 2
            })
        );
        // Eight-byte sizes (the segment's `01 xx xx …` form).
        let big = [0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x5B, 0xB9];
        assert_eq!(
            parse_vint_size(&big),
            Ok(VintSize {
                value: 0x01_5B_B9,
                unknown: false,
                len: 8
            })
        );
    }

    #[test]
    fn typed_values_follow_the_grammar() {
        assert_eq!(uint_from_be(&[]), Some(0));
        assert_eq!(uint_from_be(&[0x01]), Some(1));
        assert_eq!(uint_from_be(&[0x01, 0x86]), Some(0x186));
        assert_eq!(uint_from_be(&[0xFF; 9]), None);
        assert_eq!(int_from_be(&[0xFF, 0xFF]), Some(-1));
        assert_eq!(int_from_be(&[0x00, 0x01]), Some(1));
        assert_eq!(float_from_be(&[]), Some(0.0));
        assert_eq!(float_from_be(&0x41_20_00_00u32.to_be_bytes()), Some(10.0));
        assert_eq!(float_from_be(&10.0f64.to_be_bytes()), Some(10.0));
        assert_eq!(float_from_be(&[1, 2, 3]), None);
        assert_eq!(utf8_from(b"matroska"), Some("matroska".to_string()));
        assert_eq!(utf8_from(&[0xFF]), None);
    }
}
