//! Zero-allocation proof for the card 3.B2 backend surface (D-1,
//! measured per the D-6 "runtime green" discipline): after warm-up,
//! the steady-state tick paths of the multi-backend clock —
//! `now`/`try_now`/`snapshot`/`anchor` in every master mode,
//! snapped `observe_video` steers, empty-ring `take_master_change`
//! polls — plus the control-plane transitions themselves (master
//! switches, loss demotions, the Ended latch, `seek_back`) allocate
//! **zero bytes**. The switches are Arc refcount moves into
//! fixed-size core state; the event trail is an 8-slot ring; the
//! outcomes are `Copy` values — nothing here needs the allocator,
//! and now that is measured, not argued.
//!
//! One `#[test]` per binary by design (the allocator is
//! process-global; sibling test threads and concurrent harness
//! reporting would be counted — see `testkit::alloc_count` docs).

#[global_allocator]
static ALLOC: testkit::alloc_count::CountingGlobalAlloc =
    testkit::alloc_count::CountingGlobalAlloc::new();

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use media_clock::{
    ClockBackends, MasterStream, MasterTarget, MediaClock, NANO_BASE, PlaybackClock,
};
use media_types::time::{Rate, Timestamp};
use testkit::alloc_count::serialize_tests;

/// A deterministic fake sample counter (atomics only).
struct Counter {
    sample: AtomicU64,
}

impl media_clock::SampleCounterSource for Counter {
    fn sample_count(&self) -> u64 {
        self.sample.load(Ordering::Relaxed)
    }
    fn sample_rate_hz(&self) -> u32 {
        48_000
    }
}

/// A deterministic fake monotonic source (atomics only).
struct Wall(AtomicU64);

impl media_clock::MonotonicSource for Wall {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

fn ts(ns: i64) -> Timestamp {
    Timestamp::new(ns, NANO_BASE)
}

#[test]
fn backend_surface_is_zero_allocation_after_warm_up() {
    let _serial = serialize_tests();

    // Setup: allocates freely (the clock, its core, the Arcs) — all
    // outside every measured window.
    let wall = Arc::new(Wall(AtomicU64::new(0)));
    let counter = Arc::new(Counter {
        sample: AtomicU64::new(0),
    });
    let device = Arc::new(Wall(AtomicU64::new(0)));
    let clock = MediaClock::with_backends(
        ClockBackends::new(wall.clone())
            .audio(counter.clone())
            .external(device.clone())
            .video(true),
        ts(0),
        Rate::ONE,
    )
    .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(clock.master(), media_clock::ClockMaster::AudioOutput);

    // Warm-up: every measured path once, at every master it will
    // run under (first-touch sizing, if any, lands here).
    let _ = clock.now();
    let _ = clock.try_now();
    let _ = clock.snapshot();
    let _ = clock.anchor();
    let _ = clock.switch_master(MasterTarget::Video);
    let _ = clock.observe_video(ts(1_000));
    let _ = clock.take_master_change();
    let _ = clock.master_changes_lost();
    let _ = clock.switch_master(MasterTarget::Audio);

    // ---- Measured window 1: steady-state ticks + steering -----------------
    let ticks = 4_096;
    {
        let _ = clock.switch_master(MasterTarget::Video);
        let guard = ALLOC.state().guard();
        for i in 0..ticks {
            // The video master: interpolate, steer, read, drain.
            wall.0
                .store(u64::try_from(i).unwrap_or(0) * 1_000_000, Ordering::Relaxed);
            let position = clock.now();
            let _ = clock.try_now();
            let _ = clock.snapshot();
            let _ = clock.anchor();
            let steer = clock
                .observe_video(Timestamp::new(position.ticks + 1_000_000, NANO_BASE))
                .unwrap_or_else(|e| unreachable!("{e}"));
            debug_assert!(matches!(steer, media_clock::VideoSteer::Snapped { .. }));
            let _ = clock.take_master_change(); // empty-ring poll
            let _ = clock.master_changes_lost();
        }
        guard.assert_zero("video-master steady-state ticks (now/try_now/snapshot/observe_video)");
    }

    // ---- Measured window 2: the control plane itself -----------------------
    {
        let guard = ALLOC.state().guard();
        // Engine switch storm with per-switch drains (Arc moves +
        // fixed-slot ring writes only). Ends on video (odd count).
        for i in 0..256 {
            let target = if i % 2 == 0 {
                MasterTarget::External
            } else {
                MasterTarget::Video
            };
            let _ = clock.switch_master(target);
            while clock.take_master_change().is_some() {}
        }
        // Then walk the loss chain to the free-running terminal:
        // audio (recorded), external (recorded), video (the
        // demotion), then free-run ticks.
        let _ = clock.on_master_lost(MasterStream::Audio);
        let _ = clock.on_master_lost(MasterStream::External);
        let _ = clock.on_master_lost(MasterStream::Video);
        assert_eq!(clock.master(), media_clock::ClockMaster::FreeRunning);
        for i in 0..ticks {
            wall.0.store(
                10_000_000_000 + u64::try_from(i).unwrap_or(0),
                Ordering::Relaxed,
            );
            let _ = clock.now();
            let _ = clock.try_now();
            let _ = clock.snapshot();
        }
        // The Ended latch and the seek-back re-arm (control plane).
        let snap = clock.mark_ended();
        assert_eq!(snap.state, media_clock::ClockState::Ended);
        let _ = clock.is_ended();
        let re_armed = clock
            .seek_back(ts(42))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(re_armed.state, media_clock::ClockState::Paused);
        clock.resume();
        for i in 0..ticks {
            wall.0.store(
                20_000_000_000 + u64::try_from(i).unwrap_or(0),
                Ordering::Relaxed,
            );
            let _ = clock.now();
        }
        while clock.take_master_change().is_some() {}
        guard.assert_zero("backend control plane (switches, losses, ended latch, seek-back)");
    }
}
