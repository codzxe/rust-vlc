//! Scenario 7 — **A/V sync** (doc 10 §6.4 row 7): instrumented
//! sinks, full playback — drift within doc 03 §4.10 tolerance at
//! checkpoints.
//!
//! The parent plays end-to-end; at each media-time checkpoint (the
//! 25/50/75% marks of the declared 2 s), the last-delivered audio
//! stamp and the last-delivered video stamp are within one video
//! frame duration plus one audio quantum of in-flight grace
//! (≤ 40 ms — doc 03 §4.10's tolerance for the 30 fps / 30 chunk-per-second
//! interleaved parent), and at the end of media the final stamps
//! agree within one frame. The engine never fabricates a stamp:
//! every value here is the decoders' own pts.

use commands::catalog::types::{OpenOptions, SourceSpec};
use commands::envelope::CommandOutcome;
use commands::{Command, PlaybackState};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

use crate::host::{PARENT, ScenarioHost, count_events, dispatch, error_events};

/// The parent's declared duration (ns).
const DURATION_NS: i64 = 2_000_000_000;
/// The A/V drift tolerance at every checkpoint: one video frame
/// (33.3 ms) plus one audio quantum's in-flight grace — 40 ms.
const TOLERANCE_NS: i64 = 40_000_000;

/// The last audio / video stamps delivered so far (None before any).
fn last_stamps(host: &ScenarioHost) -> (Option<i64>, Option<i64>) {
    let ring = host.sink().must().frame_ring();
    let audio = ring
        .iter()
        .rev()
        .find(|(_, audio, _)| *audio)
        .map(|(_, _, pts)| *pts);
    let video = ring
        .iter()
        .rev()
        .find(|(_, audio, _)| !*audio)
        .map(|(_, _, pts)| *pts);
    (audio, video)
}

/// Assert the A/V stamp distance at the current point.
fn assert_sync(host: &ScenarioHost, checkpoint: &str) {
    let (audio, video) = last_stamps(host);
    let (Some(audio), Some(video)) = (audio, video) else {
        unreachable!("{checkpoint}: both limbs have delivered frames");
    };
    let drift = (audio - video).abs();
    assert!(
        drift <= TOLERANCE_NS,
        "{checkpoint}: A/V drift {drift} ns exceeds the tolerance \
         (audio {audio}, video {video})"
    );
}

#[test]
fn full_playback_keeps_av_stamps_within_tolerance_at_checkpoints() {
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

    // The checkpoints: 25/50/75% of the declared duration — the
    // position is the consumed-audio truth (the sample counter).
    let checkpoints = [DURATION_NS / 4, DURATION_NS / 2, 3 * DURATION_NS / 4];
    let names = ["25%", "50%", "75%"];
    for (&target_ns, name) in checkpoints.iter().zip(names) {
        let reached = host.pump_pipeline_until(|host| host.position_us() * 1_000 >= target_ns);
        assert!(reached, "the {name} checkpoint is reached");
        assert_sync(&host, name);
    }

    // The end of media: the final stamps agree within one frame.
    let (pumps, terminal) = host.run_until_terminal(512);
    assert_eq!(
        terminal,
        PlaybackState::Ended,
        "the parent ends ({pumps} pumps)"
    );
    assert_sync(&host, "end-of-media");
    assert_eq!(
        count_events(&host, "playback.end-of-media"),
        1,
        "exactly one end-of-media"
    );
    let errors = error_events(&host);
    assert!(
        errors.is_empty(),
        "a clean sync run reports no errors: {errors:?}"
    );

    let close = dispatch(&mut host, Command::SessionClose);
    assert!(close.result.is_ok(), "closes: {:?}", close.result);
}
