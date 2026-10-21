//! Typed error constructors for the audio path (doc 01 §3.4 taxonomy).
//!
//! Every failure in this crate is an [`EngineError`] at stage
//! [`Mix`](media_types::PipelineStage::Mix) — the audio-engine's stage
//! in the pipeline — with a stable dotted context code. The codes are
//! part of the compatibility surface (tests may key on them) and are
//! minted here so the mapping from failure to (kind, code) has one
//! auditable home.

use media_types::error::{CorruptDetail, EngineError, ErrorContext, ErrorKind, PipelineStage};

/// The audio path's pipeline stage tag.
const STAGE: PipelineStage = PipelineStage::Mix;

/// A gain value was not usable (not finite, or negative linear gain).
///
/// Kind: [`Unsupported`](media_types::ErrorKind::Unsupported) — a value
/// the audio path does not support.
pub fn bad_gain(code: &'static str) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Unsupported { format: None },
        ErrorContext::new(code),
    )
}

/// A channel index or channel-count combination was out of range.
pub fn channel_range(code: &'static str) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Unsupported { format: None },
        ErrorContext::new(code),
    )
}

/// A format the v0 audio path does not convert (e.g. a rate mismatch —
/// the micro-resampler is a later card, doc 03 §4.7 e).
pub fn unsupported_format(code: &'static str) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Unsupported { format: None },
        ErrorContext::new(code),
    )
}

/// Sample payload bytes are not a whole number of frames.
///
/// Kind: [`Corrupt`](media_types::ErrorKind::Corrupt) — the data is
/// malformed at the decode boundary, not merely unsupported.
pub fn partial_frame(expected: u64, found: u64) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        ErrorContext::new("audio.partial_frame"),
    )
}

/// Mixed-plane geometry mismatch (plane count vs. declared channels,
/// or uneven plane lengths).
pub fn plane_mismatch(code: &'static str) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        ErrorContext::new(code),
    )
}

/// A track index outside the pipeline's negotiated track set.
pub fn track_range(code: &'static str) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Unsupported { format: None },
        ErrorContext::new(code),
    )
}

/// Staged chunks across tracks disagree on frame count (the v0 mixer
/// mixes whole aligned chunks; cross-track timing alignment is the
/// session's job, doc 03 §4.6). The offending lengths ride in the
/// caller's test output, not the error (the taxonomy carries
/// structured causes; this is a wiring shape, reported at the site).
pub fn chunk_alignment() -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Corrupt {
            detail: CorruptDetail::Timing,
        },
        ErrorContext::new("audio.chunk_length_mismatch"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable_and_lowercase() {
        // The taxonomy contract: stage Mix, stable dotted codes, and
        // the kinds carry structured payloads (no secrets, doc 01 §3.4).
        let e = partial_frame(4, 3);
        assert_eq!(e.stage, PipelineStage::Mix);
        assert_eq!(e.context.code, "audio.partial_frame");
        assert!(e.to_string().contains("[mix]"), "{}", e);
        assert!(e.to_string().contains("audio.partial_frame"), "{}", e);
        assert!(e.to_string().contains("truncated"), "{}", e);

        let e = unsupported_format("audio.rate_conversion_unsupported");
        assert_eq!(e.context.code, "audio.rate_conversion_unsupported");
        assert!(matches!(e.kind, ErrorKind::Unsupported { format: None }));

        let e = chunk_alignment();
        assert_eq!(e.context.code, "audio.chunk_length_mismatch");
        assert!(e.to_string().contains("timing"), "{}", e);
    }
}
