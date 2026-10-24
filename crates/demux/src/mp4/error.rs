//! Typed error constructors for the MP4/MOV demuxer (card 5.C1).
//!
//! The error *taxonomy* is media-types' seven kinds — never redefined
//! here. This module mints [`EngineError`](media_types::EngineError)
//! values with `mp4`-scoped stable codes at stage
//! [`PipelineStage::Demux`], mirroring the `wav`/`ogg` constructor
//! pattern.
//!
//! # No secrets
//!
//! Constructors take byte counts, offsets and box kinds only — never
//! paths, URIs or payload content. Sniffed magic bytes are capped by
//! the taxonomy's own `Display`.

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo, LimitKind};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// The stage tag every demux-minted error carries.
const STAGE: PipelineStage = PipelineStage::Demux;

fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// The bytes are not an ISO-BMFF/QuickTime container (no `ftyp` with
/// a known brand, and no leading `moov`/`wide`/`free` either).
pub fn not_mp4(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("mp4".to_string()),
                sniffed: sniffed.to_vec(),
            }),
        },
        "demux.mp4.not-mp4",
    )
}

/// The container declares more structure than the stream carries
/// (truncated box header, `moov`/`moof` extent cut by end of file).
pub fn truncated(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "demux.mp4.truncated",
    )
}

/// A box header is structurally invalid where bytes alone cannot say
/// more (64-bit size below the 16-byte header floor, a non-advancing
/// extent, a zero timescale, a sample table that does not add up).
pub fn bad_box(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.mp4.bad-box",
    )
}

/// A child box extends past the end of its parent container — the
/// `child>parent` corruption class. `parent_end` is the parent's
/// declared end.
pub fn box_overruns_parent(parent_end: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(parent_end),
            },
        },
        "demux.mp4.box-overruns-parent",
    )
}

/// The box-descent depth exceeded the walk's guard (`moov → trak →
/// mdia → minf → stbl → …` is six; anything deeper than the guard is
/// a cycle-shaped lie, not a file).
pub fn depth_exceeded(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.mp4.depth-exceeded",
    )
}

/// A structural payload (`moov`, `moof`) exceeds the demuxer's
/// in-memory ceiling — refused as a typed resource limit rather than
/// trusted into an allocation.
pub fn oversized_box(_kind: &[u8; 4], budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        "demux.mp4.oversized-box",
    )
}

/// No `moov` box anywhere a well-formed top-level walk could reach.
pub fn no_moov() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.mp4.no-moov",
    )
}

/// The `moov` parsed but carries no track this demuxer can expose
/// (no `trak`, or every `trak`'s sample description is outside the
/// accepted vocabulary).
pub fn no_tracks() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.mp4.no-tracks",
    )
}

/// A `trak`'s sample tables are absent or internally inconsistent in
/// a progressive (non-fragmented) file — nothing to demux.
pub fn no_sample_table(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.mp4.no-sample-table",
    )
}

/// A trait method was called before `start` opened the container.
pub fn not_started() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.mp4.not-started",
    )
}

/// A seek target that cannot be mapped: before the stream start,
/// past the end, or an absent target stamp.
pub fn seek_out_of_range() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.mp4.seek.out-of-range",
    )
}

/// The fragment scan exceeded its bounded budget (MP §8: bounded
/// recovery — a pathological moof ladder is a typed failure, never a
/// hang).
pub fn scan_budget_exhausted() -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::RecoveryAttempts,
            budget: super::MAX_FRAGMENT_SCAN_BOXES as u64,
            attempted: super::MAX_FRAGMENT_SCAN_BOXES as u64,
        },
        "demux.mp4.scan-budget-exhausted",
    )
}
