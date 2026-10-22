//! Card 3.B3's steady-state queue proof (design E3.2a): the
//! push/pop loop over a pre-filled [`BoundedQueue`] allocates
//! **nothing** — measured with the testkit counting allocator. This is
//! the regression fence for 3.B3's changes (the engine's
//! backpressure additions must not introduce heap churn into the
//! queue's hot path) and the D-1 anchor for the threaded demux future
//! (2.E1: the parked producer's wake path is the same mutex/condvar
//! arithmetic exercised here).
//!
//! One `#[test]` per binary by design (the allocator is
//! process-global; `testkit::alloc_count` documents the serialization
//! discipline).
//!
//! Foreign-noise discipline (W3-05R): the measured section runs
//! through `clean_replay` — a settle pause plus a one-replay backstop
//! for libtest's own thread-spawn allocations landing in the window
//! (see the demux zero-alloc binary's helper docs for the
//! measurement and the D-5 argument; the section here is
//! deterministic — no clocks inside the window, one thread).

use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use media_types::ids::GenerationId;
use media_types::time::{Duration as MediaDuration, Nanoseconds};
use packet_queue::{BoundedQueue, PopOutcome, PushOutcome, QueueLimits, Stamped, Weigh};
use testkit::Must;
use testkit::alloc_count::{AllocDelta, CountingGlobalAlloc, serialize_tests};

#[global_allocator]
static ALLOC: CountingGlobalAlloc = CountingGlobalAlloc::new();

/// The settle half of the discipline: let an in-flight sibling-thread
/// spawn finish before the window opens (observed: 2–10 events /
/// 55 B–10 KiB per lock handoff without it; 40/40 windows clean
/// with it — see the demux twin binary for the diagnostic numbers).
fn settle_foreign_spawns() {
    std::thread::sleep(Duration::from_millis(30));
}

/// The replay half: a deterministic section replays once when the
/// first window is dirty; a real defect reproduces on every replay
/// and fails the both-dirty assert, transient harness noise does not
/// (D-5: hardens against the environment, never weakens the proof).
fn clean_replay(mut section: impl FnMut() -> AllocDelta) -> AllocDelta {
    settle_foreign_spawns();
    let first = section();
    if first.alloc_calls == 0 && first.total_bytes == 0 {
        return first;
    }
    settle_foreign_spawns();
    let second = section();
    assert!(
        second.alloc_calls == 0 && second.total_bytes == 0,
        "measured window dirty on BOTH deterministic replays — a real \
         allocation defect, not harness noise (first {first:?}, second {second:?})"
    );
    second
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Blob {
    id: u64,
}

impl Weigh for Blob {
    fn size_bytes(&self) -> usize {
        4
    }

    fn media_duration(&self) -> Nanoseconds {
        Nanoseconds(1_000)
    }
}

/// The steady-state push/pop cycle (100k iterations over a half-full
/// queue, with watermark and stats queries interleaved — the engine
/// tick's exact queue surface) allocates zero bytes. The parking
/// variants ride the same path with a live deadline (no sleep: room
/// exists, so the park is skipped).
#[test]
fn steady_state_push_pop_cycle_allocates_nothing() {
    let _serial = serialize_tests();
    let delta = clean_replay(|| {
        let limits = QueueLimits::new(
            NonZeroUsize::new(512).must(),
            NonZeroUsize::new(1 << 20).must(),
            MediaDuration::Known(Nanoseconds(1_000_000)),
        )
        .must();
        let queue = BoundedQueue::<Blob>::with_default_watermarks(limits);

        // Pre-fill to half capacity (the engine's primed posture).
        for id in 0..256u64 {
            assert_eq!(
                queue.try_push(Stamped::new(GenerationId::FIRST, Blob { id })),
                PushOutcome::Pushed
            );
        }

        // Warm-up: the first watermark/stats queries and a parking push
        // with room (the condvar fast path) settle any lazy internals.
        let _ = queue.watermark();
        let _ = queue.stats();
        assert_eq!(
            queue
                .push_wait(
                    Stamped::new(GenerationId::FIRST, Blob { id: 256 }),
                    Some(Instant::now() + Duration::from_millis(50)),
                )
                .ok(),
            Some(())
        );
        assert!(matches!(queue.try_pop(), PopOutcome::Item(_)));

        let guard = ALLOC.state().guard();
        let deadline = || Some(Instant::now() + Duration::from_millis(50));
        for round in 0..100_000u64 {
            // The engine tick's queue surface: push (non-blocking and the
            // parking flavor with room), pop, watermark, stats, usage.
            // Net-zero per round (+2 items, −2 items): the queue stays at
            // its primed half-fill for the whole loop, so the caps never
            // bind and both push flavors keep taking the accept fast path.
            assert_eq!(
                queue.try_push(Stamped::new(GenerationId::FIRST, Blob { id: round })),
                PushOutcome::Pushed
            );
            let parked = queue.push_wait(
                Stamped::new(GenerationId::FIRST, Blob { id: round + 1 }),
                deadline(),
            );
            assert!(parked.is_ok(), "room exists: the park is skipped");
            assert!(matches!(queue.try_pop(), PopOutcome::Item(_)));
            assert!(matches!(queue.try_pop(), PopOutcome::Item(_)));
            let mark = queue.watermark();
            let _ = mark.usage.packets;
            let stats = queue.stats();
            let _ = stats.pushed;
            let usage = queue.usage();
            let _ = usage.bytes;
        }
        guard.delta()
    });
    assert_eq!(
        (delta.alloc_calls, delta.total_bytes),
        (0, 0),
        "100k steady-state push/pop cycles (D-1 hot path)"
    );
}
