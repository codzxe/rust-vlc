//! Concurrency proofs (doc 03 §4.4(c)/(f), doc 01 §2.2(e)): MPMC
//! stress — no loss, no duplication, exact conservation under
//! generation storms, and a usage sampler that must never observe the
//! caps exceeded.

use std::collections::BTreeSet;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use media_types::ids::GenerationId;
use media_types::time::{Duration as MediaDuration, Nanoseconds};
use packet_queue::{
    BoundedQueue, PopOutcome, PushAborted, PushOutcome, QueueLimits, Stamped, Weigh,
};
use testkit::Must;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Blob {
    id: u64,
}

impl Weigh for Blob {
    fn size_bytes(&self) -> usize {
        4
    }

    fn media_duration(&self) -> Nanoseconds {
        Nanoseconds(2)
    }
}

fn limits(packets: usize, bytes: usize, dur_ns: u64) -> QueueLimits {
    QueueLimits::new(
        NonZeroUsize::new(packets).must(),
        NonZeroUsize::new(bytes).must(),
        MediaDuration::Known(Nanoseconds(dur_ns)),
    )
    .must()
}

fn join_or_propagate(handles: Vec<JoinHandle<()>>) {
    for handle in handles {
        if let Err(panic_payload) = handle.join() {
            std::panic::resume_unwind(panic_payload);
        }
    }
}

/// One consumer observation: either a delivered item id, or the
/// dropped-count of a discontinuity marker — the two halves of the
/// conservation law.
enum ConsumerEvent {
    Item(u64),
    Dropped(u64),
}

/// Record one delivered id into the shared sink (poison-recovering:
/// a sibling consumer's panic must not eat this observation).
fn record_id(sink: &Mutex<Vec<u64>>, id: u64) {
    match sink.lock() {
        Ok(mut ids) => ids.push(id),
        Err(poisoned) => {
            let mut ids = poisoned.into_inner();
            ids.push(id);
        }
    }
}

/// MPMC: 4 producers, 3 consumers, one generation — every one of the
/// 1000 items is delivered exactly once (no loss, no duplication),
/// and the queue's own counters conserve.
#[test]
fn mpmc_no_loss_no_duplication() {
    const PRODUCERS: u64 = 4;
    const PER_PRODUCER: u64 = 250;
    const CONSUMERS: usize = 3;
    let queue = Arc::new(BoundedQueue::<Blob>::with_default_watermarks(limits(
        8, 1_000, 2_000,
    )));

    let mut producer_handles = Vec::new();
    for producer in 0..PRODUCERS {
        let queue = Arc::clone(&queue);
        producer_handles.push(
            thread::Builder::new()
                .name(format!("mpmc-producer-{producer}"))
                .spawn(move || {
                    for step in 0..PER_PRODUCER {
                        let id = producer * PER_PRODUCER + step;
                        let mut item = Stamped::new(GenerationId::FIRST, Blob { id });
                        loop {
                            match queue.try_push(item) {
                                PushOutcome::Pushed => break,
                                // Backpressure: the typed signal hands
                                // the item back; retry after a yield
                                // (consumers are draining).
                                PushOutcome::Full { item: back, .. } => {
                                    item = back;
                                    thread::yield_now();
                                }
                                PushOutcome::Closed { .. } => {
                                    unreachable!("queue closed mid-flight (id {id})")
                                }
                                PushOutcome::Stale { .. } => {
                                    unreachable!("single generation, nothing is stale (id {id})")
                                }
                            }
                        }
                    }
                })
                .must(),
        );
    }

    let collected: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));
    let mut consumer_handles = Vec::new();
    for consumer in 0..CONSUMERS {
        let queue = Arc::clone(&queue);
        let collected = Arc::clone(&collected);
        consumer_handles.push(
            thread::Builder::new()
                .name(format!("mpmc-consumer-{consumer}"))
                .spawn(move || {
                    loop {
                        match queue.pop_wait(Some(Instant::now() + Duration::from_millis(50))) {
                            PopOutcome::Item(item) => record_id(&collected, item.value.id),
                            // Deadline: keep polling until the close.
                            PopOutcome::Empty => thread::yield_now(),
                            PopOutcome::Closed => break,
                            PopOutcome::Discontinuity(_) => {
                                unreachable!("single generation, no flushes")
                            }
                        }
                    }
                })
                .must(),
        );
    }

    // Producers finish, then close so the consumers can drain and exit.
    join_or_propagate(producer_handles);
    queue.close();
    join_or_propagate(consumer_handles);

    let collected = Arc::try_unwrap(collected).map_or_else(
        |_| unreachable!("consumers still hold the sink"),
        |sink| {
            sink.into_inner()
                .unwrap_or_else(|_| unreachable!("sink poisoned"))
        },
    );
    assert_eq!(collected.len(), (PRODUCERS * PER_PRODUCER) as usize);
    let unique: BTreeSet<u64> = collected.iter().copied().collect();
    assert_eq!(
        unique.len(),
        (PRODUCERS * PER_PRODUCER) as usize,
        "some items were lost"
    );
    assert_eq!(collected.len(), unique.len(), "some items were duplicated");

    let stats = queue.stats();
    assert_eq!(stats.pushed, PRODUCERS * PER_PRODUCER);
    assert_eq!(stats.popped, PRODUCERS * PER_PRODUCER);
    assert_eq!(stats.flushed, 0);
    assert_eq!(stats.stale_rejected, 0);
    assert_eq!(queue.usage().packets, 0);
}

/// Generation storm: producers stamp-and-push at their own pace while
/// a flusher bumps the generation repeatedly. Every accepted item is
/// accounted for exactly once (popped to a consumer or counted in a
/// consumer-observed discontinuity), the queue's counters conserve,
/// and the Q2 floor holds along each consumer's observation order —
/// no item is ever observed after a marker that supersedes it.
#[test]
fn flush_storm_conserves_every_item_and_holds_q2() {
    const PRODUCERS: u64 = 2;
    const PER_PRODUCER: u64 = 300;
    const FLUSHES: u64 = 12;
    let queue = Arc::new(BoundedQueue::<Blob>::with_default_watermarks(limits(
        8, 1_000, 64,
    )));

    let stop = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();
    for producer in 0..PRODUCERS {
        let queue = Arc::clone(&queue);
        let stop = Arc::clone(&stop);
        handles.push(
            thread::Builder::new()
                .name(format!("storm-producer-{producer}"))
                .spawn(move || {
                    for step in 0..PER_PRODUCER {
                        let id = producer * PER_PRODUCER + step;
                        // Stamp the CURRENT published generation; a
                        // flush may race the stamp — the queue's
                        // boundary check drops the loser, typed. The
                        // 1 ms read window is the demuxer's
                        // `read_packet` stand-in that makes the race
                        // substantial rather than incidental.
                        let stamp = queue.generation();
                        thread::sleep(Duration::from_millis(1));
                        let mut item = Stamped::new(stamp, Blob { id });
                        loop {
                            match queue.try_push(item) {
                                PushOutcome::Pushed => break,
                                PushOutcome::Full { item: back, .. } => {
                                    item = back;
                                    thread::yield_now();
                                }
                                // Stale: the pipeline moved on; a real
                                // demuxer drops the packet — so does
                                // the test, counted by the queue.
                                PushOutcome::Stale { .. } => break,
                                PushOutcome::Closed { .. } => {
                                    unreachable!("queue closed mid-flight (id {id})")
                                }
                            }
                        }
                        if stop.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                })
                .must(),
        );
    }

    // One consumer: the marker chain is observed by exactly one
    // thread, so item + marker accounting is exact.
    let (tx, rx) = mpsc::channel::<ConsumerEvent>();
    let consumer_queue = Arc::clone(&queue);
    let consumer = thread::Builder::new()
        .name("storm-consumer".into())
        .spawn(move || {
            let mut floor = GenerationId::FIRST;
            loop {
                match consumer_queue.pop_wait(Some(Instant::now() + Duration::from_millis(50))) {
                    PopOutcome::Item(stamped) => {
                        // Q2 along the observation order: after a
                        // marker published G, no item stamped < G.
                        assert!(
                            stamped.generation >= floor,
                            "Q2 violation: item stamped {} observed after floor {}",
                            stamped.generation.counter(),
                            floor.counter()
                        );
                        let _ = tx.send(ConsumerEvent::Item(stamped.value.id));
                        // Slow consumer: keeps the queue near capacity
                        // so flushes actually drop (the audio pump's
                        // pacing stand-in).
                        thread::sleep(Duration::from_millis(1));
                    }
                    PopOutcome::Discontinuity(gap) => {
                        if gap.new_generation > floor {
                            floor = gap.new_generation;
                        }
                        let _ = tx.send(ConsumerEvent::Dropped(gap.dropped));
                    }
                    PopOutcome::Empty => thread::yield_now(),
                    PopOutcome::Closed => break,
                }
            }
        })
        .must();

    // Flusher: bump the generation FLUSHES times, spaced out so
    // producers race it from both sides.
    for bump in 1..=FLUSHES {
        thread::sleep(Duration::from_millis(4));
        let _ = queue.flush(GenerationId::new(bump));
    }
    // Give the storm time to interleave, then stop and drain.
    thread::sleep(Duration::from_millis(30));
    stop.store(true, Ordering::Relaxed);
    join_or_propagate(handles);
    queue.close();
    join_or_propagate(vec![consumer]);

    // Account for every accepted item exactly once.
    let mut popped = 0u64;
    let mut marker_dropped = 0u64;
    for message in rx {
        match message {
            ConsumerEvent::Item(id) => {
                assert!(id < PRODUCERS * PER_PRODUCER, "phantom item id {id}");
                popped += 1;
            }
            ConsumerEvent::Dropped(dropped) => marker_dropped += dropped,
        }
    }
    let stats = queue.stats();
    assert_eq!(
        stats.pushed,
        popped + marker_dropped,
        "conservation: pushed == popped + marker-dropped (stats say flushed {})",
        stats.flushed
    );
    assert_eq!(
        marker_dropped, stats.flushed,
        "consumer markers match flushes"
    );
    assert_eq!(stats.popped, popped);
    assert_eq!(queue.usage().packets, 0, "closed and drained");
    // The storm actually raced: both signals fired.
    assert!(
        stats.flushed > 0,
        "no flush ever dropped anything — storm did not race"
    );
    assert!(
        stats.stale_rejected > 0,
        "no stale push ever rejected — storm did not race"
    );
}

/// A concurrent usage sampler must never observe the caps exceeded —
/// best-effort by nature (a violation that slips between samples is
/// invisible), but any violation it DOES observe is a real Q1 break.
#[test]
fn usage_sampler_never_observes_caps_exceeded() {
    let limits = limits(4, 64, 128);
    let queue = Arc::new(BoundedQueue::<Blob>::with_default_watermarks(limits));
    let stop = Arc::new(AtomicBool::new(false));

    let sampler_queue = Arc::clone(&queue);
    let sampler_stop = Arc::clone(&stop);
    let sampler = thread::Builder::new()
        .name("usage-sampler".into())
        .spawn(move || {
            while !sampler_stop.load(Ordering::Relaxed) {
                let usage = sampler_queue.usage();
                assert!(
                    usage.packets <= 4,
                    "Q1 packets observed over cap: {usage:?}"
                );
                assert!(usage.bytes <= 64, "Q1 bytes observed over cap: {usage:?}");
                assert!(
                    usage.duration.0 <= 128,
                    "Q1 duration observed over cap: {:?}",
                    usage.duration
                );
                thread::sleep(Duration::from_millis(1));
            }
        })
        .must();

    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::new();
    for producer in 0..2u64 {
        let queue = Arc::clone(&queue);
        let tx = tx.clone();
        handles.push(
            thread::Builder::new()
                .name(format!("churn-producer-{producer}"))
                .spawn(move || {
                    for step in 0..500u64 {
                        let id = producer * 500 + step;
                        let mut item = Stamped::new(GenerationId::FIRST, Blob { id });
                        loop {
                            match queue.try_push(item) {
                                PushOutcome::Pushed => break,
                                PushOutcome::Full { item: back, .. } => {
                                    item = back;
                                    thread::yield_now();
                                }
                                _ => unreachable!("no close/flush in this test"),
                            }
                        }
                    }
                    let _ = tx.send(());
                })
                .must(),
        );
    }
    let consumer_queue = Arc::clone(&queue);
    handles.push(
        thread::Builder::new()
            .name("churn-consumer".into())
            .spawn(move || {
                let mut drained = 0u64;
                while drained < 1_000 {
                    match consumer_queue.pop_wait(Some(Instant::now() + Duration::from_millis(20)))
                    {
                        PopOutcome::Item(_) => drained += 1,
                        PopOutcome::Empty => thread::yield_now(),
                        _ => unreachable!("no close/flush in this test"),
                    }
                }
            })
            .must(),
    );

    join_or_propagate(handles);
    for _ in 0..2 {
        rx.recv_timeout(Duration::from_secs(5)).must();
    }
    stop.store(true, Ordering::Relaxed);
    join_or_propagate(vec![sampler]);
    assert_eq!(queue.stats().pushed, 1_000);
    assert_eq!(queue.stats().popped, 1_000);
}

/// Parking MPMC (doc 03 §4.4(c) Q3): several producers push via the
/// *parking* `push_wait(None)` — the demux thread's consent to wait for
/// the pipeline — while a paced consumer drains. Every one of the items
/// is delivered exactly once, the producers complete without spinning
/// (their progress is bounded by the consumer's pace, not a retry
/// loop), and the counters conserve. The global wall-clock budget
/// turns a lost wakeup into a loud failure instead of a hung test.
#[test]
fn parked_mpmc_producers_complete_under_paced_consumption() {
    const PRODUCERS: u64 = 3;
    const PER_PRODUCER: u64 = 60;
    const TOTAL: u64 = PRODUCERS * PER_PRODUCER;
    let queue = Arc::new(BoundedQueue::<Blob>::with_default_watermarks(limits(
        8, 1_000, 2_000,
    )));

    let mut producer_handles = Vec::new();
    for producer in 0..PRODUCERS {
        let queue = Arc::clone(&queue);
        producer_handles.push(
            thread::Builder::new()
                .name(format!("parked-producer-{producer}"))
                .spawn(move || {
                    for step in 0..PER_PRODUCER {
                        let id = producer * PER_PRODUCER + step;
                        let item = Stamped::new(GenerationId::FIRST, Blob { id });
                        match queue.push_wait(item, None) {
                            Ok(()) => {}
                            // Single generation, the queue never closes
                            // before the producers join: no abort path
                            // is reachable.
                            Err(abort) => {
                                unreachable!("parked push aborted (id {id}): {abort:?}")
                            }
                        }
                    }
                })
                .must(),
        );
    }

    let collected: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));
    let consumer_queue = Arc::clone(&queue);
    let consumer_sink = Arc::clone(&collected);
    let consumer = thread::Builder::new()
        .name("paced-consumer".into())
        .spawn(move || {
            let budget = Instant::now() + Duration::from_secs(30);
            let mut drained: u64 = 0;
            while drained < TOTAL {
                assert!(
                    Instant::now() < budget,
                    "consumer stalled at {drained}/{TOTAL} — a producer lost its wakeup"
                );
                match consumer_queue.pop_wait(Some(Instant::now() + Duration::from_millis(100))) {
                    PopOutcome::Item(item) => {
                        record_id(&consumer_sink, item.value.id);
                        drained += 1;
                        // The audio pump's pacing stand-in: keeps the
                        // queue full enough that producers actually park.
                        thread::sleep(Duration::from_micros(200));
                    }
                    PopOutcome::Empty => {}
                    PopOutcome::Closed | PopOutcome::Discontinuity(_) => {
                        unreachable!("no close and no flush in this test (drained {drained})")
                    }
                }
            }
        })
        .must();

    join_or_propagate(producer_handles);
    join_or_propagate(vec![consumer]);

    let ids = Arc::try_unwrap(collected).map_or_else(
        |_| unreachable!("consumer still holds the sink"),
        |sink| {
            sink.into_inner()
                .unwrap_or_else(|_| unreachable!("sink poisoned"))
        },
    );
    assert_eq!(ids.len(), TOTAL as usize);
    let unique: BTreeSet<u64> = ids.iter().copied().collect();
    assert_eq!(unique.len(), TOTAL as usize, "some items were lost");
    assert_eq!(ids.len(), unique.len(), "some items were duplicated");

    let stats = queue.stats();
    assert_eq!(stats.pushed, TOTAL);
    assert_eq!(stats.popped, TOTAL);
    assert_eq!(queue.usage().packets, 0);
}

/// `close` is a broadcast: EVERY producer parked on the full queue
/// wakes with the typed `Closed` abort (well inside the observation
/// budget — a `notify_one` bug would strand all but one), and the
/// resident data still drains to consumers afterwards (close never
/// destroys data without a signal).
#[test]
fn close_wakes_every_parked_producer() {
    const PARKED: u64 = 4;
    const RESIDENT: u64 = 8;
    let queue = Arc::new(BoundedQueue::<Blob>::with_default_watermarks(limits(
        8, 1_000, 2_000,
    )));

    // Fill to capacity so the parked producers have something to park on.
    for id in 0..RESIDENT {
        assert_eq!(
            queue.try_push(Stamped::new(GenerationId::FIRST, Blob { id })),
            PushOutcome::Pushed,
            "id {id}"
        );
    }

    let (tx, rx) = mpsc::channel();
    let mut parked_handles = Vec::new();
    for producer in 0..PARKED {
        let queue = Arc::clone(&queue);
        let tx = tx.clone();
        parked_handles.push(
            thread::Builder::new()
                .name(format!("close-parked-{producer}"))
                .spawn(move || {
                    let item = Stamped::new(
                        GenerationId::FIRST,
                        Blob {
                            id: RESIDENT + producer,
                        },
                    );
                    let _ = tx.send(queue.push_wait(item, None));
                })
                .must(),
        );
    }
    drop(tx);

    // Let every producer reach the condvar, then broadcast the close.
    thread::sleep(Duration::from_millis(100));
    let closed_at = Instant::now();
    queue.close();

    let mut aborts = 0u64;
    for _ in 0..PARKED {
        let result = rx.recv_timeout(Duration::from_secs(5)).must();
        match result {
            Err(PushAborted::Closed { item }) => {
                assert!(
                    (RESIDENT..RESIDENT + PARKED).contains(&item.value.id),
                    "unexpected item handed back: {:?}",
                    item.value
                );
                aborts += 1;
            }
            Ok(()) => unreachable!("a parked producer completed into a full closed queue"),
            Err(other) => unreachable!("expected Closed abort, got {other:?}"),
        }
    }
    assert_eq!(aborts, PARKED, "every parked producer must wake");
    assert!(
        closed_at.elapsed() < Duration::from_secs(2),
        "the broadcast woke the producers far too late: {:?}",
        closed_at.elapsed()
    );
    join_or_propagate(parked_handles);

    // Resident data survives the close; consumers drain then see Closed.
    let mut drained = 0u64;
    loop {
        match queue.try_pop() {
            PopOutcome::Item(item) => {
                assert!(item.value.id < RESIDENT, "phantom item {:?}", item.value);
                drained += 1;
            }
            PopOutcome::Closed => break,
            other => unreachable!("expected Item or Closed, got {other:?}"),
        }
    }
    assert_eq!(drained, RESIDENT);
    let stats = queue.stats();
    assert_eq!(stats.pushed, RESIDENT, "the aborted pushes entered nothing");
    assert_eq!(stats.popped, RESIDENT);
}
