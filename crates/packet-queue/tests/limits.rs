//! Deterministic property tests of the queue invariants (doc 03
//! §4.4(e)): Q1 (bounds hold, ever) and Q2 (no stale item pops, stale
//! pushes drop at the boundary) under adversarial interleavings of
//! push/pop/flush — a single-threaded, seeded model driver (the "small
//! state machine" of doc 03 §4.4(f)), plus the duration-accounting unit
//! the doc names explicitly.

use std::collections::VecDeque;
use std::num::NonZeroUsize;

use media_types::ids::GenerationId;
use media_types::time::{Duration as MediaDuration, Nanoseconds};
use packet_queue::{
    BoundedQueue, LimitAxis, PopOutcome, PushOutcome, QueueConfigError, QueueLimits, QueueUsage,
    Stamped, Weigh,
};
use testkit::Must;

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

/// Deterministic xorshift* — the driver must be reproducible: a failed
/// seed reruns the exact interleaving.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// The oracle's view of the queue: exactly what the spec arithmetic
/// says the queue must look like. Weights tracked so the model can
/// second-guess every `Full` verdict.
struct Model {
    items: VecDeque<(GenerationId, u64, usize, u64)>,
    published: GenerationId,
    limits: QueueLimits,
}

impl Model {
    fn packets(&self) -> usize {
        self.items.len()
    }

    fn bytes(&self) -> usize {
        self.items.iter().map(|(_, _, size, _)| *size).sum()
    }

    fn duration(&self) -> Nanoseconds {
        Nanoseconds(self.items.iter().map(|(_, _, _, dur)| *dur).sum())
    }

    fn fits(&self, size: usize, dur: u64) -> Option<LimitAxis> {
        let packets = (self.items.len() + 1) as u128;
        let bytes = self.bytes() as u128 + size as u128;
        let duration = self.duration().0 as u128 + dur as u128;
        if packets > self.limits.max_packets.get() as u128 {
            Some(LimitAxis::Packets)
        } else if bytes > self.limits.max_bytes.get() as u128 {
            Some(LimitAxis::Bytes)
        } else if duration > self.limits.max_duration.0 as u128 {
            Some(LimitAxis::Duration)
        } else {
            None
        }
    }
}

fn check_q1(queue: &BoundedQueue<Blob>, limits: &QueueLimits, context: &str) {
    let usage = queue.usage();
    assert!(
        usage.packets <= limits.max_packets.get(),
        "{context}: Q1 packets {} > {}",
        usage.packets,
        limits.max_packets.get()
    );
    assert!(
        usage.bytes <= limits.max_bytes.get(),
        "{context}: Q1 bytes {} > {}",
        usage.bytes,
        limits.max_bytes.get()
    );
    assert!(
        usage.duration.0 <= limits.max_duration.0,
        "{context}: Q1 duration {} > {}",
        usage.duration.0,
        limits.max_duration.0
    );
    // The packets axis and the item count can never drift.
    assert_eq!(
        usage.packets as u64,
        queue.stats().pushed - queue.stats().popped - queue.stats().flushed,
        "{context}: usage.packets disagrees with pushed - popped - flushed"
    );
}

/// Q1 + Q2 under adversarial (seeded, reproducible) interleavings of
/// push / pop / flush on tight caps — the deterministic core of doc 03
/// §4.4(f)'s "model-check a small state machine". Threaded races are
/// covered separately in `tests/concurrency.rs`.
#[test]
fn q1_q2_hold_under_adversarial_interleavings() {
    const SEEDS: [u64; 8] = [1, 2, 3, 4, 5, 6, 7, 0x9E37_79B9_7F4A_7C15];
    const OPS: u64 = 3_000;

    for seed in SEEDS {
        let limits = QueueLimits::new(
            NonZeroUsize::new(4).must(),
            NonZeroUsize::new(40).must(),
            MediaDuration::Known(Nanoseconds(400)),
        )
        .must();
        let queue = BoundedQueue::<Blob>::with_default_watermarks(limits);
        let mut model = Model {
            items: VecDeque::new(),
            published: GenerationId::FIRST,
            limits,
        };
        let mut rng = XorShift(seed | 1);
        let mut next_id = 0u64;
        let mut markers_observed: u64 = 0;
        let mut total_flushed: u64 = 0;

        for op in 0..OPS {
            let context = format!("seed {seed} op {op}");
            match rng.below(100) {
                // ---- push (fresh, future or stale stamp) ----
                r if r < 45 => {
                    let size = (rng.below(15) + 1) as usize;
                    let dur = rng.below(150);
                    let stamp = match rng.below(10) {
                        // 7/10 current generation, 2/10 future, 1/10 stale.
                        0..=6 => model.published,
                        7..=8 => GenerationId::new(model.published.counter() + rng.below(3) + 1),
                        _ if model.published.counter() > 0 => {
                            GenerationId::new(model.published.counter() - 1)
                        }
                        _ => model.published,
                    };
                    let item = Stamped::new(
                        stamp,
                        Blob {
                            id: next_id,
                            size,
                            dur_ns: dur,
                        },
                    );
                    next_id += 1;
                    match queue.try_push(item) {
                        PushOutcome::Pushed => {
                            assert!(
                                stamp >= model.published,
                                "{context}: accepted a stale-stamped push"
                            );
                            let fits = model.fits(size, dur);
                            assert!(
                                fits.is_none(),
                                "{context}: accepted a push the oracle says is full ({fits:?})"
                            );
                            model.items.push_back((stamp, item.value.id, size, dur));
                        }
                        PushOutcome::Full { item, binding } => {
                            let oracle = model.fits(item.value.size, item.value.dur_ns);
                            assert_eq!(
                                oracle,
                                Some(binding),
                                "{context}: Full axis {binding:?} disagrees with the oracle {oracle:?}"
                            );
                            assert_eq!(
                                queue.usage().packets,
                                model.packets(),
                                "{context}: model/queue packet divergence"
                            );
                        }
                        PushOutcome::Stale { item, current } => {
                            assert!(
                                item.generation < model.published,
                                "{context}: Stale verdict for a non-stale stamp"
                            );
                            assert_eq!(current, model.published);
                        }
                        PushOutcome::Closed { .. } => {
                            unreachable!("{context}: nothing closes this queue");
                        }
                    }
                }
                // ---- pop ----
                r if r < 80 => match queue.try_pop() {
                    PopOutcome::Item(stamped) => {
                        let front = model.items.pop_front().must_or_else(|| {
                            format!("{context}: queue yielded an item the oracle does not have")
                        });
                        assert_eq!(stamped.value.id, front.1, "{context}: FIFO/no-loss/no-dup");
                        assert_eq!(
                            stamped.generation, front.0,
                            "{context}: item generation mismatch"
                        );
                        // Q2: nothing older than the published
                        // generation can be popped.
                        assert!(
                            stamped.generation >= model.published,
                            "{context}: Q2 violated — popped a stale item"
                        );
                    }
                    PopOutcome::Discontinuity(gap) => {
                        assert_eq!(gap.new_generation, model.published, "{context}: marker gen");
                        assert!(
                            gap.dropped <= total_flushed - markers_observed,
                            "{context}: marker claims more drops than happened"
                        );
                        markers_observed += gap.dropped;
                    }
                    PopOutcome::Empty => {
                        assert!(
                            model.items.is_empty(),
                            "{context}: Empty while the oracle holds {} items",
                            model.packets()
                        );
                    }
                    PopOutcome::Closed => unreachable!("{context}: nothing closes this queue"),
                },
                // ---- flush (advance, stutter or regress) ----
                r if r < 95 => {
                    let target = match rng.below(10) {
                        0..=7 => GenerationId::new(model.published.counter() + rng.below(3) + 1),
                        8 => model.published,
                        _ => GenerationId::new(model.published.counter() / 2),
                    };
                    let gap = queue.flush(target);
                    let dropped = model.packets() as u64;
                    assert_eq!(gap.dropped, dropped, "{context}: flush drop count");
                    assert_eq!(gap.dropped_bytes, model.bytes(), "{context}: flush bytes");
                    assert_eq!(
                        gap.dropped_duration,
                        model.duration(),
                        "{context}: flush duration"
                    );
                    model.items.clear();
                    // Published generation never regresses.
                    let published = if target > model.published {
                        target
                    } else {
                        model.published
                    };
                    assert_eq!(gap.new_generation, published, "{context}: flush publish");
                    model.published = published;
                    total_flushed += dropped;
                }
                // ---- invariant sampling ----
                _ => {
                    check_q1(&queue, &limits, &context);
                    assert_eq!(
                        queue.generation(),
                        model.published,
                        "{context}: published gen"
                    );
                }
            }
            // Q1 after EVERY operation, not just sampled ones.
            check_q1(&queue, &limits, &context);
        }

        // Final drain: every surviving item pops, FIFO; the marker
        // accounting closes exactly.
        let mut drained = Vec::new();
        loop {
            match queue.try_pop() {
                PopOutcome::Item(stamped) => {
                    let front = model
                        .items
                        .pop_front()
                        .must_or_else(|| format!("seed {seed}: drain yielded an unmodeled item"));
                    assert_eq!(stamped.value.id, front.1, "seed {seed}: drain FIFO");
                    assert!(
                        stamped.generation >= model.published,
                        "seed {seed}: drain Q2"
                    );
                    drained.push(stamped.value.id);
                }
                PopOutcome::Discontinuity(gap) => {
                    markers_observed += gap.dropped;
                }
                PopOutcome::Empty => break,
                PopOutcome::Closed => unreachable!("nothing closes this queue"),
            }
        }
        assert!(
            model.items.is_empty(),
            "seed {seed}: oracle still holds {} items after drain",
            model.packets()
        );
        assert_eq!(
            markers_observed, total_flushed,
            "seed {seed}: marker accounting does not close"
        );
        let stats = queue.stats();
        assert_eq!(
            stats.pushed,
            stats.popped + stats.flushed,
            "seed {seed}: conservation pushed == popped + flushed"
        );
    }
}

/// Doc 03 §4.4(f): unknown-duration items (0 toward the duration cap)
/// must not wedge the `max_duration` computation — they still count in
/// packets and bytes. All three clauses of the doc's unit, one per
/// phase:
///
/// 1. a tight byte cap refuses unknown-duration items on the *byte*
///    axis (they still count in bytes), while the resident duration
///    stays at zero;
/// 2. with bytes plentiful, the packet cap alone governs a stream of
///    unknown-duration items — no wedge behind a duration that never
///    grows;
/// 3. the duration cap bites the moment a *known* duration would
///    exceed it (with packet room available, so the refusal can only
///    be the duration axis).
#[test]
fn unknown_duration_items_do_not_wedge_the_duration_cap() {
    let untimed = |id: u64| {
        Stamped::new(
            GenerationId::FIRST,
            Blob {
                id,
                size: 1,
                dur_ns: 0,
            },
        )
    };

    // Phase 1 — bytes still count (32 × 1-byte items fill a 32-byte cap).
    let byte_tight = QueueLimits::new(
        NonZeroUsize::new(64).must(),
        NonZeroUsize::new(32).must(),
        MediaDuration::Known(Nanoseconds(10)),
    )
    .must();
    let queue = BoundedQueue::<Blob>::with_default_watermarks(byte_tight);
    for id in 0..32 {
        assert_eq!(queue.try_push(untimed(id)), PushOutcome::Pushed, "id {id}");
    }
    match queue.try_push(untimed(32)) {
        PushOutcome::Full { binding, .. } => assert_eq!(binding, LimitAxis::Bytes),
        other => unreachable!("expected Full, got {other:?}"),
    }
    assert_eq!(
        queue.usage().duration,
        Nanoseconds(0),
        "unknown durations weigh 0 toward the duration cap"
    );

    // Phase 2 — the packet cap governs alone (64 of them, duration 0).
    let packet_bound = QueueLimits::new(
        NonZeroUsize::new(64).must(),
        NonZeroUsize::new(1_000).must(),
        MediaDuration::Known(Nanoseconds(10)),
    )
    .must();
    let queue = BoundedQueue::<Blob>::with_default_watermarks(packet_bound);
    for id in 0..64 {
        assert_eq!(queue.try_push(untimed(id)), PushOutcome::Pushed, "id {id}");
    }
    assert_eq!(queue.usage().duration, Nanoseconds(0));
    assert_eq!(queue.usage().packets, 64);
    match queue.try_push(untimed(64)) {
        PushOutcome::Full { binding, .. } => assert_eq!(binding, LimitAxis::Packets),
        other => unreachable!("expected Full, got {other:?}"),
    }

    // Phase 3 — free one packet slot, then a known 11 ns item exceeds
    // the 10 ns cap: only the duration axis can refuse it (63 packets,
    // 63 bytes resident; 64 ≤ 64 and 64 ≤ 1000 both fit).
    match queue.try_pop() {
        PopOutcome::Item(item) => assert_eq!(item.value.id, 0),
        other => unreachable!("expected Item, got {other:?}"),
    }
    let timed = Stamped::new(
        GenerationId::FIRST,
        Blob {
            id: 65,
            size: 1,
            dur_ns: 11,
        },
    );
    match queue.try_push(timed) {
        PushOutcome::Full { binding, .. } => assert_eq!(binding, LimitAxis::Duration),
        other => unreachable!("expected Full, got {other:?}"),
    }
    assert_eq!(
        queue.usage().duration,
        Nanoseconds(0),
        "the refused timed item added nothing"
    );
}

/// The unbounded-impossible contract, end to end: every constructor
/// path yields strictly positive caps, and the known degenerate
/// configurations are refused with typed errors.
#[test]
fn every_constructed_queue_is_bounded() {
    for limits in [
        QueueLimits::audio_default(NonZeroUsize::new(1 << 20).must()),
        QueueLimits::video_default(NonZeroUsize::new(4 << 20).must()),
        QueueLimits::new(
            NonZeroUsize::new(1).must(),
            NonZeroUsize::new(1).must(),
            MediaDuration::Known(Nanoseconds(1)),
        )
        .must(),
    ] {
        assert!(limits.max_packets.get() > 0);
        assert!(limits.max_bytes.get() > 0);
        assert!(limits.max_duration.0 > 0);
        // And the queue it builds reports the same caps (Q1 bounds).
        let queue = BoundedQueue::<Blob>::with_default_watermarks(limits);
        assert_eq!(queue.limits(), limits);
        assert_eq!(queue.usage(), QueueUsage::EMPTY);
    }
    for bad in [
        (MediaDuration::Unknown, QueueConfigError::UnknownDuration),
        (
            MediaDuration::Known(Nanoseconds(0)),
            QueueConfigError::ZeroDuration,
        ),
    ] {
        // The exact typed error, at the integration level (the unit
        // proofs in src/limits.rs pin the Display strings).
        assert_eq!(
            QueueLimits::new(
                NonZeroUsize::new(4).must(),
                NonZeroUsize::new(40).must(),
                bad.0
            ),
            Err(bad.1),
            "unbounded config must be refused with the typed error"
        );
    }
}

/// Small `Option` helper with a lazily-built context message
/// (testkit's `Must` carries no context).
trait MustOrElse<T> {
    fn must_or_else(self, context: impl FnOnce() -> String) -> T;
}

impl<T> MustOrElse<T> for Option<T> {
    fn must_or_else(self, context: impl FnOnce() -> String) -> T {
        match self {
            Some(value) => value,
            None => unreachable!("{}", context()),
        }
    }
}
