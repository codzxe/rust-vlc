//! Card 3.D3's queue-primitive property suite (doc 03 §8 "Queue
//! bounds under adversarial interleavings §4.4(f)" + doc 13 wave 3.D
//! AC): the bounded, generation-aware queue driven by seeded
//! adversarial interleavings, against an exact model oracle.
//!
//! What this file pins that `backpressure_property.rs` (card 3.B3)
//! does not:
//!
//! - **exact accounting** — a resident model of every accepted item
//!   (generation, id, bytes, duration) is checked against
//!   `usage()`/`stats()` after **every** op: resident packets, bytes
//!   and duration are exactly the model's sums, and the conservation
//!   identity `pushed == popped + resident + flushed` holds per op;
//! - **N ≫ cap** — caps of 1/3/8/13 packets against 6000 ops per
//!   seed (memory bounded: `len ≤ cap` after every one of them);
//! - **stale generations always dropped** — pushes stamped below the
//!   published generation return the typed `Stale` signal with the
//!   item untouched and never enter (a dedicated exhaustive sweep
//!   beside the interleavings); a popped item is never stale;
//! - **FIFO within the published generation** — pop order is exactly
//!   the model's push order;
//! - **flush regression never unpublishes** — a `new_generation` at
//!   or below the current one still drops the residents but leaves
//!   the high-water mark published; merged consumer-side
//!   discontinuities accumulate, never vanish;
//! - **no deadlock** — a threaded storm (parking producers and
//!   consumers under generation movement and close) must terminate
//!   under a watchdog, seeded from the bank.
//!
//! # Determinism discipline (the `limits.rs` convention)
//!
//! Hand-rolled seeded XorShift + model oracle, single-threaded, no
//! proptest (not in Cargo.lock; C5). The seed bank is the first 64
//! odd integers (the playback-engine `tests/property/rng.rs` bank,
//! duplicated here because integration tests are separate crates);
//! `LUMEN_PROP_SEEDS=<csv>` widens the sweep. On failure the test
//! prints its seed and op index; the documented shrink procedure is
//! bisection on the failing op sequence — rerun with
//! `LUMEN_PROP_SHRINK=<seed>` and the driver reports the minimal
//! failing op prefix (each rerun rebuilds the case from the seed
//! alone, so bisection is deterministic).

use std::num::NonZeroUsize;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use media_types::ids::GenerationId;
use media_types::time::{Duration as MediaDuration, Nanoseconds};
use packet_queue::{
    BoundedQueue, Discontinuity, LimitAxis, PopOutcome, PushAborted, PushOutcome, QueueLimits,
    Stamped, Weigh,
};
use testkit::Must;

// ---------------------------------------------------------------------------
// The seeded harness (duplicated from playback-engine's
// tests/property/rng.rs — integration tests are separate crates)
// ---------------------------------------------------------------------------

struct XorShift(u64);

impl XorShift {
    fn new(seed: u64) -> XorShift {
        XorShift(seed | 1)
    }

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

/// The fixed bank: the first 64 odd integers.
fn seed_bank() -> Vec<u64> {
    let mut seeds: Vec<u64> = (0..64).map(|index| index * 2 + 1).collect();
    if let Ok(extra) = std::env::var("LUMEN_PROP_SEEDS") {
        for part in extra.split(',') {
            if let Ok(seed) = part.trim().parse::<u64>() {
                seeds.push(seed | 1);
            }
        }
    }
    seeds
}

fn shrink_requested(seed: u64) -> bool {
    match std::env::var("LUMEN_PROP_SHRINK") {
        Ok(requested) => requested.trim().parse::<u64>() == Ok(seed),
        Err(_) => false,
    }
}

/// The documented shrink procedure: bisection on the op prefix —
/// log₂ N deterministic reruns to the minimal failing prefix.
fn shrink_prefix<T: Clone>(ops: &[T], still_fails: impl Fn(&[T]) -> bool) -> Vec<T> {
    if ops.is_empty() || !still_fails(ops) {
        return Vec::new();
    }
    let mut lo = 1usize;
    let mut hi = ops.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if still_fails(&ops[..mid]) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    ops[..lo].to_vec()
}

fn run_case<T: Clone + std::fmt::Debug>(
    seed: u64,
    ops: &[T],
    run: impl Fn(u64, &[T]) -> Result<(), String>,
) -> Result<(), String> {
    match run(seed, ops) {
        Ok(()) => Ok(()),
        Err(failure) => {
            if shrink_requested(seed) {
                let minimal = shrink_prefix(ops, |prefix| run(seed, prefix).is_err());
                Err(format!(
                    "{failure}\n[shrunk] seed {seed}: minimal failing prefix {}/{} ops: \
                     {minimal:?}",
                    minimal.len(),
                    ops.len()
                ))
            } else {
                Err(format!(
                    "{failure}\n[rerun] LUMEN_PROP_SEEDS={seed} to reproduce; \
                     LUMEN_PROP_SHRINK={seed} to shrink"
                ))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The payload
// ---------------------------------------------------------------------------

/// A weighed payload with a distinct id (identity for FIFO checks).
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

// ---------------------------------------------------------------------------
// The op stream and the model
// ---------------------------------------------------------------------------

/// One adversarial queue op.
#[derive(Debug, Clone, Copy)]
enum Op {
    TryPush {
        stamp_offset: i64,
        size: usize,
        dur_ns: u64,
    },
    ParkPush {
        size: usize,
        dur_ns: u64,
    },
    TryPop,
    ParkPop,
    Flush {
        advance: u64,
        regress: bool,
    },
    Close,
}

fn generate_ops(rng: &mut XorShift, count: usize) -> Vec<Op> {
    (0..count)
        .map(|_| match rng.below(100) {
            r if r < 46 => Op::TryPush {
                stamp_offset: match rng.below(10) {
                    0 => -1,
                    1 => -2,
                    2 => 1,
                    3 => 5,
                    _ => 0,
                },
                size: (rng.below(24) + 1) as usize,
                dur_ns: if rng.below(4) == 0 { 0 } else { rng.below(9) },
            },
            r if r < 58 => Op::ParkPush {
                size: (rng.below(8) + 1) as usize,
                dur_ns: rng.below(4),
            },
            r if r < 84 => Op::TryPop,
            r if r < 92 => Op::ParkPop,
            r if r < 98 => Op::Flush {
                advance: rng.below(3),
                regress: rng.below(3) == 0,
            },
            _ => Op::Close,
        })
        .collect()
}

/// The resident model: exactly what the queue must hold.
#[derive(Default)]
struct Model {
    residents: Vec<(u64, Blob)>,
    published: u64,
    pushed: u64,
    popped: u64,
    flushed: u64,
    stale_rejected: u64,
    full_rejections: u64,
    closed: bool,
    /// The merged, not-yet-observed wholesale drop of flushes.
    pending_gap: Option<(u64, usize, u64)>,
    next_id: u64,
}

impl Model {
    fn bytes(&self) -> usize {
        self.residents.iter().map(|(_, blob)| blob.size).sum()
    }

    fn duration(&self) -> u64 {
        self.residents.iter().map(|(_, blob)| blob.dur_ns).sum()
    }
}

/// Run one seed's op stream against a fresh queue, checking every
/// law after every op. `cap_packets` parametrizes the N ≫ cap shape.
fn run(seed: u64, cap_packets: usize, ops: &[Op]) -> Result<(), String> {
    let canonical = ops.is_empty();
    let mut rng = XorShift::new(seed);
    let ops = if canonical {
        generate_ops(&mut rng, 6000)
    } else {
        ops.to_vec()
    };
    let limits = QueueLimits::new(
        NonZeroUsize::new(cap_packets).must(),
        NonZeroUsize::new(64).must(),
        MediaDuration::Known(Nanoseconds(96)),
    )
    .must();
    let queue: BoundedQueue<Blob> = BoundedQueue::with_default_watermarks(limits);
    // The queue publishes FIRST on construction; the model starts
    // there and only moves on flushes.
    let mut model = Model {
        published: queue.generation().counter(),
        ..Model::default()
    };

    for (index, op) in ops.iter().enumerate() {
        let step = format!("seed {seed} cap {cap_packets} op {index} ({op:?})");
        match *op {
            Op::TryPush {
                stamp_offset,
                size,
                dur_ns,
            } => {
                let stamp =
                    GenerationId::new((model.published as i64 + stamp_offset).max(0) as u64);
                let blob = Blob {
                    id: model.next_id,
                    size,
                    dur_ns,
                };
                model.next_id += 1;
                let item = Stamped::new(stamp, blob);
                let fits = (model.residents.len() + 1) <= limits.max_packets.get()
                    && model.bytes() + size <= limits.max_bytes.get()
                    && model.duration() + dur_ns <= limits.max_duration.0;
                match queue.try_push(item) {
                    PushOutcome::Pushed => {
                        if stamp.counter() < model.published || !fits || model.closed {
                            return Err(format!(
                                "{step}: Pushed a stamp {}/{}, fits={fits}, closed={} — the \
                                 oracle disagrees",
                                stamp.counter(),
                                model.published,
                                model.closed
                            ));
                        }
                        model.residents.push((stamp.counter(), blob));
                        model.pushed += 1;
                    }
                    PushOutcome::Full { item, binding } => {
                        // Full is the answer exactly when the item is
                        // current, the queue is open, and a cap would
                        // be violated — anything else disagrees.
                        if fits || model.closed || stamp.counter() < model.published {
                            return Err(format!(
                                "{step}: Full while fits={fits}, closed={}, stale={} — the \
                                 oracle disagrees",
                                model.closed,
                                stamp.counter() < model.published
                            ));
                        }
                        // The binding axis must be the first violated
                        // cap, checked in u128 like the crate's own.
                        let packets = (model.residents.len() + 1) as u128;
                        let bytes = model.bytes() as u128 + size as u128;
                        let duration = model.duration() as u128 + dur_ns as u128;
                        let oracle = if packets > limits.max_packets.get() as u128 {
                            Some(LimitAxis::Packets)
                        } else if bytes > limits.max_bytes.get() as u128 {
                            Some(LimitAxis::Bytes)
                        } else if duration > limits.max_duration.0 as u128 {
                            Some(LimitAxis::Duration)
                        } else {
                            None
                        };
                        if Some(binding) != oracle {
                            return Err(format!(
                                "{step}: Full axis {binding:?} ≠ oracle {oracle:?}"
                            ));
                        }
                        if item.value != blob {
                            return Err(format!("{step}: Full handed back a mutated item"));
                        }
                        model.full_rejections += 1;
                    }
                    PushOutcome::Stale { item, current } => {
                        if stamp.counter() >= model.published {
                            return Err(format!(
                                "{step}: Stale for stamp {} ≥ published {} — only strictly \
                                 past generations are stale",
                                stamp.counter(),
                                model.published
                            ));
                        }
                        if current.counter() != model.published {
                            return Err(format!(
                                "{step}: Stale reports current {}, published is {}",
                                current.counter(),
                                model.published
                            ));
                        }
                        if item.value != blob {
                            return Err(format!("{step}: Stale handed back a mutated item"));
                        }
                        model.stale_rejected += 1;
                    }
                    PushOutcome::Closed { item } => {
                        if !model.closed {
                            return Err(format!("{step}: Closed while the model is open"));
                        }
                        if item.value != blob {
                            return Err(format!("{step}: Closed handed back a mutated item"));
                        }
                    }
                }
            }
            Op::ParkPush { size, dur_ns } => {
                // The bounded park under an already-expired deadline:
                // deterministic, sleep-free — the no-deadlock shape.
                let stamp = GenerationId::new(model.published);
                let blob = Blob {
                    id: model.next_id,
                    size,
                    dur_ns,
                };
                model.next_id += 1;
                let fits = (model.residents.len() + 1) <= limits.max_packets.get()
                    && model.bytes() + size <= limits.max_bytes.get()
                    && model.duration() + dur_ns <= limits.max_duration.0;
                match queue.push_wait(Stamped::new(stamp, blob), Some(Instant::now())) {
                    Ok(()) => {
                        if !fits || model.closed {
                            return Err(format!(
                                "{step}: park accepted while fits={fits}, closed={}",
                                model.closed
                            ));
                        }
                        model.residents.push((model.published, blob));
                        model.pushed += 1;
                    }
                    Err(PushAborted::Deadline { item }) => {
                        if fits && !model.closed {
                            return Err(format!("{step}: Deadline abort while the item fit"));
                        }
                        if item.value != blob {
                            return Err(format!("{step}: Deadline handed back a mutated item"));
                        }
                        model.full_rejections += 1;
                    }
                    Err(PushAborted::Stale { item, current }) => {
                        if item.generation.counter() >= model.published {
                            return Err(format!("{step}: Stale abort for a current stamp"));
                        }
                        if current.counter() != model.published {
                            return Err(format!("{step}: Stale abort reports the wrong current"));
                        }
                        model.stale_rejected += 1;
                    }
                    Err(PushAborted::Closed { item }) => {
                        if !model.closed {
                            return Err(format!("{step}: Closed abort while the model is open"));
                        }
                        if item.value != blob {
                            return Err(format!("{step}: Closed abort mutated the item"));
                        }
                    }
                }
            }
            Op::TryPop | Op::ParkPop => {
                let outcome = if matches!(op, Op::TryPop) {
                    queue.try_pop()
                } else {
                    queue.pop_wait(Some(Instant::now()))
                };
                match outcome {
                    PopOutcome::Discontinuity(gap) => {
                        let Some((dropped, bytes, duration)) = model.pending_gap.take() else {
                            return Err(format!(
                                "{step}: a discontinuity surfaced with no pending flush"
                            ));
                        };
                        check_gap(&gap, dropped, bytes, duration, model.published, &step)?;
                    }
                    PopOutcome::Item(item) => {
                        let Some((generation, blob)) = model.residents.first().copied() else {
                            return Err(format!("{step}: an item popped from an empty model"));
                        };
                        if item.value != blob {
                            return Err(format!(
                                "{step}: FIFO violated — popped id {}, front is {}",
                                item.value.id, blob.id
                            ));
                        }
                        if item.generation.counter() != generation {
                            return Err(format!(
                                "{step}: popped stamp {} ≠ model {}",
                                item.generation.counter(),
                                generation
                            ));
                        }
                        if item.generation.counter() < model.published {
                            return Err(format!(
                                "{step}: a stale item (stamp {}, published {}) was popped — \
                                 stale generations are always dropped",
                                item.generation.counter(),
                                model.published
                            ));
                        }
                        model.residents.remove(0);
                        model.popped += 1;
                    }
                    PopOutcome::Empty => {
                        if !model.residents.is_empty() && model.pending_gap.is_none() {
                            return Err(format!(
                                "{step}: Empty while the model holds {} residents",
                                model.residents.len()
                            ));
                        }
                        if model.closed && !model.residents.is_empty() {
                            return Err(format!(
                                "{step}: Empty while closed with residents left — closing \
                                 never destroys resident data"
                            ));
                        }
                    }
                    PopOutcome::Closed => {
                        if !model.closed {
                            return Err(format!("{step}: Closed while the model is open"));
                        }
                        if !model.residents.is_empty() {
                            return Err(format!(
                                "{step}: Closed with {} residents undrained — closing never \
                                 destroys resident data",
                                model.residents.len()
                            ));
                        }
                    }
                }
            }
            Op::Flush { advance, regress } => {
                let dropped = model.residents.len() as u64;
                let dropped_bytes = model.bytes();
                let dropped_duration = model.duration();
                let target = if regress {
                    // A regressing target: at/below the published mark.
                    GenerationId::new(model.published / 2)
                } else {
                    GenerationId::new(model.published + advance + 1)
                };
                let gap: Discontinuity = queue.flush(target);
                model.residents.clear();
                model.flushed += dropped;
                // The published generation never regresses.
                model.published = model.published.max(target.counter());
                check_gap(
                    &gap,
                    dropped,
                    dropped_bytes,
                    dropped_duration,
                    model.published,
                    &step,
                )?;
                // Merge into the pending consumer-side marker.
                let merged = match model.pending_gap.take() {
                    Some((d, b, n)) => (d + dropped, b + dropped_bytes, n + dropped_duration),
                    None => (dropped, dropped_bytes, dropped_duration),
                };
                model.pending_gap = Some(merged);
            }
            Op::Close => {
                queue.close();
                model.closed = true;
            }
        }
        // ---- the per-op exact-accounting law set ----
        let usage = queue.usage();
        if usage.packets != model.residents.len() {
            return Err(format!(
                "{step}: usage.packets {} ≠ model {} — exact accounting broke",
                usage.packets,
                model.residents.len()
            ));
        }
        if usage.bytes != model.bytes() {
            return Err(format!(
                "{step}: usage.bytes {} ≠ model {}",
                usage.bytes,
                model.bytes()
            ));
        }
        if usage.duration.0 != model.duration() {
            return Err(format!(
                "{step}: usage.duration {} ≠ model {}",
                usage.duration.0,
                model.duration()
            ));
        }
        // Memory bounded: all three caps hold after EVERY op (N ≫ cap).
        if usage.packets > limits.max_packets.get() {
            return Err(format!(
                "{step}: Q1 packets {} > cap {}",
                usage.packets,
                limits.max_packets.get()
            ));
        }
        if usage.bytes > limits.max_bytes.get() {
            return Err(format!(
                "{step}: Q1 bytes {} > cap {}",
                usage.bytes,
                limits.max_bytes.get()
            ));
        }
        if usage.duration.0 > limits.max_duration.0 {
            return Err(format!(
                "{step}: Q1 duration {} > cap {}",
                usage.duration.0, limits.max_duration.0
            ));
        }
        // The queue's published generation never regresses either.
        if queue.generation().counter() != model.published {
            return Err(format!(
                "{step}: published {} ≠ model {} — the generation regressed or jumped",
                queue.generation().counter(),
                model.published
            ));
        }
        // Conservation: pushed == popped + resident + flushed.
        let stats = queue.stats();
        if stats.pushed != model.pushed
            || stats.popped != model.popped
            || stats.flushed != model.flushed
            || stats.stale_rejected != model.stale_rejected
            || stats.full_rejections != model.full_rejections
        {
            return Err(format!(
                "{step}: stats ({:?}) ≠ model (pushed {}, popped {}, flushed {}, stale {}, \
                 full {})",
                stats,
                model.pushed,
                model.popped,
                model.flushed,
                model.stale_rejected,
                model.full_rejections
            ));
        }
        if stats.pushed != stats.popped + usage.packets as u64 + stats.flushed {
            return Err(format!(
                "{step}: conservation broke — pushed {} ≠ popped {} + resident {} + flushed {}",
                stats.pushed, stats.popped, usage.packets, stats.flushed
            ));
        }
    }
    // Terminal: close if needed, drain, and the identity closes.
    if !model.closed {
        queue.close();
        model.closed = true;
    }
    loop {
        match queue.try_pop() {
            PopOutcome::Item(item) => {
                let Some((generation, blob)) = model.residents.first().copied() else {
                    return Err(format!("seed {seed}: drain popped past the model"));
                };
                if item.value != blob || item.generation.counter() != generation {
                    return Err(format!(
                        "seed {seed}: drain FIFO/stamp mismatch (got id {} stamp {}, want id \
                         {} stamp {})",
                        item.value.id,
                        item.generation.counter(),
                        blob.id,
                        generation
                    ));
                }
                model.residents.remove(0);
                model.popped += 1;
            }
            PopOutcome::Discontinuity(gap) => {
                let Some((dropped, bytes, duration)) = model.pending_gap.take() else {
                    return Err(format!(
                        "seed {seed}: a drain discontinuity with no pending flush"
                    ));
                };
                check_gap(&gap, dropped, bytes, duration, model.published, "drain")?;
            }
            PopOutcome::Empty => {
                // The queue is closed here — `Empty` is unreachable
                // by the contract, and honoring it silently would
                // hang the drain; surface it as the defect it is.
                return Err(format!(
                    "seed {seed}: Empty at drain with {} residents left (the queue is \
                     closed — Empty is unreachable)",
                    model.residents.len()
                ));
            }
            PopOutcome::Closed => break,
        }
    }
    let stats = queue.stats();
    if stats.pushed != stats.popped + stats.flushed {
        return Err(format!(
            "seed {seed}: terminal conservation broke — {} ≠ {} + {}",
            stats.pushed, stats.popped, stats.flushed
        ));
    }
    if queue.usage().packets != 0 {
        return Err(format!("seed {seed}: the drained queue is not empty"));
    }
    Ok(())
}

fn check_gap(
    gap: &Discontinuity,
    dropped: u64,
    bytes: usize,
    duration: u64,
    published: u64,
    step: &str,
) -> Result<(), String> {
    if gap.dropped != dropped {
        return Err(format!(
            "{step}: gap dropped {} ≠ model {}",
            gap.dropped, dropped
        ));
    }
    if gap.dropped_bytes != bytes {
        return Err(format!(
            "{step}: gap bytes {} ≠ model {}",
            gap.dropped_bytes, bytes
        ));
    }
    if gap.dropped_duration.0 != duration {
        return Err(format!(
            "{step}: gap duration {} ≠ model {}",
            gap.dropped_duration.0, duration
        ));
    }
    if gap.new_generation.counter() != published {
        return Err(format!(
            "{step}: gap generation {} ≠ published {}",
            gap.new_generation.counter(),
            published
        ));
    }
    Ok(())
}

/// The interleaving law set over the full bank: 64 seeds × 4 caps
/// (1, 3, 8, 13) × 6000 ops (N ≫ cap) — exact accounting, Q1 bounds,
/// stale-drop, FIFO, non-regressing generations, conservation, and a
/// clean terminal drain.
#[test]
fn bounded_accounting_and_stale_laws_hold_under_adversarial_interleavings() {
    for seed in seed_bank() {
        for cap in [1usize, 3, 8, 13] {
            let mut rng = XorShift::new(seed);
            let ops = generate_ops(&mut rng, 6000);
            if let Err(failure) = run_case(seed, &ops, |seed, ops| run(seed, cap, ops)) {
                unreachable!("{failure}");
            }
        }
    }
}

/// The focused Q2 sweep: for every (published, stamp) pair in a dense
/// window, a strictly-past stamp NEVER enters and a current-or-future
/// stamp takes the capacity path; after a flush, no pop ever yields a
/// stale item. Exhaustive over the window, seeded interleavings
/// around it.
#[test]
fn stale_generation_packets_are_always_dropped() {
    for published in 0u64..48 {
        let limits = QueueLimits::new(
            NonZeroUsize::new(4).must(),
            NonZeroUsize::new(4096).must(),
            MediaDuration::Known(Nanoseconds(1_000)),
        )
        .must();
        let queue: BoundedQueue<Blob> = BoundedQueue::with_default_watermarks(limits);
        // Publish `published` (a single flush reaches it).
        let _ = queue.flush(GenerationId::new(published));
        for stamp in 0u64..(published + 6) {
            let blob = Blob {
                id: stamp,
                size: 8,
                dur_ns: 1,
            };
            let pushed_before = queue.stats().pushed;
            let usage_before = queue.usage();
            match queue.try_push(Stamped::new(GenerationId::new(stamp), blob)) {
                PushOutcome::Stale { current, item } => {
                    if stamp >= published {
                        unreachable!(
                            "published {published}: stamp {stamp} is not stale but was dropped"
                        );
                    }
                    assert_eq!(
                        current.counter(),
                        published,
                        "the stale signal reports the published generation"
                    );
                    assert_eq!(item.value, blob, "the item came back untouched");
                    assert_eq!(
                        queue.stats().pushed,
                        pushed_before,
                        "a stale push never entered"
                    );
                    assert_eq!(queue.usage().packets, usage_before.packets);
                }
                PushOutcome::Pushed => {
                    assert!(
                        stamp >= published,
                        "published {published}: stale stamp {stamp} entered the queue"
                    );
                    assert_eq!(queue.stats().pushed, pushed_before + 1);
                }
                PushOutcome::Full { item, binding } => {
                    // The capacity path: a current stamp refused by a
                    // cap. Nothing entered; the packets axis is the
                    // only one this sweep's weights can violate (8-B
                    // blobs under a 4096-B cap, 1-ns blobs under
                    // 1000 ns — at most 4 residents).
                    assert!(
                        stamp >= published,
                        "published {published}: stale stamp {stamp} answered Full, not Stale"
                    );
                    assert_eq!(
                        queue.stats().pushed,
                        pushed_before,
                        "a full push never enters"
                    );
                    assert_eq!(queue.usage().packets, usage_before.packets);
                    assert_eq!(
                        binding,
                        LimitAxis::Packets,
                        "the 4-packet cap is the only binding axis in this sweep"
                    );
                    assert_eq!(item.value, blob, "the item came back untouched");
                }
                other => unreachable!("published {published}, stamp {stamp}: {other:?}"),
            }
        }
        // Drain: every popped item is at or after the published
        // generation — stale generations are unreachable by pop.
        while let PopOutcome::Item(item) = queue.try_pop() {
            assert!(
                item.generation.counter() >= published,
                "a stale item ({} < {published}) was popped",
                item.generation.counter()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The threaded no-deadlock storm (parking producers + consumers under
// generation movement and close, watchdogged)
// ---------------------------------------------------------------------------

/// A parking producer: bounded-deadline pushes in a loop; reports
/// completion on `done`. Never parks unboundedly (deadlines of a few
/// ms), so a hang is a queue defect, not a test choice.
fn producer(
    queue: Arc<BoundedQueue<Blob>>,
    published: Arc<Mutex<u64>>,
    done: mpsc::Sender<u64>,
    iterations: u32,
) {
    let mut accepted = 0u64;
    for index in 0..iterations {
        let stamp = {
            let guard = published
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            GenerationId::new(*guard)
        };
        let blob = Blob {
            id: u64::from(index),
            size: 8,
            dur_ns: 1,
        };
        let deadline = Instant::now() + Duration::from_millis(3);
        if queue
            .push_wait(Stamped::new(stamp, blob), Some(deadline))
            .is_ok()
        {
            accepted += 1;
        }
    }
    let _ = done.send(accepted);
}

/// A parking consumer: bounded-deadline pops in a loop.
fn consumer(queue: Arc<BoundedQueue<Blob>>, done: mpsc::Sender<u64>, iterations: u32) {
    let mut received = 0u64;
    for _ in 0..iterations {
        let deadline = Instant::now() + Duration::from_millis(2);
        if let PopOutcome::Item(_) = queue.pop_wait(Some(deadline)) {
            received += 1;
        }
    }
    let _ = done.send(received);
}

/// The threaded storm: 2 producers + 1 consumer + a main-thread storm
/// of flushes and the final close. Everything must terminate within
/// the watchdog — a parked thread that never wakes is a deadlock
/// defect (doc 03 §4.4(f): bounded wakeups under generation
/// movement).
#[test]
fn threaded_storm_terminates_under_generation_movement() {
    // The bank's first 8 seeds — the threaded storm is the slowest
    // suite; the single-threaded bank above already covers the rest.
    for seed in seed_bank().into_iter().take(8) {
        let context = format!("seed {seed}");
        let limits = QueueLimits::new(
            NonZeroUsize::new(8).must(),
            NonZeroUsize::new(1 << 16).must(),
            MediaDuration::Known(Nanoseconds(1_000_000)),
        )
        .must();
        let queue: Arc<BoundedQueue<Blob>> =
            Arc::new(BoundedQueue::with_default_watermarks(limits));
        let published = Arc::new(Mutex::new(0u64));
        let (done, finished) = mpsc::channel();
        let mut workers = Vec::new();
        for worker in 0..2u32 {
            let queue = Arc::clone(&queue);
            let published = Arc::clone(&published);
            let done = done.clone();
            workers.push(std::thread::spawn(move || {
                producer(queue, published, done, 750 + worker * 250);
            }));
        }
        {
            let queue = Arc::clone(&queue);
            let done = done.clone();
            workers.push(std::thread::spawn(move || {
                consumer(queue, done, 1500);
            }));
        }
        drop(done);
        // The storm: seeded flushes (generation movement — parked
        // producers must wake with the typed stale abort), then the
        // close (everyone wakes).
        let mut rng = XorShift::new(seed);
        let mut generation = 0u64;
        for _ in 0..12 {
            std::thread::sleep(Duration::from_micros(300 + rng.below(400)));
            generation += rng.below(2) + 1;
            let _ = queue.flush(GenerationId::new(generation));
            {
                let mut guard = published
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *guard = generation;
            }
        }
        queue.close();
        // The watchdog: every worker reports within the budget.
        let mut reports = 0usize;
        while reports < 3 {
            match finished.recv_timeout(Duration::from_secs(20)) {
                Ok(_) => reports += 1,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    unreachable!(
                        "{context}: a queue worker never woke within 20 s — deadlock under \
                         generation movement/close"
                    );
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    unreachable!("{context}: the reporting channel broke early");
                }
            }
        }
        // Join everything (all reported; joins cannot hang now).
        for worker in workers {
            worker.join().unwrap_or_else(|poisoned| {
                unreachable!("{context}: a worker panicked: {poisoned:?}")
            });
        }
        // The queue is closable and drainable to the end (bounded:
        // only the model's own residents can come out — a longer
        // spin is a defect, not a slow drain).
        queue.close();
        let mut drain_spins = 0usize;
        while !matches!(queue.try_pop(), PopOutcome::Closed) {
            drain_spins += 1;
            if drain_spins > 1 << 16 {
                unreachable!(
                    "{context}: the post-storm drain never reached Closed in {} spins",
                    drain_spins
                );
            }
        }
        assert_eq!(queue.usage().packets, 0, "{context}: drained");
    }
}

/// The documented shrink procedure, self-tested: the bisection finds
/// the minimal failing prefix of a synthetic predicate (log2 N reruns
/// over a deterministic rebuild), and a non-failing sequence shrinks
/// to nothing. The property cases above drive it through `run_case`
/// on every failure; this pins the machinery itself.
#[test]
fn shrink_prefix_bisects_to_the_minimal_failing_prefix() {
    let ops: Vec<u64> = (0..64).collect();
    // Fails iff the prefix contains the op valued 40 — the minimal
    // failing prefix is ops[0..=40] (41 ops).
    let fails = |prefix: &[u64]| prefix.contains(&40);
    let minimal = shrink_prefix(&ops, fails);
    assert_eq!(minimal.len(), 41, "the prefix must run through op 40");
    assert_eq!(minimal.last().copied(), Some(40));
    // An empty or non-failing sequence has nothing to shrink.
    assert!(shrink_prefix(&ops, |_| false).is_empty());
    assert!(shrink_prefix(&[], fails).is_empty());
}
