//! Scenario 15 — **reopen cycles** (doc 10 §6.4 row 15), driven
//! through the command layer.
//!
//! Pass criteria: thread/fd/byte counters return to baseline (leak
//! counters).
//!
//! The driver row says "1000× open/play/stop": one
//! [`ScenarioHost`] (one `Engine` — the reuse is the point; a fresh
//! engine per cycle would prove nothing about the engine's own
//! session teardown) runs 1_000 full cycles of
//! `session.open` → settle → `playback.stop` (from `Playing`) →
//! `session.close` over the well-formed 1-frame AVI built by the
//! scenario-14 surgery. Measured cost per cycle (report table): the
//! whole 1_000-cycle loop runs in well under the 10 s bound, so the
//! doc's full count is served, not a reduced one.
//!
//! The leak oracles (all exact, all in-process):
//!
//! - **allocator live bytes** — the D-2 stand-in for RSS (not
//!   measurable in-proc): after a warm-up that sizes the pools, the
//!   live-bytes reading returns to baseline at three sampled
//!   checkpoints *and* at the end (a grow-then-free pattern inside a
//!   cycle would still end equal; the checkpoints pin the plateau).
//! - **open fds** — the `FileAccess` handle of each cycle's session
//!   is one fd; the count returns to baseline after the last close.
//! - **threads** — the poll-model engine is threadless; the count
//!   must not move across 1_000 cycles.
//! - **engine counters** — `EngineStats` across the whole run: zero
//!   dropped events/receipts (the bounded outbox never overflowed),
//!   zero starvation episodes, and the cumulative `frames_delivered`
//!   exactly the per-cycle accounting (every cycle decoded its two
//!   frames — no silent skips, no double deliveries).

use commands::catalog::types::{OpenOptions, SourceSpec};
use commands::envelope::CommandOutcome;
use commands::{Command, PlaybackState};
use testkit::alloc_count::serialize_tests;

use crate::alloc_state;
use crate::host::{ScenarioHost, TempDir, dispatch, open_fds, thread_count};

/// The doc's full count (§6.4 row 15: "1000× open/play/stop").
const CYCLES: usize = 1_000;
/// Pool-sizing warm-up cycles before the baseline is taken.
const WARM_UP: usize = 3;
/// Frames the 1-frame fixture delivers per cycle (1 video + 1 audio).
const FRAMES_PER_CYCLE: u64 = 2;

/// One full open/play/stop/close cycle through the command layer.
fn cycle(host: &mut ScenarioHost, media: &std::path::Path, index: usize) {
    let open = dispatch(
        host,
        Command::SessionOpen {
            source: SourceSpec::File {
                path: media.to_path_buf(),
            },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&open.result, Ok(CommandOutcome::Accepted { .. })),
        "cycle {index}: opens: {:?}",
        open.result
    );
    host.settle();
    let settled = host.state();
    assert!(
        matches!(settled, PlaybackState::Playing | PlaybackState::Ended),
        "cycle {index}: the 1-frame clip settles active or drained (got {settled:?})"
    );
    // The doc's "stop" step: from Playing it is the mid-play teardown
    // (flush, generation bump); a clip that already drained to Ended
    // has no pipeline left to stop — Close from Ended is the exit.
    if settled == PlaybackState::Playing {
        let stop = dispatch(host, Command::PlaybackStop);
        assert!(
            stop.result.is_ok(),
            "cycle {index}: stops: {:?}",
            stop.result
        );
    }
    let close = dispatch(host, Command::SessionClose);
    assert!(
        close.result.is_ok(),
        "cycle {index}: closes: {:?}",
        close.result
    );
    assert_eq!(
        host.state(),
        PlaybackState::Idle,
        "cycle {index}: idle again"
    );
    assert!(
        host.session_id().is_none(),
        "cycle {index}: no session survives"
    );
}

/// 1_000 open/play/stop cycles on one engine: every counter returns
/// to its baseline.
#[test]
fn thousand_reopen_cycles_return_every_counter_to_baseline() {
    let _serial = serialize_tests();
    let scratch = TempDir::new("s15");
    let media = scratch.build_short_avi("reopen-1frame.avi", 1);

    let mut host = ScenarioHost::new();
    for index in 0..WARM_UP {
        cycle(&mut host, &media, index);
    }

    // The baselines (post-warm-up: pools sized, first-decode caches
    // populated — the measured window is the steady state).
    let baseline = alloc_state().snapshot().live_bytes;
    let fds = open_fds();
    let threads = thread_count();
    let frames_before = host.stats().frames_delivered;

    for index in 0..CYCLES {
        cycle(&mut host, &media, index);
        // Plateau checkpoints at thirds (a cycle-internal grow-free
        // pattern would pass the end check; these pin the plateau).
        if (index + 1) % (CYCLES / 3) == 0 {
            let sample = alloc_state().snapshot().live_bytes;
            assert_eq!(
                sample, baseline,
                "cycle {index}: live bytes drifted ({baseline} → {sample})"
            );
        }
    }

    // Byte counters: the allocator's live bytes are exactly back.
    let after = alloc_state().snapshot().live_bytes;
    assert_eq!(
        after, baseline,
        "1_000 cycles are memory-neutral (live bytes {baseline} → {after})"
    );

    // fd counters: the last cycle's file handle is closed.
    assert_eq!(open_fds(), fds, "the fd count returns to baseline");

    // Thread counters: nothing leaked (the engine is threadless; the
    // harness thread pool is stable under the serializer).
    assert_eq!(thread_count(), threads, "the thread count never moved");

    // Engine counters: the bounded outbox never overflowed, the
    // pacing never starved, and the frame accounting is exact.
    let stats = host.stats();
    assert_eq!(stats.outbox_dropped_events, 0, "no event overflow");
    assert_eq!(stats.outbox_dropped_receipts, 0, "no receipt overflow");
    assert_eq!(stats.starve_episodes, 0, "no starvation episodes");
    assert_eq!(
        stats.frames_delivered - frames_before,
        CYCLES as u64 * FRAMES_PER_CYCLE,
        "every cycle delivered exactly its two frames"
    );
}
