//! FLAC metadata: the `fLaC` probe, the STREAMINFO parse, the
//! metadata-chain walk, and the SEEKTABLE → [`SeekIndex`] conversion
//! (card 5.A1).
//!
//! Everything here is a hand-rolled, spec-driven parser (Xiph FLAC
//! format specification, public document) — no entropy coding, no
//! guessing: fixed-layout big-endian fields, bounds-checked slices,
//! typed errors on every violation, zero panics. The parse side is
//! ours because the facts it yields (stream geometry for
//! [`configure`](codec_api::Decoder::configure), seek points for
//! [`seek`]) must be available *before* any frame decoding; the claxon
//! engine is reserved for what it is best at — the entropy-coded frame
//! decode ([`decoder`]).
//!
//! # The metadata walk ([`parse_stream`])
//!
//! `fLaC` marker → block headers (`1 bit last + 7 bits type`, 24-bit
//! length) → STREAMINFO first (type 0, 34-byte body), SEEKTABLE
//! (type 3, 18 bytes per point) picked out, everything else skipped by
//! length. The frame-data start offset is the byte after the block
//! with the last-block flag — the anchor every seek offset is relative
//! to (SEEKTABLE offsets are relative to the *first frame*, spec §8).
//!
//! # Dirty-world posture
//!
//! Truncations, bad block types (127 is invalid per spec), non-multiple
//! SEEKTABLE lengths, and duplicate STREAMINFO blocks are typed
//! [`FlacError::Corrupt`]s. A *broken* seek table never fails the walk:
//! the table is advisory — resolution falls back to the binary search
//! ([`seek::SeekPlanner`]) whenever a table-derived candidate fails
//! verification.

use super::error::{FlacError, corrupt, unsupported};
use media_types::EngineError;
use media_types::error::CorruptDetail;

/// The FLAC stream marker (`fLaC`, spec §2).
pub(crate) const STREAM_MARKER: [u8; 4] = [0x66, 0x4C, 0x61, 0x43];

/// STREAMINFO body size in bytes (spec §5.1).
pub(crate) const STREAMINFO_LEN: usize = 34;

/// One seek point in the stream (spec §8), offset-relative form.
///
/// `offset` is relative to the first frame's first byte — FLAC's own
/// convention — and is translated to an absolute offset only together
/// with the frames-start anchor from [`StreamLayout`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeekPoint {
    /// First inter-channel sample number of the target frame.
    pub sample: u64,
    /// Byte offset of the target frame's header, relative to the
    /// first frame.
    pub offset: u64,
}

/// The parsed SEEKTABLE: points in ascending sample order per spec,
/// placeholders (sample `u64::MAX`) dropped — they carry no position
/// and only ever padded the table so an encoder could rewrite it in
/// place.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeekIndex {
    /// The usable points, ascending by `sample`.
    pub points: Vec<SeekPoint>,
}

impl SeekIndex {
    /// Parse a SEEKTABLE body (`18 * n` bytes). Malformed lengths are
    /// the caller's problem (the walk rejects them); placeholder rows
    /// are filtered here.
    pub(crate) fn from_body(body: &[u8]) -> SeekIndex {
        let mut points = Vec::new();
        for row in body.as_chunks::<18>().0 {
            let sample = u64::from_be_bytes(row[0..8].try_into().unwrap_or([0u8; 8]));
            let offset = u64::from_be_bytes(row[8..16].try_into().unwrap_or([0u8; 8]));
            if sample != u64::MAX {
                points.push(SeekPoint { sample, offset });
            }
        }
        SeekIndex { points }
    }
}

/// STREAMINFO (spec §5.1) — the stream geometry every later decision
/// (decode normalization, seeking, duration) is anchored on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamInfo {
    /// Minimum block size in inter-channel samples (spec floor 16).
    pub min_block_size: u16,
    /// Maximum block size in inter-channel samples (spec floor 16,
    /// ceiling 65 535).
    pub max_block_size: u16,
    /// Minimum frame size in bytes, `None` when the encoder declared
    /// unknown (0).
    pub min_frame_size: Option<u32>,
    /// Maximum frame size in bytes, `None` when unknown.
    pub max_frame_size: Option<u32>,
    /// Sample rate in Hz (spec ceiling 65 535; 0 is invalid).
    pub sample_rate: u32,
    /// Channel count, 1..=8.
    pub channels: u32,
    /// Bits per sample, 4..=32.
    pub bits_per_sample: u32,
    /// Total inter-channel samples; `None` when the encoder did not
    /// know (streaming).
    pub total_samples: Option<u64>,
    /// MD5 of the unencoded audio (verbatim; the decode path can
    /// verify it — future card).
    pub md5: [u8; 16],
}

impl StreamInfo {
    /// Whether the stream is fixed-blocksize (min == max): the fast
    /// containment test of the seek fallback.
    pub fn is_fixed_blocksize(&self) -> bool {
        self.min_block_size == self.max_block_size
    }
}

/// The result of the metadata walk: geometry, optional seek index,
/// and the absolute offset where frame data starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamLayout {
    /// The STREAMINFO facts.
    pub stream_info: StreamInfo,
    /// The SEEKTABLE, when the stream carries one (and it parsed).
    pub seek_index: Option<SeekIndex>,
    /// Absolute byte offset of the first frame header (every SEEKTABLE
    /// `offset` is relative to this).
    pub frames_start: u64,
}

/// Probe: does the buffer begin with the FLAC stream marker?
#[must_use]
pub fn is_flac(data: &[u8]) -> bool {
    data.starts_with(&STREAM_MARKER)
}

/// Parse the 34-byte STREAMINFO body (spec §5.1 layout; all fields
/// big-endian, the tail is a 64-bit packed field: 20-bit rate, 3-bit
/// channels-1, 5-bit bps-1, 36-bit total samples).
///
/// # Errors
/// [`FlacError::Corrupt`] (`flac.streaminfo.short`) when fewer than 34
/// bytes; [`FlacError::Unsupported`] when a field is outside the
/// spec's own domain (zero rate, 0 channels, bit depth out of 4..=32).
/// The internal taxonomy surfaces as the typed
/// [`ErrorKind::Unsupported`](media_types::ErrorKind::Unsupported) /
/// [`ErrorKind::Corrupt`](media_types::ErrorKind::Corrupt) pair with a
/// stable `flac.*` context code.
pub fn parse_streaminfo(body: &[u8]) -> Result<StreamInfo, EngineError> {
    parse_streaminfo_inner(body).map_err(FlacError::into_engine_error)
}

/// The taxonomy-preserving form of [`parse_streaminfo`] (module
/// internal: callers that match on [`FlacError`] variants).
pub(crate) fn parse_streaminfo_inner(body: &[u8]) -> Result<StreamInfo, FlacError> {
    if body.len() < STREAMINFO_LEN {
        return Err(corrupt(
            "flac.streaminfo.short",
            CorruptDetail::Truncated {
                expected: STREAMINFO_LEN as u64,
                found: body.len() as u64,
            },
            format!("streaminfo body is {} bytes, spec requires 34", body.len()),
        ));
    }
    let min_block_size = u16::from_be_bytes([body[0], body[1]]);
    let max_block_size = u16::from_be_bytes([body[2], body[3]]);
    let min_frame_size = read_u24(&body[4..7]);
    let max_frame_size = read_u24(&body[7..10]);
    // Packed 64-bit field: 20 + 3 + 5 + 36 bits.
    let packed = u64::from_be_bytes(body[10..18].try_into().unwrap_or([0u8; 8]));
    let sample_rate = (packed >> 44) as u32;
    let channels = ((packed >> 41) & 0x7) as u32 + 1;
    let bits_per_sample = ((packed >> 36) & 0x1f) as u32 + 1;
    let total_raw = packed & 0x0000_000f_ffff_ffff;
    let total_samples = (total_raw != 0).then_some(total_raw);
    let mut md5 = [0u8; 16];
    md5.copy_from_slice(&body[18..34]);

    if sample_rate == 0 {
        return Err(unsupported(
            "flac.streaminfo.rate",
            "sample rate 0 is invalid",
        ));
    }
    if !(4..=32).contains(&bits_per_sample) {
        return Err(unsupported(
            "flac.streaminfo.bps",
            format!("bits per sample {bits_per_sample} outside the spec range 4..=32"),
        ));
    }
    if min_block_size == 0 || max_block_size < min_block_size {
        return Err(unsupported(
            "flac.streaminfo.blocksize",
            format!("block sizes {min_block_size}..={max_block_size} violate min<=max, min>=1"),
        ));
    }
    Ok(StreamInfo {
        min_block_size,
        max_block_size,
        min_frame_size,
        max_frame_size,
        sample_rate,
        channels,
        bits_per_sample,
        total_samples,
        md5,
    })
}

/// Walk the metadata chain of a native FLAC byte stream.
///
/// # Errors
/// [`FlacError::Unsupported`] (`flac.probe.magic`) when the marker is
/// absent; [`FlacError::Corrupt`] for truncated headers/bodies, an
/// invalid block type, a missing first STREAMINFO, or a chain that
/// never sets the last-block flag. The internal taxonomy surfaces as
/// the typed [`ErrorKind::Unsupported`](media_types::ErrorKind::Unsupported)
/// / [`ErrorKind::Corrupt`](media_types::ErrorKind::Corrupt) pair with
/// a stable `flac.*` context code.
pub fn parse_stream(data: &[u8]) -> Result<StreamLayout, EngineError> {
    parse_stream_inner(data).map_err(FlacError::into_engine_error)
}

/// The taxonomy-preserving form of [`parse_stream`] (module
/// internal).
pub(crate) fn parse_stream_inner(data: &[u8]) -> Result<StreamLayout, FlacError> {
    if !is_flac(data) {
        return Err(unsupported(
            "flac.probe.magic",
            "stream does not begin with the fLaC marker",
        ));
    }
    let mut pos: usize = STREAM_MARKER.len();
    let mut stream_info: Option<StreamInfo> = None;
    let mut seek_index: Option<SeekIndex> = None;

    loop {
        // Block header: 1 byte (last flag + type), 3 bytes length.
        let header = data.get(pos..pos + 4).ok_or_else(|| {
            corrupt(
                "flac.metadata.header_truncated",
                CorruptDetail::Truncated {
                    expected: 4,
                    found: data.len().saturating_sub(pos) as u64,
                },
                "metadata block header truncated",
            )
        })?;
        let last = header[0] & 0x80 != 0;
        let block_type = header[0] & 0x7f;
        let length = u32::from_be_bytes([0, header[1], header[2], header[3]]) as usize;
        let body_start = pos + 4;
        let body = data.get(body_start..body_start + length).ok_or_else(|| {
            corrupt(
                "flac.metadata.body_truncated",
                CorruptDetail::Truncated {
                    expected: length as u64,
                    found: data.len().saturating_sub(body_start) as u64,
                },
                format!("metadata block type {block_type} truncated"),
            )
        })?;

        match block_type {
            0 => {
                if stream_info.is_some() {
                    return Err(corrupt(
                        "flac.metadata.duplicate_streaminfo",
                        CorruptDetail::InvalidStructure {
                            offset: Some(pos as u64),
                        },
                        "second STREAMINFO block",
                    ));
                }
                stream_info = Some(parse_streaminfo_inner(body)?);
            }
            3 => {
                if body.len() % 18 != 0 {
                    return Err(corrupt(
                        "flac.metadata.seektable_length",
                        CorruptDetail::InvalidStructure {
                            offset: Some(pos as u64),
                        },
                        format!("seektable length {} not a multiple of 18", body.len()),
                    ));
                }
                seek_index = Some(SeekIndex::from_body(body));
            }
            // 1 padding, 2 application, 4 vorbis comment, 5 cue sheet,
            // 6 picture, 7..=126 reserved — all skippable by length.
            // 127 is invalid per spec; reserved block types 7..=126 are
            // legal to skip (a reader must skip what it does not know).
            127 => {
                return Err(corrupt(
                    "flac.metadata.block_type",
                    CorruptDetail::InvalidStructure {
                        offset: Some(pos as u64),
                    },
                    "metadata block type 127 is invalid",
                ));
            }
            _ => {}
        }

        pos = body_start + length;
        if last {
            break;
        }
    }

    let stream_info = stream_info.ok_or_else(|| {
        corrupt(
            "flac.metadata.streaminfo_missing",
            CorruptDetail::InvalidStructure { offset: Some(4) },
            "no STREAMINFO block in the metadata chain",
        )
    })?;
    Ok(StreamLayout {
        stream_info,
        seek_index,
        frames_start: pos as u64,
    })
}

/// Read a 3-byte big-endian integer, 0 meaning "unknown" per spec.
fn read_u24(bytes: &[u8]) -> Option<u32> {
    let value = u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]);
    (value != 0).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_accepts_only_the_marker() {
        assert!(is_flac(b"fLaC\x00\x00"));
        assert!(!is_flac(b"RIFF"));
        assert!(!is_flac(b"fLa"));
        assert!(!is_flac(b""));
    }

    #[test]
    fn streaminfo_parses_the_packed_field() {
        // Hand-built body: 4096/4096 blocks, unknown frame sizes,
        // 44100 Hz, 2 channels, 16 bps, 44100 total samples, MD5 0f..ff.
        let mut body = [0u8; 34];
        body[0..2].copy_from_slice(&4096u16.to_be_bytes());
        body[2..4].copy_from_slice(&4096u16.to_be_bytes());
        // packed: rate 44100 (20 bits) << 44 | (2-1) << 41 | (16-1) << 36 | 44100
        let packed: u64 = (44_100u64 << 44) | (1 << 41) | (15 << 36) | 44_100;
        body[10..18].copy_from_slice(&packed.to_be_bytes());
        body[18] = 0xde;
        body[33] = 0xad;
        let info = match parse_streaminfo(&body) {
            Ok(i) => i,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(info.min_block_size, 4096);
        assert_eq!(info.max_block_size, 4096);
        assert_eq!(info.min_frame_size, None);
        assert_eq!(info.max_frame_size, None);
        assert_eq!(info.sample_rate, 44_100);
        assert_eq!(info.channels, 2);
        assert_eq!(info.bits_per_sample, 16);
        assert_eq!(info.total_samples, Some(44_100));
        assert_eq!(info.md5[0], 0xde);
        assert_eq!(info.md5[15], 0xad);
        assert!(info.is_fixed_blocksize());
    }

    #[test]
    fn streaminfo_rejects_short_bodies_and_bad_geometry() {
        let err = match parse_streaminfo(&[0u8; 33]) {
            Err(e) => e,
            Ok(i) => unreachable!("parsed: {i:?}"),
        };
        assert_eq!(err.context.code, "flac.streaminfo.short");
        assert!(matches!(err.kind, media_types::ErrorKind::Corrupt { .. }));
        let mut body = [0u8; 34];
        body[10..18].copy_from_slice(&0u64.to_be_bytes()); // rate 0
        let err = match parse_streaminfo(&body) {
            Err(e) => e,
            Ok(i) => unreachable!("parsed: {i:?}"),
        };
        assert_eq!(err.context.code, "flac.streaminfo.rate");
        assert!(matches!(
            err.kind,
            media_types::ErrorKind::Unsupported { .. }
        ));
    }

    #[test]
    fn seek_index_drops_placeholders_and_keeps_points() {
        let mut body = Vec::new();
        body.extend_from_slice(&100u64.to_be_bytes());
        body.extend_from_slice(&0u64.to_be_bytes());
        body.extend_from_slice(&4096u16.to_be_bytes());
        body.extend_from_slice(&u64::MAX.to_be_bytes()); // placeholder
        body.extend_from_slice(&999u64.to_be_bytes());
        body.extend_from_slice(&4096u16.to_be_bytes());
        body.extend_from_slice(&200u64.to_be_bytes());
        body.extend_from_slice(&8192u64.to_be_bytes());
        body.extend_from_slice(&4096u16.to_be_bytes());
        let index = SeekIndex::from_body(&body);
        assert_eq!(
            index.points,
            vec![
                SeekPoint {
                    sample: 100,
                    offset: 0
                },
                SeekPoint {
                    sample: 200,
                    offset: 8192
                },
            ]
        );
    }

    #[test]
    fn parse_stream_walks_blocks_and_finds_frames_start() {
        let mut data = Vec::new();
        data.extend_from_slice(&STREAM_MARKER);
        // STREAMINFO block: last=false.
        let info_body = [7u8; 34];
        data.push(0x00); // not last, type 0
        data.extend_from_slice(&u32::from(34u16).to_be_bytes()[1..4]);
        data.extend_from_slice(&info_body);
        // SEEKTABLE block: last=true, one point.
        data.push(0x83); // last=true, type 3
        data.extend_from_slice(&u32::from(18u16).to_be_bytes()[1..4]);
        data.extend_from_slice(&50u64.to_be_bytes());
        data.extend_from_slice(&1234u64.to_be_bytes());
        data.extend_from_slice(&4096u16.to_be_bytes());
        data.extend_from_slice(b"FRAME");
        let layout = match parse_stream(&data) {
            Ok(l) => l,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(layout.stream_info.md5[0], 7);
        assert_eq!(layout.frames_start as usize, data.len() - 5);
        assert_eq!(layout.seek_index.map(|i| i.points.len()), Some(1));
    }

    #[test]
    fn parse_stream_rejects_magic_and_truncation_typed() {
        for (data, code) in [
            (&b"not flac at all"[..], "flac.probe.magic"),
            (&b"fLaC"[..], "flac.metadata.header_truncated"),
        ] {
            let err = match parse_stream(data) {
                Err(e) => e,
                Ok(l) => unreachable!("{data:?} parsed as {l:?}"),
            };
            assert_eq!(err.context.code, code, "{data:?}");
        }
        // Header announces a body that is not there.
        let mut data = Vec::new();
        data.extend_from_slice(&STREAM_MARKER);
        data.push(0x80);
        data.extend_from_slice(&[0, 1, 0]);
        let err = match parse_stream(&data) {
            Err(e) => e,
            Ok(l) => unreachable!("parsed as {l:?}"),
        };
        assert_eq!(err.context.code, "flac.metadata.body_truncated");
    }
}
