//! Scenario 13 — **long media** (doc 10 §6.4 row 13), driven
//! through the command layer.
//!
//! Pass criteria: `Ended` reached; memory growth within budget.
//!
//! The doc 10 row names the 4 h synthetic stream (soak tier). This
//! suite's shape (per the card: "long = synthetically scripted long
//! timeline on virtual clock (NOT wall-clock waiting)"): a scripted
//! demux stage producing a **4-hour audio timeline on demand** —
//! 14_400 packets of 1 s each — decoded one frame per packet, the
//! consumer counting and dropping (the soak memory model: the sink
//! retains nothing). Every observable is a pure function of the pump
//! count; the run takes ~2_000 pumps, not wall-clock time.
//!
//! The bounded-memory assertion is exact in-process (D-2): the
//! counting allocator's live-bytes reading at the timeline's halfway
//! point equals the reading at `Ended` — a measured plateau across
//! the second half of a 4 h-equivalent play, the in-proc stand-in
//! for the nightly soak's RSS slope (§6.5's 5 MB/h budget is a
//! nightly-tier proposal; this pins the stronger zero-growth claim
//! for the engine's steady state on a virtual clock).

use commands::envelope::CommandOutcome;
use commands::catalog::types::{OpenOptions, SourceSpec};
use commands::{Command, PlaybackState};
use testkit::alloc_count::serialize_tests;
use testkit::Must;

use crate::alloc_state;
use crate::host::{count_events, dispatch, error_events, ScenarioHost};

/// The timeline: 14_400 one-second packets = exactly 4 h.
const PACKETS: u64 = 14_400;
const STEP_US: i64 = 1_000_000;
/// The pump bound: 14_400 packets at 8 per poll ≈ 1_800 polls, with
/// the prime/drain slack — a hang is a bound breach, never a
/// timeout mystery.
const BOUND: usize = 6_144;

/// The 4-hour scripted timeline plays to `Ended` with exact
/// accounting and a measured memory plateau.
#[test]
fn four_hour_timeline_ends_with_bounded_memory() {
    let _serial = serialize_tests();
    let mut host = ScenarioHost::new();
    let response = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::Url {
                url: format!("timeline://{PACKETS}x{STEP_US}"),
            },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&response.result, Ok(CommandOutcome::Accepted { .. })),
        "the long timeline opens: {:?}",
        response.result
    );
    host.settle();
    assert_eq!(host.state(), PlaybackState::Playing, "the timeline primes");

    // Drive to the halfway point, then measure the plateau window.
    // The bound's arithmetic: 7_200 packets to reach at 8 per poll
    // ≈ 900 polls, with priming slack — 4_096 is comfortably past it.
    let half = PACKETS / 2;
    assert!(
        host.pump_pipeline_until_within(4_096, |host| {
            host.sink().map(|sink| sink.audio_accepted()).must() >= half
        }),
        "the timeline reaches its halfway point within the bound"
    );
    let mid = alloc_state().snapshot().live_bytes;
    let frames_at_mid = host.sink().map(|sink| sink.audio_accepted()).must();

    let (pumps, terminal) = host.run_until_terminal(BOUND);
    assert_eq!(
        terminal,
        PlaybackState::Ended,
        "the 4 h timeline reaches Ended (pumps {pumps})"
    );
    let frames = host.sink().map(|sink| sink.audio_accepted()).must();
    assert_eq!(frames, PACKETS, "every packet decoded and delivered");
    assert!(
        frames > frames_at_mid,
        "the plateau window covers the second half ({frames_at_mid} → {frames})"
    );
    assert_eq!(
        count_events(&host, "playback.end-of-media"),
        1,
        "exactly one end-of-media"
    );
    let errors = error_events(&host);
    assert!(
        errors.is_empty(),
        "a clean timeline reports no errors: {errors:?}"
    );
    // The timeline truth: the last delivered frame's pts is the
    // final packet's start (4 h minus one step).
    assert_eq!(
        host.sink().map(|sink| sink.last_pts_ns()).must(),
        u64::try_from((PACKETS - 1) * STEP_US as u64).must() * 1_000,
        "the last frame's pts is the final packet's"
    );

    // D-2: the measured plateau — live bytes at the halfway point
    // equal live bytes at the end (zero growth across 2 h-equivalent
    // of steady-state playback).
    let end = alloc_state().snapshot().live_bytes;
    assert_eq!(
        mid, end,
        "memory plateau: live bytes at half ({mid}) == at end ({end})"
    );

    // The engine's own feeding counters stayed clean: no underrun
    // episodes, nothing dropped at the outbox.
    let stats = host.stats();
    assert_eq!(stats.starve_episodes, 0, "the pacing never starves");
    assert_eq!(stats.outbox_dropped_events, 0, "no event overflow");
    assert_eq!(stats.outbox_dropped_receipts, 0, "no receipt overflow");

    let close = dispatch(&mut host, Command::SessionClose);
    assert!(close.result.is_ok(), "closes: {:?}", close.result);
}
