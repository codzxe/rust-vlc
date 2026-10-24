//! Decode-stage error taxonomy for Vorbis (card 5.B2; the 5.A1 FLAC and
//! 5.A3 MP3 pattern): typed, stage-tagged internal errors that map
//! one-to-one onto [`EngineError`] kinds at the contract boundary.
//!
//! The taxonomy mirrors where Vorbis damage actually occurs:
//!
//! - **Configuration** (`Unsupported`): a missing/short header chain,
//!   a bad magic, an unsupported Vorbis version, or impossible
//!   identification geometry (zero rate, inverted block sizes).
//!   Deterministic per stream — the engine treats these as terminal,
//!   correctly.
//! - **Packet damage** (`Corrupt`): a header that fails its framing
//!   bit, truncation, or an engine decode rejection of an audio
//!   packet. **Isolation is overlap-scoped (doc 03 §4.5(d), honest
//!   variant):** Vorbis is MDCT overlap-add, so a damaged packet
//!   poisons at most itself *and* the next packet's overlap half — the
//!   decoder resets cleanly and keeps decoding from the following
//!   packet (never a poison-everything loop, never a crash; the
//!   boundary is one block, stated, not hidden).
//!
//! Nothing panics; every constructor is total; no `unwrap` on any
//! input path (workspace lints).

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// One Vorbis decode-stage failure (internal vocabulary).
///
/// Mapping to the public contract (doc 01 §3.4, doc 03 §4.5(d)):
///
/// | variant | public kind | stage |
/// |---|---|---|
/// | [`VorbisError::Unsupported`] | [`ErrorKind::Unsupported`] | Decode |
/// | [`VorbisError::Corrupt`] | [`ErrorKind::Corrupt`] | Decode |
/// | [`VorbisError::BadState`] | [`ErrorKind::Decoder`] (`BadState`) | Decode |
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VorbisError {
    /// A configuration or stream feature outside the supported subset
    /// (absent/malformed header chain, bad magic, impossible
    /// geometry, an engine `Unsupported` feature). `code` names the
    /// exact rule; `detail` rides in the error's `source` chain.
    Unsupported {
        /// Stable dotted context code naming the exact rule broken.
        code: &'static str,
        /// Human-readable reason (rides in `source`, not `Display`).
        detail: String,
    },
    /// Damaged media inside this packet/file region: structural
    /// corruption, truncation, a framing-bit failure or an engine
    /// decode rejection. Typed with the exact [`CorruptDetail`] so
    /// reporting is observable, not vague.
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

impl VorbisError {
    /// Convert to the public contract error (stage `Decode`, doc 01
    /// §3.4 — the 2.B4/2.B5/5.A1 `into_engine_error` pattern).
    pub(crate) fn into_engine_error(self) -> EngineError {
        let stage = PipelineStage::Decode;
        match self {
            VorbisError::Unsupported { code, detail } => EngineError::new(
                stage,
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("vorbis".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail)),
            VorbisError::Corrupt {
                code,
                detail,
                detail_text,
            } => EngineError::new(
                stage,
                ErrorKind::Corrupt { detail },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail_text)),
            VorbisError::BadState => EngineError::new(
                stage,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                ErrorContext::new("vorbis.not_configured"),
            ),
        }
    }
}

impl From<VorbisError> for EngineError {
    fn from(value: VorbisError) -> EngineError {
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
pub(crate) fn unsupported(code: &'static str, detail: impl Into<String>) -> VorbisError {
    VorbisError::Unsupported {
        code,
        detail: detail.into(),
    }
}

/// Shorthand constructor: corrupt input with a structured class.
pub(crate) fn corrupt(
    code: &'static str,
    detail: CorruptDetail,
    detail_text: impl Into<String>,
) -> VorbisError {
    VorbisError::Corrupt {
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
        let cases: [(VorbisError, &str, &str); 3] = [
            (
                unsupported("vorbis.ident.magic", "no \\x01vorbis magic"),
                "[decode] unsupported in vorbis (vorbis.ident.magic)",
                "no \\x01vorbis magic",
            ),
            (
                corrupt(
                    "vorbis.packet.framing",
                    CorruptDetail::ChecksumMismatch,
                    "comment framing bit not set",
                ),
                "[decode] checksum mismatch (vorbis.packet.framing)",
                "comment framing bit not set",
            ),
            (
                VorbisError::BadState,
                "[decode] decoder: invalid state transition (vorbis.not_configured)",
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
            "vorbis.setup.truncated",
            CorruptDetail::Truncated {
                expected: 30,
                found: 7,
            },
            "setup header shorter than its codebooks",
        )
        .into_engine_error();
        match engine.kind {
            ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated { expected, found },
            } => {
                assert_eq!((expected, found), (30, 7));
            }
            other => unreachable!("must be Corrupt/Truncated, got {other:?}"),
        }
    }

    #[test]
    fn bad_state_is_a_decoder_kind_not_corrupt() {
        let engine = VorbisError::BadState.into_engine_error();
        assert!(matches!(
            engine.kind,
            ErrorKind::Decoder {
                cause: media_types::error::DecodeCause::BadState
            }
        ));
        assert!(!matches!(engine.kind, ErrorKind::Corrupt { .. }));
    }
}
