//! Decode-stage error taxonomy for FLAC (card 5.A1; the 2.B5 PCM and
//! 2.B4 MJPEG pattern): typed, stage-tagged internal errors that map
//! one-to-one onto [`EngineError`] kinds at the contract boundary.
//!
//! The taxonomy mirrors where FLAC damage actually occurs:
//!
//! - **Configuration** (`Unsupported`): a missing/invalid STREAMINFO
//!   blob, an out-of-vocabulary stream geometry (bit depth, channel
//!   count), an unparseable metadata chain. Deterministic per stream —
//!   the engine treats these as terminal, correctly.
//! - **Packet damage** (`Corrupt`): a frame that fails header/CRC
//!   validation, truncation, or a claxon decode error. Per-frame
//!   isolation applies (doc 03 §4.5(d)): FLAC frames are independent
//!   (every frame header + subframes + CRC is self-contained), so a
//!   damaged packet never poisons the next.
//!
//! Nothing panics; every constructor is total; no `unwrap` on any
//! input path (workspace lints).

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// One FLAC decode-stage failure (internal vocabulary).
///
/// Mapping to the public contract (doc 01 §3.4, doc 03 §4.5(d)):
///
/// | variant | public kind | stage |
/// |---|---|---|
/// | [`FlacError::Unsupported`] | [`ErrorKind::Unsupported`] | Decode |
/// | [`FlacError::Corrupt`] | [`ErrorKind::Corrupt`] | Decode |
/// | [`FlacError::BadState`] | [`ErrorKind::Decoder`] (`BadState`) | Decode |
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FlacError {
    /// A configuration or stream feature outside the supported subset
    /// (absent/malformed STREAMINFO, impossible geometry, a claxon
    /// `Unsupported` feature). `code` names the exact rule; `detail`
    /// rides in the error's `source` chain.
    Unsupported {
        /// Stable dotted context code naming the exact rule broken.
        code: &'static str,
        /// Human-readable reason (rides in `source`, not `Display`).
        detail: String,
    },
    /// Damaged media inside this packet/file region: structural
    /// corruption, truncation, or a CRC mismatch (the claxon
    /// `FormatError` class). Typed with the exact
    /// [`CorruptDetail`] so reporting is observable, not vague.
    Corrupt {
        /// Stable dotted context code naming the failure site.
        code: &'static str,
        /// The structured corruption class.
        detail: CorruptDetail,
        /// Human-readable reason (rides in `source`).
        detail_text: String,
    },
    /// The decoder was asked to `decode`/`flush` before a successful
    /// `configure` (contract misuse — typed, never a panic).
    BadState,
}

impl FlacError {
    /// Convert to the public contract error (stage `Decode`, doc 01
    /// §3.4 — the 2.B4/2.B5 `into_engine_error` pattern).
    pub(crate) fn into_engine_error(self) -> EngineError {
        let stage = PipelineStage::Decode;
        match self {
            FlacError::Unsupported { code, detail } => EngineError::new(
                stage,
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("flac".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail)),
            FlacError::Corrupt {
                code,
                detail,
                detail_text,
            } => EngineError::new(
                stage,
                ErrorKind::Corrupt { detail },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail_text)),
            FlacError::BadState => EngineError::new(
                stage,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                ErrorContext::new("flac.not_configured"),
            ),
        }
    }
}

impl From<FlacError> for EngineError {
    fn from(value: FlacError) -> EngineError {
        value.into_engine_error()
    }
}

/// Chain-root carrier for the human-readable reason (the 2.B4
/// `FeatureDetail` pattern verbatim: the reason rides in `source`,
/// never in `Display`'s no-secrets surface).
#[derive(Debug, Clone)]
struct Detail(String);

impl std::fmt::Display for Detail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Detail {}

/// Shorthand constructor: an unsupported configuration with a reason.
pub(crate) fn unsupported(code: &'static str, detail: impl Into<String>) -> FlacError {
    FlacError::Unsupported {
        code,
        detail: detail.into(),
    }
}

/// Shorthand constructor: corrupt input with a structured class.
pub(crate) fn corrupt(
    code: &'static str,
    detail: CorruptDetail,
    detail_text: impl Into<String>,
) -> FlacError {
    FlacError::Corrupt {
        code,
        detail,
        detail_text: detail_text.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taxonomy_maps_each_variant_to_its_public_kind() {
        let cases: [(FlacError, &str, &str); 3] = [
            (
                unsupported("flac.streaminfo.missing", "no extra_data"),
                "[decode] unsupported in flac (flac.streaminfo.missing)",
                "no extra_data",
            ),
            (
                corrupt(
                    "flac.frame.crc16",
                    CorruptDetail::ChecksumMismatch,
                    "frame CRC-16 mismatch",
                ),
                "[decode] checksum mismatch (flac.frame.crc16)",
                "frame CRC-16 mismatch",
            ),
            (
                FlacError::BadState,
                "[decode] decoder: invalid state transition (flac.not_configured)",
                "",
            ),
        ];
        for (internal, rendered, source_text) in cases {
            let engine = internal.into_engine_error();
            assert_eq!(engine.to_string(), rendered);
            assert_eq!(engine.stage, PipelineStage::Decode);
            let source = std::error::Error::source(&engine)
                .map(|s| s.to_string())
                .unwrap_or_default();
            assert_eq!(source, source_text);
        }
    }

    #[test]
    fn corrupt_variant_carries_the_structured_class() {
        let engine = corrupt(
            "flac.metadata.truncated",
            CorruptDetail::Truncated {
                expected: 34,
                found: 10,
            },
            "streaminfo shorter than 34 bytes",
        )
        .into_engine_error();
        match engine.kind {
            ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated { expected, found },
            } => {
                assert_eq!((expected, found), (34, 10));
            }
            other => unreachable!("must be Corrupt/Truncated, got {other:?}"),
        }
    }

    #[test]
    fn bad_state_is_a_decoder_kind_not_corrupt() {
        let engine = FlacError::BadState.into_engine_error();
        assert!(matches!(
            engine.kind,
            ErrorKind::Decoder {
                cause: media_types::error::DecodeCause::BadState
            }
        ));
        assert!(!matches!(engine.kind, ErrorKind::Corrupt { .. }));
    }
}
