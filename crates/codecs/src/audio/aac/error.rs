//! Decode-stage error taxonomy for AAC (card 5.C2a; the 5.A3 MP3 and
//! 5.A1 FLAC pattern): typed, stage-tagged internal errors that map
//! one-to-one onto [`EngineError`] kinds at the contract boundary.
//!
//! The taxonomy mirrors where AAC damage actually occurs:
//!
//! - **Configuration** (`Unsupported`): a codec descriptor outside
//!   `aac`; a profile the LC-only engine does not implement (Main,
//!   SSR, LTP, reserved — ADTS profile field or the ASC's audio
//!   object type); **HE-AAC (SBR/PS), honestly absent** — an ASC that
//!   signals object type 5/29 is refused typedly, never silently
//!   downgraded (doc 04 §3.2's honesty note); channel configuration 0
//!   (PCE-in-band, engine TODO upstream); more than 2 channels.
//! - **Packet/stream damage** (`Corrupt`): a bad ADTS syncword, a
//!   reserved/non-zero layer field, a reserved sampling-frequency
//!   index, a frame shorter than its own header claims, a frame whose
//!   parameters disagree with the configured stream. Per-frame
//!   isolation holds (doc 03 §4.5(d)): ADTS frames carry their full
//!   side information, so a damaged packet never poisons the next —
//!   the walker resyncs at the next syncword.
//!
//! Nothing panics; every constructor is total; no `unwrap` on any
//! input path (workspace lints).

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// One AAC decode-stage failure (internal vocabulary).
///
/// Mapping to the public contract (doc 01 §3.4, doc 03 §4.5(d)):
///
/// | variant | public kind | stage |
/// |---|---|---|
/// | [`AacError::Unsupported`] | [`ErrorKind::Unsupported`] | Decode |
/// | [`AacError::Corrupt`] | [`ErrorKind::Corrupt`] | Decode |
/// | [`AacError::BadState`] | [`ErrorKind::Decoder`] (`BadState`) | Decode |
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AacError {
    /// A configuration or stream feature outside the supported subset.
    /// `code` names the exact rule; `detail` rides in `source`.
    Unsupported {
        /// Stable dotted context code naming the exact rule broken.
        code: &'static str,
        /// Human-readable reason (rides in `source`, not `Display`).
        detail: String,
    },
    /// Damaged media inside this packet/file region.
    Corrupt {
        /// Stable dotted context code naming the failure site.
        code: &'static str,
        /// The structured corruption class.
        detail: CorruptDetail,
        /// Human-readable reason (rides in `source`).
        detail_text: String,
    },
    /// The decoder was used before a successful `configure`.
    BadState,
}

impl AacError {
    /// Convert to the public contract error (stage `Decode`; the
    /// 5.A1 `into_engine_error` pattern).
    pub(crate) fn into_engine_error(self) -> EngineError {
        let stage = PipelineStage::Decode;
        match self {
            AacError::Unsupported { code, detail } => EngineError::new(
                stage,
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("aac".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail)),
            AacError::Corrupt {
                code,
                detail,
                detail_text,
            } => EngineError::new(
                stage,
                ErrorKind::Corrupt { detail },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail_text)),
            AacError::BadState => EngineError::new(
                stage,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                ErrorContext::new("aac.not_configured"),
            ),
        }
    }
}

impl From<AacError> for EngineError {
    fn from(value: AacError) -> EngineError {
        value.into_engine_error()
    }
}

/// Chain-root carrier for the human-readable reason (the 2.B4
/// `FeatureDetail` pattern verbatim).
#[derive(Debug, Clone)]
struct Detail(String);

impl std::fmt::Display for Detail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Detail {}

/// Shorthand constructor: an unsupported configuration with a reason.
pub(crate) fn unsupported(code: &'static str, detail: impl Into<String>) -> AacError {
    AacError::Unsupported {
        code,
        detail: detail.into(),
    }
}

/// Shorthand constructor: corrupt input with a structured class.
pub(crate) fn corrupt(
    code: &'static str,
    detail: CorruptDetail,
    detail_text: impl Into<String>,
) -> AacError {
    AacError::Corrupt {
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
        let cases: [(AacError, &str, &str); 3] = [
            (
                unsupported("aac.he_sbr", "HE-AAC (SBR/PS) is honestly absent"),
                "[decode] unsupported in aac (aac.he_sbr)",
                "HE-AAC (SBR/PS) is honestly absent",
            ),
            (
                corrupt(
                    "aac.adts.sync",
                    CorruptDetail::InvalidStructure { offset: Some(1) },
                    "syncword is not 0xFFF",
                ),
                "[decode] corrupt structure at byte 1 (aac.adts.sync)",
                "syncword is not 0xFFF",
            ),
            (
                AacError::BadState,
                "[decode] decoder: invalid state transition (aac.not_configured)",
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
    fn bad_state_is_a_decoder_kind_not_corrupt() {
        let engine = AacError::BadState.into_engine_error();
        assert!(matches!(
            engine.kind,
            ErrorKind::Decoder {
                cause: media_types::error::DecodeCause::BadState
            }
        ));
        assert!(!matches!(engine.kind, ErrorKind::Corrupt { .. }));
    }
}
