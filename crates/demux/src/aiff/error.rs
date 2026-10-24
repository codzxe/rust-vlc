//! Typed error constructors for the AIFF demuxer (card 5.A2).
//!
//! The error *taxonomy* is media-types' seven kinds (doc 01 §3.4) —
//! never redefined here. `aiff`-scoped stable codes at stage
//! [`PipelineStage::Demux`], mirroring the crate-root `avi::error`
//! pattern (the constructors live under `aiff/` for the same scope
//! reason the WAV ones live under `wav/`).
//!
//! # No secrets
//!
//! Constructors take byte counts and offsets only — never paths, URIs
//! or payload content.

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo, LimitKind};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// The stage tag every demux-minted error carries (doc 03 §4.3(d)).
const STAGE: PipelineStage = PipelineStage::Demux;

fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// The bytes are not an IFF FORM/AIFF (or AIFC) container. `sniffed`
/// carries the leading bytes for the known-format hint (capped by the
/// taxonomy's Display, never a secret).
pub fn not_aiff(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("aiff".to_string()),
                sniffed: sniffed.to_vec(),
            }),
        },
        "demux.aiff.not-aiff",
    )
}

/// The container declares more structure than the stream carries
/// (truncated FORM head, truncated chunk header, truncated COMM,
/// SSND extent cut by the end of the file).
pub fn truncated(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "demux.aiff.truncated",
    )
}

/// A chunk is structurally invalid where bytes alone cannot say more
/// (COMM shorter than 18 bytes, SSND without its offset/blockSize
/// pair, a sample rate that is not a normalized IEEE 754 extended
/// value, a declared frame count that rounds to nonsense).
pub fn bad_structure(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.aiff.bad-structure",
    )
}

/// The COMM chunk parses but declares parameters no PCM stream can
/// honor (zero channels, zero sample size).
pub fn bad_comm(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.aiff.bad-comm",
    )
}

/// The COMM names a compression type or sample width outside the PCM
/// vocabulary this demuxer exposes (classic AIFF 8-bit *signed* has
/// no registry descriptor — the honest gap, not a guess; `ulaw`,
/// `alaw`, `ima4` and friends have no decoders). The container is an
/// AIFF; the stream is not one this demuxer can emit.
pub fn unsupported_compression(_offset: u64) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("aiff".to_string()),
                sniffed: Vec::new(),
            }),
        },
        "demux.aiff.unsupported-compression",
    )
}

/// The FORM walk found no COMM chunk — an AIFF without a COMM is not
/// demuxable.
pub fn no_comm() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.aiff.no-comm",
    )
}

/// Parsed the header but the SSND chunk is absent or carries no
/// sound bytes — there is nothing to demux.
pub fn no_ssnd() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.aiff.no-ssnd",
    )
}

/// A trait method was called before `start` opened the container (or
/// after a constructor rejection): operational shape, not container
/// data.
pub fn not_started() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.aiff.not-started",
    )
}

/// A seek target that cannot be mapped: before the stream start,
/// past the declared end, or an absent target stamp.
pub fn seek_out_of_range() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.aiff.seek.out-of-range",
    )
}

/// A payload ask exceeds the demuxer's packet ceiling. Vocabulary
/// without a minting site today (the packetizer never asks above
/// [`crate::aiff::MAX_PACKET_BYTES`]; the `avi::payload_too_large`
/// precedent).
pub fn payload_too_large(budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        "demux.aiff.payload-too-large",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_carry_demux_stage_and_aiff_codes() {
        let cases: Vec<(EngineError, &str)> = vec![
            (not_aiff(b"FORM\x00\x00\x00\x00WAVE"), "demux.aiff.not-aiff"),
            (truncated(425_516, 12), "demux.aiff.truncated"),
            (bad_structure(1_024), "demux.aiff.bad-structure"),
            (bad_comm(200), "demux.aiff.bad-comm"),
            (
                unsupported_compression(200),
                "demux.aiff.unsupported-compression",
            ),
            (no_comm(), "demux.aiff.no-comm"),
            (no_ssnd(), "demux.aiff.no-ssnd"),
            (not_started(), "demux.aiff.not-started"),
            (seek_out_of_range(), "demux.aiff.seek.out-of-range"),
            (payload_too_large(100, 200), "demux.aiff.payload-too-large"),
        ];
        for (error, code) in cases {
            assert_eq!(error.stage, STAGE, "{code}");
            assert_eq!(error.context.code, code);
            let text = error.to_string();
            assert!(text.starts_with("[demux] "), "{code}: {text}");
            assert!(text.ends_with(&format!(" ({code})")), "{code}: {text}");
        }
    }
}
