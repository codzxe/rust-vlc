//! Typed error constructors for the Matroska/WebM demuxer (card 5.D1).
//!
//! The error *taxonomy* is media-types' (doc 01 §3.4) — never redefined
//! here, exactly like the crate-level [`crate::error`] module. This
//! sibling exists so the mkv zone stays self-contained (`mkv/**` is the
//! card's owned surface; `src/error.rs` is the AVI vertical's shared
//! file that five Phase-5 container agents would otherwise collide on —
//! the R3 registry merge is expected to hoist a shared set later).
//!
//! Every constructor mints stage [`PipelineStage::Demux`] with a stable
//! `demux.mkv.*` context code.
//!
//! # No secrets
//!
//! Constructors take byte counts, offsets and format *names* only —
//! never paths, URIs or file content. Sniffed magic / DocType bytes are
//! capped by the taxonomy's own `Display` (16 hex bytes), never a
//! payload.

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo, LimitKind};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// The stage tag every mkv-minted error carries (doc 03 §4.3(d)).
const STAGE: PipelineStage = PipelineStage::Demux;

/// Mint a stage-`Demux` error with a stable dotted code.
fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// The bytes are not an EBML/Matroska container (broken `0x1A45DFA3`
/// magic or no EBML header as the first element). `sniffed` carries the
/// leading bytes for the known-format hint (capped by the taxonomy's
/// Display, never a secret).
pub fn not_matroska(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("matroska".to_string()),
                sniffed: sniffed.to_vec(),
            }),
        },
        "demux.mkv.not-matroska",
    )
}

/// The EBML header parsed, but its `DocType` is neither `matroska` nor
/// `webm` — the unknown-DocType policy is the typed refusal, not a
/// guess (card 5.D1 error-path list). `doctype` carries the rejected
/// identifier for the error rendering (a format *name*, not payload).
pub fn unknown_doctype(doctype: &str) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some(doctype.to_string()),
                sniffed: Vec::new(),
            }),
        },
        "demux.mkv.unknown-doctype",
    )
}

/// The EBML header declares `EBMLMaxIDLength`/`EBMLMaxSizeLength`
/// outside the 1..=8 the spec caps — not a file this parser promises to
/// read (the typed refusal, never a guess).
pub fn unsupported_ebml_limits() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.mkv.ebml-limits",
    )
}

/// The container declares more structure than the stream carries
/// (truncated EBML header, truncated master element, truncated block
/// payload at a structure boundary). `expected` is the absolute end the
/// structure needed; `found` is the stream length actually available.
pub fn truncated(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "demux.mkv.truncated",
    )
}

/// An EBML variable-length integer is structurally invalid: a zero
/// length byte (leading zero), an element ID longer than 4 octets, an
/// unknown-size marker on a non-master element, or a size vint that
/// overruns its read window.
pub fn bad_vint(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.mkv.bad-vint",
    )
}

/// Structure the parser cannot honor at `offset`: a master element
/// nested deeper than the parse-depth guard, a fixed-value field out of
/// grammar (float width ≠ 0/4/8), or a known element whose declared
/// extent contradicts its parent.
pub fn bad_structure(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.mkv.bad-structure",
    )
}

/// A block's timecode arithmetic overflows the presentation timeline
/// (cluster timecode + relative timecode outside `i64` scale ticks, or
/// a negative absolute stamp). Wrong timestamps are corruption, not a
/// rounding choice — the typed `Corrupt::Timing` flavor.
pub fn timecode_overflow(_offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Timing,
        },
        "demux.mkv.timecode-overflow",
    )
}

/// The header parsed but no track survived the track table
/// (a Matroska file with zero `TrackEntry` records cannot demux).
pub fn no_tracks() -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("matroska".to_string()),
                sniffed: Vec::new(),
            }),
        },
        "demux.mkv.no-tracks",
    )
}

/// The segment carries no `Cluster` element — tracks without packet
/// data are metadata-only files (the honest refusal, never a fake
/// empty stream).
pub fn no_clusters() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.mkv.no-clusters",
    )
}

/// The `Cues` element exceeds the cue-entry ceiling (MP §12 index
/// memory) — the index is refused (the seek falls back to the bounded
/// cluster scan) rather than trusted into an unbounded allocation.
pub fn cue_table_too_large(budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        "demux.mkv.cue-table-too-large",
    )
}

/// A trait method was called before `start` opened the container (or
/// after a constructor rejection) — the same operational shape the AVI
/// vertical mints (`crate::error::not_started`), mirrored here so the
/// mkv zone stays self-contained.
pub fn not_started() -> EngineError {
    mint(ErrorKind::Unsupported { format: None }, "demux.not-started")
}

/// A seek target that cannot be mapped: before the first cue point /
/// cluster, past the declared duration, or an absent target stamp.
pub fn seek_out_of_range() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.mkv.seek.out-of-range",
    )
}

/// The no-cues linear seek scan exceeded its parse budget (doc 03 §6
/// step 4's `SlowLinear` cap: `SeekFailed{TooSlow}` with the session
/// still at its old position and playable).
pub fn scan_too_slow(budget_bytes: u64, attempted_bytes: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::CpuTime,
            budget: budget_bytes,
            attempted: attempted_bytes,
        },
        "demux.mkv.seek.too-slow",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_carry_demux_stage_and_stable_codes() {
        let cases: Vec<(EngineError, &str)> = vec![
            (not_matroska(b"\x1a\x45\xdf"), "demux.mkv.not-matroska"),
            (unknown_doctype("ebml"), "demux.mkv.unknown-doctype"),
            (unsupported_ebml_limits(), "demux.mkv.ebml-limits"),
            (truncated(425_516, 12), "demux.mkv.truncated"),
            (bad_vint(1_024), "demux.mkv.bad-vint"),
            (bad_structure(2_048), "demux.mkv.bad-structure"),
            (timecode_overflow(9_999), "demux.mkv.timecode-overflow"),
            (no_tracks(), "demux.mkv.no-tracks"),
            (no_clusters(), "demux.mkv.no-clusters"),
            (
                cue_table_too_large(100, 200),
                "demux.mkv.cue-table-too-large",
            ),
            (not_started(), "demux.not-started"),
            (seek_out_of_range(), "demux.mkv.seek.out-of-range"),
            (scan_too_slow(100, 200), "demux.mkv.seek.too-slow"),
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
            not_matroska(b"EBML").kind,
            ErrorKind::Unsupported { .. }
        ));
        assert!(matches!(
            unknown_doctype("weird").kind,
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
            bad_vint(7).kind,
            ErrorKind::Corrupt {
                detail: CorruptDetail::InvalidStructure { offset: Some(7) }
            }
        );
        assert_eq!(
            timecode_overflow(9).kind,
            ErrorKind::Corrupt {
                detail: CorruptDetail::Timing
            }
        );
        assert_eq!(
            cue_table_too_large(10, 20).kind,
            ErrorKind::ResourceLimit {
                limit: LimitKind::Memory,
                budget: 10,
                attempted: 20
            }
        );
    }

    #[test]
    fn not_matroska_sniff_capped_at_sixteen_bytes_by_display() {
        let error = not_matroska(&[0xAB; 64]);
        let text = error.to_string();
        // The taxonomy caps sniffed bytes at 16 in Display; 64 input
        // bytes must not survive into the rendering.
        assert!(text.matches("ab").count() <= 16, "{text}");
    }
}
