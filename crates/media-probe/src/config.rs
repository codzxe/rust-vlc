//! The probe configuration (doc 03 §4.2 `ProbeConfig`) and the
//! constants it bounds.
//!
//! Defaults are the doc's own values, both `[PROPOSAL, overridable from
//! product.toml resource limits]` in the spec: `max_lookahead = 64 KiB`
//! hard-capped at 1 MiB, `min_score = 60`. Construction validates —
//! a zero or over-ceiling lookahead and an out-of-range minimum score
//! are typed errors at the boundary, never silent clamps (the input
//! crate's capacity-bounds pattern).

use media_types::EngineError;

use crate::error;
use crate::report::CONFIDENCE_MAX;

/// Default probe window: 64 KiB (doc 03 §4.2 — "a comfortable probe
/// window", the same figure as input's read-ahead chunk).
pub const DEFAULT_MAX_LOOKAHEAD: usize = 64 * 1024;

/// Hard ceiling on the probe window: 1 MiB (doc 03 §4.2 — "hard-capped
/// at 1 MiB"). A configured lookahead above this is a typed
/// `ResourceLimit` error, the memory bound MP §12 demands.
pub const MAX_LOOKAHEAD_CEILING: usize = 1024 * 1024;

/// Default minimum score for a selectable hit: 60 (doc 03 §4.2
/// `[PROPOSAL]`).
pub const DEFAULT_MIN_SCORE: u8 = 60;

/// The probe configuration (doc 03 §4.2).
///
/// Probing reads **only** up to `max_lookahead` bytes into its window;
/// there is no code path that probes more (the fill loop in
/// [`probe_stream`](crate::probe_stream) is the single reader and its
/// budget is this field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeConfig {
    /// The bounded header window, in bytes (1..=[`MAX_LOOKAHEAD_CEILING`]).
    pub max_lookahead: usize,
    /// The minimum score a hit needs to select a demuxer (0..=100).
    pub min_score: u8,
}

impl ProbeConfig {
    /// Build and validate a config.
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::Probe`]) when
    /// `max_lookahead` is zero or exceeds
    /// [`MAX_LOOKAHEAD_CEILING`] (`ResourceLimit` /
    /// `LimitKind::ProbeDepth` — the doc's hard cap), or `min_score`
    /// exceeds [`CONFIDENCE_MAX`].
    pub fn new(max_lookahead: usize, min_score: u8) -> Result<ProbeConfig, EngineError> {
        if max_lookahead == 0 {
            return Err(error::lookahead_zero());
        }
        if max_lookahead > MAX_LOOKAHEAD_CEILING {
            return Err(error::lookahead_over_ceiling(max_lookahead));
        }
        if min_score > CONFIDENCE_MAX {
            return Err(error::min_score_out_of_range(min_score));
        }
        Ok(ProbeConfig {
            max_lookahead,
            min_score,
        })
    }
}

impl Default for ProbeConfig {
    /// The doc 03 §4.2 defaults (in range by construction).
    fn default() -> ProbeConfig {
        ProbeConfig {
            max_lookahead: DEFAULT_MAX_LOOKAHEAD,
            min_score: DEFAULT_MIN_SCORE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::PipelineStage;
    use media_types::error::{ErrorKind, LimitKind};

    fn ok(r: Result<ProbeConfig, EngineError>) -> ProbeConfig {
        match r {
            Ok(c) => c,
            Err(e) => unreachable!("valid config: {e}"),
        }
    }

    fn err(r: Result<ProbeConfig, EngineError>) -> EngineError {
        match r {
            Ok(_) => unreachable!("must fail"),
            Err(e) => e,
        }
    }

    #[test]
    fn defaults_are_the_doc_values() {
        let d = ProbeConfig::default();
        assert_eq!(d.max_lookahead, 64 * 1024);
        assert_eq!(d.min_score, 60);
    }

    #[test]
    fn boundaries_construct() {
        // Smallest legal window, the ceiling itself, and the full
        // score range (0 selectable-everything, 100 perfect-only).
        let one = ok(ProbeConfig::new(1, 0));
        assert_eq!((one.max_lookahead, one.min_score), (1, 0));
        let ceil = ok(ProbeConfig::new(MAX_LOOKAHEAD_CEILING, CONFIDENCE_MAX));
        assert_eq!(
            (ceil.max_lookahead, ceil.min_score),
            (MAX_LOOKAHEAD_CEILING, CONFIDENCE_MAX)
        );
    }

    #[test]
    fn zero_lookahead_is_a_typed_error() {
        let e = err(ProbeConfig::new(0, 60));
        assert_eq!(e.stage, PipelineStage::Probe);
        assert_eq!(e.context.code, "probe.lookahead-zero");
        assert!(matches!(e.kind, ErrorKind::Unsupported { format: None }));
    }

    #[test]
    fn over_ceiling_lookahead_is_a_typed_probe_depth_limit() {
        let e = err(ProbeConfig::new(MAX_LOOKAHEAD_CEILING + 1, 60));
        assert_eq!(e.stage, PipelineStage::Probe);
        assert_eq!(e.context.code, "probe.lookahead-over-ceiling");
        match e.kind {
            ErrorKind::ResourceLimit {
                limit,
                budget,
                attempted,
            } => {
                assert_eq!(limit, LimitKind::ProbeDepth);
                assert_eq!(budget, MAX_LOOKAHEAD_CEILING as u64);
                assert_eq!(attempted, (MAX_LOOKAHEAD_CEILING + 1) as u64);
            }
            other => unreachable!("wrong kind: {other:?}"),
        }
    }

    #[test]
    fn out_of_range_min_score_is_a_typed_error() {
        let e = err(ProbeConfig::new(4096, 101));
        assert_eq!(e.stage, PipelineStage::Probe);
        assert_eq!(e.context.code, "probe.min-score-out-of-range");
    }
}
