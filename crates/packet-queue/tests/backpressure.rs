//! Backpressure proofs (card 2.B1 AC-a; doc 03 §4.4 Q1/Q3, §4.4(f)).
//!
//! The acceptance criterion: a queue at capacity rejects/signals the
//! producer's push (typed `Full`, item handed back untouched); once a
//! consumer drains, the producer succeeds. The parking variant proves
//! the wake-on-space path, and the deadline variant proves the park is
//! bounded.

use std::num::NonZeroUsize;
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use media_types::ids::GenerationId;
use media_types::time::{Duration as MediaDuration, Nanoseconds};
use packet_queue::{
    BoundedQueue, LimitAxis, PopOutcome, PushAborted, PushOutcome, QueueLimits, QueueUsage,
    Stamped, Weigh,
};
use testkit::Must;

/// Minimal weighed payload: an id to track ordering, a byte weight and
/// a media duration the test controls per-axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Blob {
    id: u64,
    size: usize,
    dur_ns: u64,
}

impl Weigh for Blob {
    fn size_bytes(&self) -> usize {
        self.size
    }

    fn media_duration(&self) -> Nanoseconds {
        Nanoseconds(self.dur_ns)
    }
}

fn blob(id: u64) -> Blob {
    Blob {
        id,
        size: 1,
        dur_ns: 0,
    }
}

fn stamped(id: u64) -> Stamped<Blob> {
    Stamped::new(GenerationId::FIRST, blob(id))
}

fn limits(packets: usize, bytes: usize, dur_ns: u64) -> QueueLimits {
    QueueLimits::new(
        NonZeroUsize::new(packets).must(),
        NonZeroUsize::new(bytes).must(),
        MediaDuration::Known(Nanoseconds(dur_ns)),
    )
    .must()
}

fn join_or_propagate(handle: JoinHandle<()>) {
    if let Err(panic_payload) = handle.join() {
        std::panic::resume_unwind(panic_payload);
    }
}

/// AC(a), packet axis: at capacity the push is rejected with the typed
/// full signal and the item intact; after a drain the producer
/// succeeds; FIFO order is preserved across the cycle.
#[test]
fn ac_overflow_backpressure_proof_packets_axis() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(4, 1_000, 1_000));
    for id in 0..4 {
        assert_eq!(queue.try_push(stamped(id)), PushOutcome::Pushed, "id {id}");
    }
    // At capacity on the packet axis.
    assert_eq!(
        queue.usage(),
        QueueUsage {
            packets: 4,
            bytes: 4,
            duration: Nanoseconds(0),
        }
    );

    // The producer's push is rejected — typed signal, item untouched.
    let overflow = queue.try_push(stamped(4));
    match overflow {
        PushOutcome::Full { item, binding } => {
            assert_eq!(binding, LimitAxis::Packets);
            assert_eq!(item.value, blob(4));
        }
        other => unreachable!("expected Full, got {other:?}"),
    }
    assert_eq!(queue.stats().full_rejections, 1);
    assert_eq!(queue.usage().packets, 4, "a rejected push changes nothing");

    // The consumer drains one; the producer's retry succeeds.
    match queue.try_pop() {
        PopOutcome::Item(item) => assert_eq!(item.value.id, 0),
        other => unreachable!("expected Item, got {other:?}"),
    }
    assert_eq!(queue.try_push(stamped(4)), PushOutcome::Pushed);

    // FIFO order across the overflow cycle.
    let mut popped = Vec::new();
    for _ in 0..4 {
        match queue.try_pop() {
            PopOutcome::Item(item) => popped.push(item.value.id),
            other => unreachable!("expected Item, got {other:?}"),
        }
    }
    assert_eq!(popped, vec![1, 2, 3, 4]);
    assert!(matches!(queue.try_pop(), PopOutcome::Empty));
    assert_eq!(queue.stats().pushed, 5);
    assert_eq!(queue.stats().popped, 5);
}

/// AC(a), byte axis: the byte cap refuses the push before the packet
/// cap is anywhere near.
#[test]
fn ac_overflow_backpressure_proof_byte_axis() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(1_000, 100, 1_000_000));
    for id in 0..3 {
        let item = Stamped::new(
            GenerationId::FIRST,
            Blob {
                id,
                size: 30,
                dur_ns: 0,
            },
        );
        assert_eq!(queue.try_push(item), PushOutcome::Pushed, "id {id}");
    }
    let fourth = Stamped::new(
        GenerationId::FIRST,
        Blob {
            id: 3,
            size: 30,
            dur_ns: 0,
        },
    );
    match queue.try_push(fourth) {
        PushOutcome::Full { item, binding } => {
            assert_eq!(binding, LimitAxis::Bytes);
            assert_eq!(item.value.id, 3);
        }
        other => unreachable!("expected Full, got {other:?}"),
    }
    // Drain frees the byte budget; the producer succeeds.
    match queue.try_pop() {
        PopOutcome::Item(item) => assert_eq!(item.value.id, 0),
        other => unreachable!("expected Item, got {other:?}"),
    }
    assert_eq!(queue.try_push(fourth), PushOutcome::Pushed);
}

/// The duration cap is a first-class backpressure axis.
#[test]
fn duration_cap_refuses_pushes() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(1_000, 1_000, 1_000));
    let timed = |id: u64| {
        Stamped::new(
            GenerationId::FIRST,
            Blob {
                id,
                size: 1,
                dur_ns: 300,
            },
        )
    };
    for id in 0..3 {
        assert_eq!(queue.try_push(timed(id)), PushOutcome::Pushed, "id {id}");
    }
    match queue.try_push(timed(3)) {
        PushOutcome::Full { item, binding } => {
            assert_eq!(binding, LimitAxis::Duration);
            assert_eq!(item.value.id, 3);
        }
        other => unreachable!("expected Full, got {other:?}"),
    }
    assert_eq!(queue.usage().duration, Nanoseconds(900));
}

/// An item heavier than the whole byte budget is admitted **alone**
/// into the empty queue, and the caps bound accumulation behind it:
/// a second oversized item is refused while the first is resident,
/// and the drain re-opens admission (the livelock invariant,
/// 6.FIX-pcm — the old absolute refusal stalled real lanes: the wav
/// 64 KiB packetizer's >1 s packets against the audio lane's 1 s
/// duration cap never admitted, freezing the session at its anchor).
#[test]
fn oversized_item_admits_alone_and_bounds_accumulation() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(8, 100, 1_000));
    let huge = |id: u64| {
        Stamped::new(
            GenerationId::FIRST,
            Blob {
                id,
                size: 101,
                dur_ns: 0,
            },
        )
    };
    // The empty queue admits the oversized item unconditionally.
    assert_eq!(queue.try_push(huge(0)), PushOutcome::Pushed);
    assert_eq!(queue.usage().bytes, 101);
    // While it is resident, accumulation is refused on the binding
    // axis — the caps still bound what piles up behind it.
    match queue.try_push(huge(1)) {
        PushOutcome::Full { item, binding } => {
            assert_eq!(binding, LimitAxis::Bytes);
            assert_eq!(item.value.id, 1);
        }
        other => unreachable!("expected Full, got {other:?}"),
    }
    // The drain frees the queue; the next oversized item admits again.
    match queue.try_pop() {
        PopOutcome::Item(item) => assert_eq!(item.value.id, 0),
        other => unreachable!("expected Item, got {other:?}"),
    }
    assert_eq!(queue.try_push(huge(2)), PushOutcome::Pushed);
    assert_eq!(queue.usage().packets, 1);
}

/// A parked producer wakes when the consumer drains space (Q3: parks,
/// never spins) and completes its push.
#[test]
fn push_wait_parks_until_consumer_drains() {
    let queue = Arc::new(BoundedQueue::<Blob>::with_default_watermarks(limits(
        1, 1_000, 1_000,
    )));
    assert_eq!(queue.try_push(stamped(0)), PushOutcome::Pushed);

    let (tx, rx) = mpsc::channel();
    let parked_queue = Arc::clone(&queue);
    let producer = thread::Builder::new()
        .name("ac-parked-producer".into())
        .spawn(move || {
            let started = Instant::now();
            let result = parked_queue.push_wait(stamped(1), None);
            let _ = tx.send((result, started.elapsed()));
        })
        .must();

    // Let the producer reach the condvar, then drain one slot.
    thread::sleep(Duration::from_millis(100));
    match queue.try_pop() {
        PopOutcome::Item(item) => assert_eq!(item.value.id, 0),
        other => unreachable!("expected Item, got {other:?}"),
    }

    let (result, _elapsed) = rx.recv_timeout(Duration::from_secs(5)).must();
    assert!(result.is_ok(), "parked producer must complete: {result:?}");
    join_or_propagate(producer);
}

/// The park is bounded: with nobody draining, a deadline expires and
/// the item comes back untouched.
#[test]
fn push_wait_deadline_bounds_the_park() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(1, 1_000, 1_000));
    assert_eq!(queue.try_push(stamped(0)), PushOutcome::Pushed);

    let started = Instant::now();
    let deadline = started + Duration::from_millis(150);
    match queue.push_wait(stamped(1), Some(deadline)) {
        Err(PushAborted::Deadline { item }) => assert_eq!(item.value, blob(1)),
        other => unreachable!("expected Deadline abort, got {other:?}"),
    }
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_millis(120),
        "expired too early: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "parked far beyond the deadline: {elapsed:?}"
    );
    assert_eq!(queue.stats().full_rejections, 1);
    assert_eq!(queue.usage().packets, 1, "the aborted push changed nothing");
}

/// With room available the parking push returns immediately.
#[test]
fn push_wait_returns_immediately_with_room() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(4, 1_000, 1_000));
    let started = Instant::now();
    let deadline = started + Duration::from_secs(5);
    queue
        .push_wait(stamped(0), Some(deadline))
        .map_err(|abort| format!("immediate push must succeed: {abort:?}"))
        .must();
    assert!(started.elapsed() < Duration::from_millis(500));
}

/// Closing wakes a parked producer with the typed Closed abort, item
/// in hand (teardown never blocks a producer forever).
#[test]
fn close_wakes_parked_producer() {
    let queue = Arc::new(BoundedQueue::<Blob>::with_default_watermarks(limits(
        1, 1_000, 1_000,
    )));
    assert_eq!(queue.try_push(stamped(0)), PushOutcome::Pushed);

    let (tx, rx) = mpsc::channel();
    let parked_queue = Arc::clone(&queue);
    let producer = thread::Builder::new()
        .name("close-parked-producer".into())
        .spawn(move || {
            let _ = tx.send(parked_queue.push_wait(stamped(1), None));
        })
        .must();

    thread::sleep(Duration::from_millis(100));
    queue.close();
    let result = rx.recv_timeout(Duration::from_secs(5)).must();
    match result {
        Err(PushAborted::Closed { item }) => assert_eq!(item.value, blob(1)),
        other => unreachable!("expected Closed abort, got {other:?}"),
    }
    join_or_propagate(producer);
}

/// A parked consumer wakes on the next push (the consumer side of the
/// same condvar protocol).
#[test]
fn parked_consumer_wakes_on_push() {
    let queue = Arc::new(BoundedQueue::<Blob>::with_default_watermarks(limits(
        4, 1_000, 1_000,
    )));
    let (tx, rx) = mpsc::channel();
    let parked_queue = Arc::clone(&queue);
    let consumer = thread::Builder::new()
        .name("parked-consumer".into())
        .spawn(move || {
            let _ = tx.send(parked_queue.pop_wait(None));
        })
        .must();

    thread::sleep(Duration::from_millis(100));
    assert_eq!(queue.try_push(stamped(42)), PushOutcome::Pushed);
    let outcome = rx.recv_timeout(Duration::from_secs(5)).must();
    match outcome {
        PopOutcome::Item(item) => assert_eq!(item.value.id, 42),
        other => unreachable!("expected Item, got {other:?}"),
    }
    join_or_propagate(consumer);
}

/// Closing refuses producers immediately and lets consumers drain
/// resident data before reporting Closed — close never destroys data
/// without a signal.
#[test]
fn close_refuses_producers_and_drains_consumers() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(4, 1_000, 1_000));
    assert_eq!(queue.try_push(stamped(0)), PushOutcome::Pushed);
    assert_eq!(queue.try_push(stamped(1)), PushOutcome::Pushed);
    queue.close();

    match queue.try_push(stamped(2)) {
        PushOutcome::Closed { item } => assert_eq!(item.value, blob(2)),
        other => unreachable!("expected Closed, got {other:?}"),
    }
    match queue.push_wait(stamped(3), Some(Instant::now() + Duration::from_millis(50))) {
        Err(PushAborted::Closed { item }) => assert_eq!(item.value, blob(3)),
        other => unreachable!("expected Closed abort, got {other:?}"),
    }

    match queue.try_pop() {
        PopOutcome::Item(item) => assert_eq!(item.value.id, 0),
        other => unreachable!("resident data must survive close: {other:?}"),
    }
    match queue.try_pop() {
        PopOutcome::Item(item) => assert_eq!(item.value.id, 1),
        other => unreachable!("resident data must survive close: {other:?}"),
    }
    assert!(matches!(queue.try_pop(), PopOutcome::Closed));
    assert!(matches!(queue.pop_wait(None), PopOutcome::Closed));
    assert!(queue.is_closed());
}

/// The watermark flags track the fill/drain cycle that backpressure
/// drives (Q4: the single input to Buffering transitions).
#[test]
fn watermark_flags_track_the_backpressure_cycle() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(8, 1_000, 1_000));
    assert!(queue.watermark().low, "empty queue is at its low mark");

    for id in 0..8 {
        assert_eq!(queue.try_push(stamped(id)), PushOutcome::Pushed);
    }
    let full_mark = queue.watermark();
    assert!(full_mark.high, "queue at capacity hits the high mark");
    assert!(!full_mark.low);
    assert_eq!(full_mark.usage.packets, 8);

    // Drain below the quarter line: low returns.
    for _ in 0..7 {
        match queue.try_pop() {
            PopOutcome::Item(_) => {}
            other => unreachable!("expected Item, got {other:?}"),
        }
    }
    let drained_mark = queue.watermark();
    assert!(drained_mark.low);
    assert!(!drained_mark.high);
    assert_eq!(drained_mark.usage.packets, 1);
}

/// Backpressure events are counted (doc 03 §5 metrics surface).
#[test]
fn stats_count_backpressure_events() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(2, 1_000, 1_000));
    assert_eq!(queue.try_push(stamped(0)), PushOutcome::Pushed);
    assert_eq!(queue.try_push(stamped(1)), PushOutcome::Pushed);
    // Two refused pushes (typed full signals) …
    assert!(matches!(
        queue.try_push(stamped(2)),
        PushOutcome::Full { .. }
    ));
    assert!(matches!(
        queue.try_push(stamped(3)),
        PushOutcome::Full { .. }
    ));
    // … and one deadline expiry on a still-full queue.
    assert!(matches!(
        queue.push_wait(stamped(4), Some(Instant::now() + Duration::from_millis(30))),
        Err(PushAborted::Deadline { .. })
    ));
    let stats = queue.stats();
    assert_eq!(stats.pushed, 2);
    assert_eq!(stats.full_rejections, 3);
    assert_eq!(stats.popped, 0);
}

/// The lock is never held while parked (doc 03 §4.4(c)): a producer
/// parked on a full queue leaves the queue fully serviceable — the
/// session thread's non-blocking operations answer immediately (Q3's
/// "never blocks the session thread" clause), and the pop that frees
/// the slot does not deadlock behind the parked producer.
#[test]
fn parked_producer_does_not_hold_the_lock() {
    let queue = Arc::new(BoundedQueue::<Blob>::with_default_watermarks(limits(
        1, 1_000, 1_000,
    )));
    assert_eq!(queue.try_push(stamped(0)), PushOutcome::Pushed);

    let parked_queue = Arc::clone(&queue);
    let producer = thread::Builder::new()
        .name("lock-proof-producer".into())
        .spawn(move || {
            // Parks (nobody drains until the proof's own pop below).
            let _ = parked_queue.push_wait(stamped(1), None);
        })
        .must();

    // Let the producer reach the condvar, then verify every other
    // operation still completes immediately — a parked producer must
    // hold no lock while waiting.
    thread::sleep(Duration::from_millis(100));
    let started = Instant::now();
    assert!(
        matches!(queue.try_push(stamped(2)), PushOutcome::Full { .. }),
        "try_push must answer a full queue without parking"
    );
    match queue.try_pop() {
        PopOutcome::Item(item) => assert_eq!(item.value.id, 0),
        other => unreachable!("expected Item, got {other:?}"),
    }
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "non-blocking operations stalled behind a parked producer: {:?}",
        started.elapsed()
    );

    // The pop freed the slot: the parked producer wakes and completes,
    // and its item is the next to pop (FIFO).
    join_or_propagate(producer);
    match queue.try_pop() {
        PopOutcome::Item(item) => assert_eq!(item.value.id, 1),
        other => unreachable!("expected the parked producer's item, got {other:?}"),
    }
    assert!(matches!(queue.try_pop(), PopOutcome::Empty));
    let stats = queue.stats();
    assert_eq!(stats.pushed, 2);
    assert_eq!(stats.popped, 2);
}
