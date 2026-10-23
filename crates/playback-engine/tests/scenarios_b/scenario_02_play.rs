//! Scenario 2 — **play** (doc 10 §6.4 row 2): first-frame event +
//! samples flow; A/V stamps within tolerance.
//!
//! The parent plays end-to-end through the command layer: the first
//! delivered frame carries pts 0 (the fixture's `[first_frame]`
//! stamp), samples flow to the full declared counts (60 audio + 60
//! video = 120 frames), the A/V stamp distance at the end stays
//! within one frame duration (30 fps: 33.4 ms — doc 03 §4.10's
//! checkpoint tolerance applied at the end-of-media checkpoint),
//! and exactly one `playback.end-of-media` precedes `Ended`.

use commands::catalog::types::{OpenOptions, SourceSpec};
use commands::envelope::CommandOutcome;
use commands::{Command, PlaybackState};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

use crate::host::{PARENT, ScenarioHost, count_events, dispatch, error_events};

/// One 30 fps frame duration (ns) — the A/V stamp tolerance.
const FRAME_NS: i64 = 33_333_400;

#[test]
fn play_flows_first_frame_samples_and_honest_stamps() {
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
    assert_eq!(host.state(), PlaybackState::Playing, "plays");

    // The first frame: pts 0 (the fixture's [first_frame] stamp).
    let first = host
        .sink()
        .must()
        .frame_ring()
        .first()
        .copied()
        .unwrap_or_else(|| unreachable!("the first frame landed at settle"));
    assert_eq!(first.2, 0, "the first frame's pts is zero: {first:?}");

    // Full play-out to the honest end.
    let (pumps, terminal) = host.run_until_terminal(512);
    assert_eq!(
        terminal,
        PlaybackState::Ended,
        "the parent ends cleanly ({pumps} pumps)"
    );
    let sink = host.sink().must();
    let frames = sink.frames_accepted();
    assert_eq!(frames, 120, "every declared frame delivered ({frames})");
    assert_eq!(sink.audio_accepted(), 60, "all 60 audio samples flow");
    assert_eq!(
        count_events(&host, "playback.end-of-media"),
        1,
        "exactly one end-of-media"
    );
    let errors = error_events(&host);
    assert!(
        errors.is_empty(),
        "a clean play reports no errors: {errors:?}"
    );

    // A/V stamp distance at the end-of-media checkpoint: the last
    // audio and last video stamps are within one frame duration.
    let ring = sink.frame_ring();
    let last_audio = ring
        .iter()
        .rev()
        .find(|(_, audio, _)| *audio)
        .map(|(_, _, pts)| pts)
        .unwrap_or_else(|| unreachable!("audio samples flowed"));
    let last_video = ring
        .iter()
        .rev()
        .find(|(_, audio, _)| !*audio)
        .map(|(_, _, pts)| pts)
        .unwrap_or_else(|| unreachable!("video frames flowed"));
    let drift = (last_audio - last_video).abs();
    assert!(
        drift <= FRAME_NS,
        "A/V stamps within one frame at the end ({drift} ns)"
    );

    let close = dispatch(&mut host, Command::SessionClose);
    assert!(close.result.is_ok(), "closes: {:?}", close.result);
}
