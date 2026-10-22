//! Typed error constructors for the demux layer (card 2.B2).
//!
//! The error *taxonomy* is media-types' seven kinds (doc 01 §3.4) — this
//! module never redefines or wraps it. It mints
//! [`EngineError`](media_types::EngineError) values with demux-specific
//! context codes at stage [`PipelineStage::Demux`], the stage doc 03
//! §4.3(d) assigns to container failures. The one exception: errors
//! raised by the underlying [`SourceStream`](plugin_api::SourceStream)
//! are propagated unchanged (they are already stage-tagged by the access
//! layer that minted them — re-tagging an open-stage I/O failure as
//! demux corruption would lie about the boundary).
//!
//! # No secrets
//!
//! Constructors take byte counts and offsets only — never paths, URIs or
//! file content. Sniffed magic bytes are capped by the taxonomy's own
//! `Display` (16 bytes of hex, `ErrorKind` rendering); a chunk's payload
//! never enters an error.

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo, LimitKind};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// The stage tag every demux-minted error carries (doc 03 §4.3(d)).
const STAGE: PipelineStage = PipelineStage::Demux;

/// Mint a stage-`Demux` error with a stable dotted code.
fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// The bytes are not a RIFF/AVI 1.0 container (broken magic, wrong RIFF
/// form). `sniffed` carries the leading bytes for the known-format hint
/// (capped by the taxonomy's Display, never a secret).
pub fn not_avi(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("avi".to_string()),
                sniffed: sniffed.to_vec(),
            }),
        },
        "demux.avi.not-avi",
    )
}

/// The container parsed far enough to prove structure, but no `hdrl`
/// list exists inside a well-formed-sized RIFF extent — an AVI file
/// without a header is not demuxable.
pub fn no_header() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.avi.no-header",
    )
}

/// The container declares more structure than the stream carries
/// (truncated header, truncated `movi`, truncated chunk payload, chunk
/// header past the end). `expected` is the absolute end the structure
/// needed; `found` is the stream length actually available.
pub fn truncated(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "demux.avi.truncated",
    )
}

/// A chunk header is structurally invalid (size overflowing the parent
/// extent in a way the bounded-recovery clamp cannot express, or a
/// list nesting deeper than the parser accepts).
pub fn bad_structure(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.avi.bad-structure",
    )
}

/// A stream header (`strh`/`strf`) declares parameters the timing model
/// cannot honor — zero `dwRate`/`dwScale`, a PCM block align of zero
/// with a non-zero sample size, and friends (card: "bad stream params"
/// → typed error). `offset` is the stream chunk's position.
pub fn bad_stream_params(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.avi.bad-stream-params",
    )
}

/// The `movi` walk hit a damaged chunk whose cursor jump past the
/// walk bound could not be re-synchronized within the resync budget
/// (no `idx1` entry, no acceptable chunk signature in the scanned
/// window, or the attempt budget spent — W2-07, completed by the
/// W3-07 salvage continuation). The old behavior
/// here was a silent `EndOfStream` that could be forfeiting every
/// playable byte after the damage; the walk now refuses loudly
/// (D-3: recover, never forfeit silently). `offset` is the damaged
/// chunk's position.
pub fn walk_resync_failed(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.avi.walk-resync-failed",
    )
}

/// Parsed the whole header but no track survived the v0 codec filter
/// (MJPEG video, PCM audio) — the file is an AVI, just not one this
/// demuxer can emit packets for.
pub fn no_supported_streams() -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("avi".to_string()),
                sniffed: Vec::new(),
            }),
        },
        "demux.avi.no-supported-streams",
    )
}

/// A `movi` chunk payload exceeds the payload ceiling (MP §12). The
/// v0 demuxer recovers positionally instead — the oversized chunk is
/// skipped and counted as damage (`chunk-payload-over-ceiling`), never
/// read — so this constructor is currently part of the error
/// vocabulary without a minting site; it documents the ceiling's typed
/// flavor for callers that choose to fail fast.
pub fn payload_too_large(budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        "demux.avi.payload-too-large",
    )
}

/// The `idx1` table exceeds the index-entry ceiling — same MP §12 rule
/// for index memory.
pub fn index_too_large(budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        "demux.avi.index-too-large",
    )
}

/// Parsed the header and found no `LIST movi` anywhere a well-formed
/// top-level walk could reach — an AVI with tracks but no packet data
/// region is not demuxable (a lying header size can swallow the movi;
/// recognition-based resumption recovers every case the walk can).
pub fn no_movi() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.avi.no-movi",
    )
}

/// A trait method was called before `start` opened the container (or
/// after a constructor rejection): the demuxer is not in a state that
/// supports the operation. Operational shape, not container data — the
/// caller sees a stable code instead of a panic.
pub fn not_started() -> EngineError {
    mint(ErrorKind::Unsupported { format: None }, "demux.not-started")
}

/// A seek target that cannot be mapped: before the stream start, past
/// the declared end, or an absent target stamp (`TimeSpan::UNKNOWN`).
pub fn seek_out_of_range() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.avi.seek.out-of-range",
    )
}

/// A seek was asked of a container with no `idx1` **and** a source that
/// cannot be positionally scanned (unseekable stream): the linear
/// fallback is impossible (doc 03 §4.3(b) `SeekSupport::SlowLinear`
/// needs a seekable source).
pub fn seek_unindexed() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.avi.seek.unindexed",
    )
}

/// The no-index linear seek scan exceeded its parse budget (doc 03
/// §6 step 4's `SlowLinear` cap: `SeekFailed{TooSlow}` with the
/// session still at its old position and playable). `LimitKind::CpuTime`
/// is the closest L0 vocabulary for a parse-work budget
/// `[PROPOSAL: a dedicated ScanBudget LimitKind is an L0 addition for
/// the Lead if this fires in practice]`.
pub fn scan_too_slow(budget_bytes: u64, attempted_bytes: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::CpuTime,
            budget: budget_bytes,
            attempted: attempted_bytes,
        },
        "demux.avi.seek.too-slow",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_carry_demux_stage_and_stable_codes() {
        let cases: Vec<(EngineError, &str)> = vec![
            (not_avi(b"RIGF\x00\x00\x00\x00"), "demux.avi.not-avi"),
            (no_header(), "demux.avi.no-header"),
            (truncated(425_516, 12), "demux.avi.truncated"),
            (bad_structure(1_024), "demux.avi.bad-structure"),
            (bad_stream_params(200), "demux.avi.bad-stream-params"),
            (no_supported_streams(), "demux.avi.no-supported-streams"),
            (payload_too_large(100, 200), "demux.avi.payload-too-large"),
            (index_too_large(100, 200), "demux.avi.index-too-large"),
            (no_movi(), "demux.avi.no-movi"),
            (
                walk_resync_failed(1_400_632),
                "demux.avi.walk-resync-failed",
            ),
            (not_started(), "demux.not-started"),
            (seek_out_of_range(), "demux.avi.seek.out-of-range"),
            (seek_unindexed(), "demux.avi.seek.unindexed"),
            (scan_too_slow(100, 200), "demux.avi.seek.too-slow"),
        ];
        for (error, code) in cases {
            assert_eq!(error.stage, STAGE, "{code}");
            assert_eq!(error.context.code, code);
            // Display is structured data only: stage + kind + code, no
            // payload bytes, no paths (the no-secrets rule, MP §12).
            let text = error.to_string();
            assert!(text.starts_with("[demux] "), "{code}: {text}");
            assert!(text.ends_with(&format!(" ({code})")), "{code}: {text}");
        }
    }

    #[test]
    fn kinds_map_to_the_documented_taxonomy() {
        assert!(matches!(
            not_avi(b"RIFF").kind,
            ErrorKind::Unsupported { .. }
        ));
        assert_eq!(
            truncated(10, 4).kind,
            ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated {
                    expected: 10,
                    found: 4
                }
            }
        );
        assert_eq!(
            payload_too_large(10, 20).kind,
            ErrorKind::ResourceLimit {
                limit: LimitKind::Memory,
                budget: 10,
                attempted: 20
            }
        );
    }

    #[test]
    fn not_avi_sniff_capped_at_sixteen_bytes_by_display() {
        let error = not_avi(&[0xAB; 64]);
        let text = error.to_string();
        // The taxonomy caps sniffed bytes at 16 in Display; 64 input
        // bytes must not survive into the rendering.
        assert!(text.matches("ab").count() <= 16, "{text}");
    }
}
