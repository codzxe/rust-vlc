//! The bounded queue core: [`BoundedQueue`] and its typed signals
//! (doc 03 §4.4).
//!
//! Threading is doc 03 §4.4(c) verbatim: a `Mutex<VecDeque>` guarded
//! by two condvars (`space` for parked producers, `items` for parked
//! consumers) — MPMC-capable, and the lock is held only for deque
//! pushes/pops, never across I/O, decode or user code. Every critical
//! section is infallible by construction ([`Weigh`] runs once per push
//! *outside* the lock; in-lock code is deque pushes/pops, saturating
//! field arithmetic and condvar notifications), which is what makes the
//! poison-recovery helper below sound.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex, PoisonError};
use std::time::Instant;

use media_types::ids::GenerationId;
use media_types::time::Nanoseconds;

use crate::limits::{LimitAxis, QueueLimits, QueueUsage, Watermark, WatermarkThresholds};
use crate::stamped::{Stamped, Weigh};

/// Recover a mutex guard (or a `(guard, timeout)` pair) from poisoning.
///
/// Sound here because critical sections are infallible by construction
/// (see the module docs): a panic can never leave the queue state
/// half-updated, so the *only* way this mutex gets poisoned is a panic
/// in code that does not touch queue state at all.
fn unpoison<G>(result: Result<G, PoisonError<G>>) -> G {
    match result {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// The outcome of a non-blocking push — the typed backpressure signal.
///
/// Every non-`Pushed` variant hands the item back untouched: the queue
/// never silently swallows data, full or not (doc 03 Q3: fresh data is
/// never dropped).
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "the outcome carries the item back on every non-Pushed path; ignoring it can lose data"]
pub enum PushOutcome<T> {
    /// Accepted; resident usage grew within the caps.
    Pushed,
    /// Bounded: at least one cap would be exceeded. The producer parks
    /// (via [`BoundedQueue::push_wait`]), retries, or reacts — its
    /// choice; the item is returned untouched.
    Full {
        /// The untouched item, handed back.
        item: Stamped<T>,
        /// Which cap refused the push.
        binding: LimitAxis,
    },
    /// The queue is closed for production (teardown). The item is
    /// returned untouched; the producer must stop, not retry.
    Closed {
        /// The untouched item, handed back.
        item: Stamped<T>,
    },
    /// The item's generation is behind the queue's published
    /// generation — dropped at the boundary (doc 03 §5 checking point
    /// 3, invariant Q2), counted in [`QueueStats::stale_rejected`].
    /// The item is returned untouched; the producer must stop, not
    /// retry (doc 03 §4.4(d)).
    Stale {
        /// The untouched item, handed back.
        item: Stamped<T>,
        /// The queue's published generation the item fell behind.
        current: GenerationId,
    },
}

/// Why a parking push gave up (doc 03 §4.4(d)): the producer must stop,
/// not retry — except for `Deadline`, where retrying with a fresh
/// deadline is the caller's policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushAborted<T> {
    /// The queue closed while the producer was parking (or before it
    /// started).
    Closed {
        /// The untouched item, handed back.
        item: Stamped<T>,
    },
    /// The deadline expired with the queue still full — the bounded-park
    /// guarantee. The item is returned untouched.
    Deadline {
        /// The untouched item, handed back.
        item: Stamped<T>,
    },
    /// The queue's published generation moved past the item's while
    /// the producer was parking (a flush happened) — the exact race
    /// doc 03 §4.4(f) requires to wake within a bounded number of
    /// wakeups. The item is returned untouched; the producer must stop,
    /// not retry.
    Stale {
        /// The untouched item, handed back.
        item: Stamped<T>,
        /// The queue's published generation the item fell behind.
        current: GenerationId,
    },
}

/// The outcome of a pop (doc 03 §4.4(b): `Err(Empty)` means "would
/// block" — callers poll or await).
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "the outcome discriminates real data from empty/closed; ignoring it can drop data"]
pub enum PopOutcome<T> {
    /// A generation boundary was crossed since the consumer's last
    /// observation: a flush dropped older-generation items wholesale.
    /// The typed discontinuity signal — the drop is *never* silent
    /// (card 2.B1): the consumer sees exactly how much went away and
    /// which generation now rules.
    Discontinuity(Discontinuity),
    /// An item left the queue, FIFO within its generation.
    Item(Stamped<T>),
    /// Nothing resident — a polling consumer would block; a parking
    /// one uses [`BoundedQueue::pop_wait`].
    Empty,
    /// The queue is closed and fully drained.
    Closed,
}

/// The typed wholesale-drop signal (doc 03 §4.4 `flush` + Q2).
///
/// Produced by [`BoundedQueue::flush`] — synchronously for the flushing
/// caller, and once for the consuming side via
/// [`PopOutcome::Discontinuity`] so a consumer that was not polling at
/// flush time still observes the boundary before any new-generation
/// item. Consecutive flushes the consumer has not yet observed merge —
/// counts accumulate, never vanish.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "carries the accounting of what was dropped; ignoring it makes the drop silent"]
pub struct Discontinuity {
    /// The generation now published; items stamped older than this can
    /// neither be popped nor pushed (Q2).
    pub new_generation: GenerationId,
    /// How many resident items the flush dropped.
    pub dropped: u64,
    /// Their combined byte weight.
    pub dropped_bytes: usize,
    /// Their combined media duration.
    pub dropped_duration: Nanoseconds,
}

/// Lifetime counters for observability (doc 03 §5's `stale_dropped`
/// metric is [`QueueStats::stale_rejected`] + [`QueueStats::flushed`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QueueStats {
    /// Items accepted into the queue.
    pub pushed: u64,
    /// Items handed to consumers.
    pub popped: u64,
    /// Pushes refused because a cap was full (try-push `Full` signals
    /// and push-wait deadline expiries).
    pub full_rejections: u64,
    /// Pushes dropped at the boundary for a stale generation (doc 03
    /// §5 checking point 3).
    pub stale_rejected: u64,
    /// Resident items dropped wholesale by generation flushes (doc 03
    /// §5 "Interaction with bounded queues").
    pub flushed: u64,
}

/// One resident item with its weight, cached at push time so pops never
/// re-run user code under the lock.
struct Slot<T> {
    item: Stamped<T>,
    size: usize,
    duration: Nanoseconds,
}

/// The guarded queue state. All fields are private to this module and
/// every mutation happens under the queue's mutex.
struct State<T> {
    items: VecDeque<Slot<T>>,
    resident_bytes: usize,
    resident_duration: Nanoseconds,
    closed: bool,
    generation: GenerationId,
    /// The not-yet-observed wholesale drop of the last flush(es):
    /// surfaced (once) by the next pop, before any item.
    pending_gap: Option<Discontinuity>,
    limits: QueueLimits,
    marks: WatermarkThresholds,
    stats: QueueStats,
}

impl<T> State<T> {
    fn accept(&mut self, item: Stamped<T>, size: usize, duration: Nanoseconds) {
        self.items.push_back(Slot {
            item,
            size,
            duration,
        });
        self.resident_bytes = self.resident_bytes.saturating_add(size);
        self.resident_duration = Nanoseconds(self.resident_duration.0.saturating_add(duration.0));
        self.stats.pushed = self.stats.pushed.saturating_add(1);
    }

    fn take_front(&mut self) -> Option<Stamped<T>> {
        self.items.pop_front().map(|slot| {
            self.resident_bytes = self.resident_bytes.saturating_sub(slot.size);
            self.resident_duration =
                Nanoseconds(self.resident_duration.0.saturating_sub(slot.duration.0));
            self.stats.popped = self.stats.popped.saturating_add(1);
            slot.item
        })
    }

    fn usage(&self) -> QueueUsage {
        QueueUsage {
            packets: self.items.len(),
            bytes: self.resident_bytes,
            duration: self.resident_duration,
        }
    }
}

/// A bounded, generation-aware queue (doc 03 §4.4).
///
/// Generic over any [`Weigh`] payload — media-types `Packet` between
/// demux and decode, frames between decode and the engines (the
/// [`PacketQueue`](crate::PacketQueue) alias pins the packet flavor).
/// See the crate docs for the four invariants (Q1–Q4) and the threading
/// contract.
pub struct BoundedQueue<T> {
    state: Mutex<State<T>>,
    space: Condvar,
    items: Condvar,
}

impl<T> BoundedQueue<T> {
    /// A queue with the given caps and watermark thresholds.
    ///
    /// A fresh queue publishes [`GenerationId::FIRST`]; the session
    /// flushes it to the live generation before any producer starts
    /// (doc 03 §5 checking point 4).
    #[must_use]
    pub fn new(limits: QueueLimits, marks: WatermarkThresholds) -> BoundedQueue<T> {
        BoundedQueue {
            state: Mutex::new(State {
                items: VecDeque::with_capacity(limits.max_packets.get().min(1024)),
                resident_bytes: 0,
                resident_duration: Nanoseconds(0),
                closed: false,
                generation: GenerationId::FIRST,
                pending_gap: None,
                limits,
                marks,
                stats: QueueStats::default(),
            }),
            space: Condvar::new(),
            items: Condvar::new(),
        }
    }

    /// A queue with default watermark thresholds derived from its caps
    /// ([`WatermarkThresholds::from_limits`]).
    #[must_use]
    pub fn with_default_watermarks(limits: QueueLimits) -> BoundedQueue<T> {
        BoundedQueue::new(limits, WatermarkThresholds::from_limits(&limits))
    }

    /// The caps this queue was built with.
    #[must_use]
    pub fn limits(&self) -> QueueLimits {
        unpoison(self.state.lock()).limits
    }

    /// The currently published generation.
    #[must_use]
    pub fn generation(&self) -> GenerationId {
        unpoison(self.state.lock()).generation
    }

    /// Whether the queue is closed for production.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        unpoison(self.state.lock()).closed
    }

    /// Current resident usage on all three axes.
    #[must_use]
    pub fn usage(&self) -> QueueUsage {
        unpoison(self.state.lock()).usage()
    }

    /// The watermark state (Q4: the single input to `Buffering`
    /// transitions — this crate reports, the playback engine decides).
    #[must_use]
    pub fn watermark(&self) -> Watermark {
        let state = unpoison(self.state.lock());
        Watermark::evaluate(state.usage(), &state.marks)
    }

    /// Lifetime counters (doc 03 §5 metrics).
    #[must_use]
    pub fn stats(&self) -> QueueStats {
        unpoison(self.state.lock()).stats
    }

    /// Close the queue for production (teardown).
    ///
    /// Producers are refused immediately ([`PushOutcome::Closed`] /
    /// [`PushAborted::Closed`], item in hand); parked ones wake.
    /// Consumers drain what is still resident and then observe
    /// [`PopOutcome::Closed`] — closing never destroys resident data
    /// without a signal.
    pub fn close(&self) {
        let mut state = unpoison(self.state.lock());
        state.closed = true;
        self.space.notify_all();
        self.items.notify_all();
    }

    /// Drop every resident item wholesale and publish `new_generation`
    /// (doc 03 §4.4(b) `flush`, invariant Q2).
    ///
    /// Called by the session before any new producer starts (doc 03
    /// §5 checking point 4). After it returns, no item stamped older
    /// than `new_generation` can be popped, and a later push of such
    /// an item is dropped at the boundary with a typed
    /// [`PushOutcome::Stale`] — parked producers wake with
    /// [`PushAborted::Stale`] within one condvar notification (doc 03
    /// §4.4(f)). Consumers observe the drop via
    /// [`PopOutcome::Discontinuity`] before any new item: never silent.
    ///
    /// The published generation never regresses: a `new_generation`
    /// at or below the current one still drops the residents (the
    /// caller asked for a flush) but leaves the published generation
    /// at its high-water mark.
    ///
    /// Returns the synchronous report of what this flush dropped; the
    /// consumer-side marker merges with any not-yet-observed one.
    pub fn flush(&self, new_generation: GenerationId) -> Discontinuity {
        let mut state = unpoison(self.state.lock());
        let dropped = state.items.len() as u64;
        let dropped_bytes = state.resident_bytes;
        let dropped_duration = state.resident_duration;
        let published = if new_generation > state.generation {
            new_generation
        } else {
            state.generation
        };
        let gap = Discontinuity {
            new_generation: published,
            dropped,
            dropped_bytes,
            dropped_duration,
        };
        state.items.clear();
        state.resident_bytes = 0;
        state.resident_duration = Nanoseconds(0);
        state.stats.flushed = state.stats.flushed.saturating_add(dropped);
        state.generation = published;
        state.pending_gap = match state.pending_gap.take() {
            Some(previous) => Some(Discontinuity {
                new_generation: published,
                dropped: previous.dropped.saturating_add(dropped),
                dropped_bytes: previous.dropped_bytes.saturating_add(dropped_bytes),
                dropped_duration: Nanoseconds(
                    previous
                        .dropped_duration
                        .0
                        .saturating_add(dropped_duration.0),
                ),
            }),
            None => Some(gap),
        };
        self.space.notify_all();
        self.items.notify_all();
        gap
    }
}

impl<T: Weigh> BoundedQueue<T> {
    /// Non-blocking push — the typed backpressure signal.
    ///
    /// Never parks, never drops: every non-`Pushed` outcome returns the
    /// item untouched. The queue is not modified unless the outcome is
    /// [`PushOutcome::Pushed`].
    pub fn try_push(&self, item: Stamped<T>) -> PushOutcome<T> {
        // Weigh outside the lock (module docs: infallible critical
        // sections).
        let size = item.value.size_bytes();
        let duration = item.value.media_duration();
        let mut state = unpoison(self.state.lock());
        if state.closed {
            return PushOutcome::Closed { item };
        }
        // Generation boundary check (doc 03 §5 checking point 3, Q2):
        // stale items are dropped here, counted, and handed back.
        if item.generation < state.generation {
            state.stats.stale_rejected = state.stats.stale_rejected.saturating_add(1);
            return PushOutcome::Stale {
                item,
                current: state.generation,
            };
        }
        match state.limits.binding_axis(state.usage(), 1, size, duration) {
            None => {
                state.accept(item, size, duration);
                self.items.notify_one();
                PushOutcome::Pushed
            }
            Some(binding) => {
                state.stats.full_rejections = state.stats.full_rejections.saturating_add(1);
                PushOutcome::Full { item, binding }
            }
        }
    }

    /// Parking push (doc 03 §4.4(b)): parks the producer while full.
    ///
    /// Wakes on: space, generation movement ([`BoundedQueue::flush`]),
    /// queue close, or `deadline` — never spins (Q3), never blocks
    /// unboundedly without the caller's consent (`deadline: None` *is*
    /// that consent: the demux thread parks until the pipeline moves
    /// it). On every wake the producer's own position is re-checked
    /// before parking again — a stale stamp aborts, exactly the
    /// `gen_check` recheck of the doc's sketch.
    ///
    /// # Errors
    /// [`PushAborted`] — always with the item handed back — when the
    /// queue closes, the generation moves past the item's stamp, or
    /// the deadline expires while full.
    pub fn push_wait(
        &self,
        item: Stamped<T>,
        deadline: Option<Instant>,
    ) -> Result<(), PushAborted<T>> {
        let size = item.value.size_bytes();
        let duration = item.value.media_duration();
        let stamped_generation = item.generation;
        // Park while the queue is full AND open AND the item is not
        // stale — a close or a generation movement must wake the
        // producer immediately (doc 03 §4.4(f)).
        let parkable = |s: &mut State<T>| -> bool {
            !s.closed
                && stamped_generation >= s.generation
                && s.limits
                    .binding_axis(s.usage(), 1, size, duration)
                    .is_some()
        };
        let mut state = unpoison(self.state.lock());
        loop {
            if state.closed {
                return Err(PushAborted::Closed { item });
            }
            // Recheck the item's own generation after every wake
            // (doc 03 §4.4(b)).
            if item.generation < state.generation {
                state.stats.stale_rejected = state.stats.stale_rejected.saturating_add(1);
                return Err(PushAborted::Stale {
                    item,
                    current: state.generation,
                });
            }
            if state
                .limits
                .binding_axis(state.usage(), 1, size, duration)
                .is_none()
            {
                state.accept(item, size, duration);
                self.items.notify_one();
                return Ok(());
            }
            match deadline {
                None => {
                    state = unpoison(self.space.wait_while(state, parkable));
                }
                Some(at) => {
                    let now = Instant::now();
                    if now >= at {
                        state.stats.full_rejections = state.stats.full_rejections.saturating_add(1);
                        return Err(PushAborted::Deadline { item });
                    }
                    let (next, _) = unpoison(self.space.wait_timeout_while(
                        state,
                        at.saturating_duration_since(now),
                        parkable,
                    ));
                    state = next;
                }
            }
        }
    }

    /// Non-blocking pop (doc 03 §4.4(b): `Err(Empty)` means "would
    /// block" — callers poll or await).
    ///
    /// Infallible by design: the [`PopOutcome`] carries every case as
    /// data, so there is nothing to propagate. A pending wholesale
    /// drop ([`BoundedQueue::flush`]) is surfaced *before* any item.
    pub fn try_pop(&self) -> PopOutcome<T> {
        let mut state = unpoison(self.state.lock());
        if let Some(gap) = state.pending_gap.take() {
            return PopOutcome::Discontinuity(gap);
        }
        match state.take_front() {
            Some(item) => {
                self.space.notify_one();
                PopOutcome::Item(item)
            }
            None if state.closed => PopOutcome::Closed,
            None => PopOutcome::Empty,
        }
    }

    /// Parking pop: sleeps until an item is resident, a discontinuity
    /// is pending, the queue closes, or `deadline` expires (then
    /// [`PopOutcome::Empty`]).
    pub fn pop_wait(&self, deadline: Option<Instant>) -> PopOutcome<T> {
        let parkable = |s: &mut State<T>| -> bool {
            s.items.is_empty() && s.pending_gap.is_none() && !s.closed
        };
        let mut state = unpoison(self.state.lock());
        loop {
            if let Some(gap) = state.pending_gap.take() {
                return PopOutcome::Discontinuity(gap);
            }
            if let Some(item) = state.take_front() {
                self.space.notify_one();
                return PopOutcome::Item(item);
            }
            if state.closed {
                return PopOutcome::Closed;
            }
            match deadline {
                None => {
                    state = unpoison(self.items.wait_while(state, parkable));
                }
                Some(at) => {
                    let now = Instant::now();
                    if now >= at {
                        return PopOutcome::Empty;
                    }
                    let (next, _) = unpoison(self.items.wait_timeout_while(
                        state,
                        at.saturating_duration_since(now),
                        parkable,
                    ));
                    state = next;
                }
            }
        }
    }
}
