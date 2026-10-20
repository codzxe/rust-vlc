//! The bounded presentation queue (card 2.C4): in-order delivery of
//! [`PresentRequest`]s from the scheduler to a [`Presenter`], with a
//! hard capacity bound and generation-aware wholesale replacement.
//!
//! # Where this sits (doc 03 §4.10 c, §5 checking point 7)
//!
//! The scheduler emits at most one request per tick; this queue is the
//! hand-off the render side drains (per vsync in production wiring —
//! the render thread is 2.D1's, the pacing is the playback engine's).
//! On a generation change (seek, track switch, stop) the session calls
//! [`PresentationQueue::publish`]: every resident request is dropped
//! wholesale — "the renderer mailbox is replaced wholesale" (doc 03
//! §5(7)) — and later pushes of the old generation are refused at the
//! boundary, counted, never silent.
//!
//! # Boundedness
//!
//! Capacity is a [`NonZeroUsize`] at construction: an unbounded
//! presentation queue is unconstructible (MP §8). A full queue refuses
//! the push with a typed [`EnqueueOutcome::Full`] signal and hands the
//! request back untouched — the scheduler's tick cadence (one request
//! per tick vs one drain per vsync) makes depth 1 sufficient in the
//! steady state `[PROPOSAL: capacity is the wiring's choice; doc 03
//! §4.11 c proposes a depth-1 latest-wins mailbox on the renderer side
//! of this seam]`.

use std::collections::VecDeque;
use std::num::NonZeroUsize;

use media_types::GenerationId;

use crate::sink::{PresentRequest, Presenter};

/// The result of pushing one request into the presentation queue.
///
/// Every non-`Enqueued` variant hands the request back untouched: the
/// queue never silently swallows a present (fresh data is never
/// dropped — the packet-queue Q3 discipline, applied to requests).
#[must_use = "the outcome carries the request back on every non-Enqueued path; ignoring it can lose a present"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnqueueOutcome {
    /// Accepted; delivery order is FIFO within the generation.
    Enqueued,
    /// The queue is at capacity. The request is returned untouched;
    /// the caller retries after a drain or reacts — its choice.
    Full {
        /// The untouched request, handed back.
        request: PresentRequest,
        /// The capacity that refused the push.
        capacity: usize,
    },
    /// The request's generation is behind the queue's published
    /// generation — dropped at the boundary (doc 03 §5 checking point
    /// 7), counted in [`QueueStats::stale_rejected`]. The request is
    /// returned untouched; the caller must stop, not retry.
    Stale {
        /// The untouched request, handed back.
        request: PresentRequest,
        /// The queue's published generation the request fell behind.
        published: GenerationId,
    },
}

/// A drain stopped partway: the presenter refused a request (surface
/// lost, device busy — its [`EngineError`](media_types::EngineError)),
/// after `delivered` earlier requests had been handed over.
///
/// The refused request stays at the front of the queue: a retry
/// re-attempts it first (in-order delivery resumes exactly where it
/// stopped — no request is skipped or duplicated by the error path).
///
/// Not `Clone`/`PartialEq`: the wrapped [`EngineError`] is neither
/// (its `source` chain is opaque by design, doc 01 §3.4).
#[derive(Debug)]
pub struct DrainError {
    /// How many requests were delivered before the failure.
    pub delivered: usize,
    /// The presenter's error, verbatim.
    pub source: media_types::EngineError,
}

impl std::fmt::Display for DrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "presentation drain stopped after {} delivered: {}",
            self.delivered, self.source
        )
    }
}

impl std::error::Error for DrainError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        std::error::Error::source(&self.source)
    }
}

/// Lifetime counters for observability (doc 01 §2.2 e: dropped-frame
/// observability, MP §11 metrics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QueueStats {
    /// Requests accepted into the queue.
    pub enqueued: u64,
    /// Requests successfully handed to a presenter.
    pub delivered: u64,
    /// Pushes refused at capacity.
    pub full_rejections: u64,
    /// Pushes refused for a stale generation (doc 03 §5(7)).
    pub stale_rejected: u64,
    /// Resident requests dropped wholesale by generation publishes.
    pub flushed: u64,
}

/// The bounded, in-order presentation queue (see the [module
/// docs](self)).
#[derive(Debug)]
pub struct PresentationQueue {
    capacity: NonZeroUsize,
    slots: VecDeque<PresentRequest>,
    published: GenerationId,
    stats: QueueStats,
}

impl PresentationQueue {
    /// A queue of the given capacity publishing [`GenerationId::FIRST`].
    #[must_use = "constructing a queue without using it gains nothing"]
    pub fn new(capacity: NonZeroUsize) -> PresentationQueue {
        PresentationQueue::with_generation(capacity, GenerationId::FIRST)
    }

    /// A queue of the given capacity publishing `generation`.
    #[must_use = "constructing a queue without using it gains nothing"]
    pub fn with_generation(capacity: NonZeroUsize, generation: GenerationId) -> PresentationQueue {
        PresentationQueue {
            capacity,
            slots: VecDeque::new(),
            published: generation,
            stats: QueueStats::default(),
        }
    }

    /// Replace the published generation: every resident request is
    /// dropped wholesale (doc 03 §5 checking point 7), whatever its own
    /// generation — the mailbox is replaced, not filtered. Returns the
    /// number dropped (also counted in [`QueueStats::flushed`]).
    ///
    /// Publishing the generation it already publishes still flushes:
    /// the session's intent ("everything before this point is dead")
    /// is the operation, not the generation delta.
    pub fn publish(&mut self, generation: GenerationId) -> u64 {
        let dropped = self.slots.len() as u64;
        self.stats.flushed += dropped;
        self.slots.clear();
        self.published = generation;
        dropped
    }

    /// Offer one request. Bounded, generation-checked, FIFO within a
    /// generation — see [`EnqueueOutcome`] for the refusal shapes.
    pub fn try_push(&mut self, request: PresentRequest) -> EnqueueOutcome {
        if request.generation < self.published {
            self.stats.stale_rejected += 1;
            return EnqueueOutcome::Stale {
                request,
                published: self.published,
            };
        }
        if self.slots.len() >= self.capacity.get() {
            self.stats.full_rejections += 1;
            return EnqueueOutcome::Full {
                request,
                capacity: self.capacity.get(),
            };
        }
        self.slots.push_back(request);
        self.stats.enqueued += 1;
        EnqueueOutcome::Enqueued
    }

    /// Deliver every resident request to `sink`, in order, stopping at
    /// the first presenter refusal (the refused request stays queued —
    /// see [`DrainError`]).
    ///
    /// # Errors
    /// [`DrainError`] carrying the presenter's error and how many
    /// requests were delivered before it; the queue keeps the refused
    /// request and everything behind it.
    pub fn drain_to(&mut self, sink: &mut dyn Presenter) -> Result<usize, DrainError> {
        let mut delivered = 0usize;
        while let Some(front) = self.slots.front() {
            match sink.present(front) {
                Ok(_) => {
                    self.slots.pop_front();
                    delivered += 1;
                    self.stats.delivered += 1;
                }
                Err(source) => return Err(DrainError { delivered, source }),
            }
        }
        Ok(delivered)
    }

    /// Resident request count (0..=capacity).
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether no requests are resident.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The configured capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity.get()
    }

    /// The currently published generation.
    #[must_use]
    pub fn published(&self) -> GenerationId {
        self.published
    }

    /// Lifetime counters (see [`QueueStats`]).
    #[must_use]
    pub fn stats(&self) -> QueueStats {
        self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{RecordingPresenter, request};
    use media_clock::NANO_BASE;
    use media_types::GenerationId;
    use media_types::time::Timestamp;
    use testkit::Must;

    fn ns(ticks: i64) -> Timestamp {
        Timestamp::new(ticks, NANO_BASE)
    }

    #[test]
    fn fifo_delivery_in_order_to_the_sink() {
        let mut q = PresentationQueue::new(NonZeroUsize::new(4).must());
        for pts in [10i64, 20, 30] {
            assert_eq!(
                q.try_push(request(pts, GenerationId::FIRST)),
                EnqueueOutcome::Enqueued
            );
        }
        let mut sink = RecordingPresenter::new();
        let delivered = q.drain_to(&mut sink).must();
        assert_eq!(delivered, 3);
        assert_eq!(q.stats().delivered, 3);
        assert!(q.is_empty());
        assert_eq!(sink.slots_seen(), [10, 20, 30], "in-order FIFO");
    }

    #[test]
    fn capacity_is_a_hard_bound_with_the_request_handed_back() {
        let mut q = PresentationQueue::new(NonZeroUsize::new(2).must());
        assert_eq!(
            q.try_push(request(1, GenerationId::FIRST)),
            EnqueueOutcome::Enqueued
        );
        assert_eq!(
            q.try_push(request(2, GenerationId::FIRST)),
            EnqueueOutcome::Enqueued
        );
        let refused = match q.try_push(request(3, GenerationId::FIRST)) {
            EnqueueOutcome::Full { request, capacity } => {
                assert_eq!(capacity, 2);
                request
            }
            other => unreachable!("expected Full, got {other:?}"),
        };
        assert_eq!(refused.when, ns(3), "request handed back untouched");
        assert_eq!(q.stats().full_rejections, 1);
        assert_eq!(q.len(), 2);
        // Drain frees room; the retry then succeeds.
        let mut sink = RecordingPresenter::new();
        assert_eq!(q.drain_to(&mut sink).must(), 2);
        assert_eq!(
            q.try_push(request(3, GenerationId::FIRST)),
            EnqueueOutcome::Enqueued
        );
        assert_eq!(q.drain_to(&mut sink).must(), 1);
        assert_eq!(sink.slots_seen(), [1, 2, 3]);
    }

    #[test]
    fn capacity_one_works() {
        let mut q = PresentationQueue::new(NonZeroUsize::new(1).must());
        assert_eq!(
            q.try_push(request(5, GenerationId::FIRST)),
            EnqueueOutcome::Enqueued
        );
        match q.try_push(request(6, GenerationId::FIRST)) {
            EnqueueOutcome::Full { capacity, .. } => assert_eq!(capacity, 1),
            other => unreachable!("expected Full, got {other:?}"),
        }
        let mut sink = RecordingPresenter::new();
        assert_eq!(q.drain_to(&mut sink).must(), 1);
    }

    #[test]
    fn stale_generation_pushes_are_refused_counted_and_returned() {
        let mut q =
            PresentationQueue::with_generation(NonZeroUsize::new(4).must(), GenerationId::new(7));
        match q.try_push(request(1, GenerationId::new(3))) {
            EnqueueOutcome::Stale { request, published } => {
                assert_eq!(published, GenerationId::new(7));
                assert_eq!(request.when, ns(1), "handed back untouched");
            }
            other => unreachable!("expected Stale, got {other:?}"),
        }
        assert_eq!(q.stats().stale_rejected, 1);
        assert!(q.is_empty());
        // Current-generation pushes flow; newer generations wait
        // (packet-queue boundary semantics: strictly older is stale).
        assert_eq!(
            q.try_push(request(2, GenerationId::new(7))),
            EnqueueOutcome::Enqueued
        );
        assert_eq!(
            q.try_push(request(3, GenerationId::new(9))),
            EnqueueOutcome::Enqueued
        );
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn publish_flushes_wholesale_and_rejects_the_old_generation() {
        let mut q = PresentationQueue::new(NonZeroUsize::new(4).must());
        for pts in [1i64, 2, 3] {
            let _ = q.try_push(request(pts, GenerationId::FIRST));
        }
        let next = GenerationId::FIRST.next().must();
        assert_eq!(q.publish(next), 3);
        assert!(q.is_empty());
        assert_eq!(q.stats().flushed, 3);
        assert_eq!(q.published(), next);
        // The old generation can no longer enter.
        match q.try_push(request(4, GenerationId::FIRST)) {
            EnqueueOutcome::Stale { published, .. } => assert_eq!(published, next),
            other => unreachable!("expected Stale, got {other:?}"),
        }
        // Re-publishing the same generation still flushes (the intent
        // is "everything before this point is dead").
        assert_eq!(
            q.try_push(request(5, next)),
            EnqueueOutcome::Enqueued,
            "current generation flows after publish"
        );
        assert_eq!(q.publish(next), 1, "same-gen publish flushes too");
    }

    #[test]
    fn presenter_failure_keeps_the_refused_request_at_the_front() {
        let mut q = PresentationQueue::new(NonZeroUsize::new(4).must());
        for pts in [1i64, 2, 3] {
            let _ = q.try_push(request(pts, GenerationId::FIRST));
        }
        // Fail on the second present (index 1).
        let mut sink = RecordingPresenter::failing_at(1);
        let err = match q.drain_to(&mut sink) {
            Ok(_) => unreachable!("the scripted failure must fire"),
            Err(e) => e,
        };
        assert_eq!(err.delivered, 1, "one delivered before the refusal");
        assert_eq!(
            err.source.context.code, "video.presenter.refused",
            "the presenter's error, verbatim"
        );
        assert_eq!(q.len(), 2, "refused request + the one behind it stay");
        assert_eq!(q.stats().delivered, 1);
        // A healthy sink resumes in order, re-attempting the refused
        // request first — no skip, no duplication.
        let mut healthy = RecordingPresenter::new();
        assert_eq!(q.drain_to(&mut healthy).must(), 2);
        assert_eq!(healthy.slots_seen(), [2, 3]);
    }
}
