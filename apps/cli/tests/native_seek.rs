//! The native-seek acceptance suite (C8 / ED-FIX-4, card W1-06
//! host-side): `playback.seek` runs the ENGINE's seven-step flow
//! (doc 03 §6) through [`SessionHandle::seek`] — in-pipeline, the
//! session epoch surviving — and the host performs the boundary
//! duties behind its own seams (ring-flush prefix, window-full hold
//! clear, presentation-queue publish).
//!
//! The pinned invariants (the directives-audit C8 fix):
//!
//! - **No reopen** (D-4/D-7): the session id and every session-scoped
//!   half (device, ring, clock, decoders, renderer) survive the seek;
//!   the state trail shows the in-pipeline transitions only — a
//!   `Stopped`/`Idle`/`Opening` after the seek would be the forbidden
//!   stop→close→reopen composition.
//! - **Typed failures surface** (D4): the state table's refusals
//!   answer the wire honestly (seek from `Ended`), and the flow's own
//!   `timing.seek-failed` terminal rides the event stream when a
//!   seek fails after accept.
//! - **Position reports the actual, not the requested** (doc 03 §6
//!   step 7): the demuxer's keyframe-safe landing is the truth; the
//!   clock re-anchors on the first audio frame's own timestamp.
//! - **The boundary counters move**: the pre-seek scheduler window
//!   purges as stale at the first post-seek tick and the presentation
//!   queue carries the seek's mint (the host-duty proofs).

use commands::catalog::types::{OpenOptions, SeekTarget, SourceSpec, Timestamp};
use commands::envelope::RequestId;
use commands::{Command, EventKind, PlaybackState, WireRequest};
use lumen_cli::dispatch::lumen_dispatch;
use lumen_cli::host::CliHost;
use testkit::Must;

/// The AVI parent fixture (class A): MJPEG+PCM, 2.000 s, 320x240
/// @30 fps, stereo s16le 44.1 kHz — every frame a keyframe, so the
/// landing granularity is the 33.333 ms frame grid.
fn parent() -> SourceSpec {
    SourceSpec::File {
        path: format!(
            "{}/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi",
            env!("CARGO_MANIFEST_DIR")
        )
        .into(),
    }
}

/// Dispatch one command, asserting the envelope's success (the
/// honest-failure tests match on the `Err` themselves).
fn dispatch(host: &mut CliHost, request_id: u64, command: Command) {
    let response = lumen_dispatch(
        host,
        &WireRequest {
            request_id: RequestId(request_id),
            command,
        },
    );
    match response.result {
        Ok(_) => {}
        Err(error) => unreachable!("command failed: {error}"),
    }
}

/// Open the parent fixture (settled) and answer the host.
fn opened() -> CliHost {
    let mut host = CliHost::new();
    dispatch(
        &mut host,
        1,
        Command::SessionOpen {
            source: parent(),
            options: OpenOptions::default(),
        },
    );
    host.settle();
    host
}

/// One `playback.seek` through the wire.
fn seek(host: &mut CliHost, request_id: u64, target: SeekTarget) {
    dispatch(
        host,
        request_id,
        Command::PlaybackSeek { target },
    );
}

/// The position payload of the first seek terminal in `host`'s event
/// stream (the landing truth — `timing.seek-completed`, doc 05 §3.2).
fn seek_completed_position(host: &CliHost) -> Option<Timestamp> {
    host.events().iter().find_map(|event| {
        match event.kind {
            EventKind::TimingSeekCompleted { position, .. } => Some(position),
            _ => None,
        }
    })
}

/// The teardown edges a reopen composition would have produced
/// (the stop→close half of the forbidden D-4/D-7 shape; its
/// `Idle → Opening` twin is pinned separately as the single open).
/// Answers how many appear in the trail — these tests never issue an
/// explicit `stop`, so the honest native-seek answer is zero.
fn teardown_edges(host: &CliHost) -> usize {
    host.state_history()
        .iter()
        .filter(|(from, to)| {
            matches!(
                (from, to),
                (PlaybackState::Playing, PlaybackState::Stopped)
                    | (PlaybackState::Paused, PlaybackState::Stopped)
                    | (PlaybackState::Stopped, PlaybackState::Idle)
            )
        })
        .count()
}

/// How many `Idle → Opening` transitions the trail holds — the
/// native seek answers ONE (the open's); a reopen composition
/// answers one per seek.
fn open_edges(host: &CliHost) -> usize {
    host.state_history()
        .iter()
        .filter(|(from, to)| matches!((from, to), (PlaybackState::Idle, PlaybackState::Opening)))
        .count()
}

/// THE C8 acceptance: a 50% seek from `Playing` is the engine's
/// native in-pipeline flow. The session epoch survives (same id, one
/// `Idle → Opening` in the whole trail — the open's), the generation
/// mints at accept, the terminal reports the actual landing (1.000 s
/// of the 2.000 s media — a frame-aligned target), the boundary
/// counters move (the pre-seek scheduler window purges as stale, the
/// presentation queue carries the mint), and the remainder still
/// plays out to the exact duration (the ring-prefix discard absorbed
/// into the landing anchor — an unabsorbed discard would overshoot).
#[test]
fn native_seek_keeps_the_epoch_and_closes_the_accounting() {
    let mut host = opened();
    let session_before = host.session_id();
    let generation_before = host.generation();
    seek(
        &mut host,
        2,
        SeekTarget::Percent { fraction: 0.5 },
    );

    // Class-A truth: the SAME session answers under a minted
    // generation (the reopen composition answered session=2).
    assert_eq!(host.session_id(), session_before, "the epoch survives");
    assert_eq!(
        host.generation(),
        generation_before.map(|generation| commands::envelope::Generation(
            generation.0 + 1
        )),
        "the seek mints at accept"
    );
    // The landing truth: the terminal's actual position, the clock at
    // the same, and the wire state back in `Playing` (the rebuffer
    // episode ran through the watermark machinery, in-pipeline).
    assert_eq!(
        seek_completed_position(&host),
        Some(Timestamp(1_000_000)),
        "the terminal reports the actual landing"
    );
    assert_eq!(host.position_us(), 1_000_000, "the clock re-anchored");
    assert_eq!(host.state(), PlaybackState::Playing);
    // The forbidden edges: the trail holds the open's transitions and
    // the seek's rebuffer only — no stop, no close, no reopen (one
    // open, zero teardowns).
    assert_eq!(teardown_edges(&host), 0, "{:?}", host.state_history());
    assert_eq!(open_edges(&host), 1, "{:?}", host.state_history());

    // The remainder plays out to the exact duration.
    host.advance(lumen_cli::host::RUN_PUMPS as u32);
    assert_eq!(host.state(), PlaybackState::Ended);
    assert_eq!(host.position_us(), 2_000_000, "the tail play-out closes");

    // The boundary counters (the host-duty proofs): the pre-seek
    // scheduler window — the 16-frame cap the settle pumps filled —
    // purged wholesale as stale at the first post-seek tick, and the
    // presentation queue's published generation carries the seek's
    // mint. Nothing was refused and nothing presented late.
    let accounting = host.headless_accounting().must();
    assert_eq!(accounting.scheduler_dropped_stale, 16, "{accounting:?}");
    assert_eq!(accounting.queue_published, 1, "{accounting:?}");
    assert_eq!(accounting.frames_refused, 0, "{accounting:?}");
    assert_eq!(accounting.video_refused, 0, "{accounting:?}");
    assert_eq!(accounting.scheduler_dropped_late, 0, "{accounting:?}");
    // The decode accounting proves the pipeline was not rebuilt: the
    // pre-seek decode-ahead (94 deliveries — the settle pumps decode
    // ahead of the clock by design) PLUS the landing's own 60 (30
    // video + 30 audio) — a reopen would have answered exactly 60.
    assert_eq!(accounting.frames_accepted, 154, "{accounting:?}");
    assert_eq!(accounting.scheduler_pushed, 46, "{accounting:?}");
}

/// Position truth is the ACTUAL, not the requested (doc 03 §6 step
/// 7): a 250 ms target on the 30 fps grid lands at the demuxer's
/// keyframe-safe chunk — 266.667 ms — and every observer (the
/// terminal, the clock) reports that landing, never the request.
#[test]
fn native_seek_reports_the_actual_not_the_requested() {
    let mut host = opened();
    seek(
        &mut host,
        2,
        SeekTarget::Absolute {
            time: Timestamp(250_000),
        },
    );
    assert_eq!(
        seek_completed_position(&host),
        Some(Timestamp(266_667)),
        "the terminal reports the demuxer's landing, not the request"
    );
    assert_eq!(
        host.position_us(),
        266_666,
        "the clock carries the same actual (ns → µs truncation)"
    );
    assert_eq!(teardown_edges(&host), 0, "{:?}", host.state_history());
    assert_eq!(open_edges(&host), 1, "{:?}", host.state_history());
    // The landing still plays out to the exact duration.
    host.advance(lumen_cli::host::RUN_PUMPS as u32);
    assert_eq!(host.state(), PlaybackState::Ended);
    assert_eq!(host.position_us(), 2_000_000);
}

/// A Paused seek runs the decode carve-out (seek.rs): the wire state
/// never leaves `paused`, the pre-roll still completes, the terminal
/// still reports, and a later play resumes from the landing.
#[test]
fn paused_seek_completes_and_resumes_from_the_landing() {
    let mut host = opened();
    dispatch(&mut host, 2, Command::PlaybackPause);
    assert_eq!(host.state(), PlaybackState::Paused);
    seek(
        &mut host,
        3,
        SeekTarget::Percent { fraction: 0.5 },
    );
    // The carve-out: still paused, but the landing truth arrived.
    assert_eq!(host.state(), PlaybackState::Paused);
    assert_eq!(
        seek_completed_position(&host),
        Some(Timestamp(1_000_000)),
        "the Paused seek still reports its terminal"
    );
    assert_eq!(host.position_us(), 1_000_000);
    assert_eq!(teardown_edges(&host), 0, "{:?}", host.state_history());
    assert_eq!(open_edges(&host), 1, "{:?}", host.state_history());
    // Resume: the remainder plays from the landing to the end.
    dispatch(&mut host, 4, Command::PlaybackPlay);
    host.advance(lumen_cli::host::RUN_PUMPS as u32);
    assert_eq!(host.state(), PlaybackState::Ended);
    assert_eq!(host.position_us(), 2_000_000);
}

/// A backward seek below `--start-time` lands and plays out: the
/// open-time skip watermark dies at the re-anchor (the landing is the
/// new position truth — a standing `--start-time 1s` filter would
/// starve the pre-roll of a 250 ms landing and the media would end
/// at the landing, honest-looking and wrong).
#[test]
fn backward_seek_below_start_time_lands_and_plays_out() {
    let mut host = CliHost::new();
    dispatch(
        &mut host,
        1,
        Command::SessionOpen {
            source: parent(),
            options: OpenOptions {
                start_time: Some(Timestamp(1_000_000)),
                ..OpenOptions::default()
            },
        },
    );
    host.settle();
    assert_eq!(host.position_us(), 1_000_000, "the open anchors at 1 s");
    seek(
        &mut host,
        2,
        SeekTarget::Absolute {
            time: Timestamp(250_000),
        },
    );
    assert_eq!(
        seek_completed_position(&host),
        Some(Timestamp(266_667)),
        "the backward landing is below the start time"
    );
    assert_eq!(host.position_us(), 266_666);
    host.advance(lumen_cli::host::RUN_PUMPS as u32);
    assert_eq!(host.state(), PlaybackState::Ended, "the pre-roll was fed");
    assert_eq!(host.position_us(), 2_000_000);
}

/// A typed failure surfaces honestly (D4): seek from `Ended` is the
/// state table's refusal — the wire answers the typed error, the
/// position stays at the old one, and the session survives for a
/// later close (nothing was torn down to refuse).
#[test]
fn seek_from_ended_refuses_typed_at_the_old_position() {
    let mut host = opened();
    host.advance(lumen_cli::host::RUN_PUMPS as u32);
    assert_eq!(host.state(), PlaybackState::Ended);
    let session_before = host.session_id();
    let response = lumen_dispatch(
        &mut host,
        &WireRequest {
            request_id: RequestId(2),
            command: Command::PlaybackSeek {
                target: SeekTarget::Percent { fraction: 0.5 },
            },
        },
    );
    match response.result {
        Err(ref error) => {
            assert_eq!(error.stage, commands::Stage::Session, "{error}");
            assert!(
                error.message.contains("seek in Ended"),
                "the refusal names the state: {error}"
            );
        }
        Ok(_) => unreachable!("seek from Ended must refuse typed"),
    }
    // D4's shape: nothing moved, nothing died.
    assert_eq!(host.position_us(), 2_000_000, "the old position stands");
    assert_eq!(host.session_id(), session_before, "the session survives");
    assert!(
        seek_completed_position(&host).is_none(),
        "no seek terminal was fabricated"
    );
    assert_eq!(teardown_edges(&host), 0, "{:?}", host.state_history());
    assert_eq!(open_edges(&host), 1, "{:?}", host.state_history());
}

/// The no-session refusal (the dispatch table's honest not-ready).
#[test]
fn seek_without_a_session_refuses_not_ready() {
    let mut host = CliHost::new();
    let response = lumen_dispatch(
        &mut host,
        &WireRequest {
            request_id: RequestId(1),
            command: Command::PlaybackSeek {
                target: SeekTarget::Percent { fraction: 0.5 },
            },
        },
    );
    match response.result {
        Err(ref error) => {
            assert_eq!(error.stage, commands::Stage::Session, "{error}");
            assert!(error.message.contains("no session open"), "{error}");
        }
        Ok(_) => unreachable!("seek without a session must refuse"),
    }
}
