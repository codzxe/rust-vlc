//! Typed error constructors for the Ogg demuxer (card 5.B1).
//!
//! The taxonomy is media-types' (never redefined here); this module
//! mints `EngineError` values with `ogg`-scoped stable codes at stage
//! `Demux`, mirroring the `wav::error` pattern (the crate-root
//! `error.rs` is the AVI card's file — scope discipline, R3
//! consolidates when the registry merges).
//!
//! # No secrets
//!
//! Constructors take byte counts and offsets only — never paths, URIs
//! or payload content.

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo, LimitKind};
use media_types::{EngineError, ErrorKind, PipelineStage};

const STAGE: PipelineStage = PipelineStage::Demux;

fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// The bytes are not an Ogg container (bad or absent capture).
pub fn not_ogg(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("ogg".to_string()),
                sniffed: sniffed.to_vec(),
            }),
        },
        "demux.ogg.not-ogg",
    )
}

/// A capture candidate that failed structural parse (short header,
/// wrong magic at the claimed position).
pub fn bad_capture(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.ogg.bad-capture",
    )
}

/// A page declares a nonzero stream-version byte (RFC 3533 §6.1
/// fixes it at 0 — a future version is not demuxable here).
pub fn bad_version(offset: u64, _version: u8) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.ogg.bad-version",
    )
}

/// A structurally invalid page where bytes cannot say more (zero
/// segment count).
pub fn bad_structure(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.ogg.bad-structure",
    )
}

/// A page (or its header) runs past the end of the stream — the
/// truncated-page error path. `expected` is the page's declared end;
/// `found` is the stream length.
pub fn truncated_page(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "demux.ogg.truncated-page",
    )
}

/// A header packet accumulated past the per-stream ceiling —
/// refusing to trust the declaration into an allocation (MP §12).
pub fn header_overrun(budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        "demux.ogg.header-overrun",
    )
}

/// A cross-page packet accumulated past the packet ceiling (a
/// lacing-255 run that never terminates).
pub fn packet_overrun(budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        "demux.ogg.packet-overrun",
    )
}

/// The stream walk found no beginning-of-stream page (no logical
/// stream to demux).
pub fn no_streams() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.ogg.no-streams",
    )
}

/// A trait method was called before `start` opened the container.
pub fn not_started() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.ogg.not-started",
    )
}

/// A seek the container cannot map (no clock-bearing stream, target
/// out of range, or the linear scan budget exhausted).
pub fn seek_out_of_range() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.ogg.seek.out-of-range",
    )
}

/// The bounded sync-recovery scan exhausted its byte budget without
/// finding a valid page (the walk refuses to read unbounded).
pub fn recovery_budget_exhausted(scanned: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget: super::MAX_RECOVERY_SCAN_BYTES,
            attempted: scanned,
        },
        "demux.ogg.recovery-budget",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_carry_demux_stage_and_ogg_codes() {
        let cases: Vec<(EngineError, &str)> = vec![
            (not_ogg(b"RIFFxxxx"), "demux.ogg.not-ogg"),
            (bad_capture(12), "demux.ogg.bad-capture"),
            (bad_version(4, 2), "demux.ogg.bad-version"),
            (bad_structure(8), "demux.ogg.bad-structure"),
            (truncated_page(900, 800), "demux.ogg.truncated-page"),
            (header_overrun(100, 200), "demux.ogg.header-overrun"),
            (packet_overrun(100, 200), "demux.ogg.packet-overrun"),
            (no_streams(), "demux.ogg.no-streams"),
            (not_started(), "demux.ogg.not-started"),
            (seek_out_of_range(), "demux.ogg.seek.out-of-range"),
            (
                recovery_budget_exhausted(4096),
                "demux.ogg.recovery-budget",
            ),
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
    fn not_ogg_sniff_capped_by_display() {
        let error = not_ogg(&[0xCD; 64]);
        let text = error.to_string();
        assert!(text.matches("cd").count() <= 16, "{text}");
    }
}
