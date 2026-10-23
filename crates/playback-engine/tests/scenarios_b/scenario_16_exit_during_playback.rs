//! Scenario 16 — **exit during playback** (doc 10 §6.4 row 16),
//! driven through the command layer.
//!
//! Pass criteria: teardown within deadline; device closed; exit code
//! clean; teardown report.
//!
//! The v0 mapping (the honest in-process shape of "drop session
//! mid-decode"): the abrupt exit is **dropping the whole host** —
//! engine, session handle, wired stage objects — mid-playback, with
//! no `playback.stop`, no `session.close`, nothing orderly at all.
//! In the real product the process exits and the OS reclaims; here
//! the reclaim is Rust's drop glue, and the oracles measure exactly
//! what the teardown report names:
//!
//! - **resources released (drop-hook counters)** — every pipeline
//!   stage object the engine owned is observed to drop
//!   ([`host::DropHooks`]: the demux stage and the consumer seam —
//!   the null-device limb's "device closed"). The engine is
//!   poll-driven and threadless, so the object graph's release IS
//!   the whole teardown.
//! - **the file handle** — the open `FileAccess` fd count returns to
//!   baseline immediately after the drop (the "device closed" row
//!   for the file limb).
//! - **no temp state written** — the scratch dir that holds the
//!   media copy still contains exactly that one file afterwards: no
//!   sidecars, no journals, no caches (the engine never writes
//!   anything anywhere; the assertion pins it).
//! - **teardown within deadline** — the drop's wall time is measured
//!   and asserted against doc 03 §4.6 S2's proposed 2 s teardown
//!   deadline (measured value recorded in the report; the bound is
//!   generous on purpose — the point is *bounded*, not fast).
//! - **exit code clean** — the in-process arm of this criterion is
//!   the test binary itself exiting 0; the process-level arm (real
//!   CLI exit mid-playback) is the first-vertical suite's evidence
//!   (W5-07 scenario 5's stop/teardown and the CLI's own suites).
//!
//! NOT tested (named, not hidden): dropping while the session is
//! still in `Opening`/`Buffering` — the open path is synchronous in
//! this host (the `session.open` response lands with the state
//! already settled or failed), so there is no deterministic instant
//! to catch the transient; the threaded open (2.E1/3.B3) will own
//! that arm.

use std::time::{Duration, Instant};

use commands::catalog::types::{OpenOptions, SourceSpec};
use commands::envelope::CommandOutcome;
use commands::{Command, PlaybackState};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

use crate::host::{ScenarioHost, TempDir, dispatch, open_fds, thread_count};

/// Doc 03 §4.6 S2's proposed teardown deadline.
const TEARDOWN_DEADLINE: Duration = Duration::from_secs(2);

fn open_file(host: &mut ScenarioHost, path: std::path::PathBuf, name: &str) {
    let open = dispatch(
        host,
        Command::SessionOpen {
            source: SourceSpec::File { path },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&open.result, Ok(CommandOutcome::Accepted { .. })),
        "{name}: opens: {:?}",
        open.result
    );
    host.settle();
    assert_eq!(host.state(), PlaybackState::Playing, "{name}: playing");
}

/// The abrupt drop, mid-playback of the real parent fixture (a temp
/// copy, so the no-temp-state oracle has a dir to watch): every
/// resource is released, nothing is written, and the teardown is
/// bounded.
#[test]
fn abrupt_drop_mid_playback_releases_everything_within_deadline() {
    let _serial = serialize_tests();
    let scratch = TempDir::new("s16");
    let media = scratch.copy_parent("exit-mid-play.avi");

    let fds_before = open_fds();
    let threads_before = thread_count();

    let mut host = ScenarioHost::new();
    let hooks = host.arm_drop_hooks();
    open_file(&mut host, media, "the parent");

    // Play into the media: well past the first frames, far from the
    // end — the pipeline is loaded (queues resident, decoders hot)
    // when the exit hits.
    assert!(
        host.pump_pipeline_until_within(256, |host| {
            host.sink().map(|sink| sink.frames_accepted()).must() >= 30
        }),
        "the session plays into the media before the exit"
    );
    let frames = host.sink().map(|sink| sink.frames_accepted()).must();
    assert!(
        (30..120).contains(&frames),
        "mid-playback, not at the start or the end ({frames} frames)"
    );
    // The open session holds exactly its one file handle.
    assert_eq!(
        open_fds(),
        Some(fds_before.must() + 1),
        "the playing session holds one fd"
    );
    let entries_before = scratch.entries();

    // The abrupt exit: no stop, no close — just drop everything.
    let started = Instant::now();
    drop(host);
    let teardown = started.elapsed();

    // Drop-hook counters: the engine released the demux stage and
    // the consumer seam (the device).
    assert_eq!(
        hooks
            .demuxer_drops
            .load(std::sync::atomic::Ordering::Relaxed),
        1,
        "the demux stage was released by the abrupt teardown"
    );
    assert_eq!(
        hooks.sink_drops.load(std::sync::atomic::Ordering::Relaxed),
        1,
        "the consumer seam (the device) was released"
    );
    // The file handle is closed.
    assert_eq!(
        open_fds(),
        fds_before,
        "the file handle is closed immediately after the drop"
    );
    // No threads leaked.
    assert_eq!(thread_count(), threads_before, "no threads leaked");
    // No temp state: the scratch dir still holds exactly the media
    // copy — nothing was written beside it.
    let entries_after = scratch.entries();
    assert_eq!(
        entries_after, entries_before,
        "the teardown wrote nothing ({entries_after:?})"
    );
    assert_eq!(
        entries_after,
        vec!["exit-mid-play.avi".to_string()],
        "the scratch dir holds exactly the media copy"
    );
    // Bounded teardown (doc 03 §4.6 S2).
    assert!(
        teardown < TEARDOWN_DEADLINE,
        "the abrupt teardown is bounded ({teardown:?} vs {TEARDOWN_DEADLINE:?})"
    );
}

/// The same abrupt drop on the scripted long timeline — the
/// mid-decode exit with a synthesized (non-file) source: the stage
/// objects are released just the same, and with no file in play the
/// fd baseline never moves.
#[test]
fn abrupt_drop_mid_timeline_releases_stage_objects() {
    let _serial = serialize_tests();
    let scratch = TempDir::new("s16-timeline");

    let fds_before = open_fds();
    let mut host = ScenarioHost::new();
    let hooks = host.arm_drop_hooks();
    let open = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::Url {
                url: "timeline://14400x1000000".to_string(),
            },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&open.result, Ok(CommandOutcome::Accepted { .. })),
        "the long timeline opens: {:?}",
        open.result
    );
    host.settle();
    assert_eq!(host.state(), PlaybackState::Playing, "the timeline plays");
    assert!(
        host.pump_pipeline_until_within(256, |host| {
            host.sink().map(|sink| sink.audio_accepted()).must() >= 100
        }),
        "the timeline decodes into the media before the exit"
    );

    let started = Instant::now();
    drop(host);
    let teardown = started.elapsed();

    assert_eq!(
        hooks
            .demuxer_drops
            .load(std::sync::atomic::Ordering::Relaxed),
        1,
        "the timeline's demux stage was released"
    );
    assert_eq!(
        hooks.sink_drops.load(std::sync::atomic::Ordering::Relaxed),
        1,
        "the timeline's consumer seam was released"
    );
    assert_eq!(
        open_fds(),
        fds_before,
        "a memory source never touched the fd baseline"
    );
    assert!(
        scratch.entries().is_empty(),
        "nothing was written anywhere ({:?})",
        scratch.entries()
    );
    assert!(
        teardown < TEARDOWN_DEADLINE,
        "the abrupt teardown is bounded ({teardown:?})"
    );
}
