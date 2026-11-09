//! Typed error constructors for the ASF demuxer (card 6.ASF-a).
//!
//! The error *taxonomy* is media-types' seven kinds (doc 01 §3.4) —
//! never redefined here. This module mints
//! [`EngineError`](media_types::EngineError) values with `asf`-scoped
//! stable codes at stage [`PipelineStage::Demux`], the
//! `wav`/`mkv` per-module pattern (container zones stay
//! merge-disjoint, mkv/error.rs:1–11 rationale).
//!
//! # No secrets
//!
//! Constructors take byte counts, offsets and stream numbers only —
//! never paths, URIs or payload content. Sniffed magic bytes are
//! capped by the taxonomy's own `Display`.

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo, LimitKind};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// The stage tag every demux-minted error carries (doc 03 §4.3(d)).
const STAGE: PipelineStage = PipelineStage::Demux;

fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

fn asf_info() -> FormatInfo {
    FormatInfo {
        name: Some("asf".to_string()),
        sniffed: Vec::new(),
    }
}

/// The bytes are not an ASF header (neither the offset-0 Header GUID
/// nor the §2.5 broadcast prefix shape matched). `sniffed` carries
/// the leading bytes for the known-format hint (capped by the
/// taxonomy's Display, never a secret).
pub fn not_asf(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("asf".to_string()),
                sniffed: sniffed.to_vec(),
            }),
        },
        "demux.asf.not-asf",
    )
}

/// The container declares more structure than the stream carries
/// (truncated Header Object head, truncated header child, an FPO/SP
/// payload cut short). `expected` is the absolute end the structure
/// needed; `found` is what the stream actually carries.
pub fn truncated(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "demux.asf.truncated",
    )
}

/// An object's declared size is structurally invalid (below the
/// 24-byte GUID+size+… floor for its kind, or overrunning the parent
/// object / the stream) at `offset`.
pub fn bad_object_size(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.asf.bad-object-size",
    )
}

/// The Header Object's declared size exceeds the demuxer's memory
/// ceiling (MP §12): headers are read into bounded memory; a size
/// above [`crate::asf::MAX_HEADER_BYTES`](super::MAX_HEADER_BYTES) is
/// corruption or a hostile file, never a promise to allocate.
pub fn header_too_large(budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        "demux.asf.header-too-large",
    )
}

/// The header parsed but carries no audio/video stream at all — an
/// ASF with nothing to demux (the AVI no-streams / WAV no-data
/// family).
pub fn no_streams() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.asf.no-streams",
    )
}

/// The File Properties flags declare a broadcast (live, unbounded)
/// file: sizes and packet counts are unknown by design and ffprobe
/// itself refuses such files (plan §2.5, verified). v1 policy: the
/// typed refusal, never a partial parse.
pub fn broadcast_unsupported() -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(asf_info()),
        },
        "demux.asf.broadcast-unsupported",
    )
}

/// A stream declares content encryption the demuxer cannot serve.
///
/// Vocabulary without a minting site in v1: the SP field suspected of
/// carrying the encryption bit (the 2 bytes at SP+52, plan §2.3) is
/// **not** spec-verified, and a byte-patch experiment this session
/// (flipping each bit of SP+52/+53 and the stream-number high bit)
/// showed ffprobe 7.1.5 ignoring those bytes entirely — so ASF-a
/// refuses to guess and exposes them raw
/// ([`StreamRecord::flags_raw`](super::StreamRecord::flags_raw))
/// pending the spec pin or a real encrypted corpus file. The C-V6
/// acquisition and the ASF spec gate own the closing evidence.
pub fn encrypted_stream(stream: u16) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(asf_info()),
        },
        "demux.asf.encrypted-stream",
    )
    .with_source(StreamTag(stream))
}

/// Tiny source-chaining helper (the plugin-api `DemuxSourceInfo`
/// precedent): gives the refusal a stream-bearing leaf.
#[derive(Debug)]
struct StreamTag(u16);

impl std::fmt::Display for StreamTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "asf stream #{}", self.0)
    }
}
impl std::error::Error for StreamTag {}

/// An audio Stream Properties' type-specific data is not a usable
/// WAVEFORMATEX (below the 18-byte floor, or zero channels/rate —
/// the WAV `bad-fmt` consistency family).
pub fn bad_waveformatex(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.asf.bad-waveformatex",
    )
}

/// A video Stream Properties' type-specific data is not the verified
/// short VIDEOINFOHEADER form (missing the 11-byte prefix, a
/// `data_size` below the 40-byte BITMAPINFOHEADER floor, or a
/// `biSize` ≠ 40 — the only form byte-verified against ffmpeg
/// output; anything else is an unverified shape, refused rather than
/// guessed).
pub fn bad_video_info(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.asf.bad-video-info",
    )
}

/// A trait method was called before `start` opened the container —
/// operational shape, not container data.
pub fn not_started() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.asf.not-started",
    )
}

/// `next_event` before ASF-b: the Data Object is located and its
/// packet count is known, but the packet/payload grammar is the next
/// unit's scope. The honest refusal — never an empty-stream shape
/// that would read as "file has no packets" (plan §4.1).
pub fn packets_not_implemented() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.asf.packets-not-implemented",
    )
}

/// `seek` before ASF-c: the Simple Index is located but the seek
/// table and re-anchor flow are that unit's scope (plan §4.3).
///
/// Vocabulary without a minting site once ASF-c lands: kept for the
/// error-table history; seek now answers `seek.out-of-range` /
/// `seek.too-slow` or succeeds.
pub fn seek_unsupported() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.asf.seek.unsupported",
    )
}

/// A data packet's flag bytes carry a shape outside the
/// ffmpeg-verified subset (packet-flags bits 1-0 ≠ 01, an unknown
/// property-flags length-type code, or a payload-count byte without
/// its high bit) at `offset`. Refused typed, never guessed.
pub fn packet_bad_flags(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.asf.packet.bad-flags",
    )
}

/// A data packet's lengths do not close: a payload overruns the
/// packet slice, fragments land at the wrong offset, or payloads +
/// padding do not sum to the packet size (plan §4.2 vocabulary).
pub fn packet_bad_length(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.asf.packet.bad-length",
    )
}

/// A payload's replicated data is the compressed 1-byte form (no
/// fixture exercises it — ffmpeg never writes it), or a first
/// fragment arrives without the 8-byte size/time replication.
pub fn payload_bad_replication(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.asf.payload.bad-replication",
    )
}

/// A media object under reassembly would exceed the demuxer's memory
/// ceiling (MP §12): the replicated-data size field is container
/// input, never a promise to allocate.
pub fn payload_oversized(budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        "demux.asf.payload.oversized",
    )
}

/// The File Properties declares a non-fixed packet size (min ≠ max):
/// the verified packet grammar carries no per-packet length field —
/// its extent rides the FPO size — so variable-size files are an
/// unverified shape, refused typed rather than guessed.
pub fn packet_variable_size() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.asf.packet.variable-size-unsupported",
    )
}

/// A seek target beyond the container's declared play extent (past
/// the last index entry's interval or the Data Object's packets).
pub fn seek_out_of_range() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.asf.seek.out-of-range",
    )
}

/// The no-index linear seek would scan past the bounded scan budget
/// (the mkv `seek.too-slow` precedent — the engine's SlowLinear
/// contract, refused honestly when the walk cannot stay bounded).
pub fn seek_too_slow() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.asf.seek.too-slow",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_carry_demux_stage_and_asf_codes() {
        let cases: Vec<(EngineError, &str)> = vec![
            (not_asf(b"RIFT\x00\x00\x00\x00"), "demux.asf.not-asf"),
            (truncated(757, 12), "demux.asf.truncated"),
            (bad_object_size(1_024), "demux.asf.bad-object-size"),
            (
                header_too_large(1 << 20, 1 << 30),
                "demux.asf.header-too-large",
            ),
            (no_streams(), "demux.asf.no-streams"),
            (broadcast_unsupported(), "demux.asf.broadcast-unsupported"),
            (encrypted_stream(2), "demux.asf.encrypted-stream"),
            (bad_waveformatex(200), "demux.asf.bad-waveformatex"),
            (bad_video_info(388), "demux.asf.bad-video-info"),
            (not_started(), "demux.asf.not-started"),
            (
                packets_not_implemented(),
                "demux.asf.packets-not-implemented",
            ),
            (seek_unsupported(), "demux.asf.seek.unsupported"),
        ];
        for (error, code) in cases {
            assert_eq!(error.stage, STAGE, "{code}");
            assert_eq!(error.context.code, code);
            let text = error.to_string();
            assert!(text.starts_with("[demux] "), "{code}: {text}");
            assert!(text.ends_with(&format!(" ({code})")), "{code}: {text}");
        }
    }

    #[test]
    fn not_asf_sniff_capped_at_sixteen_bytes_by_display() {
        let error = not_asf(&[0xAB; 64]);
        let text = error.to_string();
        assert!(text.matches("ab").count() <= 16, "{text}");
    }

    #[test]
    fn encrypted_stream_names_the_stream_on_the_source_chain() {
        let error = encrypted_stream(3);
        assert!(std::error::Error::source(&error).is_some());
        let text = std::error::Error::source(&error)
            .map(|s| s.to_string())
            .unwrap_or_default();
        assert!(text.contains("#3"), "{text}");
    }
}
