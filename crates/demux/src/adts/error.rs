//! Typed error constructors for the ADTS/ADIF demuxer (card 5.C2b).
//!
//! The taxonomy is media-types' (never redefined here); this module
//! mints `EngineError` values with `adts`-scoped stable codes at stage
//! `Demux`, mirroring the `ogg::error` pattern (the crate-root
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

/// The bytes are neither ADTS nor ADIF (no usable syncword or `ADIF`
/// magic within the open-scan window). `sniffed` carries the leading
/// bytes for the known-format hint (capped by the taxonomy's Display).
pub fn not_adts(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("adts".to_string()),
                sniffed: sniffed.to_vec(),
            }),
        },
        "demux.adts.not-adts",
    )
}

/// A frame header that parses as a syncword claim but declares
/// impossible structure (frame length shorter than its own header).
pub fn bad_header(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.adts.bad-header",
    )
}

/// The first frame's `sampling_frequency_index` is 13–15 (outside the
/// 13-entry MPEG-4 rate table) — the stream declares a rate this
/// demuxer cannot resolve. Mid-stream candidates with a bad index are
/// resync-skipped instead (damage, not a declaration).
pub fn bad_sampling_index() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.adts.bad-sampling-index",
    )
}

/// The first frame's `channel_configuration` is 0. Legal per ISO
/// 13818-7 (channels ride an in-band program_config_element), but the
/// configuration is decode-domain — this demuxer refuses rather than
/// report channels it cannot know (MP §8: never invent). Mid-stream
/// candidates are resync-skipped.
pub fn bad_channel_config(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.adts.bad-channel-config",
    )
}

/// The first frame carries protection (`protection_absent == 0`) and
/// the CRC-16 over its 56 header bits does not match the declared
/// `crc_check`. Scope note: the raw-data-block protection half of the
/// spec CRC needs syntax-element parsing (decode domain); this check
/// covers the deterministic header half only.
pub fn crc_mismatch(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.adts.crc-mismatch",
    )
}

/// A frame's declared length runs past the end of the stream (the
/// truncated-frame path; sticky, per the Ogg page-truncation
/// precedent). `expected` is the frame's declared end; `found` the
/// stream length.
pub fn truncated(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "demux.adts.truncated",
    )
}

/// The bounded resync scan exhausted its byte budget without finding
/// a usable syncword (the walk refuses to read unbounded).
pub fn recovery_budget_exhausted(scanned: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget: super::MAX_RECOVERY_SCAN_BYTES,
            attempted: scanned,
        },
        "demux.adts.recovery-budget",
    )
}

/// An ADIF header parsed, but the bitstream is VBR: ADIF carries no
/// per-frame delimiters, and VBR frame boundaries are only derivable
/// by decoding (syntax-element lengths) — the typed honest refusal,
/// not a guessed walk. (ffmpeg has no ADIF demuxer at all; GStreamer's
/// aacparse likewise refuses ADIF.)
pub fn adif_vbr() -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("adif".to_string()),
                sniffed: Vec::new(),
            }),
        },
        "demux.adts.adif-vbr",
    )
}

/// An ADIF header with more than one program (a multi-program
/// bitstream interleaves programs inside every raw_data_block; the
/// per-program split is decode-domain).
pub fn adif_multi_program() -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("adif".to_string()),
                sniffed: Vec::new(),
            }),
        },
        "demux.adts.adif-multi-program",
    )
}

/// The ADIF header (or its program_config_element) declares
/// structure this demuxer cannot honor: a sampling index outside the
/// rate table, zero total channels, a CBR block size that rounds to
/// zero, or an unparseable/overlong copyright string.
pub fn adif_bad_params() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.adts.adif-bad-params",
    )
}

/// An ADIF header with no raw data after it (nothing to demux).
pub fn adif_no_data() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated {
                expected: 1,
                found: 0,
            },
        },
        "demux.adts.adif-no-data",
    )
}

/// A trait method was called before `start` opened the container.
pub fn not_started() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.adts.not-started",
    )
}

/// A seek the container cannot map (target past the walked extent, or
/// the linear scan budget exhausted).
pub fn seek_out_of_range() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.adts.seek.out-of-range",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_carry_demux_stage_and_adts_codes() {
        let cases: Vec<(EngineError, &str)> = vec![
            (not_adts(b"\xff\xe0xxxx"), "demux.adts.not-adts"),
            (bad_header(64), "demux.adts.bad-header"),
            (bad_sampling_index(), "demux.adts.bad-sampling-index"),
            (bad_channel_config(0), "demux.adts.bad-channel-config"),
            (crc_mismatch(7), "demux.adts.crc-mismatch"),
            (truncated(900, 800), "demux.adts.truncated"),
            (
                recovery_budget_exhausted(4096),
                "demux.adts.recovery-budget",
            ),
            (adif_vbr(), "demux.adts.adif-vbr"),
            (adif_multi_program(), "demux.adts.adif-multi-program"),
            (adif_bad_params(), "demux.adts.adif-bad-params"),
            (adif_no_data(), "demux.adts.adif-no-data"),
            (not_started(), "demux.adts.not-started"),
            (seek_out_of_range(), "demux.adts.seek.out-of-range"),
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
    fn not_adts_sniff_capped_by_display() {
        let error = not_adts(&[0xEE; 64]);
        let text = error.to_string();
        assert!(text.matches("ee").count() <= 16, "{text}");
    }
}
