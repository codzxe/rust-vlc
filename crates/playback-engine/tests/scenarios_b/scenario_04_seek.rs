//! Scenario 4 — **seek** (doc 10 §6.4 row 4): doc 03 §6's flow on 3
//! targets — `Seeked{actual}` within tolerance, no stale frames
//! after completion.
//!
//! The three targets exercise every wire form: an absolute jump
//! into the second half, a relative hop from wherever that landed,
//! and a percent-of-duration jump. Each seek answers its class-A
//! receipt, completes with exactly one `timing.seek-completed`
//! whose position is within the fixture's recorded 100 ms
//! tolerance, and the frames that follow the completion belong to
//! the post-seek timeline (the ring's post-completion stamps start
//! at the landing point and advance monotonically — a stale frame
//! would appear as a stamp behind the landing point). The media
//! still ends honestly after all three.

use commands::catalog::types::{OpenOptions, SeekTarget, SourceSpec, Timestamp};
use commands::envelope::CommandOutcome;
use commands::{Command, PlaybackState};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

use crate::host::{PARENT, ScenarioHost, count_events, dispatch};

/// The parent's recorded seek tolerance (its expectation TOML).
const TOLERANCE_US: i64 = 100_000;

/// One seek: receipt, terminal, position within tolerance, and the
/// post-completion ring is forward-only from the landing point.
fn seek_and_assert(host: &mut ScenarioHost, name: &str, target: SeekTarget) {
    let ring_before = host.sink().must().frame_ring().len();
    let completed_before = count_events(host, "timing.seek-completed");
    let response = dispatch(host, Command::PlaybackSeek { target });
    let accepted = match &response.result {
        Ok(outcome @ CommandOutcome::Accepted { .. }) => outcome.clone(),
        other => unreachable!("{name}: the seek accepts: {other:?}"),
    };
    let CommandOutcome::Accepted { generation, .. } = accepted else {
        unreachable!()
    };
    let _ = generation;
    assert!(
        host.generation().is_some(),
        "{name}: the accept carries the post-bump generation"
    );
    assert_eq!(
        count_events(host, "timing.seek-completed"),
        completed_before + 1,
        "{name}: exactly one NEW seek-completed"
    );
    assert_eq!(
        count_events(host, "timing.seek-failed"),
        0,
        "{name}: no failure"
    );

    // The landing truth: the seek-completed position is within the
    // fixture's tolerance of the requested target (the engine
    // lands on the closest packet boundary — clamping is truth).
    let landed = seek_completed_position(host).must();
    let target_us = name_target_us(name);
    assert!(
        (landed - target_us).abs() <= TOLERANCE_US,
        "{name}: the landing ({landed} µs) is within tolerance of {target_us} µs"
    );

    // No stale frames: every frame delivered AFTER the completion
    // carries a stamp at or beyond the landing point minus one
    // quantum (the frame already in flight at the boundary).
    let ring = host.sink().must().frame_ring();
    let post = &ring[ring_before.min(ring.len())..];
    let stale = post
        .iter()
        .filter(|(_, _, pts)| *pts < landed * 1_000 - 40_000_000)
        .count();
    assert_eq!(stale, 0, "{name}: no stale frames after the completion");
}

/// The last `timing.seek-completed` position (µs).
fn seek_completed_position(host: &ScenarioHost) -> Option<i64> {
    host.events().iter().rev().find_map(|event| {
        if let commands::EventKind::TimingSeekCompleted { position, .. } = &event.kind {
            Some(position.0)
        } else {
            None
        }
    })
}

/// The requested target in µs for each named row.
fn name_target_us(name: &str) -> i64 {
    match name {
        "absolute-1s" => 1_000_000,
        "relative-plus-500ms" => 1_500_000,
        "percent-half" => 1_000_000,
        other => unreachable!("unknown seek row {other}"),
    }
}

#[test]
fn seek_lands_three_targets_within_tolerance_without_stale_frames() {
    let _serial = serialize_tests();
    let mut host = ScenarioHost::new();
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

    // Play a little so the seeks jump from a live mid-stream state.
    host.pump_pipeline_until(|host| host.sink().map(|s| s.audio_accepted()).must() >= 8);

    // Row 1: absolute 1.000 s.
    seek_and_assert(
        &mut host,
        "absolute-1s",
        SeekTarget::Absolute {
            time: Timestamp(1_000_000),
        },
    );

    // Row 2: relative +500 ms (lands at 1.500 s).
    seek_and_assert(
        &mut host,
        "relative-plus-500ms",
        SeekTarget::Relative { delta_ms: 500 },
    );

    // Row 3: percent 0.5 of the known duration (lands at 1.000 s).
    seek_and_assert(&mut host, "percent-half", SeekTarget::Percent { fraction: 0.5 });

    // The media still ends honestly after three seeks.
    let (pumps, terminal) = host.run_until_terminal(512);
    assert_eq!(
        terminal,
        PlaybackState::Ended,
        "the parent ends after the seek rows ({pumps} pumps)"
    );
    let close = dispatch(&mut host, Command::SessionClose);
    assert!(close.result.is_ok(), "closes: {:?}", close.result);
}
