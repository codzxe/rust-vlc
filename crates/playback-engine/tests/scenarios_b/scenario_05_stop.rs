//! Scenario 5 — **stop** (doc 10 §6.4 row 5): stop mid-stream —
//! teardown complete (stage objects released) within deadline.
//!
//! The v0 synchronous engine's teardown evidence: `playback.stop`
//! mid-stream answers its receipt, the state settles to `Stopped`
//! (the session survives, inert until close — state.rs's `Stopped`
//! row family), the wired stage objects are released at the stop's
//! completion boundary (the close — `DestroySession` drops the
//! demuxer, the host's close drops the sink twin) within a bounded
//! number of pumps (the deadline — a hang is a bound breach, never
//! a mystery), the position snapshot after the CLOSE answers the
//! typed `session.not-ready` (a stopped-but-open session answers
//! its frozen position — the CLI's own truth), and the host reopens
//! cleanly (no poisoned state).

use commands::catalog::types::{OpenOptions, SourceSpec};
use commands::envelope::CommandOutcome;
use commands::{Command, ErrorCode, PlaybackState, Stage};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

use crate::host::{PARENT, ScenarioHost, dispatch};

#[test]
fn stop_midstream_releases_stage_objects_within_deadline() {
    let _serial = serialize_tests();
    let mut host = ScenarioHost::new();
    // The teardown oracle: every wired stage object counts its drop.
    let hooks = host.arm_drop_hooks();

    let path = testkit::fixture_path(PARENT).must();
    let response = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::File { path },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&response.result, Ok(CommandOutcome::Accepted { .. })),
        "the parent opens: {:?}",
        response.result
    );
    host.settle();
    assert_eq!(host.state(), PlaybackState::Playing);

    // Play into the media, then stop mid-stream.
    host.pump_pipeline_until(|host| host.sink().map(|s| s.audio_accepted()).must() >= 10);
    let position_at_stop = host.position_us();
    assert!(position_at_stop > 0, "the stop lands mid-stream");

    let stop = dispatch(&mut host, Command::PlaybackStop);
    assert!(stop.result.is_ok(), "the stop answers: {:?}", stop.result);

    // The state settles to Stopped — the pinned `Playing | Stop |
    // Stopped` row (state.rs:588-593) and `Stopped: the session
    // survives, inert until close` (state.rs:846). The green oracle
    // is tests/session_flow.rs:359 (`stop_while_playing_tears_down_…`).
    // Idle is the CLOSE's landing state, not the stop's.
    assert_eq!(host.state(), PlaybackState::Stopped, "settled to Stopped");

    // Inert until close: no further frames are decoded after the
    // stop (tests/session_flow.rs:371-377's oracle).
    let frames_at_stop = host.sink().map(|sink| sink.frames_accepted()).must();
    for _ in 0..8 {
        host.pump_pipeline();
    }
    assert_eq!(
        host.sink().map(|sink| sink.frames_accepted()).must(),
        frames_at_stop,
        "stopped sessions are inert (no frame decoded after the stop)"
    );

    // The position snapshot while stopped-but-open: the session
    // survives, so the CLI truth is the frozen position, NOT
    // not-ready (apps/cli/src/host.rs:1064-1071 answers not-ready
    // only when no session is open; the stop's `StopClock` effect
    // freezes the clock — state.rs:588-593's row family).
    let frozen_us = host.position_us();
    let snapshot = dispatch(&mut host, Command::MediaGetCurrentTime);
    match snapshot.result {
        Ok(CommandOutcome::Completed(commands::CommandResult::CurrentTime { time, .. })) => {
            assert_eq!(time.0, frozen_us, "the stopped clock's frozen position");
        }
        other => unreachable!("a stopped-but-open session answers its frozen position: {other:?}"),
    }
    for _ in 0..8 {
        host.pump_pipeline();
    }
    assert_eq!(
        host.position_us(),
        frozen_us,
        "the position stays frozen after the stop"
    );

    // The close from Stopped returns to Idle (state.rs:868's
    // `(Stopped, Close)` row; tests/session_flow.rs:378-389) — and
    // the teardown deadline lands HERE, the stop's completion
    // boundary: `DestroySession` drops the demuxer and its input
    // stream (session.rs:1008-1011), and the host's settled-close
    // teardown drops the session-scoped halves (the sink's host
    // twin — host.rs's `SessionClose` route).
    let close = dispatch(&mut host, Command::SessionClose);
    assert!(
        close.result.is_ok(),
        "closes from Stopped: {:?}",
        close.result
    );
    assert_eq!(host.state(), PlaybackState::Idle, "idle after the close");
    assert_eq!(host.session_id(), None, "no session survives the close");
    assert!(
        hooks.demuxer_dropped(),
        "the demuxer is released at the close (DestroySession)"
    );
    assert!(
        hooks.sink_dropped(),
        "the sink twin is released at the close"
    );

    // The position snapshot after the close: the typed not-ready —
    // never a fabricated position.
    let snapshot = dispatch(&mut host, Command::MediaGetCurrentTime);
    match snapshot.result {
        Err(error) => {
            assert_eq!(error.stage, Stage::Session, "{error}");
            assert_eq!(error.code, ErrorCode::NotReady, "{error}");
        }
        Ok(_) => unreachable!("a closed session answers not-ready"),
    }

    // The host reopens cleanly after the stop (no poisoned state).
    let path = testkit::fixture_path(PARENT).must();
    let reopen = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::File { path },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&reopen.result, Ok(CommandOutcome::Accepted { .. })),
        "the host reopens after a stop: {:?}",
        reopen.result
    );
    assert_eq!(host.position_us(), 0, "the reopen starts from zero");
    let close = dispatch(&mut host, Command::SessionClose);
    assert!(close.result.is_ok(), "closes: {:?}", close.result);
}
