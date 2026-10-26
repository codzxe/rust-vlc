//! Decode-stage error taxonomy for Opus (card 6.M-opus; the 5.B2
//! Vorbis pattern): typed, stage-tagged internal errors that map
//! one-to-one onto [`EngineError`] kinds at the contract boundary.
//!
//! The taxonomy mirrors where Opus damage actually occurs:
//!
//! - **Configuration** (`Unsupported`): a missing/short OpusHead, a
//!   bad `OpusHead` magic, a version the mapping does not define, an
//!   impossible channel geometry (zero channels, mapping family 0
//!   with >2 channels, a multistream head whose stream/coupled/
//!   mapping arithmetic does not close). Deterministic per stream —
//!   these are **pre-constructor validations** (the ruopus
//!   constructors panic on channels∉{1,2}/rate∉{8..48}k — the
//!   admission record's binding condition 2: validate from OpusHead
//!   BEFORE the engine is ever built, so the panic contract is
//!   unreachable through this wrapper).
//! - **Packet damage** (`Corrupt`): a truncated TOC/body, garbage,
//!   an over-long frame — the engine's typed `PacketError`s mapped
//!   one-to-one. **Isolation is packet-scoped (doc 03 §4.5(d))**: a
//!   damaged packet poisons at most itself (Opus has no cross-packet
//!   overlap); the decoder keeps decoding from the following packet
//!   (never a poisoned stream, never a panic — the admission
//!   record's 6-case hostile class measured zero panics).
//!
//! Nothing panics; every constructor is total; no `unwrap` on any
//! input path (workspace lints).

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// One Opus decode-stage failure (internal vocabulary).
///
/// Mapping to the public contract (doc 01 §3.4, doc 03 §4.5(d)):
///
/// | variant | public kind | stage |
/// |---|---|---|
/// | [`OpusError::Unsupported`] | [`ErrorKind::Unsupported`] | Decode |
/// | [`OpusError::Corrupt`] | [`ErrorKind::Corrupt`] | Decode |
/// | [`OpusError::BadState`] | [`ErrorKind::Decoder`] (`BadState`) | Decode |
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OpusError {
    /// A configuration or stream feature outside the supported subset
    /// (absent/malformed OpusHead, bad magic, impossible geometry).
    /// `code` names the exact rule; `detail` rides in `source`.
    Unsupported {
        /// Stable dotted context code naming the exact rule broken.
        code: &'static str,
        /// Human-readable reason (rides in `source`, not `Display`).
        detail: String,
    },
    /// Damaged media inside this packet: structural corruption,
    /// truncation, an engine decode rejection. Typed with the exact
    /// [`CorruptDetail`] so reporting is observable.
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

impl OpusError {
    /// Convert to the public contract error (stage `Decode`).
    pub(crate) fn into_engine_error(self) -> EngineError {
        let stage = PipelineStage::Decode;
        match self {
            OpusError::Unsupported { code, detail } => EngineError::new(
                stage,
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("opus".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail)),
            OpusError::Corrupt {
                code,
                detail,
                detail_text,
            } => EngineError::new(
                stage,
                ErrorKind::Corrupt { detail },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail_text)),
            OpusError::BadState => EngineError::new(
                stage,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                ErrorContext::new("opus.not_configured"),
            ),
        }
    }
}

impl From<OpusError> for EngineError {
    fn from(value: OpusError) -> EngineError {
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
pub(crate) fn unsupported(code: &'static str, detail: impl Into<String>) -> OpusError {
    OpusError::Unsupported {
        code,
        detail: detail.into(),
    }
}

/// Shorthand constructor: corrupt input with a structured class.
pub(crate) fn corrupt(
    code: &'static str,
    detail: CorruptDetail,
    detail_text: impl Into<String>,
) -> OpusError {
    OpusError::Corrupt {
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
        let cases: [(OpusError, &str, &str); 3] = [
            (
                unsupported("opus.head.magic", "no OpusHead magic"),
                "[decode] unsupported in opus (opus.head.magic)",
                "no OpusHead magic",
            ),
            (
                corrupt(
                    "opus.packet.decode",
                    CorruptDetail::Truncated {
                        expected: 4,
                        found: 1,
                    },
                    "truncated TOC",
                ),
                "[decode] truncated data: expected 4 bytes, found 1 (opus.packet.decode)",
                "truncated TOC",
            ),
            (
                OpusError::BadState,
                "[decode] decoder: invalid state transition (opus.not_configured)",
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
}
