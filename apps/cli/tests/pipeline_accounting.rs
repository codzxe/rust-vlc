//! The pipeline-accounting AC (lib-level): the whole first vertical —
//! access → probe → AVI demux → queues → MJPEG/PCM decode → audio
//! engine → null output → video scheduler → software renderer —
//! driven by the engine through the ONE command layer, with the
//! headless frame accounting asserted (the card's "engine-driven"
//! clause), plus the in-process determinism proof.

use commands::catalog::types::SourceSpec;
use commands::envelope::RequestId;
use commands::{Command, PlaybackState, WireRequest};
use lumen_cli::dispatch::lumen_dispatch;
use lumen_cli::host::CliHost;
use testkit::Must;

fn parent() -> SourceSpec {
    SourceSpec::File {
        path: format!(
            "{}/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi",
            env!("CARGO_MANIFEST_DIR")
        )
        .into(),
    }
}

fn dispatch_open(host: &mut CliHost, source: SourceSpec) {
    let response = lumen_dispatch(
        host,
        &WireRequest {
            request_id: RequestId(1),
            command: Command::SessionOpen {
                source,
                options: Default::default(),
            },
        },
    );
    match response.result {
        Ok(_) => {}
        Err(error) => unreachable!("open failed: {error}"),
    }
    host.settle();
}

/// One whole-media run, answered as the observable aggregate: the
/// state trail, the position, the accounting, and the final frame's
/// PNG (the headless evidence).
fn run_to_end() -> (
    Vec<(PlaybackState, PlaybackState)>,
    i64,
    lumen_cli::host::HeadlessAccounting,
    Vec<u8>,
) {
    let mut host = CliHost::new();
    dispatch_open(&mut host, parent());
    host.advance(lumen_cli::host::RUN_PUMPS as u32);
    let state = host.state();
    if state != PlaybackState::Ended {
        unreachable!("the run should have ended, state: {state:?}");
    }
    let png = match host.screenshot_png().must() {
        Ok(png) => png,
        Err(error) => unreachable!("screenshot failed: {error}"),
    };
    (
        host.state_history().to_vec(),
        host.position_us(),
        host.headless_accounting().must(),
        png,
    )
}

/// The accounting: every demuxed packet decoded and delivered (both
/// limbs), every video frame scheduled and staged exactly once, and
/// the position landing on the duration (the tail play-out).
#[test]
fn whole_media_accounting_closes() {
    let (history, position, accounting, png) = run_to_end();

    // The state trail: a clean open → play → end.
    assert!(
        history.contains(&(PlaybackState::Idle, PlaybackState::Opening)),
        "{history:?}"
    );
    assert!(
        history.contains(&(PlaybackState::Buffering, PlaybackState::Playing)),
        "{history:?}"
    );
    assert!(
        history.contains(&(PlaybackState::Playing, PlaybackState::Ended)),
        "{history:?}"
    );

    // 60 MJPEG frames + 60 PCM packets = 120 deliveries.
    assert_eq!(accounting.frames_accepted, 120, "{accounting:?}");
    assert_eq!(accounting.frames_refused, 0, "{accounting:?}");
    // The video limb: every frame admitted, presented, staged —
    // exactly once per frame (the fixture's 30 fps vs the 33.33 ms
    // render quantum walk in lockstep; no late drops).
    assert_eq!(accounting.scheduler_pushed, 60, "{accounting:?}");
    assert_eq!(accounting.scheduler_dropped_late, 0, "{accounting:?}");
    assert_eq!(accounting.video_refused, 0, "{accounting:?}");
    assert!(
        accounting.scheduler_presented + accounting.scheduler_repeats >= 60,
        "at least every frame once: {accounting:?}"
    );
    assert_eq!(
        accounting.staging_frames, accounting.video_presented,
        "{accounting:?}"
    );
    assert_eq!(
        accounting.staging_bytes,
        accounting.staging_frames * 320 * 240 * 4,
        "one full-frame RGBA copy per present: {accounting:?}"
    );
    assert_eq!(
        accounting.staging_copies, accounting.staging_frames,
        "{accounting:?}"
    );

    // The position lands on the 2.000 s duration exactly.
    assert_eq!(position, 2_000_000, "position: {position}");

    // The final frame is a real PNG (magic + IHDR + IEND) of the
    // media's own extent.
    assert!(
        png.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]),
        "png head: {:?}",
        &png[..16]
    );
    assert!(png.len() > 8 + 25 + 12, "{png:?}");
}

/// In-process determinism: two whole-media runs agree on the trail,
/// the position, the accounting AND the final frame's bytes.
#[test]
fn whole_media_runs_are_deterministic() {
    let first = run_to_end();
    let second = run_to_end();
    assert_eq!(first.0, second.0, "state trails");
    assert_eq!(first.1, second.1, "positions");
    assert_eq!(first.2, second.2, "accounting");
    assert_eq!(first.3, second.3, "final frames (PNG bytes)");
}

/// The native seek at the accounting level (C8/ED-FIX-4): a 50%
/// in-pipeline seek keeps the session epoch and still ends exactly
/// at the duration. The counters prove the pipeline was NOT rebuilt:
/// the pre-seek decode-ahead deliveries stay counted (a reopen would
/// have answered exactly the landing's 60), the pre-seek scheduler
/// window purges wholesale as stale at the first post-seek tick, and
/// the presentation queue carries the seek's mint.
#[test]
fn native_seek_accounting_closes() {
    let mut host = CliHost::new();
    dispatch_open(&mut host, parent());
    let session_before = host.session_id();
    let response = lumen_dispatch(
        &mut host,
        &WireRequest {
            request_id: RequestId(2),
            command: Command::PlaybackSeek {
                target: commands::catalog::types::SeekTarget::Percent { fraction: 0.5 },
            },
        },
    );
    match response.result {
        Ok(_) => {}
        Err(error) => unreachable!("seek failed: {error}"),
    }
    host.advance(lumen_cli::host::RUN_PUMPS as u32);
    assert_eq!(host.state(), PlaybackState::Ended);
    assert_eq!(
        host.position_us(),
        2_000_000,
        "position: {}",
        host.position_us()
    );
    // The epoch survived the seek (the reopen composition minted a
    // second session here).
    assert_eq!(host.session_id(), session_before);
    let accounting = host.headless_accounting().must();
    // The decode accounting: the settle pumps' decode-ahead (94
    // deliveries — the decode stage runs ahead of the clock by
    // design) PLUS the landing's own half (30 video + 30 audio). A
    // stop→close→reopen would have answered exactly 60.
    assert_eq!(accounting.frames_accepted, 154, "{accounting:?}");
    // The scheduler saw the pre-seek window (16 — the window cap)
    // plus the landing's 30.
    assert_eq!(accounting.scheduler_pushed, 46, "{accounting:?}");
    // The seek-boundary proofs: the pre-seek window purged as stale
    // at the generation change, the presentation queue's published
    // generation carrying the seek's mint.
    assert_eq!(accounting.scheduler_dropped_stale, 16, "{accounting:?}");
    assert_eq!(accounting.queue_published, 1, "{accounting:?}");
    // Nothing refused, nothing late: the landing's frames all
    // scheduled and staged cleanly.
    assert_eq!(accounting.frames_refused, 0, "{accounting:?}");
    assert_eq!(accounting.video_refused, 0, "{accounting:?}");
    assert_eq!(accounting.scheduler_dropped_late, 0, "{accounting:?}");
}
