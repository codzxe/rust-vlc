//! Typed MPEG-PS/VOB error constructors (card 5.I2).
//!
//! Same shape as [`crate::error`]: the *taxonomy* is media-types'
//! (never redefined); constructors mint stage-`Demux`
//! [`EngineError`](media_types::EngineError) values with stable dotted
//! codes. No payload bytes, no paths — offsets and counts only.

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo, LimitKind};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// The stage tag every PS-minted error carries.
const STAGE: PipelineStage = PipelineStage::Demux;

/// Mint a stage-`Demux` error with a stable dotted code.
fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// The bytes are not an MPEG-PS/VOB container (no pack start code at
/// the head). `sniffed` carries the leading bytes for the
/// known-format hint (capped by the taxonomy's Display).
pub fn not_ps(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("mpeg-ps".to_string()),
                sniffed: sniffed.to_vec(),
            }),
        },
        "demux.ps.not-ps",
    )
}

/// A recognized magic prefix but the stream ends inside the pack
/// header (the honest flavor is truncation, not "not a PS" — the
/// AVI/WAV D-9 precedent).
pub fn truncated(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "demux.ps.truncated",
    )
}

/// A pack header is structurally invalid: neither the MPEG-2 `'01'`
/// nor the MPEG-1 `'0010'` SCR prefix convention, a stuffing length
/// that runs past the stream, or SCR field bits outside their
/// declared widths (the "SCR arithmetic" class — every extraction is
/// width-checked before it is trusted).
pub fn bad_pack(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.ps.bad-pack",
    )
}

/// A PES header is structurally invalid: a missing `'10'` prefix on
/// MPEG-2 packets, a header length that overruns the declared packet
/// length, an unbounded stuffing run, or a stamp prefix outside the
/// `0010/0011/0001/0000` vocabulary.
pub fn bad_pes_header(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.ps.bad-pes-header",
    )
}

/// The walk hit damaged bytes whose re-synchronization to the next
/// start code could not be completed inside the recovery budget.
pub fn resync_failed(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.ps.resync-failed",
    )
}

/// A declared extent exceeded the payload ceiling (MP §12).
pub fn payload_too_large(budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        "demux.ps.payload-too-large",
    )
}

/// A trait method was called before `start` opened the container.
pub fn not_started() -> EngineError {
    mint(ErrorKind::Unsupported { format: None }, "demux.not-started")
}

/// A seek target outside the SCR-observed extent, or without a
/// usable timestamp.
pub fn seek_out_of_range() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.ps.seek.out-of-range",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_carry_demux_stage_and_stable_codes() {
        let cases: Vec<(EngineError, &str)> = vec![
            (not_ps(b"RIFFWAVE"), "demux.ps.not-ps"),
            (truncated(14, 8), "demux.ps.truncated"),
            (bad_pack(1_024), "demux.ps.bad-pack"),
            (bad_pes_header(2_048), "demux.ps.bad-pes-header"),
            (resync_failed(4_096), "demux.ps.resync-failed"),
            (
                payload_too_large(65_536, 70_000),
                "demux.ps.payload-too-large",
            ),
            (not_started(), "demux.not-started"),
            (seek_out_of_range(), "demux.ps.seek.out-of-range"),
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
    fn not_ps_sniff_is_capped_by_display() {
        let error = not_ps(&[0xCD; 64]);
        let text = error.to_string();
        assert!(text.matches("cd").count() <= 16, "{text}");
    }
}
