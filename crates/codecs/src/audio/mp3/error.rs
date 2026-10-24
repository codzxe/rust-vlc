//! Decode-stage error taxonomy for MP3/MP2 (card 5.A3; the 5.A1 FLAC and
//! 2.B5 PCM pattern): typed, stage-tagged internal errors that map
//! one-to-one onto [`EngineError`] kinds at the contract boundary.
//!
//! The taxonomy mirrors where MPEG-audio damage actually occurs:
//!
//! - **Configuration** (`Unsupported`): a codec descriptor outside
//!   `mp1`/`mp2`/`mp3`, a free-format bitrate stream (doc 04 §3.2's
//!   known gap, honestly refused rather than guessed), a missing
//!   facts blob. Deterministic per stream.
//! - **Packet/stream damage** (`Corrupt`): a frame that fails
//!   header validation, truncation, an ID3 tag claiming more bytes
//!   than the file holds, a Xing frame-count inconsistent with the
//!   data. Per-frame isolation applies (doc 03 §4.5(d)): MP3 frames
//!   carry their full side information, so a damaged packet never
//!   poisons the next — the scanner resyncs at the next frame header.
//!
//! Nothing panics; every constructor is total; no `unwrap` on any
//! input path (workspace lints).

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// One MPEG-audio decode-stage failure (internal vocabulary).
///
/// Mapping to the public contract (doc 01 §3.4, doc 03 §4.5(d)):
///
/// | variant | public kind | stage |
/// |---|---|---|
/// | [`Mp3Error::Unsupported`] | [`ErrorKind::Unsupported`] | Decode |
/// | [`Mp3Error::Corrupt`] | [`ErrorKind::Corrupt`] | Decode |
/// | [`Mp3Error::BadState`] | [`ErrorKind::Decoder`] (`BadState`) | Decode |
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Mp3Error {
    /// A configuration or stream feature outside the supported subset.
    /// `code` names the exact rule; `detail` rides in `source`.
    Unsupported {
        /// Stable dotted context code naming the exact rule broken.
        code: &'static str,
        /// Human-readable reason (rides in `source`, not `Display`).
        detail: String,
    },
    /// Damaged media inside this packet/file region: structural
    /// corruption, truncation, or an inconsistent metadata claim.
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

impl Mp3Error {
    /// Convert to the public contract error (stage `Decode`, doc 01
    /// §3.4 — the 5.A1 `into_engine_error` pattern).
    pub(crate) fn into_engine_error(self) -> EngineError {
        let stage = PipelineStage::Decode;
        match self {
            Mp3Error::Unsupported { code, detail } => EngineError::new(
                stage,
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("mp3".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail)),
            Mp3Error::Corrupt {
                code,
                detail,
                detail_text,
            } => EngineError::new(
                stage,
                ErrorKind::Corrupt { detail },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail_text)),
            Mp3Error::BadState => EngineError::new(
                stage,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                ErrorContext::new("mp3.not_configured"),
            ),
        }
    }
}

impl From<Mp3Error> for EngineError {
    fn from(value: Mp3Error) -> EngineError {
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
pub(crate) fn unsupported(code: &'static str, detail: impl Into<String>) -> Mp3Error {
    Mp3Error::Unsupported {
        code,
        detail: detail.into(),
    }
}

/// Shorthand constructor: corrupt input with a structured class.
pub(crate) fn corrupt(
    code: &'static str,
    detail: CorruptDetail,
    detail_text: impl Into<String>,
) -> Mp3Error {
    Mp3Error::Corrupt {
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
        let cases: [(Mp3Error, &str, &str); 3] = [
            (
                unsupported("mp3.free_format", "free-format bitrate stream"),
                "[decode] unsupported in mp3 (mp3.free_format)",
                "free-format bitrate stream",
            ),
            (
                corrupt(
                    "mp3.header.bad_bitrate",
                    CorruptDetail::InvalidStructure { offset: Some(2) },
                    "bitrate index 15 is invalid",
                ),
                "[decode] corrupt structure at byte 2 (mp3.header.bad_bitrate)",
                "bitrate index 15 is invalid",
            ),
            (
                Mp3Error::BadState,
                "[decode] decoder: invalid state transition (mp3.not_configured)",
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
        let engine = Mp3Error::BadState.into_engine_error();
        assert!(matches!(
            engine.kind,
            ErrorKind::Decoder {
                cause: media_types::error::DecodeCause::BadState
            }
        ));
        assert!(!matches!(engine.kind, ErrorKind::Corrupt { .. }));
    }
}
