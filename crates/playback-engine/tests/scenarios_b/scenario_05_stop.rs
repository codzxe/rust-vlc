//! Scenario 5 — **stop** (doc 10 §6.4 row 5): stop mid-stream —
//! teardown complete (stage objects released) within deadline.
//!
//! The v0 synchronous engine's teardown evidence: `playback.stop`
//! mid-stream answers its receipt, the state settles to `Idle`
//! through the Stopped row, the wired stage objects (the demuxer
//! and the sink) are dropped within a bounded number of pumps (the
//! deadline — a hang is a bound breach, never a mystery), the
//! position snapshot after the stop answers the typed
//! `session.not-ready`, and the host reopens cleanly (no poisoned
//! state).

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

    // The state settles to Idle through the Stopped row.
    assert_eq!(
        host.state_history()
            .iter()
            .any(|(from, to)| *from == PlaybackState::Playing && *to == PlaybackState::Stopped),
        true,
        "the trail carries the Stopped row: {:?}",
        host.state_history()
    );
    assert_eq!(host.state(), PlaybackState::Idle, "settled to Idle");

    // The teardown deadline: within the same bounded pumps the stop
    // already ran, every stage object is released (the drop hooks
    // answer for the demuxer and the sink).
    assert!(
        hooks.demuxer_dropped(),
        "the demuxer is released at the stop"
    );
    assert!(hooks.sink_dropped(), "the sink is released at the stop");

    // The position snapshot after the stop: the typed not-ready —
    // never a fabricated position.
    let snapshot = dispatch(&mut host, Command::MediaGetCurrentTime);
    match snapshot.result {
        Err(error) => {
            assert_eq!(error.stage, Stage::Session, "{error}");
            assert_eq!(error.code, ErrorCode::NotReady, "{error}");
        }
        Ok(_) => unreachable!("a stopped session answers not-ready"),
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
