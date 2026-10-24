//! Decode-stage error taxonomy for MP2 (card 5.J3, the 5.A3 MP3
//! pattern verbatim): typed, stage-tagged internal errors that map
//! one-to-one onto [`EngineError`] kinds at the contract boundary.
//!
//! The taxonomy mirrors where Layer II damage actually occurs:
//!
//! - **Configuration** (`Unsupported`): a codec descriptor other than
//!   `mp2`, an unexpected `extra_data` facts blob (none is defined for
//!   Layer II — there is no Xing/LAME construction to carry gapless
//!   facts), an engine refusal at init. Deterministic per stream.
//! - **Packet/stream damage** (`Corrupt`): a frame header that is not
//!   Layer II, a truncated frame, a stream whose walked frames
//!   disagree on version/bitrate/rate/channels (the CBR consistency
//!   gate the seek arithmetic relies on). Per-frame isolation (doc 03
//!   §4.5(d)): Layer II frames carry their full allocation/scfsi
//!   side data, so a damaged packet never poisons the next — the
//!   walker resyncs at the next frame header.
//!
//! Nothing panics; every constructor is total; no `unwrap` on any
//! input path (workspace lints).

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// One MP2 decode-stage failure (internal vocabulary).
///
/// Mapping to the public contract (doc 01 §3.4, doc 03 §4.5(d)):
///
/// | variant | public kind | stage |
/// |---|---|---|
/// | [`Mp2Error::Unsupported`] | [`ErrorKind::Unsupported`] | Decode |
/// | [`Mp2Error::Corrupt`] | [`ErrorKind::Corrupt`] | Decode |
/// | [`Mp2Error::BadState`] | [`ErrorKind::Decoder`] (`BadState`) | Decode |
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Mp2Error {
    /// A configuration or stream feature outside the supported subset.
    /// `code` names the exact rule; `detail` rides in `source`.
    Unsupported {
        /// Stable dotted context code naming the exact rule broken.
        code: &'static str,
        /// Human-readable reason (rides in `source`, not `Display`).
        detail: String,
    },
    /// Damaged media inside this packet/file region: structural
    /// corruption, truncation, or an inconsistent stream geometry.
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

impl Mp2Error {
    /// Convert to the public contract error (stage `Decode`, doc 01
    /// §3.4 — the 5.A1/5.A3 `into_engine_error` pattern).
    pub(crate) fn into_engine_error(self) -> EngineError {
        let stage = PipelineStage::Decode;
        match self {
            Mp2Error::Unsupported { code, detail } => EngineError::new(
                stage,
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("mp2".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail)),
            Mp2Error::Corrupt {
                code,
                detail,
                detail_text,
            } => EngineError::new(
                stage,
                ErrorKind::Corrupt { detail },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail_text)),
            Mp2Error::BadState => EngineError::new(
                stage,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                ErrorContext::new("mp2.not_configured"),
            ),
        }
    }
}

impl From<Mp2Error> for EngineError {
    fn from(value: Mp2Error) -> EngineError {
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
pub(crate) fn unsupported(code: &'static str, detail: impl Into<String>) -> Mp2Error {
    Mp2Error::Unsupported {
        code,
        detail: detail.into(),
    }
}

/// Shorthand constructor: corrupt input with a structured class.
pub(crate) fn corrupt(
    code: &'static str,
    detail: CorruptDetail,
    detail_text: impl Into<String>,
) -> Mp2Error {
    Mp2Error::Corrupt {
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
        let cases: [(Mp2Error, &str, &str); 3] = [
            (
                unsupported("mp2.codec_descriptor", "codec descriptor `mp3` is not mp2"),
                "[decode] unsupported in mp2 (mp2.codec_descriptor)",
                "codec descriptor `mp3` is not mp2",
            ),
            (
                corrupt(
                    "mp2.header.not_layer_ii",
                    CorruptDetail::InvalidStructure { offset: Some(1) },
                    "layer bits 01 (Layer III) in an mp2 packet",
                ),
                "[decode] corrupt structure at byte 1 (mp2.header.not_layer_ii)",
                "layer bits 01 (Layer III) in an mp2 packet",
            ),
            (
                Mp2Error::BadState,
                "[decode] decoder: invalid state transition (mp2.not_configured)",
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
        let engine = Mp2Error::BadState.into_engine_error();
        assert!(matches!(
            engine.kind,
            ErrorKind::Decoder {
                cause: media_types::error::DecodeCause::BadState
            }
        ));
        assert!(!matches!(engine.kind, ErrorKind::Corrupt { .. }));
    }
}
