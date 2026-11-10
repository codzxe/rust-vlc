//! The job lifecycle (card 7.E1): one shared state cell per job —
//! state machine, saturating accounting, the bounded event hub, the
//! terminal outcome — plus the [`JobHandle`] the host holds.
//!
//! # The state machine
//!
//! ```text
//! queued ──(pump thread starts)── running ──┬── completed
//!                                           ├── failed
//!                                           └── cancelled
//! ```
//!
//! [`JobState::Queued`] is the post-`submit`, pre-thread window
//! (observable only racily — the pump sets [`JobState::Running`] as
//! its first action); every terminal is durable: it lands in the
//! shared cell **first-set-wins**, and [`JobHandle::wait`] joins
//! both pipeline threads before answering, so a terminal is never
//! observed before its writer finished finalizing the sink.
//!
//! # The bounded event hub (the "no unbounded growth" contract)
//!
//! [`Running`](JobEvent::Running) and [`Progress`](JobEvent::Progress)
//! events ride a bounded queue ([`JobLimits::event_slots`]); a full
//! hub **drops and counts** — the worker never blocks on
//! observability (the streaming crate's event-hub doctrine). A
//! dropped progress is subsumed by the next one; terminals are
//! *not* events: they are the outcome cell, which cannot overflow.
//!
//! # Accounting (real bytes and samples, never estimates)
//!
//! Every counter is a saturating `u64` (the streaming crate's
//! explicit overflow policy: saturate, never wrap, never grow):
//! bytes are payload lengths as they cross each stage, sample
//! frames are the writer's whole-frame counts, and the queue
//! high-water marks snapshot at termination so the bounded-resource
//! proof is a test, not a hope.
//!
//! # Drop semantics (recorded decision)
//!
//! Dropping a [`JobHandle`] **detaches** the job: the pipeline runs
//! to its source's end and finalizes its sink (nothing is
//! destructively torn down mid-write, D-4); the shared cell stays
//! alive on the threads' `Arc`s. A host that wants a prompt stop
//! calls [`cancel`](JobHandle::cancel) first — cancellation is
//! cooperative and prompt at stage boundaries (see [`crate::cancel`]).
//! The hub's bound is [`crate::limits::JobLimits::event_slots`].

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

use media_types::EngineError;

use crate::cancel::CancelToken;

/// Where a job is in its lifecycle (module docs' state machine).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    /// Submitted, no pipeline thread has acted yet.
    Queued,
    /// The pump is driving the source.
    Running,
    /// Source drained, sink finalized, outcome honest.
    Completed,
    /// A stage failed typed; no output validity is claimed.
    Failed,
    /// Cooperative stop honored; the sink was finalized best-effort
    /// (sizes tell the truth about the prefix that made it).
    Cancelled,
}

/// A job's lifetime accounting — real bytes and samples, saturating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JobStats {
    /// Packets of the selected track accepted into the stage queue.
    pub packets_read: u64,
    /// Payload bytes of those packets.
    pub bytes_read: u64,
    /// Packets of other tracks the pump dropped (selection filter).
    pub packets_skipped: u64,
    /// Damage survived at any stage (demux `Recovered`, decode
    /// `Recovered`, skipped-in-flush).
    pub recovered: u64,
    /// Container discontinuities crossed.
    pub discontinuities: u64,
    /// Decode path: frames the decoder emitted.
    pub frames_decoded: u64,
    /// Sample frames handed to the sink (whole frames only; the
    /// copy path counts payload frames, the ES sink makes no sample
    /// claim and leaves this zero).
    pub sample_frames_written: u64,
    /// Payload bytes handed to the sink (container headers excluded).
    pub bytes_written: u64,
    /// Stage-queue slot high-water mark (snapshotted at termination).
    pub queue_peak_slots: usize,
    /// Stage-queue payload-bytes high-water mark (ditto).
    pub queue_peak_bytes: usize,
}

impl JobStats {
    /// The saturating merge (a snapshot absorbing a delta; the
    /// high-water marks take the max, the counters add).
    fn absorb(&mut self, delta: &JobStats) {
        let Self {
            packets_read,
            bytes_read,
            packets_skipped,
            recovered,
            discontinuities,
            frames_decoded,
            sample_frames_written,
            bytes_written,
            queue_peak_slots,
            queue_peak_bytes,
        } = delta;
        self.packets_read = self.packets_read.saturating_add(*packets_read);
        self.bytes_read = self.bytes_read.saturating_add(*bytes_read);
        self.packets_skipped = self.packets_skipped.saturating_add(*packets_skipped);
        self.recovered = self.recovered.saturating_add(*recovered);
        self.discontinuities = self.discontinuities.saturating_add(*discontinuities);
        self.frames_decoded = self.frames_decoded.saturating_add(*frames_decoded);
        self.sample_frames_written = self.sample_frames_written.saturating_add(*sample_frames_written);
        self.bytes_written = self.bytes_written.saturating_add(*bytes_written);
        self.queue_peak_slots = self.queue_peak_slots.max(*queue_peak_slots);
        self.queue_peak_bytes = self.queue_peak_bytes.max(*queue_peak_bytes);
    }
}

/// One observability event off the running pipeline (terminals are
/// the [`JobOutcome`] cell, not events — module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobEvent {
    /// The pump thread began driving the source.
    Running,
    /// A stats snapshot (emitted on a byte cadence, plus once right
    /// before the terminal, so every finished job leaves at least
    /// one sample).
    Progress(JobStats),
}

/// The durable terminal outcome, exactly once per job.
#[derive(Debug)]
pub enum JobOutcome {
    /// Source drained, decoder flushed, sink finalized successfully.
    Completed(JobStats),
    /// A stage refused or failed; the error is the stage's own typed
    /// value (context codes are this crate's compatibility surface).
    Failed(EngineError),
    /// Cooperative cancellation honored; the sink was finalized
    /// best-effort so the partial output's sizes tell the truth.
    Cancelled(JobStats),
}

/// The bounded event hub (module docs: drop-and-count, never block).
pub(crate) struct JobEventHub {
    slots: Mutex<VecDeque<JobEvent>>,
    capacity: usize,
    dropped: Mutex<u64>,
}

impl JobEventHub {
    /// A hub with `capacity` undelivered-event slots.
    pub(crate) fn new(capacity: usize) -> JobEventHub {
        JobEventHub {
            slots: Mutex::new(VecDeque::new()),
            capacity,
            dropped: Mutex::new(0),
        }
    }

    /// Offer an event: enqueue while there is room, drop-and-count
    /// past it. Never blocks, never grows past the capacity.
    pub(crate) fn emit(&self, event: JobEvent) {
        let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        if slots.len() < self.capacity {
            slots.push_back(event);
        } else {
            drop(slots);
            let mut dropped = self.dropped.lock().unwrap_or_else(|p| p.into_inner());
            *dropped = dropped.saturating_add(1);
        }
    }

    /// Take the oldest undelivered event, if any.
    pub(crate) fn try_recv(&self) -> Option<JobEvent> {
        self.slots
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .pop_front()
    }

    /// How many events were dropped past capacity (the honesty
    /// counter: observability was bounded, and says so).
    pub(crate) fn dropped(&self) -> u64 {
        *self.dropped.lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// The guarded state cell (one mutex, infallible critical sections —
/// the queue module's unpoison doctrine). The terminal outcome lives
/// in the [`OnceLock`] beside it, not here — first-set-wins is the
/// lock's own semantics, and `wait` must be able to replay it.
struct JobCore {
    state: JobState,
    stats: JobStats,
}

/// The per-job shared cell both pipeline threads and the host see.
pub(crate) struct JobShared {
    core: Mutex<JobCore>,
    /// The terminal, set exactly once (`OnceLock`: first-set-wins is
    /// the type's own semantics) and readable forever after —
    /// `wait` is idempotent because the outcome is shared, not taken.
    outcome: OnceLock<JobOutcome>,
    hub: JobEventHub,
    cancel: CancelToken,
    /// Wakes `wait`-style pollers on terminal (not used by the
    /// threads' own protocol — they join, not poll).
    settled: Condvar,
}

impl JobShared {
    /// Assemble with the event-hub capacity and the cancel token the
    /// host-side `cancel()` fires.
    pub(crate) fn new(event_slots: usize, cancel: CancelToken) -> JobShared {
        JobShared {
            core: Mutex::new(JobCore {
                state: JobState::Queued,
                stats: JobStats::default(),
            }),
            outcome: OnceLock::new(),
            hub: JobEventHub::new(event_slots),
            cancel,
            settled: Condvar::new(),
        }
    }

    /// The pump's first action: `Queued → Running`, event emitted.
    pub(crate) fn set_running(&self) {
        let mut core = self.lock();
        if core.state == JobState::Queued {
            core.state = JobState::Running;
            drop(core);
            self.hub.emit(JobEvent::Running);
        }
    }

    /// Fold a stats delta in (saturating; module docs).
    pub(crate) fn update_stats(&self, delta: &JobStats) {
        self.lock().stats.absorb(delta);
    }

    /// Snapshot the counters (progress events and host polling).
    pub(crate) fn snapshot(&self) -> JobStats {
        self.lock().stats
    }

    /// Fold the queue's high-water marks in (called at termination —
    /// the queue's own stats are monotonic, so the max is final).
    pub(crate) fn observe_queue(&self, peak_slots: usize, peak_bytes: usize) {
        let mut core = self.lock();
        core.stats.queue_peak_slots = core.stats.queue_peak_slots.max(peak_slots);
        core.stats.queue_peak_bytes = core.stats.queue_peak_bytes.max(peak_bytes);
    }

    /// Whether the pipeline should stop cooperating: a terminal is
    /// already set, or cancellation fired.
    pub(crate) fn is_stopping(&self) -> bool {
        self.cancel.is_cancelled() || self.outcome.get().is_some()
    }

    /// The cancel token (the handle fires it; the threads poll it).
    pub(crate) fn cancel_token(&self) -> &CancelToken {
        &self.cancel
    }

    /// Take the oldest undelivered event, if any (the handle's
    /// non-blocking poll).
    pub(crate) fn try_recv_event(&self) -> Option<JobEvent> {
        self.hub.try_recv()
    }

    /// Events dropped past the hub's capacity.
    pub(crate) fn dropped_events(&self) -> u64 {
        self.hub.dropped()
    }

    /// Emit a progress snapshot (the worker's cadence call).
    pub(crate) fn emit_progress(&self) {
        let snapshot = self.snapshot();
        self.hub.emit(JobEvent::Progress(snapshot));
    }

    /// The terminal protocol: emit one final progress, set the state,
    /// store the outcome — **first-set-wins** (a racing Failed and
    /// Cancelled resolve to whichever stage actually finished first;
    /// the loser exits without a word). A duplicate final progress
    /// from the loser is harmless: progress is best-effort by
    /// contract, terminals are not.
    pub(crate) fn terminate(&self, outcome: JobOutcome) {
        if self.outcome.get().is_none() {
            self.hub.emit(JobEvent::Progress(self.snapshot()));
        }
        if self.outcome.set(outcome).is_ok() {
            let state = match self.outcome.get() {
                Some(JobOutcome::Completed(_)) => JobState::Completed,
                Some(JobOutcome::Failed(_)) => JobState::Failed,
                Some(JobOutcome::Cancelled(_)) => JobState::Cancelled,
                None => JobState::Failed,
            };
            self.lock().state = state;
            self.settled.notify_all();
        }
    }

    /// Poll the state (cheap; no join).
    pub(crate) fn state(&self) -> JobState {
        self.lock().state
    }

    /// The durable terminal, if one has landed (readable forever —
    /// `wait` replays it).
    pub(crate) fn outcome(&self) -> Option<&JobOutcome> {
        self.outcome.get()
    }

    /// Wait until a terminal lands (bounded by the pipeline joining;
    /// used by `wait`'s inner protocol, not by the hot path).
    pub(crate) fn wait_settled(&self) {
        let mut core = self.lock();
        while self.outcome.get().is_none() {
            core = self
                .settled
                .wait(core)
                .unwrap_or_else(|p| p.into_inner());
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, JobCore> {
        self.core.lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// The host's handle to a submitted job (module docs for the full
/// lifecycle story). Cheap to hold and `Send`; `Clone` is deliberate
/// — every clone sees the same shared cell, and `wait` runs its join
/// protocol once (later callers get the stored outcome).
pub struct JobHandle {
    shared: Arc<JobShared>,
    cancel: CancelToken,
    threads: Mutex<Option<crate::runner::PipelineThreads>>,
}

impl JobHandle {
    pub(crate) fn new(
        shared: Arc<JobShared>,
        cancel: CancelToken,
        threads: crate::runner::PipelineThreads,
    ) -> JobHandle {
        JobHandle {
            shared,
            cancel,
            threads: Mutex::new(Some(threads)),
        }
    }

    /// Request a prompt cooperative stop: fires the token and wakes
    /// every parked stage-queue waiter. The pipeline honors it at
    /// stage boundaries (before each demux event, between packets,
    /// inside queue parks) and still finalizes its sink.
    pub fn cancel(&self) {
        self.cancel.cancel();
        self.shared.cancel_token().cancel();
        if let Some(threads) = self.threads.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
            threads.interrupt();
        }
    }

    /// The lifecycle state (module docs' machine).
    pub fn state(&self) -> JobState {
        self.shared.state()
    }

    /// The current accounting snapshot.
    pub fn stats(&self) -> JobStats {
        self.shared.snapshot()
    }

    /// The next undelivered event, if any (non-blocking).
    pub fn try_next_event(&self) -> Option<JobEvent> {
        self.shared.try_recv_event()
    }

    /// Events dropped past the hub's capacity (the honesty counter).
    pub fn dropped_events(&self) -> u64 {
        self.shared.dropped_events()
    }

    /// Wait for the pipeline to finish and read the terminal outcome.
    ///
    /// Joins both pipeline threads (so the sink is finalized before
    /// this answers), maps a host-stage panic to the typed
    /// `transcode.job.worker-panic` failure, and is idempotent:
    /// later calls replay the same outcome (the terminal is shared
    /// state, not a consumed value).
    pub fn wait(&self) -> &JobOutcome {
        // First caller joins; everyone else waits on the settled cell.
        let first = self
            .threads
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        if let Some(threads) = first {
            if threads.join() {
                // A host stage panicked past our catch layer without
                // a terminal (belt over braces): fail typed.
                self.shared
                    .terminate(JobOutcome::Failed(crate::error::worker_panic()));
            }
        } else {
            self.shared.wait_settled();
        }
        if self.shared.outcome().is_none() {
            // Both threads joined with no terminal — by protocol
            // impossible; the honest typed answer, never a hang.
            self.shared
                .terminate(JobOutcome::Failed(crate::error::worker_panic()));
        }
        match self.shared.outcome() {
            Some(outcome) => outcome,
            None => unreachable!("terminate() above settled the outcome"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hub_drops_and_counts_past_capacity_never_growing() {
        let hub = JobEventHub::new(4);
        for i in 0..10u64 {
            hub.emit(JobEvent::Progress(JobStats {
                bytes_written: i,
                ..JobStats::default()
            }));
        }
        // Capacity kept, the rest counted.
        let mut seen = 0;
        while hub.try_recv().is_some() {
            seen += 1;
        }
        assert_eq!(seen, 4, "the queue never grows past capacity");
        assert_eq!(hub.dropped(), 6, "every dropped event is counted");
        // And the kept ones are the OLDEST four (FIFO, not newest).
        // (Re-emit and re-check order.)
        for i in 0..3u64 {
            hub.emit(JobEvent::Progress(JobStats {
                bytes_written: 100 + i,
                ..JobStats::default()
            }));
        }
        match hub.try_recv() {
            Some(JobEvent::Progress(stats)) => assert_eq!(stats.bytes_written, 100),
            other => unreachable!("oldest-first, got {other:?}"),
        }
        assert_eq!(hub.dropped(), 6, "in-capacity emits never count as drops");
    }

    #[test]
    fn stats_absorb_saturates_and_maxes_the_high_water_marks() {
        let mut a = JobStats {
            bytes_written: u64::MAX - 1,
            queue_peak_bytes: 1024,
            ..JobStats::default()
        };
        a.absorb(&JobStats {
            bytes_written: 10,
            queue_peak_bytes: 512,
            queue_peak_slots: 3,
            ..JobStats::default()
        });
        assert_eq!(a.bytes_written, u64::MAX, "saturate, never wrap");
        assert_eq!(a.queue_peak_bytes, 1024, "high-water marks take the max");
        assert_eq!(a.queue_peak_slots, 3);
    }

    #[test]
    fn terminate_is_first_set_wins_and_emits_a_final_progress() {
        let shared = JobShared::new(8, CancelToken::new());
        shared.set_running();
        assert_eq!(shared.state(), JobState::Running);
        shared.update_stats(&JobStats {
            bytes_written: 42,
            ..JobStats::default()
        });
        shared.terminate(JobOutcome::Cancelled(shared.snapshot()));
        // First-set-wins: a racing second terminal changes nothing.
        shared.terminate(JobOutcome::Failed(crate::error::worker_panic()));
        assert_eq!(shared.state(), JobState::Cancelled);
        // The final progress was emitted before the terminal.
        let mut last = None;
        while let Some(event) = shared.hub.try_recv() {
            last = Some(event);
        }
        match last {
            Some(JobEvent::Progress(stats)) => assert_eq!(stats.bytes_written, 42),
            other => unreachable!("final progress, got {other:?}"),
        }
        // And Running was the first event of all (lifecycle order).
        // (The hub above was drained; re-run the sequence.)
        let shared = JobShared::new(8, CancelToken::new());
        shared.set_running();
        shared.terminate(JobOutcome::Completed(JobStats::default()));
        assert!(matches!(shared.hub.try_recv(), Some(JobEvent::Running)));
    }

    #[test]
    fn state_starts_queued_and_stopping_needs_a_reason() {
        let cancel = CancelToken::new();
        let shared = JobShared::new(8, cancel.clone());
        assert_eq!(shared.state(), JobState::Queued);
        assert!(!shared.is_stopping());
        cancel.cancel();
        assert!(shared.is_stopping(), "the token is a stop reason");
        let other = JobShared::new(8, CancelToken::new());
        other.terminate(JobOutcome::Completed(JobStats::default()));
        assert!(other.is_stopping(), "a terminal is a stop reason");
    }
}
