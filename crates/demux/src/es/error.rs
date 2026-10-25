//! Typed error constructors for the raw elementary-stream demuxers
//! (card 5.I3a). Mirrors the `adts::error` pattern: the taxonomy is
//! media-types' (never redefined here); codes are `demux.es`-scoped,
//! stable, and carry byte counts/offsets only — never paths or
//! payload content.

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo, LimitKind};
use media_types::{EngineError, ErrorKind, PipelineStage};

const STAGE: PipelineStage = PipelineStage::Demux;

fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// The open scan window held no sync structure at all (no Annex-B
/// start code, no MPEG-audio frame header). Typed
/// `Unsupported`-after-scan: the bytes were examined and refused, not
/// skipped. `sniffed` carries the leading bytes for the hint.
pub fn no_sync(format: &'static str, sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some(format.to_string()),
                sniffed: sniffed.to_vec(),
            }),
        },
        "demux.es.no-sync",
    )
}

/// `next_event` called before `start` (contract order violation).
pub fn not_started() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.es.not-started",
    )
}

/// A declared structure ran past the end of the stream (an mp3 frame
/// header whose computed size overruns the source).
pub fn truncated(needed: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated {
                expected: needed,
                found,
            },
        },
        "demux.es.truncated",
    )
}

/// An Annex-B access unit exceeded the packet ceiling (bounded
/// memory, MP §12: the ceiling is refused, never grown).
pub fn au_too_large(attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget: super::annexb::MAX_AU_BYTES,
            attempted,
        },
        "demux.es.au-too-large",
    )
}

/// A structure that parses as a claim but declares impossible shape
/// (e.g. an mp3 header whose size arithmetic collapsed).
pub fn bad_header(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.es.bad-header",
    )
}
