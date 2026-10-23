//! Scenario 3 — **pause / resume** (doc 10 §6.4 row 3): zero
//! samples while paused; resume at the same media time.
//!
//! The virtual clock freeze: mid-play, `playback.pause` lands; the
//! pumps that follow deliver NOTHING (the audio counter and the
//! frame ring both freeze — a paused clock must not advance), and
//! the position holds at the pause point. On resume, the samples
//! continue from exactly the held position: the first post-resume
//! audio stamp is the pre-pause stamp plus one quantum (the media
//! time never jumped), and the media still ends honestly.

use commands::catalog::types::{OpenOptions, SourceSpec};
use commands::envelope::CommandOutcome;
use commands::{Command, PlaybackState};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

use crate::host::{PARENT, ScenarioHost, dispatch};

#[test]
fn pause_freezes_samples_and_resume_continues_at_same_media_time() {
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

    // Play into the media: at least 10 audio samples delivered.
    host.pump_pipeline_until(|host| host.sink().map(|s| s.audio_accepted()).must() >= 10);
    let sink = host.sink().must();
    let audio_at_pause = sink.audio_accepted();
    let position_at_pause = host.position_us();
    let last_ring = sink.frame_ring();
    let last_audio_pts = last_ring
        .iter()
        .rev()
        .find(|(_, audio, _)| *audio)
        .map(|(_, _, pts)| *pts)
        .unwrap_or_else(|| unreachable!("audio flowed before the pause"));

    // The pause.
    let pause = dispatch(&mut host, Command::PlaybackPause);
    assert!(pause.result.is_ok(), "the pause lands: {:?}", pause.result);
    assert_eq!(host.state(), PlaybackState::Paused, "paused");

    // Zero samples while paused: eight pumps, nothing moves — not
    // the audio counter, not the position, not the frame ring.
    for _ in 0..8 {
        host.pump_pipeline();
    }
    let sink = host.sink().must();
    assert_eq!(
        sink.audio_accepted(),
        audio_at_pause,
        "zero samples while paused"
    );
    assert_eq!(
        host.position_us(),
        position_at_pause,
        "the position holds at the pause point"
    );
    assert_eq!(
        sink.frame_ring().len(),
        last_ring.len(),
        "the frame ring froze too"
    );

    // The resume: the samples continue at the SAME media time — the
    // first post-pause audio stamp is one quantum past the held
    // stamp (the parent's 60 audio chunks over 2 s ≈ 33.3 ms each).
    let play = dispatch(&mut host, Command::PlaybackPlay);
    assert!(play.result.is_ok(), "resumes: {:?}", play.result);
    assert_eq!(host.state(), PlaybackState::Playing, "playing again");
    host.pump_pipeline_until(|host| {
        host.sink().map(|s| s.audio_accepted()).must() > audio_at_pause
    });

    let quantum = 2_000_000_000 / 60;
    let resumed = {
        let ring = host.sink().must().frame_ring();
        ring.iter()
            .filter(|(_, audio, _)| *audio)
            .map(|(_, _, pts)| *pts)
            .find(|pts| *pts > last_audio_pts)
            .unwrap_or_else(|| unreachable!("audio resumed"))
    };
    assert!(
        resumed - last_audio_pts <= quantum + 1_000_000,
        "the resume continues at the same media time (held {last_audio_pts}, \
         resumed {resumed}, quantum {quantum})"
    );

    // The media still ends honestly after the pause cycle.
    let (pumps, terminal) = host.run_until_terminal(512);
    assert_eq!(
        terminal,
        PlaybackState::Ended,
        "the parent ends after the pause cycle ({pumps} pumps)"
    );
    let close = dispatch(&mut host, Command::SessionClose);
    assert!(close.result.is_ok(), "closes: {:?}", close.result);
}
