//! Decode-stage error taxonomy for VP9 (card 6.M-vp9; the 6.M-opus /
//! 5.B2 pattern): typed, stage-tagged internal errors that map
//! one-to-one onto [`EngineError`] kinds at the contract boundary.
//!
//! The taxonomy mirrors where VP9 damage actually occurs:
//!
//! - **Configuration / output boundary** (`Unsupported`): a codec
//!   descriptor that is not `vp9`, and — the v0 output vocabulary's
//!   honest edge (the av1 precedent) — a decoded frame whose bit
//!   depth is not 8 or whose chroma subsampling is not 4:2:0:
//!   [`media_types::PixelFormat`] carries no 10/12-bit planar
//!   variant yet, so those frames are a typed refusal at the emit
//!   seam, never a mis-decode (the engine decoded them; the wrapper
//!   refuses to reinterpret the bytes into a format it cannot name).
//! - **Packet damage** (`Corrupt`): a frame the engine rejected as
//!   `InvalidData` (bad frame marker, bit reader past end,
//!   out-of-range dimensions) or a `DecodedFrame` whose plane
//!   geometry does not close against its own declared
//!   width/height/strides. Isolation is frame-scoped (doc 03
//!   §4.5(d), the av1 model): a damaged frame poisons at most
//!   itself — it was already consumed from the engine's queue, the
//!   reference state is whatever the engine kept, and the next
//!   packet decodes against it (the admission record's
//!   garbage-then-real lane measured exactly this recovery).
//! - **BadState**: `decode`/`flush` before a successful `configure`.
//!
//! Nothing panics; every constructor is total; no `unwrap` on any
//! input path (workspace lints).

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// One VP9 decode-stage failure (internal vocabulary).
///
/// Mapping to the public contract (doc 01 §3.4, doc 03 §4.5(d)):
///
/// | variant | public kind | stage |
/// |---|---|---|
/// | [`Vp9Error::Unsupported`] | [`ErrorKind::Unsupported`] | Decode |
/// | [`Vp9Error::Corrupt`] | [`ErrorKind::Corrupt`] | Decode |
/// | [`Vp9Error::BadState`] | [`ErrorKind::Decoder`] (`BadState`) | Decode |
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Vp9Error {
    /// A configuration or output boundary outside the supported
    /// subset (wrong descriptor, non-8-bit or non-4:2:0 output).
    /// `code` names the exact rule; `detail` rides in `source`.
    Unsupported {
        /// Stable dotted context code naming the exact rule broken.
        code: &'static str,
        /// Human-readable reason (rides in `source`, not `Display`).
        detail: String,
    },
    /// Damaged media inside this packet: structural corruption,
    /// truncation, an engine decode rejection, plane geometry that
    /// does not close. Typed with the exact [`CorruptDetail`] so
    /// reporting is observable.
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

impl Vp9Error {
    /// Convert to the public contract error (stage `Decode`).
    pub(crate) fn into_engine_error(self) -> EngineError {
        let stage = PipelineStage::Decode;
        match self {
            Vp9Error::Unsupported { code, detail } => EngineError::new(
                stage,
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("vp9".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail)),
            Vp9Error::Corrupt {
                code,
                detail,
                detail_text,
            } => EngineError::new(
                stage,
                ErrorKind::Corrupt { detail },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail_text)),
            Vp9Error::BadState => EngineError::new(
                stage,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                ErrorContext::new("vp9.not_configured"),
            ),
        }
    }
}

impl From<Vp9Error> for EngineError {
    fn from(value: Vp9Error) -> EngineError {
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
pub(crate) fn unsupported(code: &'static str, detail: impl Into<String>) -> Vp9Error {
    Vp9Error::Unsupported {
        code,
        detail: detail.into(),
    }
}

/// Shorthand constructor: corrupt input with a structured class.
pub(crate) fn corrupt(
    code: &'static str,
    detail: CorruptDetail,
    detail_text: impl Into<String>,
) -> Vp9Error {
    Vp9Error::Corrupt {
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
        let cases: [(Vp9Error, &str, &str); 3] = [
            (
                unsupported(
                    "vp9.codec_descriptor",
                    "codec descriptor `h264` is not `vp9`",
                ),
                "[decode] unsupported in vp9 (vp9.codec_descriptor)",
                "codec descriptor `h264` is not `vp9`",
            ),
            (
                corrupt(
                    "vp9.packet.decode",
                    CorruptDetail::InvalidStructure { offset: None },
                    "engine rejected the frame",
                ),
                "[decode] corrupt structure (vp9.packet.decode)",
                "engine rejected the frame",
            ),
            (
                Vp9Error::BadState,
                "[decode] decoder: invalid state transition (vp9.not_configured)",
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
