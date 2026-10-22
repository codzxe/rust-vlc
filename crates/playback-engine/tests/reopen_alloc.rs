//! The reopen-cycle bounded-memory proof (card 3.C1's
//! `reopen_cycles_1000x` measured half): 1000 × open → play → stop →
//! close on the fakes, memory measured FLAT across the measured
//! window with the testkit counting allocator — runtime evidence,
//! not syntactic green (D-1/D-6). The functional half (epoch/seq/
//! receipt continuity across the same 1000 cycles) lives in
//! `tests/model_walk.rs`; the 4 h wall-clock soak stays 3.E4's.
//!
//! One `#[test]` per binary by design (the allocator is
//! process-global — see `testkit::alloc_count` docs and the
//! `bounded_alloc.rs` / `replacement_alloc.rs` precedents): after a
//! warmup that sizes every pool the engine reuses (outbox ring,
//! minter, session bookkeeping), the measured window runs the
//! remaining cycles under one [`AllocSnapshot`] guard and asserts
//! `live_bytes` returns to the window's opening level — a reopen
//! cycle must be allocation-neutral in steady state (everything the
//! open built, the close must free; nothing may accrete per cycle).

mod common;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use media_types::{MediaKind, SourceUri, TrackId};
use playback_engine::{
    Engine, FrameConsumer, SessionConfig, SessionState, TickBudget,
};
use plugin_api::DemuxEvent;
use testkit::Must;
use testkit::alloc_count::{CountingGlobalAlloc, serialize_tests};

use common::{
    Collector, FakeDemuxer, FakeProvider, frozen_throttle, packet, tiny_queues, track,
    virtual_clock,
};

#[global_allocator]
static ALLOC: CountingGlobalAlloc = CountingGlobalAlloc::new();

/// Cycles in the measured window (the design's 1000; the warmup that
/// precedes it is another 100 — pool sizing and allocator steady
/// state, excluded from the claim).
const MEASURED_CYCLES: usize = 1000;
const WARMUP_CYCLES: usize = 100;
/// Packets per cycle's script (enough to prime and play — the
/// functional half pins the exact per-cycle behavior).
const SCRIPT_PACKETS: u64 = 40;

/// One open → play → stop → close cycle on a fresh fake source;
/// returns the collector so the caller can account the frames.
fn cycle(engine: &mut Engine, script_index: &mut u64, request: &mut u64) -> usize {
    let tracks = vec![track(1, MediaKind::Audio)];
    let script: Vec<DemuxEvent> = (*script_index..*script_index + SCRIPT_PACKETS)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    *script_index += SCRIPT_PACKETS;
    let collector = Arc::new(Collector::default());
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-reopen-alloc.media").must(),
        Box::new(FakeDemuxer {
            tracks,
            script: Arc::new(Mutex::new(VecDeque::from(script))),
            stall: Arc::new(Mutex::new(false)),
            fail_start: None,
        }),
        Arc::new(FakeProvider { fail_select: None }),
        Arc::clone(&collector) as Arc<dyn FrameConsumer>,
        virtual_clock(),
    );
    config.queues = tiny_queues();
    config.budget = TickBudget {
        demux_events: 64,
        packets_per_limb: 4,
        flush_calls_per_limb: 8,
    };
    config.throttle = frozen_throttle();
    let handle = engine
        .open(config)
        .unwrap_or_else(|error| unreachable!("open failed: {error}"));
    // Play: pump to Playing (bounded — a hang is a failed test).
    for _ in 0..24 {
        if engine.state() == SessionState::Playing {
            break;
        }
        engine.poll();
    }
    assert_eq!(
        engine.state(),
        SessionState::Playing,
        "the cycle reached playing"
    );
    // Stop → close (the settled exit), draining between the steps.
    *request += 1;
    handle
        .stop(commands::envelope::RequestId(*request))
        .unwrap_or_else(|error| unreachable!("stop failed: {error}"));
    engine.poll();
    *request += 1;
    handle
        .close(commands::envelope::RequestId(*request))
        .unwrap_or_else(|error| unreachable!("close failed: {error}"));
    engine.poll();
    engine.poll();
    assert_eq!(engine.state(), SessionState::Idle, "the cycle closed");
    let frames = collector
        .frames
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .len();
    // Drain the outbox so nothing the cycle emitted is attributed to
    // the next one.
    drop(engine.take_outputs());
    frames
}

#[test]
fn reopen_cycles_are_allocation_neutral_in_steady_state() {
    let _serial = serialize_tests();
    let mut engine = Engine::new();
    let mut script_index = 0u64;
    let mut request = 0u64;

    // Warmup: size every reused pool (the outbox ring, the minter,
    // the engine's session slot) and let the allocator reach its
    // steady state; the window's baseline is taken AFTER it.
    for _ in 0..WARMUP_CYCLES {
        let frames = cycle(&mut engine, &mut script_index, &mut request);
        assert!(frames > 0, "the warmup cycles actually played");
    }

    // The measured window: 1000 full cycles.
    let guard = ALLOC.state().guard();
    let mut played = 0usize;
    for _ in 0..MEASURED_CYCLES {
        let frames = cycle(&mut engine, &mut script_index, &mut request);
        assert!(frames > 0, "the measured cycles actually played");
        played += frames;
    }
    let delta = guard.delta();
    assert!(
        delta.live_bytes == 0,
        "memory is flat across {MEASURED_CYCLES} reopen cycles (live bytes leaked: \
         {}; allocation events: {} — the per-cycle session/pipeline builds, \
         all freed at close)",
        delta.live_bytes, delta.alloc_calls
    );
    assert!(
        delta.alloc_calls > 0,
        "sanity: the cycles actually ran (per-cycle transients were allocated)"
    );
    assert!(
        played >= MEASURED_CYCLES,
        "sanity: every cycle delivered frames ({played} frames over {MEASURED_CYCLES} cycles)"
    );
}
