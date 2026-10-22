//! Zero-allocation proof for the scheduler's dropping ticks (ED-FIX-2,
//! audit M3): after one warm-up cycle, N aggressive dropping ticks —
//! full-window late storms, supersede bursts, presents and repeats —
//! allocate **zero bytes**, measured by the counting global allocator.
//! The old per-tick `drops.late`/`superseded` `Vec` materializations
//! are gone (fixed-capacity inline trails, D-1/D-8); the sibling
//! suites pin the decisions themselves.
//!
//! One `#[test]` per binary by design (the allocator is
//! process-global; see `testkit::alloc_count` docs).

#[global_allocator]
static ALLOC: testkit::alloc_count::CountingGlobalAlloc =
    testkit::alloc_count::CountingGlobalAlloc::new();

mod common;

use std::num::NonZeroUsize;

use common::{VirtualClock, frame};
use media_types::GenerationId;
use media_types::time::Nanoseconds;
use testkit::Must;
use testkit::alloc_count::serialize_tests;
use video_engine::{DropPolicy, PresentAction, PresentationScheduler, SchedulerCfg};

/// 30 fps nominal frame duration (ns) — the sibling suites' constant.
const FRAME: u64 = 33_333_334;
/// The late-storm width: one full default reorder window per tick.
const STORM: usize = 16;
/// The supersede-burst width: due frames pushed per supersede tick.
const BURST: usize = 4;
/// Measured cycles — three ticks each (storm purge, supersede burst +
/// present, repeat): 384 aggressive dropping ticks in total.
const CYCLES: usize = 128;

/// The doc 03 §4.10 e default shape for 30 fps media (one-frame
/// threshold, half-frame lead), spelled out so the arithmetic the
/// test depends on is local.
fn cfg() -> SchedulerCfg {
    SchedulerCfg::new(
        DropPolicy::Late {
            threshold: Nanoseconds(FRAME),
        },
        3,
        Nanoseconds(FRAME / 2),
        Some(Nanoseconds(FRAME)),
        NonZeroUsize::new(16).must(),
    )
    .must()
}

#[test]
fn dropping_ticks_allocate_nothing_after_warm_up() {
    let _serial = serialize_tests();

    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(cfg(), clock.clock());

    // The clock parks far past the storm pool: every storm frame is
    // hopelessly late the moment it is pushed, and every cycle is
    // identical. No clock writes are needed inside the window.
    let now = 10_000 * FRAME;
    clock.set_ns(now);

    // Frame pools are built OUTSIDE the measured window; inside it,
    // every push is a clone of a pooled frame (`SharedBytes` is an
    // Arc: a refcount bump, not a heap visit).
    let late_pool: Vec<_> = (0..STORM as i64)
        .map(|i| frame(i * FRAME as i64, GenerationId::FIRST))
        .collect();
    // Due at `now` — inside `[now − threshold, now + lead]`: the
    // burst's largest frame presents, the rest supersede.
    let due_pool: Vec<_> = (0..BURST as i64)
        .map(|i| frame(now as i64 - (BURST as i64 - 1 - i), GenerationId::FIRST))
        .collect();

    // Warm-up: run every measured path once so one-time costs (the
    // window deque growing to its bound, the first present's
    // last-request slot) land outside the window.
    for pooled in &late_pool {
        let _ = scheduler.push(pooled.clone());
    }
    let _ = scheduler.tick(); // late purge of the full window
    for pooled in &due_pool {
        let _ = scheduler.push(pooled.clone());
    }
    let _ = scheduler.tick(); // supersede burst + present
    let _ = scheduler.tick(); // repeat

    // The measured window: aggressive dropping, cycle-shaped.
    let guard = ALLOC.state().guard();
    for cycle in 0..CYCLES {
        // (1) Late storm: a full window of hopelessly late frames,
        //     dropped and counted in one tick.
        for pooled in &late_pool {
            let _ = scheduler.push(pooled.clone());
        }
        let outcome = scheduler.tick();
        assert_eq!(
            outcome.drops.late.recorded(),
            STORM as u64,
            "cycle {cycle}: the storm is dropped and counted"
        );
        assert_eq!(
            outcome.drops.late.len(),
            STORM,
            "cycle {cycle}: not truncated at the default window"
        );
        assert!(matches!(
            outcome.action,
            PresentAction::RepeatLast(_) | PresentAction::Background
        ));

        // (2) Supersede burst: four due frames, the largest presents,
        //     the other three drop structurally.
        for pooled in &due_pool {
            let _ = scheduler.push(pooled.clone());
        }
        let outcome = scheduler.tick();
        assert_eq!(
            outcome.drops.superseded.recorded(),
            (BURST - 1) as u64,
            "cycle {cycle}: the structural drop"
        );
        assert!(matches!(outcome.action, PresentAction::Present(_)));

        // (3) A quiet tick: the bounded repeat path.
        let outcome = scheduler.tick();
        assert!(matches!(
            outcome.action,
            PresentAction::RepeatLast(_) | PresentAction::Background
        ));
    }
    guard.assert_zero(
        "scheduler dropping ticks: 128 cycles × (16-frame late storm + 4-frame supersede burst + repeat)",
    );

    // Post-window accounting (outside the measured window): the
    // cycles did exactly what they claim — 129 storms (warm-up
    // included), 129 presents, nothing lost.
    let stats = scheduler.stats();
    let storms = CYCLES as u64 + 1;
    assert_eq!(stats.pushed, storms * (STORM + BURST) as u64);
    assert_eq!(stats.dropped_late, storms * STORM as u64);
    assert_eq!(stats.dropped_superseded, storms * (BURST as u64 - 1));
    assert_eq!(stats.presented, storms);
    assert_eq!(stats.dropped_stale, 0);
    assert_eq!(scheduler.pending_len(), 0);
}
