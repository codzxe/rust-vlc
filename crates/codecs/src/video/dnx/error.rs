//! Decode-stage error taxonomy for DNxHD/DNxHR (card 6.M-dnx; the
//! 6.M-opus Opus pattern): typed, stage-tagged internal errors that
//! map one-to-one onto [`EngineError`] kinds at the contract
//! boundary.
//!
//! The taxonomy mirrors where VC-3 damage actually occurs:
//!
//! - **Configuration** (`Unsupported`): a foreign codec descriptor,
//!   an output geometry the media-types pixel vocabulary cannot
//!   express losslessly (10/12-bit samples, 4:4:4 / RGB planes,
//!   alpha, non-square pixel aspect, odd 4:2:2 width, the BT.2020
//!   constant-luminance matrix). Deterministic per stream.
//! - **Packet damage** (`Corrupt`): a CBR packet shorter than the
//!   header-declared coding-unit size (the admission record's
//!   concealment-class gate — the engine returns `Ok` on some
//!   truncated frames, so the wrapper refuses first), the engine's
//!   own truncation/invalid/scan-line errors, and plane-geometry
//!   mismatches. **Isolation is frame-scoped (doc 03 §4.5(d))**:
//!   VC-3 is intra-only and the engine holds no cross-frame state,
//!   so a damaged packet poisons at most itself (the MJPEG
//!   statelessness guarantee; the admission record's hostile
//!   battery measured zero panics on truncated/garbage/zeroed
//!   inputs).
//!
//! Nothing panics; every constructor is total; no `unwrap` on any
//! input path (workspace lints).

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// One DNxHD decode-stage failure (internal vocabulary).
///
/// Mapping to the public contract (doc 01 §3.4, doc 03 §4.5(d)):
///
/// | variant | public kind | stage |
/// |---|---|---|
/// | [`DnxError::Unsupported`] | [`ErrorKind::Unsupported`] | Decode |
/// | [`DnxError::Corrupt`] | [`ErrorKind::Corrupt`] | Decode |
/// | [`DnxError::BadState`] | [`ErrorKind::Decoder`] (`BadState`) | Decode |
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DnxError {
    /// A configuration or stream feature outside the supported subset
    /// (foreign descriptor, inexpressible output geometry). `code`
    /// names the exact rule; `detail` rides in `source`.
    Unsupported {
        /// Stable dotted context code naming the exact rule broken.
        code: &'static str,
        /// Human-readable reason (rides in `source`, not `Display`).
        detail: String,
    },
    /// Damaged media inside this frame's coding units: structural
    /// corruption, truncation, an engine decode rejection. Typed
    /// with the exact [`CorruptDetail`] so reporting is observable.
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

impl DnxError {
    /// Convert to the public contract error (stage `Decode`).
    pub(crate) fn into_engine_error(self) -> EngineError {
        let stage = PipelineStage::Decode;
        match self {
            DnxError::Unsupported { code, detail } => EngineError::new(
                stage,
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("dnxhd".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail)),
            DnxError::Corrupt {
                code,
                detail,
                detail_text,
            } => EngineError::new(
                stage,
                ErrorKind::Corrupt { detail },
                ErrorContext::new(code),
            )
            .with_source(Detail(detail_text)),
            DnxError::BadState => EngineError::new(
                stage,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                ErrorContext::new("codecs.dnx.not_configured"),
            ),
        }
    }
}

impl From<DnxError> for EngineError {
    fn from(value: DnxError) -> EngineError {
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
pub(crate) fn unsupported(code: &'static str, detail: impl Into<String>) -> DnxError {
    DnxError::Unsupported {
        code,
        detail: detail.into(),
    }
}

/// Shorthand constructor: corrupt input with a structured class.
pub(crate) fn corrupt(
    code: &'static str,
    detail: CorruptDetail,
    detail_text: impl Into<String>,
) -> DnxError {
    DnxError::Corrupt {
        code,
        detail,
        detail_text: detail_text.into(),
    }
}

/// Map the engine's own error vocabulary (one call per damaged
/// coding unit) onto the house taxonomy. The engine is typed and
/// panic-free by construction (`unsafe_code = "forbid"` + its own
/// deny-lint set — the admission record's R1); this mapping only
/// re-labels. `found` is the input length the engine saw.
pub(crate) fn engine_error(error: filmcraft_dnx::Error, found: usize) -> DnxError {
    match error {
        filmcraft_dnx::Error::Truncated => corrupt(
            "codecs.dnx.frame.truncated",
            CorruptDetail::Truncated {
                // The engine reports truncation without the required
                // byte count; 0 = "not signalled by the engine" and
                // the source chain says so. The wrapper's CBR gate
                // (decoder.rs) covers the declared-size window with
                // exact numbers; this arm is the residual (VBR
                // streams, damage past the declared size).
                expected: 0,
                found: found as u64,
            },
            "VC-3 coding unit truncated inside the engine (required size not signalled)",
        ),
        filmcraft_dnx::Error::Invalid(rule) => corrupt(
            "codecs.dnx.frame.invalid",
            CorruptDetail::InvalidStructure { offset: None },
            format!("invalid VC-3 data: {rule}"),
        ),
        filmcraft_dnx::Error::UnknownCid(cid) => unsupported(
            "codecs.dnx.frame.unknown_cid",
            format!("unknown VC-3 compression ID {cid} (outside the engine's CIDs)"),
        ),
        filmcraft_dnx::Error::Row(line) => corrupt(
            "codecs.dnx.frame.row",
            CorruptDetail::InvalidStructure { offset: None },
            format!("corrupt macroblock scan line {line}"),
        ),
        filmcraft_dnx::Error::Input(detail) => corrupt(
            "codecs.dnx.frame.engine",
            CorruptDetail::InvalidStructure { offset: None },
            format!("engine rejected the coding unit: {detail}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taxonomy_maps_each_variant_to_its_public_kind() {
        let cases: [(DnxError, &str, &str); 3] = [
            (
                unsupported("codecs.dnx.output.bit_depth", "10-bit samples"),
                "[decode] unsupported in dnxhd (codecs.dnx.output.bit_depth)",
                "10-bit samples",
            ),
            (
                corrupt(
                    "codecs.dnx.frame.short",
                    CorruptDetail::Truncated {
                        expected: 917_504,
                        found: 640,
                    },
                    "CBR gate",
                ),
                "[decode] truncated data: expected 917504 bytes, found 640 (codecs.dnx.frame.short)",
                "CBR gate",
            ),
            (
                DnxError::BadState,
                "[decode] decoder: invalid state transition (codecs.dnx.not_configured)",
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
    fn engine_error_maps_every_engine_variant() {
        // Every arm of filmcraft_dnx::Error maps to a typed house
        // error with a codecs.dnx.* code (the Input arm is
        // encoder-side and unreachable from decode — mapped
        // defensively, still total).
        let engine_errors = [
            filmcraft_dnx::Error::Truncated,
            filmcraft_dnx::Error::Invalid("unknown header version"),
            filmcraft_dnx::Error::UnknownCid(9999),
            filmcraft_dnx::Error::Row(7),
            filmcraft_dnx::Error::Input("unused by decode".to_string()),
        ];
        let expected_codes = [
            "codecs.dnx.frame.truncated",
            "codecs.dnx.frame.invalid",
            "codecs.dnx.frame.unknown_cid",
            "codecs.dnx.frame.row",
            "codecs.dnx.frame.engine",
        ];
        for (error, code) in engine_errors.iter().zip(expected_codes) {
            let mapped = engine_error(error.clone(), 1234).into_engine_error();
            assert_eq!(mapped.context.code, code, "{error:?}");
        }
        // The unknown-CID arm classifies Unsupported (a stream
        // feature outside the vocabulary), the truncation arm Corrupt.
        let unknown_cid =
            engine_error(filmcraft_dnx::Error::UnknownCid(9999), 0).into_engine_error();
        assert!(matches!(
            unknown_cid.kind,
            ErrorKind::Unsupported { format: _ }
        ));
        let truncated =
            engine_error(filmcraft_dnx::Error::Truncated, 10).into_engine_error();
        assert!(matches!(
            truncated.kind,
            ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated {
                    expected: 0,
                    found: 10
                }
            }
        ));
    }
}
