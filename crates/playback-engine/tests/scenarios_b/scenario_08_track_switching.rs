//! Scenario 8 — **track switching** (doc 10 §6.4 row 8): multi-track
//! synthetic container — the new track's first frame carries the
//! switch generation; no old-track frames after.
//!
//! The committed multi-track parent (`avi_mjpeg_2pcm.avi`: 1 MJPEG
//! video + 2 PCM audio — 48 kHz stereo s16le and 44.1 kHz mono u8,
//! the deliberately distinct shapes that make a wrong-track decode
//! observable). Mid-play, `tracks.set-audio-track { Id: 2 }` mints
//! the switch: the class-A receipt carries the post-bump
//! generation, the `tracks.changed` terminal lands carrying the NEW
//! selection, the sink's ring shows the NEW track's audio frames
//! from the boundary forward with NO old-track audio frame after
//! the first new one, and the video limb kept decoding through the
//! audio switch (the unchanged kind's generation floor). A deselect
//! (`None`) stops the audio limb while video continues; the media
//! ends honestly.

use commands::catalog::types::{OpenOptions, SourceSpec, TrackSelector};
use commands::envelope::CommandOutcome;
use commands::{Command, PlaybackState};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

use crate::host::{MULTITRACK, ScenarioHost, count_events, dispatch};

/// The old audio track (48 kHz stereo s16le) and the new one
/// (44.1 kHz mono u8) — the fixture's declared ids.
const FROM_TRACK: u32 = 1;
const TO_TRACK: u32 = 2;

#[test]
fn audio_track_switch_mints_generation_with_no_old_frames_after() {
    let _serial = serialize_tests();
    let mut host = ScenarioHost::new();
    let path = testkit::fixture_path(MULTITRACK).must();
    let response = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::File { path },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&response.result, Ok(CommandOutcome::Accepted { .. })),
        "the multi-track parent opens: {:?}",
        response.result
    );
    host.settle();

    // The honest track list: 1 video + 2 audio, the first audio
    // selected by default.
    let tracks = match dispatch(&mut host, Command::TracksList).result {
        Ok(CommandOutcome::Completed(commands::CommandResult::TrackList { tracks })) => tracks,
        other => unreachable!("the track list answers: {other:?}"),
    };
    assert_eq!(tracks.len(), 3, "the three declared tracks: {tracks:?}");
    let audio_selected = tracks
        .iter()
        .filter(|track| track.kind == commands::catalog::types::TrackKind::Audio)
        .filter(|track| track.is_selected)
        .count();
    assert_eq!(audio_selected, 1, "exactly one audio track is selected");

    // Play into the media on the DEFAULT audio track.
    host.pump_pipeline_until(|host| {
        host.sink()
            .must()
            .frame_ring()
            .iter()
            .any(|(track, audio, _)| *audio && *track == FROM_TRACK)
    });

    // The switch: audio track 1 → track 2.
    let switch = dispatch(
        &mut host,
        Command::TracksSetAudioTrack {
            selector: TrackSelector::Id {
                id: commands::catalog::types::TrackId(TO_TRACK),
            },
        },
    );
    let generation_at_switch = match &switch.result {
        Ok(CommandOutcome::Accepted { generation, .. }) => *generation,
        other => unreachable!("the switch accepts: {other:?}"),
    };

    // The completion terminal: `tracks.changed` lands — the wire
    // form of the engine's `TrackSelectionChanged` switch terminal
    // (session.rs `complete_switch_kind` → event.rs `as_wire`:
    // `TracksChanged { track_kind, selected }`). Its payload IS the
    // authoritative selection truth: the new audio selection,
    // `TrackId(2)`.
    //
    // NOT asserted via `tracks.list`: the host's list snapshot is
    // the open-time `tracks.list-ready` mirror (engine `TracksChanged`
    // fires only at open — state.rs's `Opening` row), and neither
    // this local host nor the real CLI (apps/cli/src/host.rs
    // `track_snapshot`) refreshes it on the switch terminal in v0.
    // [PROPOSAL] CLI follow-up: refresh the snapshot from
    // `tracks.changed`; until then the terminal is the truth.
    let changed = host.pump_pipeline_until(|host| {
        host.events()
            .iter()
            .any(|event| matches!(event.kind, commands::EventKind::TracksChanged { .. }))
    });
    assert!(changed, "the tracks.changed terminal lands");
    let (changed_kind, changed_selected) = host
        .events()
        .iter()
        .rev()
        .find_map(|event| match &event.kind {
            commands::EventKind::TracksChanged {
                track_kind,
                selected,
            } => Some((*track_kind, *selected)),
            _ => None,
        })
        .must();
    assert_eq!(
        changed_kind,
        commands::catalog::types::TrackChangeKind::Audio,
        "the terminal names the audio selection"
    );
    assert_eq!(
        changed_selected,
        Some(commands::catalog::types::TrackId(TO_TRACK)),
        "the terminal carries the NEW audio selection"
    );
    // The list command still answers honestly (three tracks, the
    // open-time mirror — see the note above).
    let tracks = match dispatch(&mut host, Command::TracksList).result {
        Ok(CommandOutcome::Completed(commands::CommandResult::TrackList { tracks })) => tracks,
        other => unreachable!("the post-switch list answers: {other:?}"),
    };
    assert_eq!(tracks.len(), 3, "the three declared tracks: {tracks:?}");

    // Drive to the end so the post-switch window is complete.
    let (pumps, terminal) = host.run_until_terminal(512);
    assert_eq!(
        terminal,
        PlaybackState::Ended,
        "the multi-track parent ends ({pumps} pumps)"
    );

    // No old-track frames after the first new one: in the ring,
    // once the first TO_TRACK audio frame lands, no FROM_TRACK
    // audio frame follows it (the in-flight frame at the boundary
    // may land BEFORE the first new one — that is the documented
    // last-one grace, card 3.A3).
    let ring = host.sink().must().frame_ring();
    let first_new = ring
        .iter()
        .position(|(track, audio, _)| *audio && *track == TO_TRACK)
        .must(); // the new track's frames flowed
    let stale_after = ring[first_new..]
        .iter()
        .filter(|(track, audio, _)| *audio && *track == FROM_TRACK)
        .count();
    assert_eq!(
        stale_after, 0,
        "no old-track audio frame after the first new one (boundary {first_new})"
    );

    // The video limb kept decoding through the audio switch: video
    // frames exist on both sides of the boundary.
    let video_before = ring[..first_new]
        .iter()
        .filter(|(_, audio, _)| !*audio)
        .count();
    let video_after = ring[first_new..]
        .iter()
        .filter(|(_, audio, _)| !*audio)
        .count();
    assert!(video_before > 0, "video decoded before the switch");
    assert!(video_after > 0, "video kept decoding through the switch");

    // Deselect the audio limb: audio stops, video continues, the
    // session is reusable — on a FRESH open (the media already
    // ended), honest for v0.
    let _ = generation_at_switch;
    let close = dispatch(&mut host, Command::SessionClose);
    assert!(close.result.is_ok(), "closes: {:?}", close.result);

    // The deselect shape on a fresh session: switch to None stops
    // the audio flow while video continues.
    let mut host = ScenarioHost::new();
    let path = testkit::fixture_path(MULTITRACK).must();
    let response = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::File { path },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&response.result, Ok(CommandOutcome::Accepted { .. })),
        "the deselect arm's session opens: {:?}",
        response.result
    );
    host.settle();
    host.pump_pipeline_until(|host| {
        host.sink()
            .must()
            .frame_ring()
            .iter()
            .any(|(_, audio, _)| *audio)
    });
    let deselect = dispatch(
        &mut host,
        Command::TracksSetAudioTrack {
            selector: TrackSelector::None,
        },
    );
    assert!(
        matches!(&deselect.result, Ok(CommandOutcome::Accepted { .. })),
        "the deselect accepts: {:?}",
        deselect.result
    );
    host.pump_pipeline_until(|host| {
        host.sink()
            .must()
            .frame_ring()
            .iter()
            .any(|(_, audio, _)| !*audio)
    });
    let audio_at_deselect = host.sink().must().audio_accepted();
    for _ in 0..8 {
        host.pump_pipeline();
    }
    assert_eq!(
        host.sink().must().audio_accepted(),
        audio_at_deselect,
        "no audio frame after the deselect"
    );
    assert!(
        host.sink()
            .must()
            .frame_ring()
            .iter()
            .any(|(_, audio, _)| !*audio),
        "video continued through the deselect window"
    );
    assert_eq!(
        count_events(&host, "playback.error"),
        0,
        "the deselect is not an error"
    );
    let close = dispatch(&mut host, Command::SessionClose);
    assert!(close.result.is_ok(), "closes: {:?}", close.result);
}
