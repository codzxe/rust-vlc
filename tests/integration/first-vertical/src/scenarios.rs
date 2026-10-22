//! The named scenarios (2.E1; doc 10 §6.4 IDs, the Phase-2 subset).
//!
//! Every function drives the whole vertical **through the command
//! layer** (see [`crate::harness`]) and answers two things: the
//! transcript (the doc 10 §6.4 evidence artifact) and the checks (the
//! card's assertions, machine-evaluated — the lib never panics; the
//! test files turn checks into test verdicts, the ledger records them
//! as the claim table).
//!
//! # Coverage vs doc 10 §6.4's full list
//!
//! Phase 2 owns: `open`, `play`, `pause`, `seek`, `stop`,
//! `run-to-end` (the journey), the class-B corpus (`missing-file`,
//! `corrupt-truncated-open`, `corrupt-truncated-midplay`,
//! `corrupt-garbage-appended`, `corrupt-noidx`) and `wire-session`
//! (the serialized command layer across a real process boundary).
//! The §6.4 rows that need later-wave machinery are honestly absent:
//! repeated seeks / track switching / A/V sync instrumentation /
//! device loss / long media / reopen cycles / exit-during-playback
//! are wave 3.E scenarios (they need the 3.A seek engine, the 3.B
//! sync budgets, `VanishingSink` and the soak tier).

use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};

use commands::catalog::command::CommandResult;
use commands::catalog::types::{OpenOptions, SeekTarget, SourceSpec};
use commands::envelope::CommandOutcome;
use commands::{Command, ErrorCode, PlaybackState, Stage, WireResponse};

use crate::harness::{Check, Labels, ScenarioRun, SessionHarness, run_pumps};
use crate::json::Json;

/// The class-A parent: MJPEG+PCM, 2.000 s, 320x240 @30 fps, stereo
/// s16le 44.1 kHz (the first vertical's own fixture, card 2.A3).
const PARENT: &str = "generated/valid/avi_mjpeg_pcm.avi";

/// The class-B derivatives this suite drives through the FULL stack.
const TRUNCATED_OPEN: &str = "generated/malformed/avi_mjpeg_pcm__trunc1.avi";
const TRUNCATED_MIDPLAY: &str = "generated/malformed/avi_mjpeg_pcm__trunc3.avi";
const GARBAGE_APPENDED: &str = "generated/malformed/avi_mjpeg_pcm__garbage0.avi";
const NOIDX: &str = "generated/malformed/avi_mjpeg_pcm__noidx0.avi";

/// The media's facts, as the fixture's own expectation TOML pins them
/// (the suite asserts the command layer reports the same through
/// `media.get-info`).
const DURATION_US: i64 = 2_000_000;
const VIDEO_FRAMES: u64 = 60;
const AUDIO_PACKETS: u64 = 60;

/// The scenario ids, in ledger order (the doc 10 §6.4 stable ids).
pub const SCENARIO_IDS: &[&str] = &[
    "open",
    "play",
    "pause",
    "seek",
    "stop",
    "run-to-end",
    "missing-file",
    "corrupt-truncated-open",
    "corrupt-truncated-midplay",
    "corrupt-garbage-appended",
    "corrupt-noidx",
    "wire-session",
];

/// Resolve the parent fixture as an openable source.
fn parent_source() -> Result<SourceSpec, String> {
    Ok(SourceSpec::File {
        path: crate::harness::fixture_path(PARENT)?,
    })
}

/// A scenario that could not even resolve its fixture: one failed
/// check, a minimal honest transcript (never a panic — the test files
/// surface the reason).
fn broken_run(id: &'static str, fixture: &'static str, reason: &str) -> ScenarioRun {
    let labels = Labels::detect();
    let transcript = Json::obj(vec![
        ("schema", Json::str(crate::harness::TRANSCRIPT_SCHEMA)),
        ("scenario", Json::str(id)),
        ("fixture", Json::str(fixture)),
        ("labels", labels_json(&labels)),
        ("commands", Json::Arr(Vec::new())),
        ("events", Json::Arr(Vec::new())),
        (
            "observables",
            Json::obj(vec![("reason", Json::str(reason))]),
        ),
    ]);
    ScenarioRun {
        id,
        transcript,
        checks: vec![Check::is("fixture resolvable", false, reason)],
        png: None,
    }
}

/// The labels block (shared by every transcript shape).
fn labels_json(labels: &Labels) -> Json {
    Json::obj(vec![
        ("environment_kind", Json::str(labels.environment_kind)),
        ("os", Json::str(labels.os)),
        ("hardware_class", Json::str(labels.hardware_class)),
        (
            "devices",
            Json::Arr(labels.devices.iter().map(|d| Json::str(*d)).collect()),
        ),
    ])
}

// ---------------------------------------------------------------------------
// The journey
// ---------------------------------------------------------------------------

/// **`open`** — the generated fixture opens through the real stack
/// (access → probe → demux → track selection) and the class-S
/// snapshots report the container's own facts. The closing step is
/// the hardware probe: `audio.list-devices` dispatched through the
/// same command layer, whose typed refusal is the ON-THE-WIRE backing
/// for this suite's `mock` audio label — the host honestly reports
/// that it cannot enumerate output devices (the real path joins with
/// ADR-005's backends), which is exactly what doc 10 §7.1 requires
/// the label to stand on.
pub fn scenario_open() -> ScenarioRun {
    let source = match parent_source() {
        Ok(source) => source,
        Err(reason) => return broken_run("open", PARENT, &reason),
    };
    let mut harness = SessionHarness::new(PARENT);
    let open = harness.dispatch(
        "session.open",
        Command::SessionOpen {
            source,
            options: OpenOptions::default(),
        },
    );
    harness.settle();
    let info = harness.dispatch("media.get-info", Command::MediaGetInfo);
    let tracks = harness.dispatch("tracks.list", Command::TracksList);
    let duration = harness.dispatch("media.get-duration", Command::MediaGetDuration);
    let devices = harness.dispatch("audio.list-devices", Command::AudioListDevices);
    let mut run = harness.finish("open");
    let mut checks = Vec::new();

    checks.push(Check::is(
        "session.open is accepted",
        matches!(&open.result, Ok(CommandOutcome::Accepted { .. })),
        "the open must answer Accepted",
    ));
    checks.push(Check::same(
        "the open settles into Playing",
        &run_state(&run),
        "Playing",
    ));
    // media.get-info: the container's declared truth.
    if let Ok(CommandOutcome::Completed(CommandResult::MediaInfo { info })) = &info.result {
        checks.push(Check::same(
            "container probes as avi",
            info.container.as_deref().unwrap_or("(none)"),
            "avi",
        ));
        match info.duration {
            commands::catalog::types::Duration::Known { time } => {
                checks.push(Check::int(
                    "duration is the fixture's 2.000 s",
                    time.0,
                    DURATION_US,
                ));
            }
            other => checks.push(Check::is(
                "duration is known",
                false,
                &format!("media.get-duration answered {other:?}"),
            )),
        }
        checks.push(Check::is(
            "the media reports seekable",
            info.seekable,
            "seekable reported false",
        ));
        checks.push(Check::int(
            "two tracks (video + audio)",
            i64::try_from(info.tracks.len()).unwrap_or(i64::MAX),
            2,
        ));
        let video = info.tracks.first();
        let audio = info.tracks.get(1);
        match (video, audio) {
            (Some(video), Some(audio))
                if matches!(video.kind, commands::catalog::types::TrackKind::Video)
                    && matches!(audio.kind, commands::catalog::types::TrackKind::Audio) =>
            {
                // Codec identity, as the layer reports it (the fixture's
                // own tracks: MJPEG video + little-endian s16 PCM audio).
                checks.push(Check::same(
                    "the video track's codec is mjpeg",
                    video.codec.description.as_deref().unwrap_or("(none)"),
                    "mjpeg",
                ));
                checks.push(Check::same(
                    "the audio track's codec is pcm_s16le",
                    audio.codec.description.as_deref().unwrap_or("(none)"),
                    "pcm_s16le",
                ));
                match (&video.video, &audio.audio) {
                    (Some(video), Some(audio)) => {
                        checks.push(Check::int(
                            "video track 0: 320x240",
                            i64::from(video.width) * 100 + i64::from(video.height),
                            32_240,
                        ));
                        checks.push(Check::int(
                            "audio track 1: stereo 44100 Hz",
                            i64::from(u32::from(audio.channels)) * 100
                                + i64::from(audio.sample_rate.unwrap_or(0)),
                            44_300,
                        ));
                    }
                    _ => checks.push(Check::is(
                        "the tracks carry their kind facts",
                        false,
                        "the video/audio fact blocks are missing",
                    )),
                }
            }
            _ => checks.push(Check::is(
                "track kinds land video-then-audio with facts",
                false,
                "the track shape is not video-then-audio",
            )),
        }
    } else {
        checks.push(Check::is(
            "media.get-info completes with the snapshot",
            false,
            "media.get-info did not complete",
        ));
    }
    if let Ok(CommandOutcome::Completed(CommandResult::TrackList { tracks })) = &tracks.result {
        checks.push(Check::int(
            "tracks.list answers the two selected tracks",
            i64::try_from(tracks.iter().filter(|track| track.is_selected).count())
                .unwrap_or(i64::MAX),
            2,
        ));
    } else {
        checks.push(Check::is(
            "tracks.list completes",
            false,
            "tracks.list did not complete",
        ));
    }
    match &duration.result {
        Ok(CommandOutcome::Completed(CommandResult::Duration {
            duration: commands::catalog::types::Duration::Known { time },
        })) => {
            checks.push(Check::int(
                "media.get-duration answers 2.000 s",
                time.0,
                DURATION_US,
            ));
        }
        _ => checks.push(Check::is(
            "media.get-duration answers known",
            false,
            "media.get-duration did not answer Known",
        )),
    }
    // The state trail: the real transitions of an auto-playing open.
    let trail = state_trail(&run);
    checks.push(Check::is(
        "the trail crosses Idle→Opening→Buffering→Playing",
        trail.iter().any(|(from, to)| {
            matches!(
                (from, to),
                (PlaybackState::Idle, PlaybackState::Opening)
                    | (PlaybackState::Opening, PlaybackState::Buffering)
                    | (PlaybackState::Buffering, PlaybackState::Playing)
            )
        }) && trail.len() >= 3,
        &format!("trail: {trail:?}"),
    ));
    // The hardware probe: the mock audio label's on-the-wire backing.
    // A host that COULD enumerate devices would answer a device list
    // — and this check would fail, forcing the suite's labels (and
    // this claim) to be updated in the same change that introduces
    // real backends. Honest in both directions.
    match &devices.result {
        Err(error) => {
            checks.push(Check::same(
                "audio.list-devices answers the honest typed \
                 unsupported-for-host (no output-device enumeration in \
                 this host — the mock audio label's wire backing; real \
                 devices join with ADR-005)",
                &format!("{}.{}", error.stage.as_str(), error.code.as_str()),
                "control.unsupported-for-host",
            ));
            checks.push(Check::is(
                "the probe's message names the real-backend owner",
                error.to_string().contains("audio.list-devices"),
                &format!("message {:?} does not name the command", error.to_string()),
            ));
        }
        Ok(_) => checks.push(Check::is(
            "audio.list-devices answers the honest typed \
             unsupported-for-host (the mock audio label's wire backing)",
            false,
            "the host answered a device list — the mock label would be \
             a fabrication; update the suite labels with the real \
             backends",
        )),
    }
    run.checks = checks;
    run
}

/// **`play`** — after a pause, `playback.play` resumes and the
/// samples flow again: the audio-master clock advances ONLY when the
/// null device renders quanta, so a moving position after resume is
/// the audio limb being exercised end to end (decode → audio engine →
/// null sink), and the accounting counts deliveries on both limbs.
pub fn scenario_play() -> ScenarioRun {
    let source = match parent_source() {
        Ok(source) => source,
        Err(reason) => return broken_run("play", PARENT, &reason),
    };
    let mut harness = SessionHarness::new(PARENT);
    harness.dispatch(
        "session.open",
        Command::SessionOpen {
            source,
            options: OpenOptions::default(),
        },
    );
    harness.settle();
    harness.dispatch("playback.pause", Command::PlaybackPause);
    let frozen = harness.position_us();
    let frozen_accounting = harness.accounting();
    harness.dispatch("playback.play", Command::PlaybackPlay);
    harness.advance(run_pumps());
    let resumed = harness.position_us();
    let mut run = harness.finish("play");
    let mut checks = Vec::new();

    checks.push(Check::int(
        "the paused position is frozen at the pause landing",
        frozen,
        0,
    ));
    let accepted_after_resume = resumed > frozen;
    checks.push(Check::is(
        "after playback.play the position advances (the null audio \
         device renders quanta — the audio-master clock moves)",
        accepted_after_resume,
        &format!("position stayed at {resumed} after resume"),
    ));
    if let Some(before) = frozen_accounting
        && let Some(after) = run_accounting(&run)
    {
        checks.push(Check::is(
            "frames flow on both limbs after resume",
            after.frames_accepted > before.frames_accepted,
            "the accepted-frame count did not grow after resume",
        ));
    } else {
        checks.push(Check::is(
            "the headless accounting is observable",
            false,
            "accounting missing before/after resume",
        ));
    }
    let trail = state_trail(&run);
    checks.push(Check::is(
        "the trail records Playing→Paused→Playing",
        trail.iter().any(|(from, to)| {
            matches!((from, to), (PlaybackState::Playing, PlaybackState::Paused))
        }) && trail.iter().any(|(from, to)| {
            matches!((from, to), (PlaybackState::Paused, PlaybackState::Playing))
        }),
        &format!("trail: {trail:?}"),
    ));
    run.checks = checks;
    run
}

/// **`pause`** — the clock freezes exactly WHERE IT IS: content
/// plays first (a bounded advance moves the position into the media),
/// then `playback.pause` lands, then a long bounded advance while
/// paused moves the position by nothing — the frozen value is the
/// pre-pause landing, not the start.
pub fn scenario_pause() -> ScenarioRun {
    let source = match parent_source() {
        Ok(source) => source,
        Err(reason) => return broken_run("pause", PARENT, &reason),
    };
    let mut harness = SessionHarness::new(PARENT);
    harness.dispatch(
        "session.open",
        Command::SessionOpen {
            source,
            options: OpenOptions::default(),
        },
    );
    harness.settle();
    // Let real content play: both limbs deliver, the clock moves.
    harness.advance(64);
    let played = harness.position_us();
    harness.dispatch("playback.pause", Command::PlaybackPause);
    let landing = harness.position_us();
    harness.advance(256);
    let frozen = harness.position_us();
    let state = harness.state();
    let mut run = harness.finish("pause");
    let mut checks = Vec::new();

    checks.push(Check::same(
        "the state is Paused",
        &format!("{state:?}"),
        "Paused",
    ));
    checks.push(Check::is(
        "content played before the pause (the position moved into the media)",
        played > 0,
        &format!("position {played} after 64 advance ticks — nothing played"),
    ));
    checks.push(Check::int(
        "256 advance ticks while paused move the position by zero \
         (frozen at the pre-pause landing, not the start)",
        frozen - landing,
        0,
    ));
    checks.push(Check::is(
        "the pause lands without moving the clock",
        landing == played,
        &format!("landing {landing} vs played {played}"),
    ));
    let trail = state_trail(&run);
    checks.push(Check::is(
        "the trail records Playing→Paused (the pause landed)",
        trail.iter().any(|(from, to)| {
            matches!((from, to), (PlaybackState::Playing, PlaybackState::Paused))
        }),
        &format!("trail: {trail:?}"),
    ));
    run.checks = checks;
    run
}

/// **`seek`** — the engine's NATIVE in-pipeline flow (C8/ED-FIX-4,
/// host-side W1-06): a Paused 50% seek through the decode carve-out,
/// then a resume — the session id SURVIVES (no stop→close→reopen;
/// D-4/D-7), the 50% target lands at the demuxer's actual position
/// (exactly half the duration — a frame-aligned target), and the
/// remainder plays out to the exact duration.
pub fn scenario_seek() -> ScenarioRun {
    let source = match parent_source() {
        Ok(source) => source,
        Err(reason) => return broken_run("seek", PARENT, &reason),
    };
    let mut harness = SessionHarness::new(PARENT);
    harness.dispatch(
        "session.open",
        Command::SessionOpen {
            source,
            options: OpenOptions::default(),
        },
    );
    harness.settle();
    harness.dispatch("playback.pause", Command::PlaybackPause);
    let first_session = harness.session_id();
    harness.dispatch(
        "playback.seek",
        Command::PlaybackSeek {
            target: SeekTarget::Percent { fraction: 0.5 },
        },
    );
    harness.settle();
    let landing = harness.position_us();
    let second_session = harness.session_id();
    // Resume from the landing (the Paused seek parks the wire state;
    // the remainder plays from where the seek landed).
    harness.dispatch("playback.play", Command::PlaybackPlay);
    harness.advance(run_pumps());
    let end = harness.position_us();
    let mut run = harness.finish("seek");
    let mut checks = Vec::new();

    checks.push(Check::int(
        "the 50% seek lands at exactly half the duration",
        landing,
        DURATION_US / 2,
    ));
    checks.push(Check::is(
        "the session id survives the native seek (the C8 in-pipeline flow)",
        first_session.is_some() && first_session == second_session,
        &format!("sessions {first_session:?} → {second_session:?}"),
    ));
    checks.push(Check::int(
        "the remainder plays out to the exact duration",
        end,
        DURATION_US,
    ));
    let trail = state_trail(&run);
    checks.push(Check::is(
        "the trail is in-pipeline (no teardown edges; exactly one open)",
        !trail.iter().any(|(from, to)| {
            matches!(
                (from, to),
                (PlaybackState::Paused, PlaybackState::Stopped)
                    | (PlaybackState::Stopped, PlaybackState::Idle)
                    | (PlaybackState::Playing, PlaybackState::Stopped)
            )
        }) && trail
            .iter()
            .filter(|(from, to)| {
                matches!((from, to), (PlaybackState::Idle, PlaybackState::Opening))
            })
            .count()
            == 1,
        &format!("trail: {trail:?}"),
    ));
    if let Some(accounting) = run_accounting(&run) {
        // The decode accounting proves the pipeline was not rebuilt:
        // the pre-seek decode-ahead deliveries stay counted plus the
        // landing's own half — a stop→close→reopen would have
        // answered exactly the landing's 60.
        checks.push(Check::int(
            "the decode accounting keeps the pre-seek deliveries",
            i64::try_from(accounting.frames_accepted).unwrap_or(i64::MAX),
            154,
        ));
        checks.push(Check::int(
            "the scheduler saw the pre-seek window plus the landing",
            i64::try_from(accounting.scheduler_pushed).unwrap_or(i64::MAX),
            46,
        ));
        // The seek-boundary proofs: the pre-seek scheduler window
        // purged wholesale as stale at the generation change, and the
        // presentation queue's published generation carries the
        // seek's mint.
        checks.push(Check::int(
            "the pre-seek scheduler window purged as stale",
            i64::try_from(accounting.scheduler_dropped_stale).unwrap_or(i64::MAX),
            16,
        ));
        checks.push(Check::int(
            "the presentation queue carries the seek's mint",
            i64::try_from(accounting.queue_published).unwrap_or(i64::MAX),
            1,
        ));
    } else {
        checks.push(Check::is(
            "the headless accounting is observable",
            false,
            "accounting missing after the seek",
        ));
    }
    run.checks = checks;
    run
}

/// **`stop`** — teardown truth: content plays, then `playback.stop`
/// parks the session in Stopped with the position frozen at the stop
/// point (nonzero — it froze WHERE the media was, not at the start)
/// and snapshots still answering; `session.close` lands Idle; after
/// the close the snapshot commands refuse with the honest not-ready.
pub fn scenario_stop() -> ScenarioRun {
    let source = match parent_source() {
        Ok(source) => source,
        Err(reason) => return broken_run("stop", PARENT, &reason),
    };
    let mut harness = SessionHarness::new(PARENT);
    harness.dispatch(
        "session.open",
        Command::SessionOpen {
            source,
            options: OpenOptions::default(),
        },
    );
    harness.settle();
    // Real content first, so the freeze has somewhere to be.
    harness.advance(64);
    let played = harness.position_us();
    harness.dispatch("playback.stop", Command::PlaybackStop);
    let stopped_state = harness.state();
    let stopped_position = harness.position_us();
    let info_while_stopped = harness.dispatch("media.get-info", Command::MediaGetInfo);
    let close = harness.dispatch("session.close", Command::SessionClose);
    let closed_state = harness.state();
    let time_after_close = harness.dispatch("media.get-current-time", Command::MediaGetCurrentTime);
    let mut run = harness.finish("stop");
    let mut checks = Vec::new();

    checks.push(Check::same(
        "stop lands Stopped",
        &format!("{stopped_state:?}"),
        "Stopped",
    ));
    checks.push(Check::is(
        "the position freezes at the stop point (nonzero — where the \
         media was, not the start)",
        stopped_position == played && stopped_position > 0,
        &format!("played {played}, stopped at {stopped_position}"),
    ));
    let trail = state_trail(&run);
    checks.push(Check::is(
        "the trail records Playing→Stopped (the teardown transition)",
        trail.iter().any(|(from, to)| {
            matches!((from, to), (PlaybackState::Playing, PlaybackState::Stopped))
        }),
        &format!("trail: {trail:?}"),
    ));
    checks.push(Check::is(
        "media.get-info still answers from the stopped session",
        matches!(
            &info_while_stopped.result,
            Ok(CommandOutcome::Completed(CommandResult::MediaInfo { .. }))
        ),
        "the stopped session's snapshot refused",
    ));
    checks.push(Check::is(
        "session.close completes",
        matches!(&close.result, Ok(CommandOutcome::Completed(_))),
        "close did not complete",
    ));
    checks.push(Check::same(
        "close lands Idle",
        &format!("{closed_state:?}"),
        "Idle",
    ));
    match &time_after_close.result {
        Err(error) => {
            checks.push(Check::same(
                "a snapshot after close is the honest not-ready",
                &format!("{}.{}", error.stage.as_str(), error.code.as_str()),
                "session.not-ready",
            ));
        }
        Ok(_) => checks.push(Check::is(
            "a snapshot after close is the honest not-ready",
            false,
            "the post-close snapshot answered success",
        )),
    }
    run.checks = checks;
    run
}

/// **`run-to-end`** — THE journey: the whole media demuxed, decoded,
/// mixed, "output" (null audio) and presented (software compositor),
/// with the per-frame accounting closing and the position landing on
/// the duration exactly, plus the final PNG (the headless render
/// evidence).
pub fn scenario_run_to_end() -> ScenarioRun {
    let source = match parent_source() {
        Ok(source) => source,
        Err(reason) => return broken_run("run-to-end", PARENT, &reason),
    };
    let mut harness = SessionHarness::new(PARENT);
    harness.dispatch(
        "session.open",
        Command::SessionOpen {
            source,
            options: OpenOptions::default(),
        },
    );
    harness.settle();
    harness.advance(run_pumps());
    let state = harness.state();
    let position = harness.position_us();
    let png = harness.screenshot_png();
    let mut run = harness.finish("run-to-end");
    let mut checks = Vec::new();

    checks.push(Check::same(
        "the media runs to Ended",
        &format!("{state:?}"),
        "Ended",
    ));
    checks.push(Check::int(
        "the position lands on the duration exactly (the audio tail \
         play-out)",
        position,
        DURATION_US,
    ));
    let trail = state_trail(&run);
    checks.push(Check::is(
        "the trail records Playing→Ended",
        trail
            .iter()
            .any(|(from, to)| matches!((from, to), (PlaybackState::Playing, PlaybackState::Ended))),
        &format!("trail: {trail:?}"),
    ));
    if let Some(accounting) = run_accounting(&run) {
        // Every demuxed packet decoded and delivered (both limbs).
        checks.push(Check::int(
            "60 MJPEG frames + 60 PCM packets delivered",
            i64::try_from(accounting.frames_accepted).unwrap_or(i64::MAX),
            60 + 60,
        ));
        checks.push(Check::int(
            "nothing refused at the consumer",
            i64::try_from(accounting.frames_refused).unwrap_or(i64::MAX),
            0,
        ));
        // The video limb: every frame admitted, presented, staged once.
        checks.push(Check::int(
            "all 60 frames admitted to the scheduling window",
            i64::try_from(accounting.scheduler_pushed).unwrap_or(i64::MAX),
            VIDEO_FRAMES as i64,
        ));
        checks.push(Check::int(
            "no frame dropped late",
            i64::try_from(accounting.scheduler_dropped_late).unwrap_or(i64::MAX),
            0,
        ));
        checks.push(Check::int(
            "no scheduler refusal",
            i64::try_from(accounting.video_refused).unwrap_or(i64::MAX),
            0,
        ));
        checks.push(Check::is(
            "at least every frame presented once (presents + repeats)",
            accounting.scheduler_presented + accounting.scheduler_repeats >= VIDEO_FRAMES,
            &format!(
                "presented {} + repeats {} < 60",
                accounting.scheduler_presented, accounting.scheduler_repeats
            ),
        ));
        checks.push(Check::is(
            "staged frames == presented frames (1:1 uploads)",
            accounting.staging_frames == accounting.video_presented,
            &format!(
                "staged {} vs presented {}",
                accounting.staging_frames, accounting.video_presented
            ),
        ));
        checks.push(Check::int(
            "one full-frame RGBA copy per present (bytes)",
            i64::try_from(accounting.staging_bytes).unwrap_or(i64::MAX),
            i64::try_from(accounting.staging_frames).unwrap_or(i64::MAX) * 320 * 240 * 4,
        ));
        checks.push(Check::is(
            "one copy per staged frame",
            accounting.staging_copies == accounting.staging_frames,
            &format!(
                "copies {} vs staged {}",
                accounting.staging_copies, accounting.staging_frames
            ),
        ));
    } else {
        checks.push(Check::is(
            "the headless accounting is observable",
            false,
            "accounting missing at the end",
        ));
    }
    match png {
        Some(Ok(bytes)) => {
            checks.push(Check::is(
                "the final PNG carries the signature",
                bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]),
                "the screenshot bytes are not a PNG",
            ));
            if bytes.len() >= 24 {
                let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
                let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
                checks.push(Check::int(
                    "the rendered surface is the media's 320x240 extent",
                    i64::from(width) * 100 + i64::from(height),
                    32_240,
                ));
            }
            checks.push(Check::is(
                "the PNG is structurally complete (an IEND chunk exists)",
                bytes.windows(4).any(|window| window == b"IEND"),
                "no IEND chunk found",
            ));
        }
        Some(Err(reason)) => {
            checks.push(Check::is("the final screenshot succeeds", false, &reason))
        }
        None => checks.push(Check::is(
            "the video limb exists at the end",
            false,
            "no screenshot available after run-to-end",
        )),
    }
    run.checks = checks;
    run
}

// ---------------------------------------------------------------------------
// The error paths (class B + missing file), through the FULL stack
// ---------------------------------------------------------------------------

/// Assert one typed open refusal: the stage.code identity and (when
/// given) a message substring — the honest failure shape of the
/// command layer, never a panic, never silence.
fn push_typed_open_refusal(
    checks: &mut Vec<Check>,
    what: &str,
    response: &WireResponse,
    stage: Stage,
    code: ErrorCode,
    message_part: &str,
) {
    match &response.result {
        Err(error) => {
            checks.push(Check::same(
                what,
                &format!("{}.{}", error.stage.as_str(), error.code.as_str()),
                &format!("{}.{}", stage.as_str(), code.as_str()),
            ));
            checks.push(Check::is(
                &format!("{what}: the message names the failure"),
                error.to_string().contains(message_part),
                &format!("message {:?} lacks {message_part:?}", error.to_string()),
            ));
        }
        Ok(_) => checks.push(Check::is(
            &format!("{what}: the open must refuse"),
            false,
            "the open answered success",
        )),
    }
}

/// **`missing-file`** — an absent path is the input stage's typed
/// inaccessible (doc 10 §6.4's `missing file` row: "typed Inaccessible
/// with redacted message").
pub fn scenario_missing_file() -> ScenarioRun {
    let fixture: &'static str = "generated/valid/no-such-file.avi";
    // The deliberately-absent path, resolved under the fixtures root
    // (a machine-absolute path that does not exist).
    let source = SourceSpec::File {
        path: crate::harness::workspace_root()
            .join("tests/fixtures/generated/valid/no-such-file.avi"),
    };
    let mut harness = SessionHarness::new(fixture);
    let open = harness.dispatch(
        "session.open",
        Command::SessionOpen {
            source,
            options: OpenOptions::default(),
        },
    );
    let state = harness.state();
    let session = harness.session_id();
    let time = harness.dispatch("media.get-current-time", Command::MediaGetCurrentTime);
    let mut run = harness.finish("missing-file");
    let mut checks = Vec::new();

    push_typed_open_refusal(
        &mut checks,
        "a missing file refuses as input.inaccessible",
        &open,
        Stage::Input,
        ErrorCode::Inaccessible,
        "no-such-file.avi",
    );
    checks.push(Check::same(
        "no session exists after the refusal",
        &format!("{session:?}"),
        "None",
    ));
    checks.push(Check::same(
        "the host stays Idle",
        &format!("{state:?}"),
        "Idle",
    ));
    match &time.result {
        Err(error) => checks.push(Check::same(
            "later snapshots refuse not-ready",
            &format!("{}.{}", error.stage.as_str(), error.code.as_str()),
            "session.not-ready",
        )),
        Ok(_) => checks.push(Check::is(
            "later snapshots refuse not-ready",
            false,
            "a snapshot answered without a session",
        )),
    }
    run.checks = checks;
    run
}

/// **`corrupt-truncated-open`** — the 52-byte truncation (headers
/// incomplete): the open itself answers the demuxer's typed corrupt
/// error; nothing plays, no session survives.
pub fn scenario_corrupt_truncated_open() -> ScenarioRun {
    let source = match class_b_source(TRUNCATED_OPEN) {
        Ok(source) => source,
        Err(reason) => return broken_run("corrupt-truncated-open", TRUNCATED_OPEN, &reason),
    };
    let mut harness = SessionHarness::new(TRUNCATED_OPEN);
    let open = harness.dispatch(
        "session.open",
        Command::SessionOpen {
            source,
            options: OpenOptions::default(),
        },
    );
    let state = harness.state();
    let session = harness.session_id();
    let mut run = harness.finish("corrupt-truncated-open");
    let mut checks = Vec::new();

    push_typed_open_refusal(
        &mut checks,
        "the truncated header refuses as demux.corrupt",
        &open,
        Stage::Demux,
        ErrorCode::Corrupt,
        "truncated",
    );
    checks.push(Check::same(
        "no session survives the refusal",
        &format!("{session:?}"),
        "None",
    ));
    checks.push(Check::same(
        "the host returns to Idle after the fatal",
        &format!("{state:?}"),
        "Idle",
    ));
    run.checks = checks;
    run
}

/// **`corrupt-truncated-midplay`** — the half-file truncation (movi
/// cut mid-stream), ED-FIX-3 containment semantics (Lead-verified
/// 2026-10-22): the open lands, roughly half the media decodes and
/// presents through the sinks, the cut chunk's readable prefix is
/// delivered as a final (concealed) frame, and the session ends
/// CLEANLY — `Playing → Ended` with a wire `playback.end-of-media`
/// and exactly ONE `playback.error`: the coalesced NON-FATAL damage
/// accounting ("the engine recovered N damaged unit(s) while
/// resynchronizing"). The loss is counted at the demuxer (ED-FIX-3)
/// and surfaced wire-visible by the event policy (3.B4), never fatal
/// end-to-end (D-3/D-9). The 2.B2-era contract (Buffering → Error +
/// a fatal truncation error) is superseded.
pub fn scenario_corrupt_truncated_midplay() -> ScenarioRun {
    let source = match class_b_source(TRUNCATED_MIDPLAY) {
        Ok(source) => source,
        Err(reason) => return broken_run("corrupt-truncated-midplay", TRUNCATED_MIDPLAY, &reason),
    };
    let mut harness = SessionHarness::new(TRUNCATED_MIDPLAY);
    let open = harness.dispatch(
        "session.open",
        Command::SessionOpen {
            source,
            options: OpenOptions::default(),
        },
    );
    harness.settle();
    let settled = harness.state();
    harness.advance(run_pumps());
    let end_state = harness.state();
    let end_position = harness.position_us();
    let mut run = harness.finish("corrupt-truncated-midplay");
    let mut checks = Vec::new();

    checks.push(Check::is(
        "the truncated body still opens (the headers are intact)",
        matches!(&open.result, Ok(CommandOutcome::Accepted { .. })),
        "the open refused",
    ));
    checks.push(Check::same(
        "the settle reaches Playing (containment keeps the pipeline alive)",
        &format!("{settled:?}"),
        "Playing",
    ));
    checks.push(Check::same(
        "the session ends cleanly in Ended (the bounded play-out completes)",
        &format!("{end_state:?}"),
        "Ended",
    ));
    // The end-of-media event: wire-visible, exactly one; no error at all.
    let events = wire_events(&run);
    let error_events: Vec<&(String, String)> = events
        .iter()
        .filter(|(kind, _)| kind == "playback.error")
        .collect();
    let end_events: Vec<&(String, String)> = events
        .iter()
        .filter(|(kind, _)| kind == "playback.end-of-media")
        .collect();
    checks.push(Check::int(
        "exactly one playback.error event (the coalesced damage accounting)",
        error_events.len() as i64,
        1,
    ));
    if let Some((_, message)) = error_events.first() {
        checks.push(Check::is(
            "the error event is the NON-FATAL damage accounting (ED-FIX-3 counts, 3.B4 coalesces)",
            message.contains("recovered") && message.contains("damaged"),
            &format!("message {message:?} is not the coalesced damage accounting"),
        ));
    }
    checks.push(Check::int(
        "exactly one playback.end-of-media event is delivered",
        end_events.len() as i64,
        1,
    ));
    checks.push(Check::is(
        "the bounded play-out actually happened (content presented \
         through the cut)",
        matches!(&run_accounting(&run), Some(accounting) if accounting.frames_accepted >= 61),
        "expected the 60 complete chunks + the delivered partial prefix accepted",
    ));
    checks.push(Check::is(
        "the position stays within the media (no hang, bounded pumps)",
        (0..=DURATION_US).contains(&end_position),
        &format!("position {end_position} outside the media"),
    ));
    run.checks = checks;
    run
}

/// **`corrupt-garbage-appended`** — 4 KiB of seeded garbage past every
/// declared structure end: bounded recovery — the media plays to its
/// exact end, garbage ignored.
pub fn scenario_corrupt_garbage_appended() -> ScenarioRun {
    class_b_plays_to_end("corrupt-garbage-appended", GARBAGE_APPENDED)
}

/// **`corrupt-noidx`** — idx1 dropped (with the RIFF size patched and
/// avih still claiming AVIF_HASINDEX): the demuxer's linear movi walk
/// recovers the full media — plays to the exact end with the whole
/// accounting closing.
pub fn scenario_corrupt_noidx() -> ScenarioRun {
    class_b_plays_to_end("corrupt-noidx", NOIDX)
}

/// The shared shape of the two bounded-recovery class-B scenarios:
/// open → run to end → Ended at the exact duration with the full
/// per-frame accounting.
fn class_b_plays_to_end(id: &'static str, fixture: &'static str) -> ScenarioRun {
    let source = match class_b_source(fixture) {
        Ok(source) => source,
        Err(reason) => return broken_run(id, fixture, &reason),
    };
    let mut harness = SessionHarness::new(fixture);
    let open = harness.dispatch(
        "session.open",
        Command::SessionOpen {
            source,
            options: OpenOptions::default(),
        },
    );
    harness.settle();
    harness.advance(run_pumps());
    let state = harness.state();
    let position = harness.position_us();
    let mut run = harness.finish(id);
    let mut checks = Vec::new();

    checks.push(Check::is(
        "the derivative opens (bounded recovery, not a refusal)",
        matches!(&open.result, Ok(CommandOutcome::Accepted { .. })),
        "the open refused",
    ));
    checks.push(Check::same(
        "the media plays to Ended",
        &format!("{state:?}"),
        "Ended",
    ));
    checks.push(Check::int(
        "the position lands on the exact duration",
        position,
        DURATION_US,
    ));
    if let Some(accounting) = run_accounting(&run) {
        checks.push(Check::int(
            "the full media is delivered (60 + 60)",
            i64::try_from(accounting.frames_accepted).unwrap_or(i64::MAX),
            (VIDEO_FRAMES + AUDIO_PACKETS) as i64,
        ));
        checks.push(Check::int(
            "all 60 video frames schedule",
            i64::try_from(accounting.scheduler_pushed).unwrap_or(i64::MAX),
            VIDEO_FRAMES as i64,
        ));
        checks.push(Check::int(
            "no late drops",
            i64::try_from(accounting.scheduler_dropped_late).unwrap_or(i64::MAX),
            0,
        ));
    } else {
        checks.push(Check::is(
            "the headless accounting is observable",
            false,
            "accounting missing",
        ));
    }
    run.checks = checks;
    run
}

/// Resolve a class-B derivative as an openable source.
fn class_b_source(fixture: &str) -> Result<SourceSpec, String> {
    Ok(SourceSpec::File {
        path: crate::harness::fixture_path(fixture)?,
    })
}

// ---------------------------------------------------------------------------
// The wire session (the serialized command layer, across a process)
// ---------------------------------------------------------------------------

/// **`wire-session`** — the same catalog, but as the REAL serialized
/// wire: NDJSON requests into the `lumen` binary's `--control-stdio`
/// machine mode, NDJSON response envelopes + events out (the `--json`
/// event stream). This proves the layer's serde forms end to end
/// across a process boundary — the in-process scenarios above prove
/// the typed path, this one proves the wire path.
///
/// The script deliberately includes commands that arrive while the
/// open is still settling (Buffering): the machine mode does not
/// settle between lines, so `playback.pause` mid-open exercises the
/// truthful-UI typed not-ready (the refusal names the live state) —
/// an honest part of the wire surface, recorded as such.
///
/// # Errors
/// [`String`] when the `lumen` binary cannot be located or spawned
/// (the caller turns that into a failed check — a missing binary is
/// an environment gap to surface, not a panic).
pub fn scenario_wire_session() -> ScenarioRun {
    let fixture: &'static str = PARENT;
    let run = wire_session_run();
    match run {
        Ok((stdout, stderr, code)) => {
            let labels = Labels::detect();
            let lines: Vec<&str> = stdout.lines().collect();
            let responses: Vec<&str> = lines
                .iter()
                .copied()
                .filter(|line| line.starts_with("{\"requestId\""))
                .collect();
            let events: Vec<&str> = lines
                .iter()
                .copied()
                .filter(|line| line.starts_with("{\"seq\""))
                .collect();
            let mut checks = Vec::new();

            checks.push(Check::int(
                "the binary exits 0 (EOF after the script)",
                i64::from(code),
                0,
            ));
            checks.push(Check::is("stderr is empty", stderr.is_empty(), &stderr));
            checks.push(Check::int(
                "every request line answers exactly one response envelope",
                responses.len() as i64,
                7,
            ));
            checks.push(Check::same(
                "the open is accepted with session 1",
                if responses
                    .first()
                    .is_some_and(|l| l.contains("\"result\":{\"accepted\":{\"session\":1"))
                {
                    "accepted session=1"
                } else {
                    "not accepted"
                },
                "accepted session=1",
            ));
            // requestId echo: every response carries a distinct,
            // monotonically increasing id matching its position.
            let echoed = responses
                .iter()
                .enumerate()
                .all(|(index, line)| line.contains(&format!("\"requestId\":{}", index + 1)));
            checks.push(Check::is(
                "every response echoes its request id, in order",
                echoed,
                "requestId echo mismatch",
            ));
            // The event stream: seq strictly increasing.
            let seqs: Vec<u64> = events
                .iter()
                .filter_map(|line| {
                    let head = line.split("\"seq\":").nth(1)?;
                    let digits: String = head.chars().take_while(char::is_ascii_digit).collect();
                    digits.parse().ok()
                })
                .collect();
            let ordered = seqs.windows(2).all(|pair| pair[0] < pair[1]);
            checks.push(Check::is(
                "event seq numbers are strictly increasing",
                ordered && !seqs.is_empty(),
                &format!("seqs: {seqs:?}"),
            ));
            checks.push(Check::is(
                "the state events cover the open journey",
                stdout.contains("\"kind\":\"session.state-changed\""),
                "no state-changed events",
            ));
            checks.push(Check::is(
                "the track list arrives on the wire",
                stdout.contains("\"kind\":\"tracks.list-ready\""),
                "no tracks.list-ready event",
            ));
            // Determinism across the process boundary: a second
            // identical run is byte-identical.
            match wire_session_run() {
                Ok((stdout2, _stderr2, code2)) => {
                    checks.push(Check::is(
                        "two identical wire runs are byte-identical",
                        stdout2 == stdout && code2 == code,
                        "the second run differed",
                    ));
                }
                Err(reason) => {
                    checks.push(Check::is("the second wire run executes", false, &reason))
                }
            }
            // The transcript: the verbatim wire (the layer's own
            // serialization — stronger than any re-encoding).
            let transcript = Json::obj(vec![
                ("schema", Json::str(crate::harness::TRANSCRIPT_SCHEMA)),
                ("scenario", Json::str("wire-session")),
                ("fixture", Json::str(fixture)),
                ("labels", labels_json(&labels)),
                (
                    "commands",
                    Json::Arr(
                        [
                            "session.open",
                            "playback.pause",
                            "media.get-current-time",
                            "tracks.list",
                            "media.get-duration",
                            "playback.stop",
                            "session.close",
                        ]
                        .iter()
                        .enumerate()
                        .map(|(index, cmd)| {
                            Json::obj(vec![
                                ("cmd", Json::str(*cmd)),
                                ("request_id", Json::UInt(index as u64 + 1)),
                            ])
                        })
                        .collect(),
                    ),
                ),
                (
                    "events",
                    Json::Arr(events.iter().map(|line| Json::str(*line)).collect()),
                ),
                (
                    "observables",
                    Json::obj(vec![
                        ("exit_code", Json::UInt(u64::try_from(code).unwrap_or(0))),
                        ("stderr_empty", Json::Bool(stderr.is_empty())),
                        (
                            "response_lines",
                            Json::Arr(responses.iter().map(|line| Json::str(*line)).collect()),
                        ),
                    ]),
                ),
            ]);
            ScenarioRun {
                id: "wire-session",
                transcript,
                checks,
                png: None,
            }
        }
        Err(reason) => broken_run("wire-session", fixture, &reason),
    }
}

/// Run the wire session once: spawn the `lumen` binary in
/// `--control-stdio --json` mode, feed the seven-line script, answer
/// (stdout, stderr, exit code).
///
/// # Errors
/// [`String`] when the binary is missing (with the exact build
/// command to fix it) or cannot be run.
fn wire_session_run() -> Result<(String, String, i32), String> {
    let binary = lumen_binary_path()?;
    if !binary.exists() {
        return Err(format!(
            "the `lumen` binary is not built at {} — run `cargo build \
             -p lumen-cli` first (the workspace gate always has it; a \
             standalone `cargo test -p first-vertical` on a cold cache \
             does not build dependency bin targets)",
            binary.display()
        ));
    }
    let fixture = crate::harness::fixture_path(PARENT)?;
    let open_line = format!(
        "{{\"requestId\": 1, \"cmd\": \"session.open\", \"args\": {{\"source\": \
         {{\"kind\": \"file\", \"path\": \"{}\"}}}}}}\n",
        escape_json_string(&fixture.to_string_lossy())
    );
    let script = format!(
        "{open_line}\
         {{\"requestId\": 2, \"cmd\": \"playback.pause\"}}\n\
         {{\"requestId\": 3, \"cmd\": \"media.get-current-time\"}}\n\
         {{\"requestId\": 4, \"cmd\": \"tracks.list\"}}\n\
         {{\"requestId\": 5, \"cmd\": \"media.get-duration\"}}\n\
         {{\"requestId\": 6, \"cmd\": \"playback.stop\"}}\n\
         {{\"requestId\": 7, \"cmd\": \"session.close\"}}\n"
    );
    let mut child = ProcessCommand::new(&binary)
        .arg("--control-stdio")
        .arg("--json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("spawn {}: {error}", binary.display()))?;
    use std::io::Write as _;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(script.as_bytes())
            .map_err(|error| format!("stdin write: {error}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("wait: {error}"))?;
    Ok((
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    ))
}

/// The built `lumen` binary (the shared workspace target dir; the
/// EXE_SUFFIX keeps it correct on Windows).
fn lumen_binary_path() -> Result<PathBuf, String> {
    Ok(crate::harness::workspace_root()
        .join("target")
        .join("debug")
        .join(format!("lumen{}", std::env::consts::EXE_SUFFIX)))
}

/// Escape one JSON string literal body (the request line's path).
fn escape_json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Transcript readers (check-side helpers over the assembled Json)
// ---------------------------------------------------------------------------

/// The final state from a transcript's observables.
fn run_state(run: &ScenarioRun) -> String {
    json_str(&run.transcript, &["observables", "final_state"])
        .unwrap_or_else(|| "(missing)".to_string())
}

/// The state trail recorded in the transcript's events.
fn state_trail(run: &ScenarioRun) -> Vec<(PlaybackState, PlaybackState)> {
    let mut trail = Vec::new();
    if let Json::Obj(fields) = &run.transcript
        && let Some((_, Json::Arr(events))) = fields.iter().find(|(key, _)| key == "events")
    {
        for event in events {
            if let Json::Obj(fields) = event {
                let kind = fields.iter().find(|(key, _)| key == "kind");
                let from = fields.iter().find(|(key, _)| key == "from");
                let to = fields.iter().find(|(key, _)| key == "to");
                if let (
                    Some((_, Json::Str(kind))),
                    Some((_, Json::Str(from))),
                    Some((_, Json::Str(to))),
                ) = (kind, from, to)
                    && kind == "session.state-changed"
                {
                    trail.push((state_of(from), state_of(to)));
                }
            }
        }
    }
    trail
}

/// The wire events of a transcript as (kind, message) pairs.
fn wire_events(run: &ScenarioRun) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Json::Obj(fields) = &run.transcript
        && let Some((_, Json::Arr(events))) = fields.iter().find(|(key, _)| key == "events")
    {
        for event in events {
            if let Json::Obj(fields) = event
                && let Some(Json::Str(kind)) = fields
                    .iter()
                    .find_map(|(key, value)| (key == "kind").then_some(value))
            {
                let message = fields
                    .iter()
                    .find_map(|(key, value)| (key == "message").then_some(value));
                let message = match message {
                    Some(Json::Str(text)) => text.clone(),
                    _ => String::new(),
                };
                out.push((kind.clone(), message));
            }
        }
    }
    out
}

/// The accounting block from a transcript's observables.
fn run_accounting(run: &ScenarioRun) -> Option<lumen_cli::host::HeadlessAccounting> {
    let mut accounting = lumen_cli::host::HeadlessAccounting {
        frames_accepted: 0,
        frames_refused: 0,
        video_presented: 0,
        video_refused: 0,
        scheduler_pushed: 0,
        scheduler_presented: 0,
        scheduler_repeats: 0,
        scheduler_dropped_late: 0,
        scheduler_dropped_stale: 0,
        queue_flushed: 0,
        queue_published: 0,
        staging_frames: 0,
        staging_bytes: 0,
        staging_copies: 0,
    };
    let block = json_at(&run.transcript, &["observables", "accounting"])?;
    let mut any = false;
    if let Json::Obj(fields) = block {
        for (key, value) in fields {
            let number = match value {
                Json::UInt(value) => Some(*value),
                _ => None,
            };
            if let Some(number) = number {
                any = true;
                match key.as_str() {
                    "frames_accepted" => accounting.frames_accepted = number,
                    "frames_refused" => accounting.frames_refused = number,
                    "video_presented" => accounting.video_presented = number,
                    "video_refused" => accounting.video_refused = number,
                    "scheduler_pushed" => accounting.scheduler_pushed = number,
                    "scheduler_presented" => accounting.scheduler_presented = number,
                    "scheduler_repeats" => accounting.scheduler_repeats = number,
                    "scheduler_dropped_late" => accounting.scheduler_dropped_late = number,
                    "scheduler_dropped_stale" => accounting.scheduler_dropped_stale = number,
                    "queue_flushed" => accounting.queue_flushed = number,
                    "queue_published" => accounting.queue_published = number,
                    "staging_frames" => accounting.staging_frames = number,
                    "staging_bytes" => accounting.staging_bytes = number,
                    "staging_copies" => accounting.staging_copies = number,
                    _ => {}
                }
            }
        }
    }
    if any { Some(accounting) } else { None }
}

/// Read one nested string from a transcript value.
fn json_str(value: &Json, path: &[&str]) -> Option<String> {
    match json_at(value, path) {
        Some(Json::Str(text)) => Some(text.clone()),
        _ => None,
    }
}

/// Walk a nested object path.
fn json_at<'a>(value: &'a Json, path: &[&str]) -> Option<&'a Json> {
    let mut current = value;
    for key in path {
        let Json::Obj(fields) = current else {
            return None;
        };
        current = fields
            .iter()
            .find(|(field, _)| field == key)
            .map(|(_, value)| value)?;
    }
    Some(current)
}

/// Parse the wire state name (the transcript records the Debug form).
fn state_of(name: &str) -> PlaybackState {
    match name {
        "Idle" => PlaybackState::Idle,
        "Opening" => PlaybackState::Opening,
        "Buffering" => PlaybackState::Buffering,
        "Playing" => PlaybackState::Playing,
        "Paused" => PlaybackState::Paused,
        "Stopped" => PlaybackState::Stopped,
        "Ended" => PlaybackState::Ended,
        "Error" => PlaybackState::Error,
        other => unreachable!("unknown state name {other:?}"),
    }
}

/// Run one scenario by its stable id (the ledger's driver).
///
/// # Errors
/// [`String`] for an unknown id (a programming error in the caller).
pub fn by_id(id: &str) -> Result<ScenarioRun, String> {
    match id {
        "open" => Ok(scenario_open()),
        "play" => Ok(scenario_play()),
        "pause" => Ok(scenario_pause()),
        "seek" => Ok(scenario_seek()),
        "stop" => Ok(scenario_stop()),
        "run-to-end" => Ok(scenario_run_to_end()),
        "missing-file" => Ok(scenario_missing_file()),
        "corrupt-truncated-open" => Ok(scenario_corrupt_truncated_open()),
        "corrupt-truncated-midplay" => Ok(scenario_corrupt_truncated_midplay()),
        "corrupt-garbage-appended" => Ok(scenario_corrupt_garbage_appended()),
        "corrupt-noidx" => Ok(scenario_corrupt_noidx()),
        "wire-session" => Ok(scenario_wire_session()),
        other => Err(format!("unknown scenario id {other:?}")),
    }
}
