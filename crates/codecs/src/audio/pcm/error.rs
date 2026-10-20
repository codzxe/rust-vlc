//! Decode-stage taxonomy for PCM (card 2.B5 requirement, the 2.B4
//! MJPEG pattern): typed, stage-tagged internal errors that map
//! one-to-one onto [`EngineError`] kinds at the contract boundary.
//!
//! The taxonomy is intentionally small — smaller than MJPEG's —
//! because unpacked PCM has no entropy coding and no headers inside
//! the packet: the only fatal conditions are configuration-time
//! (an unsupported descriptor or inconsistent stream facts) and
//! contract misuse (decoding before `configure`). Byte-level damage
//! inside a packet is *always* concealable (the whole-sample-frame
//! prefix is honest output — see `decoder.rs`), so it never appears
//! here. Nothing panics; every constructor is total.

use media_types::error::{ErrorContext, FormatInfo};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// One PCM decode-stage failure (internal vocabulary).
///
/// Mapping to the public contract (doc 01 §3.4, doc 03 §4.5(d)):
///
/// | variant | public kind | stage |
/// |---|---|---|
/// | [`PcmError::Unsupported`] | [`ErrorKind::Unsupported`] | Decode |
/// | [`PcmError::BadState`] | [`ErrorKind::Decoder`] (`BadState`) | Decode |
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PcmError {
    /// A descriptor, stream-facts blob or parameter combination
    /// outside the supported PCM subset (unknown codec descriptor,
    /// malformed/mismatched WAVEFORMATEX, absent stream facts,
    /// conflicting facts, zero channels/rate, …). `code` names the
    /// exact rule; `detail` is the human-readable reason carried in
    /// the error's `source` chain.
    Unsupported {
        /// Stable dotted context code naming the exact rule broken.
        code: &'static str,
        /// Human-readable reason (rides in `source`, not `Display`).
        detail: String,
    },
    /// The decoder was asked to `decode`/`flush` before a successful
    /// `configure` (contract misuse — typed, never a panic).
    BadState,
}

impl PcmError {
    /// Convert to the public contract error (stage `Decode`, doc 01
    /// §3.4 — the 2.B4 `MjpegError::into_engine_error` pattern).
    pub(crate) fn into_engine_error(self) -> EngineError {
        let stage = PipelineStage::Decode;
        match self {
            PcmError::Unsupported { code, detail } => EngineError::new(
                stage,
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("pcm".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail)),
            PcmError::BadState => EngineError::new(
                stage,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                ErrorContext::new("pcm.not_configured"),
            ),
        }
    }
}

/// Chain-root carrier for the unsupported detail (keeps the
/// human-readable reason in the error's `source` without leaking it
/// into `Display`'s no-secrets surface — the 2.B4 `FeatureDetail`
/// pattern verbatim).
#[derive(Debug, Clone)]
struct Detail(String);

impl std::fmt::Display for Detail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Detail {}

/// Shorthand constructor: an unsupported configuration with a reason.
pub(crate) fn unsupported(code: &'static str, detail: impl Into<String>) -> PcmError {
    PcmError::Unsupported {
        code,
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taxonomy_maps_each_variant_to_its_public_kind() {
        let cases: [(PcmError, &str); 2] = [
            (
                unsupported(
                    "pcm.waveformat.tag",
                    "wFormatTag 2 is not WAVE_PCM or WAVE_IEEE_FLOAT",
                ),
                "[decode] unsupported in pcm (pcm.waveformat.tag)",
            ),
            (
                PcmError::BadState,
                "[decode] decoder: invalid state transition (pcm.not_configured)",
            ),
        ];
        for (internal, rendered) in cases {
            let engine = internal.into_engine_error();
            assert_eq!(engine.to_string(), rendered);
            assert_eq!(engine.stage, PipelineStage::Decode);
        }
    }

    #[test]
    fn unsupported_detail_rides_in_the_source_chain() {
        let engine = unsupported("pcm.codec_descriptor", "codec descriptor `f64` is unknown")
            .into_engine_error();
        assert!(
            matches!(engine.kind, ErrorKind::Unsupported { .. }),
            "{engine:?}"
        );
        let source = std::error::Error::source(&engine)
            .map(|s| s.to_string())
            .unwrap_or_default();
        assert_eq!(source, "codec descriptor `f64` is unknown");
    }

    #[test]
    fn bad_state_is_a_decoder_kind_not_corrupt() {
        // Misuse must never be misreported as media damage.
        let engine = PcmError::BadState.into_engine_error();
        assert!(
            matches!(
                engine.kind,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState
                }
            ),
            "{engine:?}"
        );
        assert!(
            !matches!(engine.kind, ErrorKind::Corrupt { .. }),
            "misuse is not corruption"
        );
    }
}
