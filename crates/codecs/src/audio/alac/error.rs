//! Decode-stage error taxonomy for ALAC (card 5.J3a; the 5.A1 FLAC and
//! 5.C2a AAC pattern): typed, stage-tagged internal errors that map
//! one-to-one onto [`EngineError`] kinds at the contract boundary.
//!
//! The taxonomy mirrors where ALAC damage actually occurs:
//!
//! - **Configuration** (`Unsupported`): a missing/malformed magic cookie
//!   (ALACSpecificConfig), an out-of-range frame length / bit depth /
//!   channel count, or an engine refusal at build time. Deterministic
//!   per stream.
//! - **Packet damage** (`Corrupt`): a frame that fails the engine's
//!   structural checks, or a truncated packet (the engine's
//!   end-of-bitstream error). Per-packet isolation applies (doc 03
//!   §4.5(d)): ALAC frames are independent — every frame carries its
//!   own element headers, rice histories and (for partial frames)
//!   sample counts — so a damaged packet never poisons the next.
//!
//! Nothing panics; every constructor is total; no `unwrap` on any input
//! path (workspace lints).

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// One ALAC decode-stage failure (internal vocabulary).
///
/// Mapping to the public contract (doc 01 §3.4, doc 03 §4.5(d)):
///
/// | variant | public kind | stage |
/// |---|---|---|
/// | [`AlacError::Unsupported`] | [`ErrorKind::Unsupported`] | Decode |
/// | [`AlacError::Corrupt`] | [`ErrorKind::Corrupt`] | Decode |
/// | [`AlacError::BadState`] | [`ErrorKind::Decoder`] (`BadState`) | Decode |
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AlacError {
    /// A configuration or stream feature outside the supported subset
    /// (absent/malformed cookie, impossible geometry, an engine
    /// `Unsupported` feature). `code` names the exact rule.
    Unsupported {
        /// Stable dotted context code naming the exact rule broken.
        code: &'static str,
        /// Human-readable reason (rides in `source`, not `Display`).
        detail: String,
    },
    /// Damaged media inside this packet: structural corruption or
    /// truncation (the engine's `DecodeError`/end-of-bitstream class).
    /// Typed with the exact [`CorruptDetail`] so reporting is
    /// observable, not vague.
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

impl AlacError {
    /// Convert to the public contract error (stage `Decode`, doc 01
    /// §3.4 — the 5.A1 `into_engine_error` pattern).
    pub(crate) fn into_engine_error(self) -> EngineError {
        let stage = PipelineStage::Decode;
        match self {
            AlacError::Unsupported { code, detail } => EngineError::new(
                stage,
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("alac".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail)),
            AlacError::Corrupt {
                code,
                detail,
                detail_text,
            } => EngineError::new(
                stage,
                ErrorKind::Corrupt { detail },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail_text)),
            AlacError::BadState => EngineError::new(
                stage,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                ErrorContext::new("alac.not_configured"),
            ),
        }
    }
}

impl From<AlacError> for EngineError {
    fn from(value: AlacError) -> EngineError {
        value.into_engine_error()
    }
}

/// Chain-root carrier for the human-readable reason (the 5.A1
/// `Detail` pattern verbatim: the reason rides in `source`, never in
/// `Display`'s no-secrets surface).
#[derive(Debug, Clone)]
struct Detail(String);

impl std::fmt::Display for Detail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Detail {}

/// Shorthand constructor: an unsupported configuration with a reason.
pub(crate) fn unsupported(code: &'static str, detail: impl Into<String>) -> AlacError {
    AlacError::Unsupported {
        code,
        detail: detail.into(),
    }
}

/// Shorthand constructor: corrupt input with a structured class.
pub(crate) fn corrupt(
    code: &'static str,
    detail: CorruptDetail,
    detail_text: impl Into<String>,
) -> AlacError {
    AlacError::Corrupt {
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
        let cases: [(AlacError, &str, &str); 3] = [
            (
                unsupported("alac.config.missing", "no extra_data"),
                "[decode] unsupported in alac (alac.config.missing)",
                "no extra_data",
            ),
            (
                corrupt(
                    "alac.frame.truncated",
                    CorruptDetail::Truncated {
                        expected: 16,
                        found: 9,
                    },
                    "frame ended mid-bitread",
                ),
                "[decode] truncated data: expected 16 bytes, found 9 (alac.frame.truncated)",
                "frame ended mid-bitread",
            ),
            (
                AlacError::BadState,
                "[decode] decoder: invalid state transition (alac.not_configured)",
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
            "alac.config.length",
            CorruptDetail::Truncated {
                expected: 24,
                found: 12,
            },
            "cookie shorter than the 24-byte ALACSpecificConfig",
        )
        .into_engine_error();
        match engine.kind {
            ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated { expected, found },
            } => {
                assert_eq!((expected, found), (24, 12));
            }
            other => unreachable!("must be Corrupt/Truncated, got {other:?}"),
        }
    }

    #[test]
    fn bad_state_is_a_decoder_kind_not_corrupt() {
        let engine = AlacError::BadState.into_engine_error();
        assert!(matches!(
            engine.kind,
            ErrorKind::Decoder {
                cause: media_types::error::DecodeCause::BadState
            }
        ));
        assert!(!matches!(engine.kind, ErrorKind::Corrupt { .. }));
    }
}
