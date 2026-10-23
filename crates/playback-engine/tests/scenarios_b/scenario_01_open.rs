//! Scenario 1 — **open** (doc 10 §6.4 row 1): the valid parent and
//! the refusal variants, through the command layer.
//!
//! Pass criteria: tracks/duration correct or typed error with
//! stage+code; never success-with-garbage.
//!
//! The variants:
//! - the valid parent answers `Accepted` with the honest track list
//!   (1 MJPEG video + 1 PCM audio, the shapes the fixture's
//!   expectation TOML declares), a `duration-known` of exactly the
//!   declared 2.000 s, and a `Playing` settle;
//! - the MP4 parent (same essence, unserved container) refuses at
//!   the PROBE stage, typed — never a garbage success;
//! - an absent path refuses at the INPUT stage, typed and redacted
//!   (the scenario-10 shapes, asserted here from the open lens).

use commands::catalog::types::{OpenOptions, SourceSpec, Timestamp};
use commands::envelope::CommandOutcome;
use commands::{Command, ErrorCode, PlaybackState, Stage};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

use crate::host::{PARENT, ScenarioHost, count_events, dispatch};

/// The parent's declared duration (its expectation TOML).
const DURATION_US: i64 = 2_000_000;

#[test]
fn open_answers_honest_facts_or_typed_refusals() {
    let _serial = serialize_tests();

    // 1. The valid parent: accepted, honest facts.
    let mut host = ScenarioHost::new();
    let path = testkit::fixture_path(PARENT).must();
    let response = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::File { path },
            options: OpenOptions::default(),
        },
    );
    let accepted = match &response.result {
        Ok(outcome @ CommandOutcome::Accepted { .. }) => outcome.clone(),
        other => unreachable!("the parent opens: {other:?}"),
    };
    let CommandOutcome::Accepted {
        session,
        generation: _,
    } = accepted
    else {
        unreachable!()
    };
    assert!(session.is_some() || host.session_id().is_some(), "the accept carries a session");
    assert!(host.generation().is_some(), "the accept carries a generation");
    host.settle();
    assert_eq!(host.state(), PlaybackState::Playing, "the parent plays");

    // The track list: 1 video (mjpeg 320x240) + 1 audio (pcm), per
    // the fixture's expectation — read from the wire, never invented.
    let tracks = match dispatch(&mut host, Command::TracksList).result {
        Ok(CommandOutcome::Completed(commands::CommandResult::TrackList { tracks })) => tracks,
        other => unreachable!("the track list answers: {other:?}"),
    };
    assert_eq!(tracks.len(), 2, "the parent's two tracks: {tracks:?}");
    assert_eq!(tracks[0].kind, commands::catalog::types::TrackKind::Video);
    assert_eq!(tracks[1].kind, commands::catalog::types::TrackKind::Audio);
    assert!(tracks[0].is_selected, "the video track is selected");
    assert!(tracks[1].is_selected, "the audio track is selected");

    // The duration: exactly the declared 2.000 s, and the
    // `duration-known` fact arrived after the track list.
    let duration = match dispatch(&mut host, Command::MediaGetDuration).result {
        Ok(CommandOutcome::Completed(commands::CommandResult::Duration {
            duration: commands::catalog::types::Duration::Known { time: Timestamp(us) },
        })) => us,
        other => unreachable!("the duration is known: {other:?}"),
    };
    assert_eq!(duration, DURATION_US, "the declared duration is the truth");
    assert_eq!(
        count_events(&host, "timing.duration-known"),
        1,
        "exactly one duration-known"
    );

    let close = dispatch(&mut host, Command::SessionClose);
    assert!(close.result.is_ok(), "closes: {:?}", close.result);

    // 2. The MP4 parent: probe-stage refusal, typed — this build's
    //    first vertical serves AVI only (D-018).
    let mut host = ScenarioHost::new();
    let mp4 = testkit::fixture_path("generated/valid/mp4_mjpeg_pcm.mp4").must();
    let response = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::File { path: mp4 },
            options: OpenOptions::default(),
        },
    );
    match response.result {
        Err(error) => {
            assert_eq!(error.stage, Stage::Probe, "{error}");
            assert_eq!(error.code, ErrorCode::Unsupported, "{error}");
            assert!(
                error.message.contains("AVI"),
                "the refusal names the vertical: {error}"
            );
        }
        Ok(_) => unreachable!("an mp4 must refuse in this build"),
    }
    assert!(host.session_id().is_none(), "no session survives a refusal");

    // 3. The absent path: input-stage refusal, typed, redacted.
    let mut host = ScenarioHost::new();
    let absent = std::path::PathBuf::from("/nonexistent-lumen-scenario-01.avi");
    let response = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::File { path: absent },
            options: OpenOptions::default(),
        },
    );
    match response.result {
        Err(error) => {
            assert_eq!(error.stage, Stage::Input, "{error}");
            assert_eq!(error.code, ErrorCode::Inaccessible, "{error}");
            assert!(
                error.message.contains("nonexistent-lumen-scenario-01.avi"),
                "the basename survives (V8): {error}"
            );
        }
        Ok(_) => unreachable!("an absent path must refuse"),
    }
    assert!(host.session_id().is_none());
    assert_eq!(host.state(), PlaybackState::Idle);
}
