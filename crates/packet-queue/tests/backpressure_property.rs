//! Card 3.B3's queue-level property suite (doc 13 §6 row 3.B3 AC:
//! "property tests with synthetic producers/consumers — no deadlock,
//! bounded memory under burst", doc 03 §8): the seeded model driver
//! extended with the backpressure vocabulary and the engine-shaped
//! watermark laws.
//!
//! Convention (the repo's own, `limits.rs`): hand-rolled seeded
//! XorShift + a model oracle, single-threaded, no proptest (not in
//! Cargo.lock; C5). Every failure prints its seed. The fixed seed list
//! runs in every gate; `LUMEN_PROP_SEEDS=<n>` widens the sweep (the
//! corpus suites' env-gated precedent).

use std::num::NonZeroUsize;

use media_types::ids::GenerationId;
use media_types::time::{Duration as MediaDuration, Nanoseconds};
use packet_queue::{
    BoundedQueue, LimitAxis, PopOutcome, PushAborted, PushOutcome, QueueLimits, QueueUsage,
    Stamped, Watermark, WatermarkThresholds, Weigh,
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

/// Deterministic xorshift* — a failed seed reruns the exact
/// interleaving (the `limits.rs` discipline).
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

/// The engine-shaped thresholds of card 3.B3 (`QueuePlan::thresholds`
/// in playback-engine, mirrored here so the queue crate's property
/// suite stays engine-agnostic): high = cap − cap/4 on every axis;
/// **low binds on packets alone** — the byte/duration low marks sit at
/// their caps, never binding.
fn engine_thresholds(limits: &QueueLimits) -> WatermarkThresholds {
    let high = QueueUsage {
        packets: limits.max_packets.get() - limits.max_packets.get() / 4,
        bytes: limits.max_bytes.get() - limits.max_bytes.get() / 4,
        duration: Nanoseconds(limits.max_duration.0 - limits.max_duration.0 / 4),
    };
    let low = QueueUsage {
        packets: ((limits.max_packets.get() as u64) / 4).max(1) as usize,
        bytes: limits.max_bytes.get(),
        duration: limits.max_duration,
    };
    WatermarkThresholds::new(high, low)
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
}

/// The conservation law as a per-step identity (doc 03 §8; the
/// `limits.rs` end-of-run check promoted to an invariant):
/// `pushed == popped + resident + flushed` — every accepted item is
/// exactly one of {popped, resident, flushed}. `stale_rejected` and
/// `full_rejections` count *rejections* (items that never entered)
/// and stay outside the identity.
fn check_conservation(queue: &BoundedQueue<Blob>, context: &str) {
    let stats = queue.stats();
    let resident = queue.usage().packets as u64;
    assert_eq!(
        stats.pushed,
        stats.popped + resident + stats.flushed,
        "{context}: conservation pushed == popped + resident + flushed"
    );
}

/// The hysteresis laws under engine-shaped thresholds (design B2/E1):
/// a `low` flag implies the packets axis is at/below the stall line;
/// a `high` flag implies some axis reached the resume line.
fn check_hysteresis(
    queue: &BoundedQueue<Blob>,
    marks: &WatermarkThresholds,
    context: &str,
) {
    let mark = queue.watermark();
    if mark.low {
        assert!(
            mark.usage.packets <= marks.low.packets,
            "{context}: low with {} packets > stall line {}",
            mark.usage.packets,
            marks.low.packets
        );
    }
    if mark.high {
        let reached = mark.usage.packets >= marks.high.packets
            || mark.usage.bytes >= marks.high.bytes
            || mark.usage.duration.0 >= marks.high.duration.0;
        assert!(
            reached,
            "{context}: high without any axis at the resume line ({:?})",
            mark.usage
        );
    }
}

/// The seeds every gate runs, plus the env-gated sweep.
fn seeds() -> Vec<u64> {
    let mut seeds = vec![1, 2, 3, 4, 5, 6, 7, 0x9E37_79B9_7F4A_7C15];
    if let Ok(extra) = std::env::var("LUMEN_PROP_SEEDS") {
        for part in extra.split(',') {
            if let Ok(seed) = part.trim().parse::<u64>() {
                seeds.push(seed | 1);
            }
        }
    }
    seeds
}

/// The extended model driver (design E1): adversarial interleavings of
/// the full backpressure vocabulary — try-push, the *bounded park*
/// (`push_wait` under an already-expired deadline: the deterministic,
/// sleep-free form of the parking producer's deadline path), try-pop,
/// the bounded parking pop, flush, close, and **burst mode** (a run of
/// K pushes before any pop — the burst shape of the AC) — against the
/// oracle, with Q1 + conservation + hysteresis checked after **every**
/// step.
#[test]
fn backpressure_vocabulary_holds_q1_conservation_and_hysteresis() {
    const OPS: u64 = 4_000;

    for seed in seeds() {
        let context = format!("seed {seed}");
        let limits = QueueLimits::new(
            NonZeroUsize::new(8).must(),
            NonZeroUsize::new(96).must(),
            MediaDuration::Known(Nanoseconds(64)),
        )
        .must();
        let marks = engine_thresholds(&limits);
        // The 3.B3 engine-shaped thresholds (packets-axis low); the
        // default quarter marks stay exercised by `limits.rs`.
        let queue = BoundedQueue::<Blob>::new(limits, marks);
        let mut closed = false;
        let mut rng = XorShift(seed | 1);
        let mut next_id = 0u64;
        let mut burst_budget: u32 = 0;

        for op in 0..OPS {
            let step = format!("{context} op {op}");
            // Burst mode: while a burst is running, pushes continue
            // with no interleaved pops (the under-burst shape).
            let in_burst = burst_budget > 0;
            if in_burst {
                burst_budget -= 1;
            }
            match if in_burst { 0 } else { rng.below(100) } {
                // ---- try-push (fresh/future/stale stamps) ----
                r if r < 40 || in_burst && r < 100 => {
                    let size = (rng.below(24) + 1) as usize;
                    let dur = if rng.below(4) == 0 { 0 } else { rng.below(9) };
                    let stamp = if rng.below(10) == 0 && queue.generation().counter() > 0 {
                        GenerationId::new(queue.generation().counter() - 1)
                    } else {
                        queue.generation()
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
                        PushOutcome::Pushed => {}
                        PushOutcome::Full { item, binding } => {
                            // The oracle's binding axis must agree.
                            let usage = queue.usage();
                            let packets = (usage.packets + 1) as u128;
                            let bytes = usage.bytes as u128 + item.value.size as u128;
                            let duration = usage.duration.0 as u128 + item.value.dur_ns as u128;
                            let oracle = if packets > limits.max_packets.get() as u128 {
                                Some(LimitAxis::Packets)
                            } else if bytes > limits.max_bytes.get() as u128 {
                                Some(LimitAxis::Bytes)
                            } else if duration > limits.max_duration.0 as u128 {
                                Some(LimitAxis::Duration)
                            } else {
                                None
                            };
                            assert_eq!(
                                oracle,
                                Some(binding),
                                "{step}: Full axis {binding:?} disagrees with the oracle {oracle:?}"
                            );
                        }
                        PushOutcome::Stale { .. } => {}
                        PushOutcome::Closed { .. } => assert!(closed, "{step}: unexpected close"),
                    }
                }
                // ---- bounded park: push_wait under an expired
                // deadline (the deadline path without sleeping) ----
                r if r < 55 => {
                    let item = Stamped::new(
                        queue.generation(),
                        Blob {
                            id: next_id,
                            size: 4,
                            dur_ns: 2,
                        },
                    );
                    next_id += 1;
                    let outcome = queue.push_wait(item, Some(std::time::Instant::now()));
                    if !closed {
                        // With an expired deadline the park is bounded
                        // by the deadline check: the item is either
                        // accepted (room existed) or handed back.
                        match outcome {
                            Ok(()) => {}
                            Err(PushAborted::Deadline { .. }) => {}
                            Err(PushAborted::Closed { .. }) => assert!(closed),
                            Err(PushAborted::Stale { .. }) => {}
                        }
                    }
                }
                // ---- try-pop ----
                r if r < 85 => match queue.try_pop() {
                    PopOutcome::Item(_) => {}
                    PopOutcome::Discontinuity(_) => {}
                    PopOutcome::Empty => {}
                    PopOutcome::Closed => assert!(closed, "{step}: unexpected close"),
                },
                // ---- bounded parking pop (expired deadline) ----
                r if r < 92 => {
                    match queue.pop_wait(Some(std::time::Instant::now())) {
                        PopOutcome::Item(_) => {}
                        PopOutcome::Discontinuity(_) => {}
                        PopOutcome::Empty => {}
                        PopOutcome::Closed => assert!(closed, "{step}: unexpected close"),
                    }
                }
                // ---- flush (advance/stutter/regress) ----
                r if r < 97 => {
                    let current = queue.generation();
                    let target = match rng.below(4) {
                        0 => current,
                        1 => GenerationId::new(current.counter() / 2),
                        _ => GenerationId::new(current.counter() + rng.below(2) + 1),
                    };
                    let _ = queue.flush(target);
                }
                // ---- close (terminal; one per run) ----
                r if r < 98 && !closed => {
                    queue.close();
                    closed = true;
                }
                // ---- arm a burst of 1..6 pushes ----
                _ => {
                    burst_budget = (rng.below(6) + 1) as u32;
                }
            }
            check_q1(&queue, &limits, &step);
            check_conservation(&queue, &step);
            check_hysteresis(&queue, &marks, &step);
        }

        // Terminal conservation: with the queue closed and drained the
        // identity still closes exactly.
        if closed {
            while !matches!(queue.try_pop(), PopOutcome::Closed) {}
        } else {
            queue.close();
            while !matches!(queue.try_pop(), PopOutcome::Closed) {}
        }
        check_conservation(&queue, &context);
        let stats = queue.stats();
        assert_eq!(queue.usage().packets, 0, "{context}: drained");
        assert_eq!(
            stats.pushed,
            stats.popped + stats.flushed,
            "{context}: terminal conservation (rejections never entered)"
        );
    }
}

/// The unreachable-low fix (design B2 gap 2, the property form): under
/// engine-shaped thresholds, **every** payload mix reaches `low` by
/// packet count alone — including the two wedge cases the all-axes low
/// could strand (bytes-heavy items far above the byte stall line, and
/// unknown-duration items that never grow the duration axis).
#[test]
fn engine_shaped_low_is_reachable_for_every_mix() {
    for seed in seeds() {
        let limits = QueueLimits::new(
            NonZeroUsize::new(16).must(),
            NonZeroUsize::new(1 << 20).must(),
            MediaDuration::Known(Nanoseconds(1_000)),
        )
        .must();
        let marks = engine_thresholds(&limits);
        let queue = BoundedQueue::<Blob>::new(limits, marks);
        let stall_line = marks.low.packets;

        // Wedge mix A: bytes-heavy, duration-less (size grows, dur 0).
        for id in 0..stall_line as u64 {
            let item = Stamped::new(
                GenerationId::FIRST,
                Blob {
                    id,
                    size: 4096,
                    dur_ns: 0,
                },
            );
            assert_eq!(queue.try_push(item), PushOutcome::Pushed);
        }
        assert!(
            queue.watermark().low,
            "seed {seed}: a bytes-heavy duration-less mix at {stall_line} packets must be low"
        );

        // Drain, then wedge mix B: huge single items (one item dwarfs
        // the byte stall line, duration axis untouched).
        while matches!(queue.try_pop(), PopOutcome::Item(_)) {}
        for id in 0..stall_line as u64 {
            let item = Stamped::new(
                GenerationId::FIRST,
                Blob {
                    id,
                    size: 262_144,
                    dur_ns: 0,
                },
            );
            match queue.try_push(item) {
                PushOutcome::Pushed => {}
                // The byte cap may refuse the giant items — the low
                // flag must still be reachable through the packets
                // that did land.
                PushOutcome::Full { .. } => break,
                other => unreachable!("seed {seed}: {other:?}"),
            }
        }
        let usage = queue.usage();
        let mark = Watermark::evaluate(usage, &marks);
        if usage.packets <= stall_line {
            assert!(
                mark.low,
                "seed {seed}: {} packets ≤ stall line {stall_line} must be low",
                usage.packets
            );
        }

        // And the watermark flags track the fill/drain cycle under the
        // engine's marks (the E1 hysteresis shape, burst-flavored):
        // flood to high, drain to low, repeatedly.
        let mut rng = XorShift(seed | 1);
        for _ in 0..50 {
            // Burst-flood.
            for id in 0..16u64 {
                let item = Stamped::new(
                    GenerationId::FIRST,
                    Blob {
                        id,
                        size: 1,
                        dur_ns: 1,
                    },
                );
                if !matches!(queue.try_push(item), PushOutcome::Pushed) {
                    break;
                }
            }
            let mark = queue.watermark();
            if mark.usage.packets >= marks.high.packets {
                assert!(mark.high, "seed {seed}: at the resume line, high");
            }
            check_hysteresis(&queue, &marks, &format!("seed {seed} flood"));
            // Seeded drain.
            let pops = rng.below(8);
            for _ in 0..pops {
                if !matches!(queue.try_pop(), PopOutcome::Item(_)) {
                    break;
                }
            }
            check_hysteresis(&queue, &marks, &format!("seed {seed} drain"));
            check_q1(&queue, &limits, &format!("seed {seed} drain"));
            check_conservation(&queue, &format!("seed {seed} drain"));
        }
    }
}
