//! The job resource envelope (card 7.E1): every bound the transcode
//! runner enforces, in one self-validating type (MP §12 — bounded
//! memory is a contract, not a suggestion).
//!
//! # The bounds (documented — the "no unbounded growth" map)
//!
//! | axis                  | field            | default | hard ceiling | enforced at |
//! |-----------------------|------------------|---------|--------------|-------------|
//! | stage-queue slots     | [`queue_slots`]  | 64      | 1024         | [`PacketStageQueue`](crate::queue) push |
//! | stage-queue payload   | [`queue_bytes`]  | 1 MiB   | 16 MiB       | same, byte axis |
//! | event-hub slots       | [`event_slots`]  | 32      | 1024         | [`JobEventHub`](crate::job) emit (drop + count) |
//! | source track count    | [`max_tracks`]   | 64      | 4096         | runner, right after `Demuxer::start` |
//! | requested filter rows | [`max_filter_stages`] | 4 | 16           | graph validation |
//!
//! Streams never accumulate anywhere else: packet payloads live in the
//! bounded stage queue (refcounted [`SharedBytes`] views — the
//! demuxer's pooled arena), decoded frames are written through to the
//! sink frame-by-frame (the WAV writer's only buffer is a reused
//! conversion scratch of at most one frame), and progress accounting
//! is saturating `u64` counters (the streaming crate's explicit
//! overflow policy: saturate, never wrap, never grow).
//!
//! [`queue_slots`]: JobLimits::queue_slots
//! [`queue_bytes`]: JobLimits::queue_bytes
//! [`event_slots`]: JobLimits::event_slots
//! [`max_tracks`]: JobLimits::max_tracks
//! [`max_filter_stages`]: JobLimits::max_filter_stages

use crate::error::{limit_over_budget, limit_zero};
use media_types::EngineError;

/// A job's resource envelope — every queue bounded, every ceiling
/// explicit (module docs table).
///
/// Built only through [`JobLimits::new`], which refuses zero values
/// (a zero-slot queue is a deadlock, not a limit) and values above
/// the hard ceilings (a misconfiguration this loud should surface at
/// submit, not as an OOM later). [`JobLimits::default`] is the
/// documented posture for interactive transcodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobLimits {
    queue_slots: usize,
    queue_bytes: usize,
    event_slots: usize,
    max_tracks: usize,
    max_filter_stages: usize,
}

impl JobLimits {
    /// Stage-queue slot ceiling `[PROPOSAL: 1024 — a deeper queue only
    /// hides a stalled consumer; the demux pump parks instead]`.
    pub const MAX_QUEUE_SLOTS: usize = 1024;
    /// Stage-queue payload ceiling `[PROPOSAL: 16 MiB — the demux
    /// crate's own pooled arena is 2 × 64 KiB; anything deeper is a
    /// stuck sink, not throughput]`.
    pub const MAX_QUEUE_BYTES: usize = 16 * 1024 * 1024;
    /// Event-hub slot ceiling `[PROPOSAL: 1024 — the hub drops and
    /// counts beyond capacity (the streaming `StreamEventHub`
    /// doctrine), so the ceiling is politeness, not memory]`.
    pub const MAX_EVENT_SLOTS: usize = 1024;
    /// Track-table ceiling `[PROPOSAL: 4096 — four times the largest
    /// track count in the committed corpus; the WAV demuxer's own
    /// 64-channel vocabulary fits far below]`.
    pub const MAX_TRACKS: usize = 4096;
    /// Filter-row ceiling `[PROPOSAL: 16 — the graph model carries
    /// filter stages; v1 refuses every one of them, the ceiling bounds
    /// the request table itself]`.
    pub const MAX_FILTER_STAGES: usize = 16;

    /// The documented default posture (the module-doc table's middle
    /// column) — in-construction constant, so it cannot fail.
    const DEFAULT_QUEUE_SLOTS: usize = 64;
    const DEFAULT_QUEUE_BYTES: usize = 1024 * 1024;
    const DEFAULT_EVENT_SLOTS: usize = 32;
    const DEFAULT_MAX_TRACKS: usize = 64;
    const DEFAULT_MAX_FILTER_STAGES: usize = 4;

    /// Validate and assemble an envelope.
    ///
    /// # Errors
    /// Typed at stage `Transcode`: [`limit_zero`] for any zero field,
    /// [`limit_over_budget`] for any field above its hard ceiling
    /// (first offender wins, in declaration order).
    pub fn new(
        queue_slots: usize,
        queue_bytes: usize,
        event_slots: usize,
        max_tracks: usize,
        max_filter_stages: usize,
    ) -> Result<JobLimits, EngineError> {
        let axis = [
            (queue_slots, Self::MAX_QUEUE_SLOTS),
            (queue_bytes, Self::MAX_QUEUE_BYTES),
            (event_slots, Self::MAX_EVENT_SLOTS),
            (max_tracks, Self::MAX_TRACKS),
            (max_filter_stages, Self::MAX_FILTER_STAGES),
        ];
        for (requested, ceiling) in axis {
            if requested == 0 {
                return Err(limit_zero());
            }
            if requested > ceiling {
                return Err(limit_over_budget(requested, ceiling));
            }
        }
        Ok(JobLimits {
            queue_slots,
            queue_bytes,
            event_slots,
            max_tracks,
            max_filter_stages,
        })
    }

    /// Stage-queue slot bound (packets resident between demux and the
    /// encode/copy worker).
    #[must_use]
    pub fn queue_slots(&self) -> usize {
        self.queue_slots
    }

    /// Stage-queue payload bound (sum of resident packet payload
    /// bytes).
    #[must_use]
    pub fn queue_bytes(&self) -> usize {
        self.queue_bytes
    }

    /// Event-hub bound (undelivered [`JobEvent`](crate::JobEvent)
    /// slots; overflow drops and counts, never blocks the worker).
    #[must_use]
    pub fn event_slots(&self) -> usize {
        self.event_slots
    }

    /// Track-table bound (`Demuxer::start` enumerations above this
    /// fail the job typed).
    #[must_use]
    pub fn max_tracks(&self) -> usize {
        self.max_tracks
    }

    /// Filter-row bound (graph validation refuses requests above it;
    /// v1 refuses every request within it too — honest capability).
    #[must_use]
    pub fn max_filter_stages(&self) -> usize {
        self.max_filter_stages
    }
}

impl Default for JobLimits {
    /// The documented default posture (module docs table).
    fn default() -> Self {
        JobLimits {
            queue_slots: Self::DEFAULT_QUEUE_SLOTS,
            queue_bytes: Self::DEFAULT_QUEUE_BYTES,
            event_slots: Self::DEFAULT_EVENT_SLOTS,
            max_tracks: Self::DEFAULT_MAX_TRACKS,
            max_filter_stages: Self::DEFAULT_MAX_FILTER_STAGES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::{ErrorKind, PipelineStage};

    #[test]
    fn defaults_are_the_documented_posture() {
        let d = JobLimits::default();
        assert_eq!(d.queue_slots(), 64);
        assert_eq!(d.queue_bytes(), 1024 * 1024);
        assert_eq!(d.event_slots(), 32);
        assert_eq!(d.max_tracks(), 64);
        assert_eq!(d.max_filter_stages(), 4);
    }

    #[test]
    fn zero_fields_are_refused_typed() {
        for limits in [
            JobLimits::new(0, 1024, 32, 64, 4),
            JobLimits::new(64, 0, 32, 64, 4),
            JobLimits::new(64, 1024, 0, 64, 4),
            JobLimits::new(64, 1024, 32, 0, 4),
            JobLimits::new(64, 1024, 32, 64, 0),
        ] {
            match limits {
                Err(err) => {
                    assert_eq!(err.stage, PipelineStage::Transcode);
                    assert_eq!(err.context.code, "transcode.limits.zero");
                    assert!(
                        matches!(err.kind, ErrorKind::Unsupported { format: None }),
                        "zero is an unusable limit, not a resource report"
                    );
                }
                Ok(_) => unreachable!("zero limits must be refused"),
            }
        }
    }

    #[test]
    fn over_ceiling_fields_are_refused_typed_first_offender() {
        // Each axis alone over its ceiling, in declaration order.
        let over_slots = JobLimits::new(1025, 1024, 32, 64, 4);
        match over_slots {
            Err(err) => {
                assert_eq!(err.context.code, "transcode.limits.over-budget");
                assert_eq!(
                    err.kind,
                    ErrorKind::ResourceLimit {
                        limit: media_types::error::LimitKind::Memory,
                        budget: 1024,
                        attempted: 1025,
                    }
                );
            }
            Ok(_) => unreachable!("over-ceiling must be refused"),
        }
        // The first offender wins (queue slots before queue bytes).
        let both = JobLimits::new(2000, 32 * 1024 * 1024, 32, 64, 4);
        match both {
            Err(err) => assert_eq!(
                err.kind,
                ErrorKind::ResourceLimit {
                    limit: media_types::error::LimitKind::Memory,
                    budget: 1024,
                    attempted: 2000,
                }
            ),
            Ok(_) => unreachable!("two over-ceiling fields must still refuse once"),
        }
        // The remaining axes each refuse too.
        for limits in [
            JobLimits::new(64, 16 * 1024 * 1024 + 1, 32, 64, 4),
            JobLimits::new(64, 1024, 1025, 64, 4),
            JobLimits::new(64, 1024, 32, 4097, 4),
            JobLimits::new(64, 1024, 32, 64, 17),
        ] {
            assert!(
                limits.is_err(),
                "every over-ceiling axis refuses individually"
            );
        }
    }

    #[test]
    fn exact_ceilings_are_legal() {
        let edge = JobLimits::new(
            JobLimits::MAX_QUEUE_SLOTS,
            JobLimits::MAX_QUEUE_BYTES,
            JobLimits::MAX_EVENT_SLOTS,
            JobLimits::MAX_TRACKS,
            JobLimits::MAX_FILTER_STAGES,
        );
        match edge {
            Ok(limits) => {
                assert_eq!(limits.queue_slots(), JobLimits::MAX_QUEUE_SLOTS);
                assert_eq!(limits.queue_bytes(), JobLimits::MAX_QUEUE_BYTES);
            }
            Err(_) => unreachable!("the ceiling itself is inside the contract"),
        }
    }
}
