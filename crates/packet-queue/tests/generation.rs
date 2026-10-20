//! Generation stale-drop proofs (card 2.B1 AC-b; doc 03 §4.4 Q2, §5).
//!
//! The acceptance criterion: fill with generation N, bump to N+1 — all
//! gen-N packets are dropped wholesale *with the typed signal*
//! (`Discontinuity`, never silent), and subsequent gen-N+1 packets
//! flow. The parked-producer race (doc 03 §4.4(f)) proves a producer
//! parked on a full queue wakes and observes `PushAborted::Stale`
//! within a bounded number of wakeups.

use std::num::NonZeroUsize;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use media_types::ids::GenerationId;
use media_types::time::{Duration as MediaDuration, Nanoseconds};
use packet_queue::{
    BoundedQueue, Discontinuity, PopOutcome, PushAborted, PushOutcome, QueueLimits, QueueUsage,
    Stamped, Weigh,
};
use testkit::Must;

/// Minimal weighed payload with a controllable id (mirrors
/// `tests/backpressure.rs`; kept local so each proof file reads alone).
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

fn stamped(generation: GenerationId, id: u64) -> Stamped<Blob> {
    Stamped::new(
        generation,
        Blob {
            id,
            size: 10,
            dur_ns: 100,
        },
    )
}

fn limits(packets: usize) -> QueueLimits {
    QueueLimits::new(
        NonZeroUsize::new(packets).must(),
        NonZeroUsize::new(1_000).must(),
        MediaDuration::Known(Nanoseconds(1_000)),
    )
    .must()
}

fn join_or_propagate(handle: thread::JoinHandle<()>) {
    if let Err(panic_payload) = handle.join() {
        std::panic::resume_unwind(panic_payload);
    }
}

/// AC(b): fill with gen N, bump to N+1, all gen-N packets dropped with
/// the typed signal; subsequent gen-N+1 packets flow; a late gen-N
/// push is dropped at the boundary with its own typed signal.
#[test]
fn ac_stale_generation_drop_proof() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(4));
    let first = GenerationId::FIRST;
    let second = first.next().must();

    // Fill with gen N (4 × 10 bytes, 4 × 100 ns).
    for id in 0..4 {
        assert_eq!(
            queue.try_push(stamped(first, id)),
            PushOutcome::Pushed,
            "id {id}"
        );
    }
    assert_eq!(
        queue.usage(),
        QueueUsage {
            packets: 4,
            bytes: 40,
            duration: Nanoseconds(400),
        }
    );

    // Bump to gen N+1: the flush reports the wholesale drop.
    let gap = queue.flush(second);
    assert_eq!(
        gap,
        Discontinuity {
            new_generation: second,
            dropped: 4,
            dropped_bytes: 40,
            dropped_duration: Nanoseconds(400),
        }
    );
    assert_eq!(queue.generation(), second);
    assert_eq!(queue.usage(), QueueUsage::EMPTY);
    assert_eq!(queue.stats().flushed, 4);

    // The consumer sees the drop as the typed discontinuity signal —
    // never silent — and then an empty queue (all gen-N gone).
    match queue.try_pop() {
        PopOutcome::Discontinuity(observed) => assert_eq!(observed, gap),
        other => unreachable!("expected Discontinuity, got {other:?}"),
    }
    assert!(matches!(queue.try_pop(), PopOutcome::Empty));

    // A late gen-N push is dropped at the boundary, typed, counted,
    // item handed back.
    match queue.try_push(stamped(first, 99)) {
        PushOutcome::Stale { item, current } => {
            assert_eq!(item.value, stamped(first, 99).value);
            assert_eq!(current, second);
        }
        other => unreachable!("expected Stale, got {other:?}"),
    }
    assert_eq!(queue.stats().stale_rejected, 1);
    assert_eq!(queue.usage(), QueueUsage::EMPTY);

    // Subsequent gen-N+1 packets flow.
    for id in 0..2 {
        assert_eq!(
            queue.try_push(stamped(second, id)),
            PushOutcome::Pushed,
            "id {id}"
        );
    }
    match queue.try_pop() {
        PopOutcome::Item(item) => {
            assert_eq!(item.generation, second);
            assert_eq!(item.value.id, 0);
        }
        other => unreachable!("expected Item, got {other:?}"),
    }
    let stats = queue.stats();
    assert_eq!(stats.pushed, 6);
    assert_eq!(stats.popped, 1);
    assert_eq!(stats.flushed, 4);
    assert_eq!(stats.stale_rejected, 1);
}

/// A push of an already-stale item aborts immediately (no park) with
/// the item handed back — the boundary check runs before any parking.
#[test]
fn stale_push_wait_aborts_immediately() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(4));
    let first = GenerationId::FIRST;
    let second = first.next().must();
    let _ = queue.flush(second);

    let started = Instant::now();
    match queue.push_wait(
        stamped(first, 7),
        Some(Instant::now() + Duration::from_secs(5)),
    ) {
        Err(PushAborted::Stale { item, current }) => {
            assert_eq!(item.value, stamped(first, 7).value);
            assert_eq!(current, second);
        }
        other => unreachable!("expected Stale abort, got {other:?}"),
    }
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(queue.stats().stale_rejected, 1);
}

/// Doc 03 §4.4(f) race: a producer parked on a full queue when the
/// flush happens must wake and observe `PushAborted::Stale` within a
/// bounded number of wakeups — here: one condvar notification, proven
/// by receiving the abort well inside the 5 s observation budget.
#[test]
fn parked_producer_wakes_on_flush_with_stale_abort() {
    let queue = Arc::new(BoundedQueue::<Blob>::with_default_watermarks(limits(1)));
    let first = GenerationId::FIRST;
    let second = first.next().must();
    assert_eq!(queue.try_push(stamped(first, 0)), PushOutcome::Pushed);

    let (tx, rx) = mpsc::channel();
    let parked_queue = Arc::clone(&queue);
    let started = Instant::now();
    let producer = thread::Builder::new()
        .name("flush-race-producer".into())
        .spawn(move || {
            let _ = tx.send(parked_queue.push_wait(stamped(first, 1), None));
        })
        .must();

    // Let the producer park on the full queue, then flush.
    thread::sleep(Duration::from_millis(100));
    let _ = queue.flush(second);

    let result = rx.recv_timeout(Duration::from_secs(5)).must();
    let elapsed = started.elapsed();
    match result {
        Err(PushAborted::Stale { item, current }) => {
            assert_eq!(item.value, stamped(first, 1).value);
            assert_eq!(current, second);
        }
        other => unreachable!("expected Stale abort, got {other:?}"),
    }
    assert!(
        elapsed < Duration::from_secs(2),
        "parked producer woke far too late: {elapsed:?}"
    );
    join_or_propagate(producer);
}

/// A parked consumer wakes on a flush and observes the discontinuity
/// before any new-generation item (the consumer side of §4.4(f)).
#[test]
fn parked_consumer_wakes_on_flush_with_discontinuity() {
    let queue = Arc::new(BoundedQueue::<Blob>::with_default_watermarks(limits(4)));
    let first = GenerationId::FIRST;
    let second = first.next().must();

    let (tx, rx) = mpsc::channel();
    let parked_queue = Arc::clone(&queue);
    let consumer = thread::Builder::new()
        .name("flush-race-consumer".into())
        .spawn(move || {
            let _ = tx.send(parked_queue.pop_wait(None));
        })
        .must();

    thread::sleep(Duration::from_millis(100));
    let gap = queue.flush(second);
    let outcome = rx.recv_timeout(Duration::from_secs(5)).must();
    match outcome {
        PopOutcome::Discontinuity(observed) => {
            assert_eq!(observed, gap);
            assert_eq!(observed.new_generation, second);
            assert_eq!(observed.dropped, 0);
        }
        other => unreachable!("expected Discontinuity, got {other:?}"),
    }
    join_or_propagate(consumer);
}

/// Consecutive flushes a consumer has not yet observed merge into one
/// marker — counts accumulate, never vanish.
#[test]
fn unobserved_flush_markers_merge() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(4));
    let first = GenerationId::FIRST;
    let second = first.next().must();
    let third = second.next().must();

    for id in 0..2 {
        assert_eq!(queue.try_push(stamped(first, id)), PushOutcome::Pushed);
    }
    let gap_one = queue.flush(second);
    assert_eq!(gap_one.dropped, 2);

    // New-generation items, then a second flush before any pop.
    for id in 0..3 {
        assert_eq!(queue.try_push(stamped(second, id)), PushOutcome::Pushed);
    }
    let gap_two = queue.flush(third);
    assert_eq!(gap_two.dropped, 3);

    // The flusher saw both reports separately …
    assert_eq!(queue.stats().flushed, 5);
    // … and the consumer's single marker accounts for both.
    match queue.try_pop() {
        PopOutcome::Discontinuity(merged) => {
            assert_eq!(merged.new_generation, third);
            assert_eq!(merged.dropped, 5);
            assert_eq!(merged.dropped_bytes, 50);
            assert_eq!(merged.dropped_duration, Nanoseconds(500));
        }
        other => unreachable!("expected Discontinuity, got {other:?}"),
    }
    assert!(matches!(queue.try_pop(), PopOutcome::Empty));
}

/// Flushing to an at-or-below generation still drops the residents
/// (the caller asked for a flush) but never regresses the published
/// generation.
#[test]
fn flush_never_regresses_the_published_generation() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(4));
    let first = GenerationId::FIRST;
    let fifth = GenerationId::new(5);
    let third = GenerationId::new(3);

    for id in 0..2 {
        assert_eq!(queue.try_push(stamped(first, id)), PushOutcome::Pushed);
    }
    let _ = queue.flush(fifth);
    for id in 0..2 {
        assert_eq!(queue.try_push(stamped(fifth, id)), PushOutcome::Pushed);
    }
    // A stale flush to 3: drops residents, publishes stay at 5.
    let gap = queue.flush(third);
    assert_eq!(gap.new_generation, fifth);
    assert_eq!(gap.dropped, 2);
    assert_eq!(queue.generation(), fifth);

    // The consumer marker also reports the high-water generation.
    match queue.try_pop() {
        PopOutcome::Discontinuity(observed) => assert_eq!(observed.new_generation, fifth),
        other => unreachable!("expected Discontinuity, got {other:?}"),
    }
}

/// Flushing an empty queue still publishes the generation and signals
/// the consumer — a zero-drop generation move is observable too, never
/// silent.
#[test]
fn flush_on_empty_queue_publishes_and_signals() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(4));
    let second = GenerationId::FIRST.next().must();
    let gap = queue.flush(second);
    assert_eq!(gap.dropped, 0);
    assert_eq!(gap.new_generation, second);
    match queue.try_pop() {
        PopOutcome::Discontinuity(observed) => {
            assert_eq!(observed.dropped, 0);
            assert_eq!(observed.new_generation, second);
        }
        other => unreachable!("expected Discontinuity, got {other:?}"),
    }
    assert!(matches!(queue.try_pop(), PopOutcome::Empty));
}

/// Staleness dominates backpressure: on a full queue a stale push
/// reports `Stale` (cancellation), not `Full` (flow control) — the
/// producer must stop, not park.
#[test]
fn stale_check_precedes_the_capacity_check() {
    let queue = BoundedQueue::<Blob>::with_default_watermarks(limits(2));
    let second = GenerationId::FIRST.next().must();
    let _ = queue.flush(second);
    // Fill with current-generation items.
    for id in 0..2 {
        assert_eq!(queue.try_push(stamped(second, id)), PushOutcome::Pushed);
    }
    // Full AND stale: the stale signal wins.
    match queue.try_push(stamped(GenerationId::FIRST, 9)) {
        PushOutcome::Stale { item, current } => {
            assert_eq!(current, second);
            assert_eq!(item.value.id, 9);
        }
        other => unreachable!("expected Stale, got {other:?}"),
    }
    // The parking variant likewise aborts instead of parking.
    match queue.push_wait(
        stamped(GenerationId::FIRST, 10),
        Some(Instant::now() + Duration::from_secs(5)),
    ) {
        Err(PushAborted::Stale { item, .. }) => assert_eq!(item.value.id, 10),
        other => unreachable!("expected Stale abort, got {other:?}"),
    }
    assert_eq!(queue.stats().stale_rejected, 2);
    assert_eq!(queue.usage().packets, 2, "no abort path touched the queue");
}

/// The demux-side composite: stamp from the payload's own generation
/// (`Stamped::stamped` over a real media-types `Packet`) flows through
/// flush-and-refill identically.
#[test]
fn packet_stamps_flow_through_generation_bumps() {
    use media_types::packet::SharedBytes;
    use media_types::{Packet, Stamp, TrackId};

    let queue = BoundedQueue::<Packet>::with_default_watermarks(
        QueueLimits::new(
            NonZeroUsize::new(4).must(),
            NonZeroUsize::new(1_000).must(),
            MediaDuration::Known(Nanoseconds(1_000)),
        )
        .must(),
    );
    let second = GenerationId::FIRST.next().must();

    let packet = |generation: GenerationId| Packet {
        track: TrackId::new(1),
        pts: Stamp::Missing,
        dts: Stamp::Missing,
        duration: Some(MediaDuration::Known(Nanoseconds(100))),
        data: SharedBytes::from_vec(vec![0xA5; 10]),
        keyframe: true,
        discontinuity: false,
        generation,
    };

    assert_eq!(
        queue.try_push(Stamped::stamped(packet(GenerationId::FIRST))),
        PushOutcome::Pushed
    );
    let gap = queue.flush(second);
    assert_eq!(gap.dropped, 1);
    assert_eq!(gap.dropped_bytes, 10);
    assert_eq!(gap.dropped_duration, Nanoseconds(100));

    assert_eq!(
        queue.try_push(Stamped::stamped(packet(second))),
        PushOutcome::Pushed
    );
    match queue.try_pop() {
        PopOutcome::Discontinuity(_) => {}
        other => unreachable!("expected Discontinuity, got {other:?}"),
    }
    match queue.try_pop() {
        PopOutcome::Item(item) => {
            assert_eq!(item.generation, second);
            assert_eq!(item.value.generation, second);
        }
        other => unreachable!("expected Item, got {other:?}"),
    }
}
