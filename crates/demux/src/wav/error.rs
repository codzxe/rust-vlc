//! Typed error constructors for the WAV demuxer (card 5.A2).
//!
//! The error *taxonomy* is media-types' seven kinds (doc 01 §3.4) —
//! never redefined here. This module mints
//! [`EngineError`](media_types::EngineError) values with `wav`-scoped
//! stable codes at stage [`PipelineStage::Demux`], mirroring the
//! crate-root `avi::error` pattern; it lives under `wav/` because the
//! crate-root `error.rs` is the AVI card's file (scope discipline —
//! R3 can consolidate the constructors when the registry merges).
//!
//! # No secrets
//!
//! Constructors take byte counts and offsets only — never paths, URIs
//! or payload content. Sniffed magic bytes are capped by the
//! taxonomy's own `Display`.

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo, LimitKind};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// The stage tag every demux-minted error carries (doc 03 §4.3(d)).
const STAGE: PipelineStage = PipelineStage::Demux;

fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// The bytes are not a RIFF/WAVE (or RF64/BW64) container. `sniffed`
/// carries the leading bytes for the known-format hint (capped by the
/// taxonomy's Display, never a secret).
pub fn not_wav(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("wav".to_string()),
                sniffed: sniffed.to_vec(),
            }),
        },
        "demux.wav.not-wav",
    )
}

/// The container declares more structure than the stream carries
/// (truncated RIFF head, truncated chunk header, truncated `fmt`,
/// `data` extent cut by the end of the file). `expected` is the
/// absolute end the structure needed; `found` is the stream length
/// actually available.
pub fn truncated(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "demux.wav.truncated",
    )
}

/// A chunk or list is structurally invalid where bytes alone cannot
/// say more (fmt size below the 16-byte PCMWAVEFORMAT floor, ds64 in
/// a plain RIFF, a list form cut by the end of the file).
pub fn bad_structure(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.wav.bad-structure",
    )
}

/// The `fmt ` chunk parses but declares parameters no PCM stream can
/// honor (zero channels, zero rate, `nBlockAlign` ≠ channels ×
/// sample width — the exact consistency the PCM decoder re-checks on
/// the facts blob; accepting here would mint a stream the decode
/// path always rejects).
pub fn bad_format_params(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.wav.bad-fmt",
    )
}

/// The `fmt ` chunk names a format outside the PCM vocabulary this
/// demuxer exposes (unknown `wFormatTag`, `WAVE_FORMAT_EXTENSIBLE`
/// without the ≥ 40-byte extensible body, a SubFormat GUID outside
/// {PCM, IEEE float}, a bit depth with no registry descriptor such
/// as 20-bit or 24-valid-in-32). The container is a WAV; the stream
/// is not one this demuxer can emit.
pub fn unsupported_format(_offset: u64) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("wav".to_string()),
                sniffed: Vec::new(),
            }),
        },
        "demux.wav.unsupported-format",
    )
}

/// The RIFF walk found no `fmt ` chunk anywhere a well-formed-sized
/// extent could reach — a WAVE file without a format is not
/// demuxable.
pub fn no_fmt() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.wav.no-fmt",
    )
}

/// Parsed the header but the `data` chunk is absent, or present with
/// a zero payload — there is nothing to demux (the WAV twin of
/// `avi::no_movi`).
pub fn no_data() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "demux.wav.no-data",
    )
}

/// A trait method was called before `start` opened the container (or
/// after a constructor rejection): operational shape, not container
/// data.
pub fn not_started() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.wav.not-started",
    )
}

/// A seek target that cannot be mapped: before the stream start,
/// past the declared end, or an absent target stamp.
pub fn seek_out_of_range() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.wav.seek.out-of-range",
    )
}

/// A payload ask exceeds the demuxer's packet ceiling. Vocabulary
/// without a minting site today: the packetizer never asks above
/// [`crate::wav::MAX_PACKET_BYTES`], and larger chunk payloads are
/// positionally packetized instead of read whole (the
/// `avi::payload_too_large` precedent — the typed flavor is kept for
/// callers that choose to fail fast).
pub fn payload_too_large(budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        "demux.wav.payload-too-large",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_carry_demux_stage_and_wav_codes() {
        let cases: Vec<(EngineError, &str)> = vec![
            (not_wav(b"RIFX\x00\x00\x00\x00"), "demux.wav.not-wav"),
            (truncated(425_516, 12), "demux.wav.truncated"),
            (bad_structure(1_024), "demux.wav.bad-structure"),
            (bad_format_params(200), "demux.wav.bad-fmt"),
            (unsupported_format(44), "demux.wav.unsupported-format"),
            (no_fmt(), "demux.wav.no-fmt"),
            (no_data(), "demux.wav.no-data"),
            (not_started(), "demux.wav.not-started"),
            (seek_out_of_range(), "demux.wav.seek.out-of-range"),
            (payload_too_large(100, 200), "demux.wav.payload-too-large"),
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
    fn not_wav_sniff_capped_at_sixteen_bytes_by_display() {
        let error = not_wav(&[0xAB; 64]);
        let text = error.to_string();
        assert!(text.matches("ab").count() <= 16, "{text}");
    }
}
