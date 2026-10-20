//! Decode-stage taxonomy for MJPEG (card 2.B4 requirement): typed,
//! stage-tagged internal errors that map one-to-one onto
//! [`EngineError`] kinds at the contract boundary.
//!
//! The taxonomy names *where* in the decode a failure happened, per the
//! card: corrupt entropy data, unsupported JPEG feature, truncation —
//! plus the structural-marker and resource-limit cases the fuzz corpus
//! produces. Nothing here panics; every constructor is total.

use media_types::error::{CorruptDetail, ErrorContext, FormatInfo};
use media_types::{EngineError, ErrorKind, PipelineStage};

/// One decoded stage's failure (internal vocabulary).
///
/// Mapping to the public contract (doc 01 §3.4, doc 03 §4.5(d)):
///
/// | variant | public kind | stage |
/// |---|---|---|
/// | [`MjpegError::UnsupportedFeature`] | [`ErrorKind::Unsupported`] | Decode |
/// | [`MjpegError::Truncated`] | [`ErrorKind::Corrupt`] (`Truncated`) | Decode |
/// | [`MjpegError::BadMarker`] | [`ErrorKind::Corrupt`] (`InvalidStructure`) | Decode |
/// | [`MjpegError::CorruptEntropy`] | [`ErrorKind::Decoder`] (`Bitstream`) | Decode |
/// | [`MjpegError::LimitExceeded`] | [`ErrorKind::ResourceLimit`] | Decode |
///
/// [`CorruptEntropy`] and post-SOS [`Truncated`] are *also* concealable:
/// the decoder turns them into
/// [`DecodeOutcome::Recovered`](codec_api::DecodeOutcome::Recovered)
/// with [`RecoverReason::ConcealedFrame`](codec_api::RecoverReason::ConcealedFrame)
/// when enough header structure survived to emit an honest
/// partially-decoded frame (see `decoder.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MjpegError {
    /// A JPEG feature outside the implemented baseline subset
    /// (progressive scans, 12-bit precision, arithmetic coding,
    /// non-interleaved scans, sampling factors above 2, …).
    UnsupportedFeature(String),
    /// The byte stream ended before the structure it declared.
    Truncated {
        /// Bytes the structure needed (a lower bound; segments declare
        /// their own length, entropy scans are open-ended).
        expected: u64,
        /// Bytes actually present.
        found: u64,
    },
    /// A marker segment is structurally invalid (bad length, bad
    /// component count, missing table, zero dimensions, …) at a byte
    /// offset into the packet.
    BadMarker {
        /// Stable dotted context code naming the exact rule broken.
        code: &'static str,
        /// Byte offset into the JPEG where the violation was detected.
        offset: u64,
    },
    /// Entropy-coded data violated the Huffman/grammar rules (code not
    /// in any table, coefficient run past 63, restart sequence
    /// mismatch, …).
    CorruptEntropy {
        /// Stable dotted context code naming the exact rule broken.
        code: &'static str,
    },
    /// A decode resource limit was exceeded (dimensions/pixels above
    /// the cap — the fuzz-safety memory bound).
    LimitExceeded {
        /// Which limit, its size, and the attempted value.
        limit: media_types::error::LimitKind,
        budget: u64,
        attempted: u64,
    },
}

impl MjpegError {
    /// Convert to the public contract error (stage `Decode`, doc 01 §3.4).
    pub(crate) fn into_engine_error(self) -> EngineError {
        let stage = PipelineStage::Decode;
        match self {
            MjpegError::UnsupportedFeature(detail) => EngineError::new(
                stage,
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("mjpeg".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                ErrorContext::new("mjpeg.unsupported"),
            )
            .with_source(FeatureDetail(detail)),
            MjpegError::Truncated { expected, found } => EngineError::new(
                stage,
                ErrorKind::Corrupt {
                    detail: CorruptDetail::Truncated { expected, found },
                },
                ErrorContext::new("mjpeg.truncated"),
            ),
            MjpegError::BadMarker { code, offset } => EngineError::new(
                stage,
                ErrorKind::Corrupt {
                    detail: CorruptDetail::InvalidStructure {
                        offset: Some(offset),
                    },
                },
                ErrorContext::new(code),
            ),
            MjpegError::CorruptEntropy { code } => EngineError::new(
                stage,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::Bitstream,
                },
                ErrorContext::new(code),
            ),
            MjpegError::LimitExceeded {
                limit,
                budget,
                attempted,
            } => EngineError::new(
                stage,
                ErrorKind::ResourceLimit {
                    limit,
                    budget,
                    attempted,
                },
                ErrorContext::new("mjpeg.limit"),
            ),
        }
    }
}

/// Chain-root carrier for the unsupported-feature detail (keeps the
/// human-readable reason in the error's `source` without leaking it into
/// `Display`'s no-secrets surface).
#[derive(Debug, Clone)]
struct FeatureDetail(String);

impl std::fmt::Display for FeatureDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FeatureDetail {}

/// Shorthand constructor: structural marker violation at `offset`.
pub(crate) fn bad_marker(code: &'static str, offset: usize) -> MjpegError {
    MjpegError::BadMarker {
        code,
        offset: offset as u64,
    }
}

/// Shorthand constructor: entropy-data violation.
pub(crate) fn corrupt_entropy(code: &'static str) -> MjpegError {
    MjpegError::CorruptEntropy { code }
}

/// Shorthand constructor: unsupported feature with a reason.
pub(crate) fn unsupported(detail: impl Into<String>) -> MjpegError {
    MjpegError::UnsupportedFeature(detail.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taxonomy_maps_each_variant_to_its_public_kind() {
        let cases: [(MjpegError, &str); 5] = [
            (
                unsupported("progressive DCT (SOF2)"),
                "[decode] unsupported in mjpeg (mjpeg.unsupported)",
            ),
            (
                MjpegError::Truncated {
                    expected: 4,
                    found: 2,
                },
                "[decode] truncated data: expected 4 bytes, found 2 (mjpeg.truncated)",
            ),
            (
                bad_marker("mjpeg.marker.bad_length", 42),
                "[decode] corrupt structure at byte 42 (mjpeg.marker.bad_length)",
            ),
            (
                corrupt_entropy("mjpeg.entropy.code_not_in_table"),
                "[decode] decoder: bitstream error (mjpeg.entropy.code_not_in_table)",
            ),
            (
                MjpegError::LimitExceeded {
                    limit: media_types::error::LimitKind::Memory,
                    budget: 33_554_432,
                    attempted: 40_000_000,
                },
                "[decode] resource limit: memory (budget 33554432, attempted 40000000) (mjpeg.limit)",
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
        let engine = unsupported("12-bit sample precision").into_engine_error();
        assert!(
            matches!(engine.kind, ErrorKind::Unsupported { .. }),
            "{engine:?}"
        );
        let source = std::error::Error::source(&engine)
            .map(|s| s.to_string())
            .unwrap_or_default();
        assert_eq!(source, "12-bit sample precision");
    }
}
