//! The bounded demux→worker stage queue (card 7.E1) — the streaming
//! stack's bounded-backpressure pattern, applied to the transcode
//! pipeline.
//!
//! # Pattern provenance (the de-risking the card names)
//!
//! * **Mutex + two condvars** — the `packet-queue` crate's own
//!   shape: `space` parks producers, `items` parks consumers; the
//!   lock is held only for deque push/pop and saturating field
//!   arithmetic, never across demux/decode/sink work.
//! * **Two-axis capacity** — slots AND payload bytes (the
//!   `QueueLimits` doctrine): a full-bytes queue with one slot
//!   refuses the next packet exactly like a full-slots one.
//! * **Interrupt, not just close** — cancellation fires the shared
//!   [`CancelToken`] and calls [`PacketStageQueue::interrupt`],
//!   which notifies both condvars so every parked waiter wakes and
//!   re-checks (the reconnect executor's wake-on-generation-move
//!   doctrine: no waiter may miss the signal).
//! * **Saturating accounting** — every counter is
//!   [`saturating_add`]/[`saturating_sub`] (the explicit overflow
//!   policy: saturate, never wrap, never grow).
//!
//! # The one honest overflow
//!
//! A packet whose payload alone exceeds the configured byte axis can
//! never be accepted — parking would be a deadlock. [`PushWait`]
//! returns it untouched and the pump fails the job with the
//! `transcode.limits.over-budget` constructor (bounded memory is a
//! contract; a hostile demuxer's oversized packet is refused, not
//! buffered).
//!
//! # Bounded-memory proof (the test below)
//!
//! Fill the queue with a parked producer and a stopped consumer:
//! resident slots never exceed the slot axis, resident bytes never
//! exceed the byte axis, and the producer's un-parked pushes happen
//! only after pops — backpressure, not growth. The high-water marks
//! ([`QueueStats::peak_bytes`]) are observable precisely so this
//! stays a test, not a hope.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex, PoisonError};

use media_types::Packet;

use crate::cancel::CancelToken;
use crate::error::limit_over_budget;

/// Recover a mutex guard from poisoning.
///
/// Sound for the same reason `packet-queue`'s helper is: the
/// critical sections are infallible by construction (deque
/// push/pop, saturating arithmetic, condvar notification), so a
/// panic can never leave the queue half-updated.
fn unpoison<G>(result: Result<G, PoisonError<G>>) -> G {
    match result {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// The outcome of a parking push. Every non-`Pushed` variant means
/// the queue is done **as a producer**: the packet was not taken.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PushWait {
    /// The packet is resident; usage grew within both axes.
    Pushed,
    /// The queue closed for production (teardown or the consumer's
    /// exit). The packet stays with the caller.
    Closed,
    /// Cancellation fired; the packet stays with the caller.
    Interrupted,
    /// The packet alone exceeds the queue's byte axis — it can never
    /// be accepted (module docs: the one honest overflow). The
    /// packet stays with the caller.
    TooLarge,
}

/// The outcome of a parking pop.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PopWait {
    /// A packet left the queue, FIFO.
    Item(Packet),
    /// The queue closed and is fully drained.
    Closed,
    /// Cancellation fired.
    Interrupted,
}

/// Lifetime observability (the bounded-memory proof's witnesses).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct QueueStats {
    /// Packets accepted.
    pub pushed: u64,
    /// Packets handed to the consumer.
    pub popped: u64,
    /// Producer park episodes (spurious wakeups re-check without a
    /// second count).
    pub producer_parks: u64,
    /// Consumer park episodes.
    pub consumer_parks: u64,
    /// High-water mark of resident payload bytes.
    pub peak_bytes: usize,
    /// High-water mark of resident slots.
    pub peak_slots: usize,
}

/// The guarded state. Every mutation happens under the queue's
/// mutex; every counter mutation is saturating.
struct State {
    slots: VecDeque<Packet>,
    resident_bytes: usize,
    closed: bool,
    max_slots: usize,
    max_bytes: usize,
    stats: QueueStats,
}

impl State {
    fn can_accept(&self, bytes: usize) -> bool {
        self.slots.len() < self.max_slots
            && self.resident_bytes.saturating_add(bytes) <= self.max_bytes
    }

    fn accept(&mut self, packet: Packet, bytes: usize) {
        self.slots.push_back(packet);
        self.resident_bytes = self.resident_bytes.saturating_add(bytes);
        self.stats.pushed = self.stats.pushed.saturating_add(1);
        self.stats.peak_bytes = self.stats.peak_bytes.max(self.resident_bytes);
        self.stats.peak_slots = self.stats.peak_slots.max(self.slots.len());
    }

    fn take(&mut self) -> Option<Packet> {
        self.slots.pop_front().map(|packet| {
            self.resident_bytes = self.resident_bytes.saturating_sub(packet.data.len());
            self.stats.popped = self.stats.popped.saturating_add(1);
            packet
        })
    }
}

/// The bounded, single-producer single-consumer stage queue between
/// the demux pump and the encode/copy worker.
///
/// Capacity is two-axis (slots and payload bytes); both waiters
/// park, never spin; both wake on close and on cancellation. Not
/// `Clone` — the runner hands out `Arc`s of one queue per job.
pub(crate) struct PacketStageQueue {
    state: Mutex<State>,
    space: Condvar,
    items: Condvar,
    cancel: CancelToken,
    max_bytes: usize,
}

impl PacketStageQueue {
    /// A queue with the given axes and the cancellation token whose
    /// firing must wake every waiter.
    pub(crate) fn new(max_slots: usize, max_bytes: usize, cancel: CancelToken) -> PacketStageQueue {
        PacketStageQueue {
            state: Mutex::new(State {
                slots: VecDeque::with_capacity(max_slots.min(64)),
                resident_bytes: 0,
                closed: false,
                max_slots,
                max_bytes,
                stats: QueueStats::default(),
            }),
            space: Condvar::new(),
            items: Condvar::new(),
            cancel,
            max_bytes,
        }
    }

    /// Fire cancellation for this queue's token and wake every
    /// parked waiter (both sides) — the prompt-stop half of
    /// [`CancelToken`].
    pub(crate) fn interrupt(&self) {
        self.cancel.cancel();
        self.space.notify_all();
        self.items.notify_all();
    }

    /// Close for production (teardown). Producers are refused;
    /// consumers drain residents and then see [`PopWait::Closed`].
    /// Idempotent.
    pub(crate) fn close(&self) {
        let mut state = unpoison(self.state.lock());
        state.closed = true;
        self.space.notify_all();
        self.items.notify_all();
    }

    /// Resident packet count (test/observability).
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        unpoison(self.state.lock()).slots.len()
    }

    /// Whether no packet is resident.
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Lifetime counters and high-water marks.
    pub(crate) fn stats(&self) -> QueueStats {
        unpoison(self.state.lock()).stats
    }

    /// Parking push: parks while both axes are full, wakes on space,
    /// close, or cancellation.
    pub(crate) fn push_wait(&self, packet: Packet) -> PushWait {
        let bytes = packet.data.len();
        if bytes > self.max_bytes {
            // Module docs: the one honest overflow — parking would be
            // a deadlock, so refuse before waiting.
            return PushWait::TooLarge;
        }
        let mut state = unpoison(self.state.lock());
        loop {
            if state.closed {
                return PushWait::Closed;
            }
            if self.cancel.is_cancelled() {
                return PushWait::Interrupted;
            }
            if state.can_accept(bytes) {
                state.accept(packet, bytes);
                self.items.notify_one();
                return PushWait::Pushed;
            }
            state.stats.producer_parks = state.stats.producer_parks.saturating_add(1);
            // The predicate re-checks the token live on every wake:
            // interrupt() fires the token FIRST, then notifies, so a
            // racing wake can never observe the notification without
            // the flag.
            state = unpoison(self.space.wait_while(state, |s| {
                !s.closed && !self.cancel.is_cancelled() && !s.can_accept(bytes)
            }));
        }
    }

    /// Parking pop: parks while empty, wakes on items, close, or
    /// cancellation. Drains residents before reporting [`PopWait::Closed`].
    pub(crate) fn pop_wait(&self) -> PopWait {
        let mut state = unpoison(self.state.lock());
        loop {
            if let Some(packet) = state.take() {
                self.space.notify_one();
                return PopWait::Item(packet);
            }
            if state.closed {
                return PopWait::Closed;
            }
            if self.cancel.is_cancelled() {
                return PopWait::Interrupted;
            }
            state.stats.consumer_parks = state.stats.consumer_parks.saturating_add(1);
            state = unpoison(self.items.wait_while(state, |s| {
                s.slots.is_empty() && !s.closed && !self.cancel.is_cancelled()
            }));
        }
    }

    /// Map a `TooLarge` push to the typed job failure (module docs).
    pub(crate) fn too_large_error(attempted: usize, budget: usize) -> media_types::EngineError {
        limit_over_budget(attempted, budget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::ids::{GenerationId, TrackId};
    use media_types::packet::SharedBytes;
    use media_types::time::Stamp;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn packet(len: usize, salt: u8) -> Packet {
        Packet {
            track: TrackId::new(0),
            pts: Stamp::Missing,
            dts: Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(vec![salt; len]),
            keyframe: true,
            discontinuity: false,
            generation: GenerationId::FIRST,
        }
    }

    /// Poll `probe` until it answers `true`, or fail after a bounded
    /// wait (deterministic tests on scheduler-dependent facts).
    fn eventually<F: FnMut() -> bool>(mut probe: F, what: &str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if probe() {
                return;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        unreachable!("timed out waiting for {what}");
    }

    #[test]
    fn push_pop_roundtrip_fifo() {
        let queue = PacketStageQueue::new(4, 1024, CancelToken::new());
        assert_eq!(queue.push_wait(packet(3, 0xAA)), PushWait::Pushed);
        assert_eq!(queue.push_wait(packet(5, 0xBB)), PushWait::Pushed);
        assert_eq!(queue.len(), 2);
        match queue.pop_wait() {
            PopWait::Item(p) => assert_eq!(p.data.as_slice(), &[0xAA; 3]),
            other => unreachable!("first in first out, got {other:?}"),
        }
        match queue.pop_wait() {
            PopWait::Item(p) => assert_eq!(p.data.as_slice(), &[0xBB; 5]),
            other => unreachable!("second in second out, got {other:?}"),
        }
        assert!(queue.is_empty());
        let stats = queue.stats();
        assert_eq!(stats.pushed, 2);
        assert_eq!(stats.popped, 2);
        assert_eq!(stats.peak_bytes, 8);
        assert_eq!(stats.peak_slots, 2);
    }

    #[test]
    fn closed_queue_refuses_pushes_keeps_residents() {
        let queue = PacketStageQueue::new(2, 1024, CancelToken::new());
        assert_eq!(queue.push_wait(packet(1, 1)), PushWait::Pushed);
        assert_eq!(queue.push_wait(packet(1, 2)), PushWait::Pushed);
        queue.close();
        assert_eq!(queue.push_wait(packet(1, 3)), PushWait::Closed);
        assert_eq!(queue.len(), 2, "a closed queue keeps its residents");
    }

    #[test]
    fn byte_axis_refuses_the_oversized_packet() {
        let queue = PacketStageQueue::new(8, 16, CancelToken::new());
        assert_eq!(queue.push_wait(packet(16, 1)), PushWait::Pushed);
        // A packet larger than the whole byte axis can never fit.
        assert_eq!(queue.push_wait(packet(17, 2)), PushWait::TooLarge);
        assert_eq!(queue.len(), 1);
        // And the typed mapping names the contract.
        let err = PacketStageQueue::too_large_error(17, 16);
        assert_eq!(err.context.code, "transcode.limits.over-budget");
        // A packet that fits only after a pop parks instead — the
        // axis is about residency, not the request.
        match queue.pop_wait() {
            PopWait::Item(_) => {}
            other => unreachable!("{other:?}"),
        }
        assert_eq!(queue.push_wait(packet(16, 3)), PushWait::Pushed);
    }

    /// THE bounded-queue proof (card 7.E1 AC): fill the queue, park
    /// the producer, assert backpressure — not growth.
    #[test]
    fn full_queue_parks_the_producer_backpressure_not_growth() {
        let cancel = CancelToken::new();
        let queue = Arc::new(PacketStageQueue::new(2, 64, cancel.clone()));
        let pump = Arc::clone(&queue);
        let producer = std::thread::spawn(move || {
            let mut results = Vec::new();
            for salt in 0..10u8 {
                results.push(pump.push_wait(packet(32, salt)));
            }
            results
        });
        // Wait until the queue is full AND the producer has parked.
        eventually(
            || queue.len() == 2 && queue.stats().producer_parks >= 1,
            "queue full and producer parked",
        );
        // Backpressure, not growth: the producer is parked yet the
        // bounds hold steady across a sampling window.
        for sample in 0..20 {
            assert!(queue.len() <= 2, "sample {sample}: slots grew");
            assert!(
                queue.stats().peak_bytes <= 64,
                "sample {sample}: bytes high-water exceeded the axis"
            );
            assert!(
                queue.stats().peak_slots <= 2,
                "sample {sample}: slot high-water grew"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        // Drain one: the producer un-parks exactly one push.
        match queue.pop_wait() {
            PopWait::Item(p) => assert_eq!(p.data.as_slice(), &[2; 32], "FIFO across the park"),
            other => unreachable!("{other:?}"),
        }
        eventually(
            || queue.stats().pushed >= 3,
            "producer un-parked after the pop",
        );
        // Interrupt: the parked (and future) pushes come back
        // un-taken, the queue never grows again.
        queue.interrupt();
        let results = producer
            .join()
            .unwrap_or_else(|_| unreachable!("producer must join"));
        assert_eq!(results.len(), 10);
        assert_eq!(results[0], PushWait::Pushed);
        assert_eq!(results[1], PushWait::Pushed);
        // Index 2 was the parked one: un-parked into the freed slot
        // by our pop, OR interrupted if the race went the other way —
        // both are legal, both are bounded.
        assert!(
            results[2] == PushWait::Pushed || results[2] == PushWait::Interrupted,
            "parked push resolves to pushed-or-interrupted, got {:?}",
            results[2]
        );
        for rest in results.iter().skip(3) {
            assert_eq!(*rest, PushWait::Interrupted, "no push past cancellation");
        }
        assert!(
            queue.len() <= 2,
            "final bound: the residents never exceed the slot axis"
        );
        assert!(cancel.is_cancelled(), "interrupt fired the token");
    }

    #[test]
    fn interrupt_wakes_a_parked_consumer() {
        let cancel = CancelToken::new();
        let queue = Arc::new(PacketStageQueue::new(2, 64, cancel));
        let consumer = Arc::clone(&queue);
        let waiter = std::thread::spawn(move || consumer.pop_wait());
        eventually(
            || queue.stats().consumer_parks >= 1,
            "consumer parked on the empty queue",
        );
        queue.interrupt();
        match waiter
            .join()
            .unwrap_or_else(|_| unreachable!("consumer must join"))
        {
            PopWait::Interrupted => {}
            other => unreachable!("interrupted empty queue must wake Interrupted, got {other:?}"),
        }
    }

    #[test]
    fn close_drains_residents_before_closed() {
        let queue = PacketStageQueue::new(4, 256, CancelToken::new());
        assert_eq!(queue.push_wait(packet(8, 7)), PushWait::Pushed);
        queue.close();
        match queue.pop_wait() {
            PopWait::Item(p) => assert_eq!(p.data.as_slice(), &[7; 8]),
            other => unreachable!("close never destroys residents: {other:?}"),
        }
        assert_eq!(queue.pop_wait(), PopWait::Closed);
        assert_eq!(queue.pop_wait(), PopWait::Closed, "idempotent");
        assert_eq!(queue.push_wait(packet(8, 8)), PushWait::Closed);
    }
}
